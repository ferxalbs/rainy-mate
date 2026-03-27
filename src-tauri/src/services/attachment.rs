// Attachment Processing Service
// Reads files selected by the user and converts them to agent-friendly content.
// Images → base64 data URIs (with optional resize).
// Documents (PDF, DOCX, XLSX, TXT) → extracted plain text.
// Unknown types → metadata summary.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::{imageops::FilterType, DynamicImage, ImageFormat};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read};
use std::path::Path;

/// Hard limits to prevent context-window overflows.
const MAX_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB
const MAX_EXTRACTED_TEXT_BYTES: usize = 50 * 1024; // 50 KB
const MAX_IMAGE_DIMENSION: u32 = 2048;
const THUMBNAIL_DIMENSION: u32 = 256;
const MAX_ATTACHMENTS: usize = 5;

/// Lightweight input from the frontend — just the file path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentInput {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Attachment content variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AttachmentContent {
    /// base64 data URI: "data:image/jpeg;base64,..."
    ImageDataUri { data_uri: String },
    /// Extracted plain text (PDF, DOCX, XLSX, TXT, etc.)
    ExtractedText { text: String },
    /// Binary file whose content cannot be extracted
    UnsupportedBinary { summary: String },
}

/// Fully processed attachment ready for agent injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedAttachment {
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub content: AttachmentContent,
    /// Small preview data URI for the chat UI (images only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_data_uri: Option<String>,
}

/// Attachment received from a cloud connector (ATM) — bytes pre-downloaded and base64-encoded.
/// The ATM downloads the file from Telegram/WhatsApp and sends this payload to the desktop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudAttachmentInput {
    pub filename: String,
    pub mime_type: String,
    /// Raw file bytes encoded as standard base64 (no data URI prefix).
    pub data_base64: String,
    pub size_bytes: u64,
}

/// Lightweight preview returned by `prepare_attachment_previews` before the user submits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentPreview {
    pub path: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: u64,
    /// "image" | "document" | "text" | "unknown"
    pub attachment_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_data_uri: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Build lightweight previews for a list of file paths (fast — no full extraction).
pub fn prepare_previews(paths: Vec<String>) -> Vec<AttachmentPreview> {
    paths
        .into_iter()
        .take(MAX_ATTACHMENTS)
        .map(|path| {
            let p = Path::new(&path);
            let filename = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
                .to_string();
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let mime_type = mime_from_ext(&ext);
            let attachment_type = attachment_type_from_ext(&ext);
            let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

            let thumbnail_data_uri = if attachment_type == "image" {
                generate_thumbnail_data_uri(&path, THUMBNAIL_DIMENSION).ok()
            } else {
                None
            };

            AttachmentPreview {
                path,
                filename,
                mime_type,
                size_bytes,
                attachment_type,
                thumbnail_data_uri,
            }
        })
        .collect()
}

/// Fully process a list of attachments for agent injection.
pub fn process_attachments(
    inputs: Vec<AttachmentInput>,
) -> Vec<ProcessedAttachment> {
    inputs
        .into_iter()
        .take(MAX_ATTACHMENTS)
        .filter_map(|input| process_single(&input).ok())
        .collect()
}

/// Process a cloud attachment (bytes pre-downloaded by the ATM connector) into agent content.
/// Returns `None` when the file is too large or the base64 payload is malformed.
pub fn process_cloud_attachment(input: CloudAttachmentInput) -> Option<ProcessedAttachment> {
    if input.size_bytes > MAX_FILE_SIZE_BYTES {
        return None;
    }

    let bytes = BASE64.decode(&input.data_base64).ok()?;

    let ext = Path::new(&input.filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let attachment_type = attachment_type_from_mime_or_ext(&input.mime_type, &ext);

    match attachment_type.as_str() {
        "image" => {
            let img = match image::load_from_memory(&bytes) {
                Ok(i) => i,
                Err(_) => {
                    // Cannot decode — wrap raw base64 as data URI anyway
                    let data_uri = format!("data:{};base64,{}", input.mime_type, input.data_base64);
                    return Some(ProcessedAttachment {
                        filename: input.filename,
                        mime_type: input.mime_type,
                        size_bytes: input.size_bytes,
                        content: AttachmentContent::ImageDataUri { data_uri },
                        thumbnail_data_uri: None,
                    });
                }
            };
            let img = resize_if_needed(img, MAX_IMAGE_DIMENSION);
            let data_uri = encode_image_to_data_uri(&img, &input.mime_type)
                .unwrap_or_else(|_| format!("data:{};base64,{}", input.mime_type, input.data_base64));
            let thumb = {
                let t = img.thumbnail(THUMBNAIL_DIMENSION, THUMBNAIL_DIMENSION);
                encode_image_to_data_uri(&t, "image/jpeg").ok()
            };
            Some(ProcessedAttachment {
                filename: input.filename,
                mime_type: input.mime_type,
                size_bytes: input.size_bytes,
                content: AttachmentContent::ImageDataUri { data_uri },
                thumbnail_data_uri: thumb,
            })
        }
        "text" => {
            let text = String::from_utf8_lossy(&bytes);
            let truncated = truncate_text(&text, MAX_EXTRACTED_TEXT_BYTES);
            Some(ProcessedAttachment {
                filename: input.filename,
                mime_type: input.mime_type,
                size_bytes: input.size_bytes,
                content: AttachmentContent::ExtractedText { text: truncated },
                thumbnail_data_uri: None,
            })
        }
        "document" => {
            let text = match ext.as_str() {
                "pdf" => {
                    let raw = pdf_extract::extract_text_from_mem(&bytes).unwrap_or_default();
                    let t = raw.trim().to_string();
                    if t.is_empty() {
                        format!("[PDF '{}' — no extractable text]", input.filename)
                    } else {
                        truncate_text(&t, MAX_EXTRACTED_TEXT_BYTES)
                    }
                }
                "docx" => extract_docx_text(&bytes).unwrap_or_else(|_| {
                    format!("[DOCX '{}' — extraction failed]", input.filename)
                }),
                "xlsx" | "xls" => extract_xlsx_text_from_bytes(&bytes, &input.filename),
                _ => {
                    let t = String::from_utf8_lossy(&bytes);
                    truncate_text(&t, MAX_EXTRACTED_TEXT_BYTES)
                }
            };
            Some(ProcessedAttachment {
                filename: input.filename,
                mime_type: input.mime_type,
                size_bytes: input.size_bytes,
                content: AttachmentContent::ExtractedText { text },
                thumbnail_data_uri: None,
            })
        }
        _ => Some(ProcessedAttachment {
            filename: input.filename.clone(),
            mime_type: input.mime_type,
            size_bytes: input.size_bytes,
            content: AttachmentContent::UnsupportedBinary {
                summary: format!("Binary file '{}' ({} bytes)", input.filename, input.size_bytes),
            },
            thumbnail_data_uri: None,
        }),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

fn process_single(input: &AttachmentInput) -> Result<ProcessedAttachment, String> {
    let p = Path::new(&input.path);
    let filename = input
        .name
        .clone()
        .or_else(|| p.file_name().and_then(|n| n.to_str()).map(String::from))
        .unwrap_or_else(|| "file".to_string());

    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let size_bytes = std::fs::metadata(&input.path)
        .map(|m| m.len())
        .unwrap_or(0);

    if size_bytes > MAX_FILE_SIZE_BYTES {
        return Err(format!(
            "File '{}' exceeds the 10 MB attachment limit.",
            filename
        ));
    }

    let mime_type = mime_from_ext(&ext);
    let attachment_type = attachment_type_from_ext(&ext);

    match attachment_type.as_str() {
        "image" => process_image(input, filename, mime_type, size_bytes),
        "text" => process_text_file(input, filename, mime_type, size_bytes),
        "document" => match ext.as_str() {
            "pdf" => process_pdf(input, filename, size_bytes),
            "docx" => process_docx(input, filename, size_bytes),
            "xlsx" | "xls" => process_xlsx(input, filename, size_bytes),
            _ => process_text_file(input, filename, mime_type, size_bytes),
        },
        _ => Ok(ProcessedAttachment {
            filename: filename.clone(),
            mime_type,
            size_bytes,
            content: AttachmentContent::UnsupportedBinary {
                summary: format!("Binary file '{}' ({} bytes)", filename, size_bytes),
            },
            thumbnail_data_uri: None,
        }),
    }
}

fn process_image(
    input: &AttachmentInput,
    filename: String,
    mime_type: String,
    size_bytes: u64,
) -> Result<ProcessedAttachment, String> {
    let img = image::open(&input.path)
        .map_err(|e| format!("Cannot open image '{}': {}", filename, e))?;

    let img = resize_if_needed(img, MAX_IMAGE_DIMENSION);
    let data_uri = encode_image_to_data_uri(&img, &mime_type)?;

    let thumbnail_data_uri = generate_thumbnail_data_uri(&input.path, THUMBNAIL_DIMENSION).ok();

    Ok(ProcessedAttachment {
        filename,
        mime_type,
        size_bytes,
        content: AttachmentContent::ImageDataUri { data_uri },
        thumbnail_data_uri,
    })
}

fn process_text_file(
    input: &AttachmentInput,
    filename: String,
    mime_type: String,
    size_bytes: u64,
) -> Result<ProcessedAttachment, String> {
    let raw = std::fs::read(&input.path)
        .map_err(|e| format!("Cannot read '{}': {}", filename, e))?;

    let text = String::from_utf8_lossy(&raw);
    let truncated = truncate_text(&text, MAX_EXTRACTED_TEXT_BYTES);

    Ok(ProcessedAttachment {
        filename,
        mime_type,
        size_bytes,
        content: AttachmentContent::ExtractedText { text: truncated },
        thumbnail_data_uri: None,
    })
}

fn process_pdf(
    input: &AttachmentInput,
    filename: String,
    size_bytes: u64,
) -> Result<ProcessedAttachment, String> {
    let bytes = std::fs::read(&input.path)
        .map_err(|e| format!("Cannot read PDF '{}': {}", filename, e))?;

    let text = pdf_extract::extract_text_from_mem(&bytes)
        .unwrap_or_default()
        .trim()
        .to_string();

    let text = if text.is_empty() {
        format!("[PDF file '{}' — text extraction yielded no content]", filename)
    } else {
        truncate_text(&text, MAX_EXTRACTED_TEXT_BYTES)
    };

    Ok(ProcessedAttachment {
        filename,
        mime_type: "application/pdf".to_string(),
        size_bytes,
        content: AttachmentContent::ExtractedText { text },
        thumbnail_data_uri: None,
    })
}

fn process_docx(
    input: &AttachmentInput,
    filename: String,
    size_bytes: u64,
) -> Result<ProcessedAttachment, String> {
    let bytes = std::fs::read(&input.path)
        .map_err(|e| format!("Cannot read DOCX '{}': {}", filename, e))?;

    let text = extract_docx_text(&bytes).unwrap_or_else(|_| {
        format!("[DOCX file '{}' — content could not be extracted]", filename)
    });

    Ok(ProcessedAttachment {
        filename,
        mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            .to_string(),
        size_bytes,
        content: AttachmentContent::ExtractedText { text },
        thumbnail_data_uri: None,
    })
}

fn process_xlsx(
    input: &AttachmentInput,
    filename: String,
    size_bytes: u64,
) -> Result<ProcessedAttachment, String> {
    let bytes = std::fs::read(&input.path)
        .map_err(|e| format!("Cannot read XLSX '{}': {}", filename, e))?;

    let text = extract_xlsx_text_from_bytes(&bytes, &filename);

    Ok(ProcessedAttachment {
        filename,
        mime_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
        size_bytes,
        content: AttachmentContent::ExtractedText { text },
        thumbnail_data_uri: None,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Image helpers
// ─────────────────────────────────────────────────────────────────────────────

fn resize_if_needed(img: DynamicImage, max_dim: u32) -> DynamicImage {
    let (w, h) = (img.width(), img.height());
    if w <= max_dim && h <= max_dim {
        return img;
    }
    img.resize(max_dim, max_dim, FilterType::Lanczos3)
}

fn encode_image_to_data_uri(img: &DynamicImage, mime_type: &str) -> Result<String, String> {
    let mut buf = Vec::new();
    let fmt = if mime_type == "image/png" {
        ImageFormat::Png
    } else {
        ImageFormat::Jpeg
    };
    img.write_to(&mut Cursor::new(&mut buf), fmt)
        .map_err(|e| format!("Image encode error: {}", e))?;

    let encoded = BASE64.encode(&buf);
    Ok(format!("data:{};base64,{}", mime_type, encoded))
}

fn generate_thumbnail_data_uri(path: &str, max_dim: u32) -> Result<String, String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let thumb = img.thumbnail(max_dim, max_dim);
    encode_image_to_data_uri(&thumb, "image/jpeg")
}

// ─────────────────────────────────────────────────────────────────────────────
// DOCX extraction
// ─────────────────────────────────────────────────────────────────────────────

fn extract_docx_text(bytes: &[u8]) -> Result<String, String> {
    let cursor = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("ZIP open error: {}", e))?;

    let mut xml_content = String::new();
    let mut file = archive
        .by_name("word/document.xml")
        .map_err(|e| format!("word/document.xml not found: {}", e))?;
    file.read_to_string(&mut xml_content)
        .map_err(|e| format!("Read error: {}", e))?;

    // Strip XML tags, keep text content
    let text = strip_xml_tags(&xml_content);
    let cleaned: String = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    Ok(truncate_text(&cleaned, MAX_EXTRACTED_TEXT_BYTES))
}

fn strip_xml_tags(xml: &str) -> String {
    let mut result = String::with_capacity(xml.len() / 2);
    let mut in_tag = false;
    for ch in xml.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                result.push(' ');
            }
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
}

// ─────────────────────────────────────────────────────────────────────────────
// Utilities
// ─────────────────────────────────────────────────────────────────────────────

fn truncate_text(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    // Truncate at char boundary
    let mut boundary = max_bytes;
    while !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    format!("{}…[truncated]", &text[..boundary])
}

fn mime_from_ext(ext: &str) -> String {
    match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "docx" => {
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        }
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "xls" => "application/vnd.ms-excel",
        "txt" | "md" | "csv" | "log" => "text/plain",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn attachment_type_from_ext(ext: &str) -> String {
    match ext {
        "jpg" | "jpeg" | "png" | "gif" | "webp" => "image",
        "pdf" | "docx" | "xlsx" | "xls" => "document",
        "txt" | "md" | "csv" | "log" | "json" | "yaml" | "toml" | "rs" | "ts" | "js"
        | "py" | "rb" | "go" | "java" | "c" | "cpp" | "h" => "text",
        _ => "unknown",
    }
    .to_string()
}

/// Classify attachment type using MIME type first, falling back to file extension.
fn attachment_type_from_mime_or_ext(mime_type: &str, ext: &str) -> String {
    if mime_type.starts_with("image/") {
        return "image".to_string();
    }
    if mime_type.starts_with("text/") {
        return "text".to_string();
    }
    if matches!(
        mime_type,
        "application/pdf"
            | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            | "application/vnd.ms-excel"
            | "application/msword"
    ) {
        return "document".to_string();
    }
    attachment_type_from_ext(ext)
}

/// Extract spreadsheet text from raw bytes (shared between local and cloud attachment paths).
fn extract_xlsx_text_from_bytes(bytes: &[u8], filename: &str) -> String {
    use calamine::{open_workbook_auto_from_rs, Data, Reader};

    let cursor = Cursor::new(bytes);
    let mut workbook = match open_workbook_auto_from_rs(cursor) {
        Ok(w) => w,
        Err(e) => return format!("[XLSX '{}' — parse error: {}]", filename, e),
    };

    let sheet_names: Vec<String> = workbook.sheet_names().to_vec();
    let mut parts: Vec<String> = Vec::new();

    for name in &sheet_names {
        if let Ok(range) = workbook.worksheet_range(name) {
            parts.push(format!("=== Sheet: {} ===", name));
            for row in range.rows() {
                let cells: Vec<String> = row
                    .iter()
                    .map(|cell| match cell {
                        Data::Empty => String::new(),
                        Data::String(s) => s.clone(),
                        Data::Float(f) => f.to_string(),
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        _ => String::new(),
                    })
                    .collect();
                parts.push(cells.join("\t"));
            }
        }
    }

    truncate_text(&parts.join("\n"), MAX_EXTRACTED_TEXT_BYTES)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_from_ext() {
        assert_eq!(mime_from_ext("jpg"), "image/jpeg");
        assert_eq!(mime_from_ext("png"), "image/png");
        assert_eq!(mime_from_ext("pdf"), "application/pdf");
        assert_eq!(mime_from_ext("txt"), "text/plain");
        assert_eq!(mime_from_ext("xyz"), "application/octet-stream");
    }

    #[test]
    fn test_attachment_type_from_ext() {
        assert_eq!(attachment_type_from_ext("png"), "image");
        assert_eq!(attachment_type_from_ext("pdf"), "document");
        assert_eq!(attachment_type_from_ext("txt"), "text");
        assert_eq!(attachment_type_from_ext("bin"), "unknown");
    }

    #[test]
    fn test_truncate_text() {
        let short = "hello";
        assert_eq!(truncate_text(short, 100), "hello");
        let long = "a".repeat(200);
        let t = truncate_text(&long, 10);
        // suffix "…[truncated]" = 13 bytes; content = 10 bytes
        assert!(t.len() <= 10 + "…[truncated]".len());
        assert!(t.contains("…[truncated]"));
    }

    #[test]
    fn test_strip_xml_tags() {
        let xml = "<w:p><w:r><w:t>Hello world</w:t></w:r></w:p>";
        let text = strip_xml_tags(xml);
        assert!(text.contains("Hello world"));
        assert!(!text.contains('<'));
    }

    #[test]
    fn test_process_attachments_empty() {
        let result = process_attachments(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_prepare_previews_empty() {
        let result = prepare_previews(vec![]);
        assert!(result.is_empty());
    }
}
