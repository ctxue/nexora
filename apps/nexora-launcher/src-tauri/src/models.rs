use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub source: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub extension: String,
    pub parent: String,
    #[serde(skip)]
    pub search_name: String,
    #[serde(skip)]
    pub search_text: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowEntry {
    pub id: String,
    pub title: String,
    pub process_name: String,
    pub process_path: Option<String>,
    pub minimized: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickPaths {
    pub home: String,
    pub desktop: String,
    pub documents: String,
    pub downloads: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutStatus {
    pub current: Option<String>,
    pub error: Option<String>,
}
