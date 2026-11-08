use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;

const MAX_TEXT_BYTES: usize = 64 * 1024;
const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PDF_METADATA_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    Text,
    Image,
    Pdf,
    Metadata,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewData {
    pub kind: String,
    pub title: String,
    pub path: String,
    pub content: Option<String>,
    pub data_url: Option<String>,
    pub metadata: String,
    pub truncated: bool,
}

pub fn preview_kind(path: &Path) -> PreviewKind {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "txt" | "md" | "markdown" | "json" | "toml" | "yaml" | "yml" | "xml" | "html" | "htm"
        | "css" | "scss" | "js" | "jsx" | "ts" | "tsx" | "rs" | "py" | "go" | "java" | "c"
        | "h" | "cpp" | "hpp" | "cs" | "sql" | "sh" | "ps1" | "bat" | "cmd" | "ini" | "cfg"
        | "log" | "csv" => PreviewKind::Text,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" => PreviewKind::Image,
        "pdf" => PreviewKind::Pdf,
        _ => PreviewKind::Metadata,
    }
}

pub fn detect_binary(bytes: &[u8]) -> bool {
    if bytes.contains(&0) {
        return true;
    }
    let controls = bytes
        .iter()
        .filter(|byte| {
            let value = **byte;
            value < 0x20 && !matches!(value, b'\n' | b'\r' | b'\t')
        })
        .count();
    controls > bytes.len().max(1) / 20
}

fn image_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        _ => "image/png",
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn count_pdf_pages(bytes: &[u8]) -> usize {
    const NEEDLE: &[u8] = b"/Type /Page";
    bytes
        .windows(NEEDLE.len())
        .filter(|window| *window == NEEDLE)
        .count()
}

pub fn read_preview(path_value: &str) -> Result<PreviewData, String> {
    let path = PathBuf::from(path_value);
    let metadata = fs::metadata(&path).map_err(|error| format!("Preview unavailable: {error}"))?;
    if !metadata.is_file() {
        return Err("Quick look is available for files only".to_string());
    }

    let title = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("File")
        .to_string();
    let size = format_bytes(metadata.len());
    let kind = preview_kind(&path);
    let kind_label = match kind {
        PreviewKind::Text => "Text file",
        PreviewKind::Image => "Image",
        PreviewKind::Pdf => "PDF document",
        PreviewKind::Metadata => "File",
    };
    let base_metadata = format!("{kind_label} · {size}");

    match kind {
        PreviewKind::Text => {
            let mut bytes = Vec::new();
            fs::File::open(&path)
                .and_then(|file| {
                    file.take(MAX_TEXT_BYTES as u64)
                        .read_to_end(&mut bytes)
                        .map(|_| ())
                })
                .map_err(|error| format!("Could not read file: {error}"))?;
            if detect_binary(&bytes) {
                return Ok(PreviewData {
                    kind: "metadata".to_string(),
                    title,
                    path: path.to_string_lossy().to_string(),
                    content: None,
                    data_url: None,
                    metadata: format!("{base_metadata} · binary content"),
                    truncated: false,
                });
            }
            let truncated = metadata.len() > MAX_TEXT_BYTES as u64;
            Ok(PreviewData {
                kind: "text".to_string(),
                title,
                path: path.to_string_lossy().to_string(),
                content: Some(String::from_utf8_lossy(&bytes).to_string()),
                data_url: None,
                metadata: base_metadata,
                truncated,
            })
        }
        PreviewKind::Image => {
            if metadata.len() > MAX_IMAGE_BYTES {
                return Err("Image is larger than the 8 MB quick-look limit".to_string());
            }
            let bytes =
                fs::read(&path).map_err(|error| format!("Could not read image: {error}"))?;
            let data_url = format!(
                "data:{};base64,{}",
                image_mime(&path),
                STANDARD.encode(bytes)
            );
            Ok(PreviewData {
                kind: "image".to_string(),
                title,
                path: path.to_string_lossy().to_string(),
                content: None,
                data_url: Some(data_url),
                metadata: base_metadata,
                truncated: false,
            })
        }
        PreviewKind::Pdf => {
            let mut bytes = Vec::new();
            fs::File::open(&path)
                .and_then(|file| {
                    file.take(MAX_PDF_METADATA_BYTES)
                        .read_to_end(&mut bytes)
                        .map(|_| ())
                })
                .map_err(|error| format!("Could not read PDF: {error}"))?;
            let pages = count_pdf_pages(&bytes);
            let page_text = if pages > 0 {
                format!(" · {pages} page{}", if pages == 1 { "" } else { "s" })
            } else {
                String::new()
            };
            Ok(PreviewData {
                kind: "metadata".to_string(),
                title,
                path: path.to_string_lossy().to_string(),
                content: Some(
                    "PDF content is not embedded for security. Open the file to read it."
                        .to_string(),
                ),
                data_url: None,
                metadata: format!("{base_metadata}{page_text}"),
                truncated: false,
            })
        }
        PreviewKind::Metadata => Ok(PreviewData {
            kind: "metadata".to_string(),
            title,
            path: path.to_string_lossy().to_string(),
            content: None,
            data_url: None,
            metadata: base_metadata,
            truncated: false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{detect_binary, preview_kind};
    use std::path::Path;

    #[test]
    fn detects_nul_bytes_as_binary_text() {
        assert!(detect_binary(b"hello\0world"));
        assert!(!detect_binary("普通文本\nwith Unicode".as_bytes()));
    }

    #[test]
    fn classifies_supported_preview_kinds() {
        assert_eq!(
            preview_kind(Path::new("notes.txt")),
            super::PreviewKind::Text
        );
        assert_eq!(
            preview_kind(Path::new("photo.PNG")),
            super::PreviewKind::Image
        );
        assert_eq!(
            preview_kind(Path::new("report.pdf")),
            super::PreviewKind::Pdf
        );
        assert_eq!(
            preview_kind(Path::new("archive.zip")),
            super::PreviewKind::Metadata
        );
    }

    #[test]
    fn reads_utf8_text_without_exposing_binary_files_as_text() {
        let path = std::env::temp_dir().join(format!("nexora-preview-{}.txt", std::process::id()));
        std::fs::write(&path, "NEXORA preview\n第二行").expect("write preview fixture");

        let preview = super::read_preview(path.to_str().expect("utf8 path")).expect("read preview");
        assert_eq!(preview.kind, "text");
        assert_eq!(preview.content.as_deref(), Some("NEXORA preview\n第二行"));

        std::fs::remove_file(path).ok();
    }
}
