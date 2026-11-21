use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const HISTORY_LIMIT_PER_PROFILE: usize = 3000;

#[derive(Clone)]
pub struct BrowserSource {
    browser_name: String,
    browser_exe: Option<String>,
    profile: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserEntry {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub url: String,
    pub browser_name: String,
    pub browser_exe: Option<String>,
    pub last_visited: Option<i64>,
    pub tab_id: Option<i64>,
    pub window_id: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeSnapshot {
    tabs: Vec<BridgeTab>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeTab {
    id: i64,
    window_id: i64,
    title: String,
    url: String,
    browser_name: String,
    browser_exe: Option<String>,
}

fn is_web_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

fn collect_bookmark_urls(value: &Value, source: &BrowserSource, entries: &mut Vec<BrowserEntry>) {
    let Some(node) = value.as_object() else {
        return;
    };

    if node.get("type").and_then(Value::as_str) == Some("url") {
        let url = node.get("url").and_then(Value::as_str).unwrap_or_default();
        if is_web_url(url) {
            let title = node
                .get("name")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(url);
            let last_visited = node
                .get("date_added")
                .and_then(Value::as_str)
                .and_then(|value| value.parse::<i64>().ok());
            entries.push(BrowserEntry {
                id: format!(
                    "bookmark:{}:{}:{}",
                    source.browser_name, source.profile, url
                ),
                kind: "bookmark".to_string(),
                title: title.to_string(),
                url: url.to_string(),
                browser_name: source.browser_name.clone(),
                browser_exe: source.browser_exe.clone(),
                last_visited,
                tab_id: None,
                window_id: None,
            });
        }
        return;
    }

    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            collect_bookmark_urls(child, source, entries);
        }
    }
}

pub fn parse_bookmarks_json(raw: &str, source: &BrowserSource) -> Vec<BrowserEntry> {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    if let Some(roots) = value.get("roots") {
        if let Some(roots) = roots.as_object() {
            for root in roots.values() {
                collect_bookmark_urls(root, source, &mut entries);
            }
        }
    }
    entries
}

fn open_history(path: &Path) -> Result<Connection, String> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let connection = Connection::open_with_flags(path, flags).map_err(|error| error.to_string())?;
    connection
        .busy_timeout(std::time::Duration::from_millis(500))
        .map_err(|error| error.to_string())?;
    Ok(connection)
}

fn snapshot_history(path: &Path) -> Result<PathBuf, String> {
    let directory = std::env::temp_dir().join(format!(
        "nexora-history-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let target = directory.join("History");
    std::fs::copy(path, &target).map_err(|error| error.to_string())?;
    for suffix in ["-wal", "-shm"] {
        let companion = PathBuf::from(format!("{}{}", path.to_string_lossy(), suffix));
        if companion.exists() {
            let target_companion = PathBuf::from(format!("{}{}", target.to_string_lossy(), suffix));
            std::fs::copy(companion, target_companion).ok();
        }
    }
    Ok(target)
}

pub fn read_history_database(
    path: &Path,
    source: &BrowserSource,
    limit: usize,
) -> Vec<BrowserEntry> {
    let direct =
        open_history(path).and_then(|connection| query_history(&connection, source, limit));
    if let Ok(entries) = direct {
        return entries;
    }

    let Ok(snapshot) = snapshot_history(path) else {
        return Vec::new();
    };
    let entries = open_history(&snapshot)
        .and_then(|connection| query_history(&connection, source, limit))
        .unwrap_or_default();
    if let Some(directory) = snapshot.parent() {
        std::fs::remove_dir_all(directory).ok();
    }
    entries
}

fn query_history(
    connection: &Connection,
    source: &BrowserSource,
    limit: usize,
) -> Result<Vec<BrowserEntry>, String> {
    let mut statement = connection
        .prepare(
            "SELECT url, title, last_visit_time
             FROM urls
             WHERE hidden = 0 AND (url LIKE 'http://%' OR url LIKE 'https://%')
             ORDER BY last_visit_time DESC
             LIMIT ?1",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([limit as i64], |row| {
            let url: String = row.get(0)?;
            let title: String = row.get(1)?;
            let last_visited: i64 = row.get(2)?;
            Ok((url, title, last_visited))
        })
        .map_err(|error| error.to_string())?;

    let mut entries = Vec::new();
    for row in rows.flatten() {
        let (url, title, last_visited) = row;
        entries.push(BrowserEntry {
            id: format!("history:{}:{}:{}", source.browser_name, source.profile, url),
            kind: "history".to_string(),
            title: if title.trim().is_empty() {
                url.clone()
            } else {
                title
            },
            url,
            browser_name: source.browser_name.clone(),
            browser_exe: source.browser_exe.clone(),
            last_visited: Some(last_visited),
            tab_id: None,
            window_id: None,
        });
    }
    Ok(entries)
}

fn chromium_roots() -> Vec<(&'static str, Option<String>, PathBuf)> {
    let local = dirs::data_local_dir();
    let mut roots = Vec::new();
    let installed = crate::commands::browsers::browsers();
    let browser_exe = |name: &str, fallback: &str| {
        installed
            .iter()
            .find(|browser| browser.name.eq_ignore_ascii_case(name))
            .map(|browser| browser.exe.clone())
            .or_else(|| Path::new(fallback).exists().then(|| fallback.to_string()))
    };
    if let Some(local) = local {
        roots.push((
            "Microsoft Edge",
            browser_exe(
                "Microsoft Edge",
                r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            ),
            local.join("Microsoft").join("Edge").join("User Data"),
        ));
        roots.push((
            "Google Chrome",
            browser_exe(
                "Google Chrome",
                r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            ),
            local.join("Google").join("Chrome").join("User Data"),
        ));
    }
    roots
}

pub fn read_browser_entries(
    include_bookmarks: bool,
    include_history: bool,
    include_tabs: bool,
) -> Vec<BrowserEntry> {
    let mut entries = Vec::new();
    for (browser_name, browser_exe, root) in chromium_roots() {
        if !root.is_dir() {
            continue;
        }
        let Ok(profiles) = std::fs::read_dir(&root) else {
            continue;
        };
        for profile in profiles.flatten().filter(|entry| entry.path().is_dir()) {
            let profile_name = profile.file_name().to_string_lossy().to_string();
            if profile_name != "Default" && !profile_name.starts_with("Profile ") {
                continue;
            }
            let source = BrowserSource {
                browser_name: browser_name.to_string(),
                browser_exe: browser_exe.clone(),
                profile: profile_name,
            };
            if include_bookmarks {
                let bookmarks = profile.path().join("Bookmarks");
                if let Ok(raw) = std::fs::read_to_string(bookmarks) {
                    entries.extend(parse_bookmarks_json(&raw, &source));
                }
            }
            if include_history {
                entries.extend(read_history_database(
                    &profile.path().join("History"),
                    &source,
                    HISTORY_LIMIT_PER_PROFILE,
                ));
            }
        }
    }
    if include_tabs {
        entries.extend(read_bridge_tabs());
    }
    entries
}

fn bridge_directory() -> Option<PathBuf> {
    dirs::data_local_dir().map(|path| path.join("NEXORA").join("browser-bridge"))
}

fn read_bridge_tabs() -> Vec<BrowserEntry> {
    let Some(directory) = bridge_directory() else {
        return Vec::new();
    };
    let Ok(raw) = std::fs::read_to_string(directory.join("tabs.json")) else {
        return Vec::new();
    };
    let Ok(snapshot) = serde_json::from_str::<BridgeSnapshot>(&raw) else {
        return Vec::new();
    };
    snapshot
        .tabs
        .into_iter()
        .filter(|tab| is_web_url(&tab.url))
        .map(|tab| BrowserEntry {
            id: format!("tab:{}:{}", tab.browser_name, tab.id),
            kind: "tab".to_string(),
            title: if tab.title.trim().is_empty() {
                tab.url.clone()
            } else {
                tab.title
            },
            url: tab.url,
            browser_name: tab.browser_name,
            browser_exe: tab.browser_exe,
            last_visited: None,
            tab_id: Some(tab.id),
            window_id: Some(tab.window_id),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse_bookmarks_json, read_history_database, BrowserSource};

    fn source() -> BrowserSource {
        BrowserSource {
            browser_name: "Edge".to_string(),
            browser_exe: Some(r"C:\Edge\msedge.exe".to_string()),
            profile: "Default".to_string(),
        }
    }

    #[test]
    fn parses_nested_bookmarks_and_skips_folders() {
        let raw = r#"{
          "roots": {
            "bookmark_bar": {
              "type": "folder",
              "children": [
                {
                  "type": "url",
                  "name": "NEXORA Docs",
                  "url": "https://example.com/docs",
                  "date_added": "10"
                },
                {
                  "type": "folder",
                  "name": "Projects",
                  "children": [
                    {
                      "type": "url",
                      "name": "GitHub",
                      "url": "https://github.com/ctxue/nexora"
                    }
                  ]
                }
              ]
            }
          }
        }"#;

        let entries = parse_bookmarks_json(raw, &source());

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title, "NEXORA Docs");
        assert_eq!(entries[1].url, "https://github.com/ctxue/nexora");
        assert_eq!(entries[0].kind, "bookmark");
    }

    #[test]
    fn reads_recent_web_history_from_a_copyable_database() {
        let root =
            std::env::temp_dir().join(format!("nexora-browser-history-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("create fixture directory");
        let database = root.join("History");
        let connection = rusqlite::Connection::open(&database).expect("open fixture database");
        connection
            .execute_batch(
                "CREATE TABLE urls (
                    url TEXT NOT NULL,
                    title TEXT NOT NULL,
                    last_visit_time INTEGER NOT NULL,
                    visit_count INTEGER NOT NULL DEFAULT 1,
                    hidden INTEGER NOT NULL DEFAULT 0
                );
                INSERT INTO urls VALUES ('https://example.com/recent', 'Recent page', 30, 4, 0);
                INSERT INTO urls VALUES ('chrome://settings', 'Settings', 40, 2, 0);
                INSERT INTO urls VALUES ('https://example.com/old', 'Old page', 10, 1, 0);",
            )
            .expect("seed fixture database");
        drop(connection);

        let entries = read_history_database(&database, &source(), 2);
        std::fs::remove_dir_all(&root).ok();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title, "Recent page");
        assert_eq!(entries[1].title, "Old page");
        assert_eq!(entries[0].kind, "history");
    }
}
