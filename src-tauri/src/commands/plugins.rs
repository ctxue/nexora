//! Plugin host: installs plugin folders and talks JSON-RPC to their processes.
//!
//! Protocol (one JSON object per line, stdio):
//!   host -> plugin: {"jsonrpc":"2.0","id":1,"method":"search",
//!                    "params":{"query":"…","keyword":"ip"}}
//!   plugin -> host: {"jsonrpc":"2.0","id":1,"result":{"items":[…]}}
//!
//! Plugins never touch the UI: they only return declarative items whose
//! actions the host executes.

use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::State;
use walkdir::WalkDir;

const DEFAULT_TIMEOUT_MS: u64 = 3_000;
const MAX_TIMEOUT_MS: u64 = 15_000;
const MAX_ITEMS: usize = 20;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    /// Executable or interpreter, relative to the plugin folder or on PATH.
    pub entry: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_MS
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginIcon {
    #[serde(rename = "type")]
    pub kind: String,
    pub value: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PluginAction {
    OpenUrl { url: String },
    OpenPath { path: String },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginItem {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    #[serde(default)]
    pub icon: Option<PluginIcon>,
    #[serde(default)]
    pub action: Option<PluginAction>,
}

struct PluginProcess {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    next_id: u64,
}

impl PluginProcess {
    fn spawn(manifest: &PluginManifest, dir: &Path) -> Result<Self, String> {
        let program = resolve_entry(dir, &manifest.entry)?;

        let mut command = Command::new(&program);
        command
            .args(&manifest.args)
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = command
            .spawn()
            .map_err(|error| format!("Could not start plugin: {error}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Plugin stdin is unavailable".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Plugin stdout is unavailable".to_string())?;

        if let Some(stderr) = child.stderr.take() {
            std::thread::spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    eprintln!("[plugin] {line}");
                }
            });
        }

        let (sender, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            child,
            stdin,
            lines,
            next_id: 0,
        })
    }

    fn request(&mut self, method: &str, params: Value, timeout: Duration) -> Result<Value, String> {
        self.next_id += 1;
        let id = self.next_id;

        let mut line = serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .map_err(|error| error.to_string())?;
        line.push('\n');

        self.stdin
            .write_all(line.as_bytes())
            .and_then(|_| self.stdin.flush())
            .map_err(|error| format!("Could not write to plugin: {error}"))?;

        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("Plugin timed out".to_string());
            }

            match self.lines.recv_timeout(remaining) {
                Ok(raw) => {
                    let Ok(value) = serde_json::from_str::<Value>(raw.trim()) else {
                        continue;
                    };
                    if value.get("id").and_then(Value::as_u64) != Some(id) {
                        continue;
                    }
                    if let Some(error) = value.get("error") {
                        let message = error
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("Plugin error");
                        return Err(message.to_string());
                    }
                    return Ok(value.get("result").cloned().unwrap_or(Value::Null));
                }
                Err(RecvTimeoutError::Timeout) => return Err("Plugin timed out".to_string()),
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("Plugin process exited".to_string())
                }
            }
        }
    }

    fn shutdown(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn resolve_entry(dir: &Path, entry: &str) -> Result<PathBuf, String> {
    let candidate = dir.join(entry);
    if candidate.exists() {
        let base = dir
            .canonicalize()
            .map_err(|error| format!("Invalid plugin folder: {error}"))?;
        let resolved = candidate
            .canonicalize()
            .map_err(|error| format!("Invalid plugin entry: {error}"))?;
        if !resolved.starts_with(&base) {
            return Err("Plugin entry escapes its folder".to_string());
        }
        Ok(resolved)
    } else if entry.contains('/') || entry.contains('\\') {
        Err(format!("Plugin entry not found: {entry}"))
    } else {
        // Command name on PATH (powershell.exe, node.exe, python.exe…).
        Ok(PathBuf::from(entry))
    }
}

fn validate_id(id: &str) -> Result<(), String> {
    let valid = (3..=64).contains(&id.len())
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-_".contains(character));
    if valid {
        Ok(())
    } else {
        Err("Plugin id must be 3-64 chars: a-z 0-9 . - _".to_string())
    }
}

fn plugin_dir_in(root: &Path, id: &str) -> Result<PathBuf, String> {
    validate_id(id)?;
    Ok(root.join(id))
}

fn read_manifest(dir: &Path) -> Result<PluginManifest, String> {
    let raw = std::fs::read_to_string(dir.join("plugin.json"))
        .map_err(|error| format!("Cannot read plugin.json: {error}"))?;
    let mut manifest: PluginManifest =
        serde_json::from_str(&raw).map_err(|error| format!("Invalid plugin.json: {error}"))?;

    manifest.id = manifest.id.trim().to_lowercase();
    manifest.entry = manifest.entry.trim().to_string();
    validate_id(&manifest.id)?;
    if manifest.entry.is_empty() {
        return Err("plugin.json needs an entry".to_string());
    }
    Ok(manifest)
}

fn copy_plugin(source: &Path, target: &Path) -> Result<(), String> {
    std::fs::create_dir_all(target).map_err(|error| error.to_string())?;

    for entry in WalkDir::new(source).follow_links(false) {
        let entry = entry.map_err(|error| error.to_string())?;
        let relative = entry
            .path()
            .strip_prefix(source)
            .map_err(|error| error.to_string())?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        if relative
            .components()
            .next()
            .map(|component| component.as_os_str() == ".git")
            .unwrap_or(false)
        {
            continue;
        }

        let destination = target.join(relative);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            std::fs::copy(entry.path(), &destination).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

pub struct PluginHost {
    root: PathBuf,
    inner: Arc<Mutex<HashMap<String, PluginProcess>>>,
}

impl PluginHost {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn shutdown_all(&self) {
        if let Ok(mut processes) = self.inner.lock() {
            for (_, mut process) in processes.drain() {
                process.shutdown();
            }
        }
    }

    fn plugin_dir(&self, id: &str) -> Result<PathBuf, String> {
        plugin_dir_in(&self.root, id)
    }

    fn stop(&self, id: &str) {
        if let Ok(mut processes) = self.inner.lock() {
            if let Some(mut process) = processes.remove(id) {
                process.shutdown();
            }
        }
    }
}

#[tauri::command]
pub fn list_plugins(host: State<'_, PluginHost>) -> Vec<PluginManifest> {
    let mut plugins = Vec::new();

    let Ok(entries) = std::fs::read_dir(&host.root) else {
        return plugins;
    };
    for entry in entries.flatten() {
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        if let Ok(manifest) = read_manifest(&entry.path()) {
            plugins.push(manifest);
        }
    }

    plugins.sort_by_key(|a| a.name.to_lowercase());
    plugins
}

#[tauri::command]
pub fn install_plugin(
    host: State<'_, PluginHost>,
    source: String,
) -> Result<PluginManifest, String> {
    let source = PathBuf::from(source.trim());
    if !source.is_dir() {
        return Err("Choose a plugin folder".to_string());
    }

    let manifest = read_manifest(&source)?;
    let target = host.plugin_dir(&manifest.id)?;

    host.stop(&manifest.id);
    if target.exists() {
        std::fs::remove_dir_all(&target)
            .map_err(|error| format!("Could not replace plugin: {error}"))?;
    }

    copy_plugin(&source, &target)?;
    read_manifest(&target)
}

#[tauri::command]
pub fn uninstall_plugin(host: State<'_, PluginHost>, id: String) -> Result<(), String> {
    let target = host.plugin_dir(&id)?;
    host.stop(&id);

    if target.exists() {
        std::fs::remove_dir_all(&target)
            .map_err(|error| format!("Could not remove plugin: {error}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn stop_plugin(host: State<'_, PluginHost>, id: String) {
    host.stop(&id);
}

#[tauri::command]
pub fn plugins_dir(host: State<'_, PluginHost>) -> String {
    host.root.to_string_lossy().to_string()
}

#[tauri::command]
pub async fn search_plugin(
    host: State<'_, PluginHost>,
    id: String,
    query: String,
    keyword: String,
) -> Result<Vec<PluginItem>, String> {
    let root = host.root.clone();
    let processes = host.inner.clone();

    tauri::async_runtime::spawn_blocking(move || {
        search_blocking(root, processes, id, query, keyword)
    })
    .await
    .map_err(|error| format!("Plugin task failed: {error}"))?
}

fn search_blocking(
    root: PathBuf,
    processes: Arc<Mutex<HashMap<String, PluginProcess>>>,
    id: String,
    query: String,
    keyword: String,
) -> Result<Vec<PluginItem>, String> {
    let dir = plugin_dir_in(&root, &id)?;
    let manifest = read_manifest(&dir)?;
    let timeout = Duration::from_millis(manifest.timeout_ms.clamp(500, MAX_TIMEOUT_MS));

    // Requests are serialized: one plugin process handles one call at a time.
    let mut processes = processes
        .lock()
        .map_err(|_| "Plugin host is busy".to_string())?;

    if !processes.contains_key(&id) {
        let process = PluginProcess::spawn(&manifest, &dir)?;
        processes.insert(id.clone(), process);
    }

    let process = processes
        .get_mut(&id)
        .ok_or_else(|| "Plugin process is unavailable".to_string())?;

    let result = process.request(
        "search",
        json!({ "query": query, "keyword": keyword }),
        timeout,
    );

    let value = match result {
        Ok(value) => value,
        Err(error) => {
            if let Some(mut process) = processes.remove(&id) {
                process.shutdown();
            }
            return Err(error);
        }
    };

    let items: Vec<PluginItem> = value
        .get("items")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| format!("Invalid plugin result: {error}"))?
        .unwrap_or_default();

    Ok(items.into_iter().take(MAX_ITEMS).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manifest() -> PluginManifest {
        PluginManifest {
            id: "com.nexora.test.echo".to_string(),
            name: "Echo".to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
            author: String::new(),
            entry: "powershell.exe".to_string(),
            args: vec![
                "-NoProfile".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-File".to_string(),
                "plugin.ps1".to_string(),
            ],
            keywords: vec!["echo".to_string()],
            permissions: Vec::new(),
            timeout_ms: 5000,
        }
    }

    fn write_script(dir: &Path, body: &str) {
        std::fs::write(dir.join("plugin.ps1"), body).expect("write test plugin");
    }

    fn test_dir(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp")
            .join(format!("{name}-{}", std::process::id()))
    }

    #[test]
    fn talks_json_rpc_with_a_plugin_process() {
        let dir = test_dir("nexora-plugin");
        std::fs::create_dir_all(&dir).expect("create plugin dir");
        write_script(
            &dir,
            r#"
$line = [Console]::In.ReadLine()
$message = $line | ConvertFrom-Json
$response = '{"jsonrpc":"2.0","id":' + $message.id + ',"result":{"items":[{"id":"x","title":"ok","subtitle":"","icon":null,"action":null}]}}'
[Console]::Out.WriteLine($response)
[Console]::Out.Flush()
"#,
        );

        let manifest = test_manifest();
        let mut process = PluginProcess::spawn(&manifest, &dir).expect("spawn plugin");
        let value = process
            .request("search", json!({ "query": "x" }), Duration::from_secs(10))
            .expect("plugin response");
        process.shutdown();
        std::fs::remove_dir_all(&dir).ok();

        let items: Vec<PluginItem> =
            serde_json::from_value(value.get("items").cloned().unwrap_or_default())
                .expect("items parse");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "ok");
    }

    #[test]
    fn installs_a_plugin_folder() {
        let root = test_dir("nexora-install");
        let source = root.join("source").join("com.nexora.test.hello");
        std::fs::create_dir_all(&source).expect("create source");
        std::fs::write(
            source.join("plugin.json"),
            r#"{
                "id": "com.nexora.test.hello",
                "name": "Hello",
                "version": "1.0.0",
                "entry": "powershell.exe",
                "keywords": ["hello"]
            }"#,
        )
        .expect("write manifest");
        std::fs::write(source.join("plugin.ps1"), "").expect("write entry");

        let host = PluginHost::new(root.join("plugins"));
        let manifest = read_manifest(&source).expect("read manifest");
        let target = host.plugin_dir(&manifest.id).expect("target dir");
        copy_plugin(&source, &target).expect("copy plugin");

        let installed = read_manifest(&target).expect("read installed manifest");
        assert_eq!(installed.name, "Hello");
        assert!(target.join("plugin.ps1").exists());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn times_out_when_a_plugin_never_answers() {
        let dir = test_dir("nexora-plugin-slow");
        std::fs::create_dir_all(&dir).expect("create plugin dir");
        write_script(&dir, "Start-Sleep -Seconds 5\n");

        let manifest = test_manifest();
        let mut process = PluginProcess::spawn(&manifest, &dir).expect("spawn plugin");
        let error = process
            .request(
                "search",
                json!({ "query": "x" }),
                Duration::from_millis(300),
            )
            .expect_err("expected a timeout");
        process.shutdown();
        std::fs::remove_dir_all(&dir).ok();

        assert!(error.to_lowercase().contains("timed out"));
    }
}
