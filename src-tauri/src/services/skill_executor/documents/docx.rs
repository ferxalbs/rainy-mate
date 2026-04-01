use super::super::args::{DocxCreateArgs, DocxParagraph, DocxReadArgs};
use super::super::SkillExecutor;
use super::limits::{ensure_output_extension, validate_docx_create};
use super::text::normalize_document_text;
use crate::models::neural::CommandResult;
use regex::Regex;
use serde_json::Value;
use std::io::{BufWriter, Read};
use std::path::PathBuf;
use zip::ZipArchive;

impl SkillExecutor {
    pub(super) async fn handle_docx_create(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: DocxCreateArgs = match serde_json::from_value(params.clone()) {
            Ok(value) => value,
            Err(error) => return self.error(&format!("Invalid parameters: {}", error)),
        };

        if let Err(error) = validate_docx_create(&args) {
            return self.error(&error);
        }

        let output_path = match self
            .resolve_path(workspace_id, &args.filename, allowed_paths, blocked_paths)
            .await
        {
            Ok(path) => path,
            Err(error) => return self.error(&error),
        };

        if let Err(error) = ensure_output_extension(&output_path, "docx") {
            return self.error(&error);
        }

        if let Some(parent) = output_path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                return self.error(&format!("Failed to create output directory: {}", error));
            }
        }

        let paragraph_count = args.paragraphs.len();
        match tokio::task::spawn_blocking(move || build_docx(&args.paragraphs, &output_path)).await
        {
            Ok(Ok(path)) => CommandResult {
                success: true,
                output: Some(
                    serde_json::json!({
                        "path": path,
                        "paragraphs": paragraph_count,
                        "message": "DOCX document created successfully"
                    })
                    .to_string(),
                ),
                error: None,
                exit_code: Some(0),
            },
            Ok(Err(error)) => self.error(&format!("DOCX generation failed: {}", error)),
            Err(error) => self.error(&format!("DOCX task panicked: {}", error)),
        }
    }

    pub(super) async fn handle_docx_read(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: DocxReadArgs = match serde_json::from_value(params.clone()) {
            Ok(value) => value,
            Err(error) => return self.error(&format!("Invalid parameters: {}", error)),
        };

        let path = match self
            .resolve_path(workspace_id, &args.path, allowed_paths, blocked_paths)
            .await
        {
            Ok(path) => path,
            Err(error) => return self.error(&error),
        };

        if let Err(error) = ensure_docx_extension(&path) {
            return self.error(&error);
        }

        let path_for_read = path.clone();
        match tokio::task::spawn_blocking(move || extract_docx_paragraphs(&path_for_read)).await {
            Ok(Ok(paragraphs)) => {
                let text = paragraphs.join("\n\n");
                CommandResult {
                    success: true,
                    output: Some(
                        serde_json::json!({
                            "path": path.to_string_lossy(),
                            "paragraph_count": paragraphs.len(),
                            "text": text,
                            "paragraphs": paragraphs,
                        })
                        .to_string(),
                    ),
                    error: None,
                    exit_code: Some(0),
                }
            }
            Ok(Err(error)) => self.error(&format!("DOCX read failed: {}", error)),
            Err(error) => self.error(&format!("DOCX read task panicked: {}", error)),
        }
    }
}

fn build_docx(paragraphs: &[DocxParagraph], output_path: &PathBuf) -> Result<String, String> {
    use docx_rs::{Docx, Paragraph, Run};

    let mut docx = Docx::new();

    for paragraph in paragraphs {
        let normalized_text = normalize_document_text(&paragraph.text);
        if normalized_text.is_empty() {
            continue;
        }

        if let Some(level) = paragraph.heading_level {
            let style_id = format!("Heading{}", level);
            let doc_paragraph = Paragraph::new()
                .add_run(Run::new().add_text(&normalized_text))
                .style(&style_id);
            docx = docx.add_paragraph(doc_paragraph);
            continue;
        }

        let mut run = Run::new().add_text(&normalized_text);
        if paragraph.bold.unwrap_or(false) {
            run = run.bold();
        }
        if paragraph.italic.unwrap_or(false) {
            run = run.italic();
        }

        docx = docx.add_paragraph(Paragraph::new().add_run(run));
    }

    let file = std::fs::File::create(output_path)
        .map_err(|error| format!("Failed to create output file: {}", error))?;
    let mut writer = BufWriter::new(file);
    docx.build()
        .pack(&mut writer)
        .map_err(|error| format!("Failed to write DOCX: {}", error))?;

    Ok(output_path.to_string_lossy().to_string())
}

fn extract_docx_paragraphs(path: &PathBuf) -> Result<Vec<String>, String> {
    let file =
        std::fs::File::open(path).map_err(|error| format!("Failed to open DOCX: {}", error))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|error| format!("Failed to parse DOCX archive: {}", error))?;
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|error| format!("DOCX document.xml missing: {}", error))?
        .read_to_string(&mut xml)
        .map_err(|error| format!("Failed to read document.xml: {}", error))?;

    // DOCX body text is organized by <w:p> paragraphs and <w:t> runs.
    let paragraph_re = Regex::new(r"(?s)<w:p[^>]*>(.*?)</w:p>")
        .map_err(|error| format!("Regex error: {}", error))?;
    let run_re = Regex::new(r"(?s)<w:t[^>]*>(.*?)</w:t>")
        .map_err(|error| format!("Regex error: {}", error))?;

    let mut paragraphs = Vec::new();
    for paragraph_caps in paragraph_re.captures_iter(&xml) {
        let block = paragraph_caps
            .get(1)
            .map(|m| m.as_str())
            .unwrap_or_default();
        let mut text = String::new();
        for run_caps in run_re.captures_iter(block) {
            if let Some(value) = run_caps.get(1) {
                text.push_str(&decode_xml_entities(value.as_str()));
            }
        }
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            paragraphs.push(trimmed.to_string());
        }
    }

    Ok(paragraphs)
}

fn ensure_docx_extension(path: &PathBuf) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Path must end with .docx".to_string())?;
    if ext.eq_ignore_ascii_case("docx") {
        Ok(())
    } else {
        Err("Path must end with .docx".to_string())
    }
}

fn decode_xml_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn docx_create_writes_file() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("test.docx");
        let paragraphs = vec![
            DocxParagraph {
                text: "Hello from Rainy MaTE".to_string(),
                heading_level: Some(1),
                bold: None,
                italic: None,
            },
            DocxParagraph {
                text: "This is body text.".to_string(),
                heading_level: None,
                bold: Some(false),
                italic: Some(false),
            },
        ];
        let result = build_docx(&paragraphs, &output);
        assert!(result.is_ok(), "DOCX build failed: {:?}", result.err());
        assert!(output.exists());
        assert!(output.metadata().unwrap().len() > 0);
    }

    #[test]
    fn docx_read_extracts_paragraphs() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("read-test.docx");
        let paragraphs = vec![
            DocxParagraph {
                text: "Release Summary".to_string(),
                heading_level: Some(1),
                bold: None,
                italic: None,
            },
            DocxParagraph {
                text: "The app now exports DOCX with better history handling.".to_string(),
                heading_level: None,
                bold: Some(false),
                italic: Some(false),
            },
        ];
        let created = build_docx(&paragraphs, &output);
        assert!(created.is_ok(), "DOCX build failed: {:?}", created.err());

        let extracted = extract_docx_paragraphs(&output).expect("extract paragraphs");
        assert_eq!(extracted.len(), 2);
        assert!(extracted[0].contains("Release Summary"));
        assert!(extracted[1].contains("history handling"));
    }
}
