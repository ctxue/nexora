use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::clipboard::{dpapi_protect, dpapi_unprotect};

const GITHUB_API_VERSION: &str = "2026-03-10";
const GITHUB_CLIENT_ID: &str = "Iv23li1SkBmqzfh1HOE6";
const GIST_FILE_NAME: &str = "nexora-config.json";
const GIST_DESCRIPTION: &str = "NEXORA configuration";
const GITHUB_AUTH_MAGIC: &[u8] = b"NEXORA-GITHUB\x01";

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GithubCredentials {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<i64>,
    login: String,
    avatar_url: String,
    gist_id: Option<String>,
    gist_updated_at: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubStatus {
    configured: bool,
    connected: bool,
    login: Option<String>,
    avatar_url: Option<String>,
    has_gist: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubDeviceStart {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
    browser_opened: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubDevicePoll {
    status: String,
    login: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubSyncResult {
    gist_id: String,
    updated_at: Option<String>,
}

#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
struct AccessTokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct GithubUser {
    login: String,
    avatar_url: String,
}

#[derive(Deserialize)]
struct GistFile {
    content: Option<String>,
}

#[derive(Deserialize)]
struct GistResponse {
    id: String,
    updated_at: Option<String>,
    #[serde(default)]
    files: HashMap<String, GistFile>,
}

#[derive(Deserialize)]
struct GistSummary {
    id: String,
    description: Option<String>,
}

pub struct GithubState {
    path: PathBuf,
    credentials: Mutex<Option<GithubCredentials>>,
    client: Client,
}

impl GithubState {
    pub fn new(path: PathBuf) -> Self {
        let credentials = load_credentials(&path).ok().flatten();
        let client = Client::builder()
            .user_agent("NEXORA/0.4.1")
            .build()
            .unwrap_or_default();
        Self {
            path,
            credentials: Mutex::new(credentials),
            client,
        }
    }

    fn credentials(&self) -> Option<GithubCredentials> {
        self.credentials
            .lock()
            .ok()
            .and_then(|credentials| credentials.clone())
    }

    fn save(&self, credentials: GithubCredentials) -> Result<(), String> {
        save_credentials(&self.path, &credentials)?;
        if let Ok(mut current) = self.credentials.lock() {
            *current = Some(credentials);
        }
        Ok(())
    }

    fn clear(&self) -> Result<(), String> {
        if let Ok(mut current) = self.credentials.lock() {
            *current = None;
        }
        if self.path.exists() {
            std::fs::remove_file(&self.path).map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn log(&self, stage: &str, detail: &str) {
        let Some(directory) = self.path.parent() else {
            return;
        };
        let log_directory = directory.join("logs");
        if std::fs::create_dir_all(&log_directory).is_err() {
            return;
        }
        let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_directory.join("github.log"))
        else {
            return;
        };
        let _ = writeln!(file, "{} [{stage}] {detail}", now_seconds());
    }
}

fn github_client_id() -> Option<String> {
    std::env::var("NEXORA_GITHUB_CLIENT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| Some(GITHUB_CLIENT_ID.to_string()))
}

fn require_client_id() -> Result<String, String> {
    github_client_id().ok_or_else(|| "githubClientIdMissing".to_string())
}

fn open_login_page(url: &str) -> Result<(), String> {
    if open::that_detached(url).is_ok() {
        return Ok(());
    }
    let browsers = crate::commands::browsers::browsers();
    let browser = browsers
        .first()
        .ok_or_else(|| "No web browser is registered".to_string())?;
    std::process::Command::new(&browser.exe)
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn load_credentials(path: &Path) -> Result<Option<GithubCredentials>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let payload = std::fs::read(path).map_err(|error| error.to_string())?;
    let encrypted = payload
        .strip_prefix(GITHUB_AUTH_MAGIC)
        .ok_or_else(|| "Invalid GitHub auth file".to_string())?;
    let plaintext = dpapi_unprotect(encrypted)?;
    serde_json::from_slice(&plaintext)
        .map(Some)
        .map_err(|error| error.to_string())
}

fn save_credentials(path: &Path, credentials: &GithubCredentials) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let json = serde_json::to_vec(credentials).map_err(|error| error.to_string())?;
    let encrypted = dpapi_protect(&json)?;
    let mut payload = Vec::with_capacity(GITHUB_AUTH_MAGIC.len() + encrypted.len());
    payload.extend_from_slice(GITHUB_AUTH_MAGIC);
    payload.extend_from_slice(&encrypted);
    std::fs::write(path, payload).map_err(|error| error.to_string())
}

async fn access_token(state: &GithubState) -> Result<String, String> {
    let credentials = state
        .credentials()
        .ok_or_else(|| "githubNotConnected".to_string())?;
    let needs_refresh = credentials
        .expires_at
        .map(|expires_at| expires_at <= now_seconds() + 60)
        .unwrap_or(false);
    if !needs_refresh {
        return Ok(credentials.access_token);
    }

    let refresh_token = credentials
        .refresh_token
        .clone()
        .ok_or_else(|| "githubLoginExpired".to_string())?;
    let client_id = require_client_id()?;
    let response = state
        .client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
        ])
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let token: AccessTokenResponse = response.json().await.map_err(|error| error.to_string())?;
    let access_token = token.access_token.ok_or_else(|| {
        token
            .error
            .unwrap_or_else(|| "githubLoginExpired".to_string())
    })?;
    let refreshed = GithubCredentials {
        access_token: access_token.clone(),
        refresh_token: token.refresh_token.or(credentials.refresh_token),
        expires_at: token.expires_in.map(|seconds| now_seconds() + seconds),
        ..credentials
    };
    state.save(refreshed)?;
    Ok(access_token)
}

fn auth_headers(request: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    request
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", GITHUB_API_VERSION)
}

async fn github_user(client: &Client, token: &str) -> Result<GithubUser, String> {
    let response = auth_headers(client.get("https://api.github.com/user"), token)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("GitHub user request failed: {}", response.status()));
    }
    response.json().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub fn github_status(state: State<'_, GithubState>) -> GithubStatus {
    let credentials = state.credentials();
    GithubStatus {
        configured: github_client_id().is_some(),
        connected: credentials.is_some(),
        login: credentials.as_ref().map(|value| value.login.clone()),
        avatar_url: credentials.as_ref().map(|value| value.avatar_url.clone()),
        has_gist: credentials
            .as_ref()
            .and_then(|value| value.gist_id.as_ref())
            .is_some(),
    }
}

#[tauri::command]
pub async fn github_device_start(
    state: State<'_, GithubState>,
) -> Result<GithubDeviceStart, String> {
    let client_id = require_client_id()?;
    let response = state
        .client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id.as_str()),
            ("scope", "offline_access"),
        ])
        .send()
        .await
        .map_err(|error| {
            state.log("device_start_network", &error.to_string());
            format!("githubNetworkError: {error}")
        })?;
    if !response.status().is_success() {
        state.log("device_start_http", &response.status().to_string());
        return Err(format!("GitHub device flow failed: {}", response.status()));
    }
    let device: DeviceCodeResponse = response.json().await.map_err(|error| {
        state.log("device_start_parse", &error.to_string());
        format!("githubResponseError: {error}")
    })?;
    let open_result = open_login_page(&device.verification_uri);
    if let Err(error) = &open_result {
        state.log("browser_open", &error.to_string());
    }
    let browser_opened = open_result.is_ok();
    state.log(
        "device_start_ok",
        &format!("browser_opened={browser_opened}"),
    );
    Ok(GithubDeviceStart {
        device_code: device.device_code,
        user_code: device.user_code,
        verification_uri: device.verification_uri,
        expires_in: device.expires_in,
        interval: device.interval.max(5),
        browser_opened,
    })
}

#[tauri::command]
pub async fn github_device_poll(
    state: State<'_, GithubState>,
    device_code: String,
) -> Result<GithubDevicePoll, String> {
    let client_id = require_client_id()?;
    let response = state
        .client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id.as_str()),
            ("device_code", device_code.as_str()),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let token: AccessTokenResponse = response.json().await.map_err(|error| error.to_string())?;

    if let Some(error) = token.error.as_deref() {
        return match error {
            "authorization_pending" | "slow_down" => Ok(GithubDevicePoll {
                status: error.to_string(),
                login: None,
                avatar_url: None,
            }),
            _ => Err(error.to_string()),
        };
    }

    let access_token = token
        .access_token
        .ok_or_else(|| "GitHub did not return an access token".to_string())?;
    let user = github_user(&state.client, &access_token).await?;
    let credentials = GithubCredentials {
        access_token,
        refresh_token: token.refresh_token,
        expires_at: token.expires_in.map(|seconds| now_seconds() + seconds),
        login: user.login.clone(),
        avatar_url: user.avatar_url.clone(),
        gist_id: None,
        gist_updated_at: None,
    };
    state.save(credentials)?;
    Ok(GithubDevicePoll {
        status: "authorized".to_string(),
        login: Some(user.login),
        avatar_url: Some(user.avatar_url),
    })
}

#[tauri::command]
pub fn github_logout(state: State<'_, GithubState>) -> Result<(), String> {
    state.clear()
}

#[tauri::command]
pub async fn github_upload_config(
    state: State<'_, GithubState>,
    contents: String,
) -> Result<GithubSyncResult, String> {
    if contents.len() > 1024 * 1024 {
        return Err("Configuration is too large for a GitHub Gist".to_string());
    }
    serde_json::from_str::<serde_json::Value>(&contents)
        .map_err(|_| "Configuration is not valid JSON".to_string())?;
    let token = access_token(&state).await?;
    let credentials = state
        .credentials()
        .ok_or_else(|| "githubNotConnected".to_string())?;
    let body = serde_json::json!({
        "description": GIST_DESCRIPTION,
        "public": false,
        "files": {
            (GIST_FILE_NAME): { "content": contents }
        }
    });

    let existing_gist_id = match credentials.gist_id.clone() {
        Some(gist_id) => Some(gist_id),
        None => find_gist_id(&state.client, &token)
            .await
            .inspect_err(|error| state.log("gist_lookup_error", error))?,
    };
    state.log(
        "upload_start",
        &format!(
            "update={} body_bytes={}",
            existing_gist_id.is_some(),
            contents.len()
        ),
    );
    let request = if let Some(gist_id) = existing_gist_id.as_ref() {
        auth_headers(
            state
                .client
                .patch(format!("https://api.github.com/gists/{gist_id}")),
            &token,
        )
    } else {
        auth_headers(state.client.post("https://api.github.com/gists"), &token)
    };
    let response = request.json(&body).send().await.map_err(|error| {
        state.log("upload_network_error", &error.to_string());
        format!("githubNetworkError: {error}")
    })?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        state.log(
            "upload_http_error",
            &format!(
                "status={status} body={}",
                body.chars().take(1200).collect::<String>()
            ),
        );
        return Err(format!("githubUploadHttpError:{status}"));
    }
    let gist: GistResponse = response.json().await.map_err(|error| {
        state.log("upload_parse_error", &error.to_string());
        format!("githubResponseError: {error}")
    })?;
    state.save(GithubCredentials {
        gist_id: Some(gist.id.clone()),
        gist_updated_at: gist.updated_at.clone(),
        ..credentials
    })?;
    state.log(
        "upload_ok",
        &format!("gist_id_present={}", !gist.id.is_empty()),
    );
    Ok(GithubSyncResult {
        gist_id: gist.id,
        updated_at: gist.updated_at,
    })
}

async fn find_gist_id(client: &Client, token: &str) -> Result<Option<String>, String> {
    let response = auth_headers(
        client.get("https://api.github.com/gists?per_page=100"),
        token,
    )
    .send()
    .await
    .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Could not list GitHub gists: {}",
            response.status()
        ));
    }
    let gists: Vec<GistSummary> = response.json().await.map_err(|error| error.to_string())?;
    Ok(gists
        .into_iter()
        .find(|gist| gist.description.as_deref() == Some(GIST_DESCRIPTION))
        .map(|gist| gist.id))
}

async fn fetch_gist(client: &Client, token: &str, gist_id: &str) -> Result<GistResponse, String> {
    let response = auth_headers(
        client.get(format!("https://api.github.com/gists/{gist_id}")),
        token,
    )
    .send()
    .await
    .map_err(|error| error.to_string())?;
    if response.status() == StatusCode::NOT_FOUND {
        return Err("githubConfigNotFound".to_string());
    }
    if !response.status().is_success() {
        return Err(format!(
            "Could not load GitHub configuration: {}",
            response.status()
        ));
    }
    response.json().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn github_download_config(state: State<'_, GithubState>) -> Result<String, String> {
    let token = access_token(&state).await?;
    let credentials = state
        .credentials()
        .ok_or_else(|| "githubNotConnected".to_string())?;
    let gist_id = match credentials.gist_id.clone() {
        Some(gist_id) => gist_id,
        None => find_gist_id(&state.client, &token)
            .await?
            .ok_or_else(|| "githubConfigNotFound".to_string())?,
    };
    let gist = fetch_gist(&state.client, &token, &gist_id).await?;
    let contents = gist
        .files
        .get(GIST_FILE_NAME)
        .and_then(|file| file.content.clone())
        .ok_or_else(|| "githubConfigNotFound".to_string())?;
    state.save(GithubCredentials {
        gist_id: Some(gist.id.clone()),
        gist_updated_at: gist.updated_at,
        ..credentials
    })?;
    Ok(contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_github_credentials_with_dpapi() {
        let path =
            std::env::temp_dir().join(format!("nexora-github-auth-{}.bin", std::process::id()));
        let credentials = GithubCredentials {
            access_token: "ghu_test".to_string(),
            refresh_token: Some("ghr_test".to_string()),
            expires_at: Some(now_seconds() + 3600),
            login: "octocat".to_string(),
            avatar_url: "https://example.com/avatar.png".to_string(),
            gist_id: Some("gist-id".to_string()),
            gist_updated_at: Some("2026-01-01T00:00:00Z".to_string()),
        };

        save_credentials(&path, &credentials).expect("save credentials");
        let raw = std::fs::read(&path).expect("read credentials");
        let loaded = load_credentials(&path)
            .expect("load credentials")
            .expect("credentials");
        std::fs::remove_file(&path).ok();

        assert!(!raw
            .windows("ghu_test".len())
            .any(|window| window == b"ghu_test"));
        assert_eq!(loaded.login, "octocat");
        assert_eq!(loaded.gist_id.as_deref(), Some("gist-id"));
    }

    #[test]
    fn ships_with_the_nexora_github_app_client_id() {
        assert_eq!(GITHUB_CLIENT_ID, "Iv23li1SkBmqzfh1HOE6");
        assert!(github_client_id().is_some());
    }

    #[test]
    #[ignore = "requires live GitHub access"]
    fn probes_github_device_flow_endpoint() {
        let result = tauri::async_runtime::block_on(async {
            let client = Client::builder()
                .user_agent("NEXORA/0.4.1")
                .build()
                .map_err(|error| error.to_string())?;
            let response = client
                .post("https://github.com/login/device/code")
                .header("Accept", "application/json")
                .form(&[("client_id", GITHUB_CLIENT_ID), ("scope", "offline_access")])
                .send()
                .await
                .map_err(|error| error.to_string())?;
            let status = response.status();
            let body = response.text().await.map_err(|error| error.to_string())?;
            Ok::<_, String>((status, body))
        });

        match result {
            Ok((status, body)) => {
                println!("status={status} body_length={}", body.len());
                assert!(status.is_success());
            }
            Err(error) => panic!("GitHub request failed: {error}"),
        }
    }
}
