use super::super::args::{DocxCreateArgs, DocxParagraph};
use super::super::SkillExecutor;
use super::limits::{ensure_output_extension, validate_docx_create};
use crate::models::neural::CommandResult;
use serde_json::Value;
use std::io::BufWriter;
use std::path::PathBuf;

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
}

fn build_docx(paragraphs: &[DocxParagraph], output_path: &PathBuf) -> Result<String, String> {
    use docx_rs::{Docx, Paragraph, Run};

    let mut docx = Docx::new();

    for paragraph in paragraphs {
        if let Some(level) = paragraph.heading_level {
            let style_id = format!("Heading{}", level);
            let doc_paragraph = Paragraph::new()
                .add_run(Run::new().add_text(&paragraph.text))
                .style(&style_id);
            docx = docx.add_paragraph(doc_paragraph);
            continue;
        }

        let mut run = Run::new().add_text(&paragraph.text);
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
}
