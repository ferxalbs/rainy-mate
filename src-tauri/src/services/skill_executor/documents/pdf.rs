use super::super::args::{PdfCreateArgs, PdfReadArgs, PdfSection};
use super::super::SkillExecutor;
use super::limits::{ensure_output_extension, normalized_pdf_read_max_pages, validate_pdf_create};
use crate::models::neural::CommandResult;
use serde_json::Value;
use std::io::BufWriter;
use std::path::PathBuf;

impl SkillExecutor {
    pub(super) async fn handle_pdf_create(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: PdfCreateArgs = match serde_json::from_value(params.clone()) {
            Ok(value) => value,
            Err(error) => return self.error(&format!("Invalid parameters: {}", error)),
        };

        if let Err(error) = validate_pdf_create(&args) {
            return self.error(&error);
        }

        let output_path = match self
            .resolve_path(workspace_id, &args.filename, allowed_paths, blocked_paths)
            .await
        {
            Ok(path) => path,
            Err(error) => return self.error(&error),
        };

        if let Err(error) = ensure_output_extension(&output_path, "pdf") {
            return self.error(&error);
        }

        if let Some(parent) = output_path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                return self.error(&format!("Failed to create output directory: {}", error));
            }
        }

        let section_count = args.sections.len();
        match tokio::task::spawn_blocking(move || {
            build_pdf(
                args.title.as_deref().unwrap_or("Document"),
                &args.sections,
                &output_path,
            )
        })
        .await
        {
            Ok(Ok(path)) => CommandResult {
                success: true,
                output: Some(
                    serde_json::json!({
                        "path": path,
                        "sections": section_count,
                        "message": "PDF created successfully"
                    })
                    .to_string(),
                ),
                error: None,
                exit_code: Some(0),
            },
            Ok(Err(error)) => self.error(&format!("PDF generation failed: {}", error)),
            Err(error) => self.error(&format!("PDF task panicked: {}", error)),
        }
    }

    pub(super) async fn handle_pdf_read(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: PdfReadArgs = match serde_json::from_value(params.clone()) {
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

        let max_pages = normalized_pdf_read_max_pages(&args);

        match tokio::task::spawn_blocking(move || extract_pdf_text(&path, max_pages)).await {
            Ok(Ok(pages)) => CommandResult {
                success: true,
                output: Some(
                    serde_json::json!({
                        "page_count": pages.len(),
                        "pages": pages
                    })
                    .to_string(),
                ),
                error: None,
                exit_code: Some(0),
            },
            Ok(Err(error)) => self.error(&format!("PDF read failed: {}", error)),
            Err(error) => self.error(&format!("PDF task panicked: {}", error)),
        }
    }
}

fn build_pdf(
    title: &str,
    sections: &[PdfSection],
    output_path: &PathBuf,
) -> Result<String, String> {
    use printpdf::{
        ops::{Op, PdfFontHandle},
        BuiltinFont, Mm, PdfDocument, PdfPage, PdfSaveOptions, Point, Pt, TextItem,
    };

    const PAGE_WIDTH_MM: f32 = 210.0;
    const PAGE_HEIGHT_MM: f32 = 297.0;
    const LEFT_MARGIN_MM: f32 = 20.0;
    const TOP_MARGIN_MM: f32 = 40.0;
    const BOTTOM_MARGIN_MM: f32 = 20.0;
    const BODY_FONT_SIZE: f32 = 11.0;
    const HEADING_FONT_SIZE: f32 = 14.0;
    const TITLE_FONT_SIZE: f32 = 18.0;
    const BODY_LINE_HEIGHT_MM: f32 = 6.0;
    const HEADING_LINE_HEIGHT_MM: f32 = 10.0;
    const TITLE_LINE_HEIGHT_MM: f32 = 12.0;
    const MAX_CHARS_PER_LINE: usize = 85;

    let mut doc = PdfDocument::new(title);
    let mut pages = Vec::new();
    let mut ops = Vec::new();
    let mut y = PAGE_HEIGHT_MM - TOP_MARGIN_MM;

    let push_text =
        |ops: &mut Vec<Op>, text: &str, size: f32, x_mm: f32, y_mm: f32, font: BuiltinFont| {
            ops.push(Op::StartTextSection);
            ops.push(Op::SetFont {
                font: PdfFontHandle::Builtin(font),
                size: Pt(size),
            });
            ops.push(Op::SetTextCursor {
                pos: Point::new(Mm(x_mm), Mm(y_mm)),
            });
            ops.push(Op::ShowText {
                items: vec![TextItem::Text(text.to_string())],
            });
            ops.push(Op::EndTextSection);
        };

    let flush_page = |ops: &mut Vec<Op>, pages: &mut Vec<PdfPage>| {
        if !ops.is_empty() {
            pages.push(PdfPage::new(
                Mm(PAGE_WIDTH_MM),
                Mm(PAGE_HEIGHT_MM),
                std::mem::take(ops),
            ));
        }
    };

    push_text(
        &mut ops,
        title,
        TITLE_FONT_SIZE,
        LEFT_MARGIN_MM,
        y,
        BuiltinFont::HelveticaBold,
    );
    y -= TITLE_LINE_HEIGHT_MM;

    for section in sections {
        if let Some(heading) = &section.heading {
            if !heading.is_empty() {
                if y <= BOTTOM_MARGIN_MM {
                    flush_page(&mut ops, &mut pages);
                    y = PAGE_HEIGHT_MM - TOP_MARGIN_MM;
                }
                push_text(
                    &mut ops,
                    heading,
                    HEADING_FONT_SIZE,
                    LEFT_MARGIN_MM,
                    y,
                    BuiltinFont::HelveticaBold,
                );
                y -= HEADING_LINE_HEIGHT_MM;
            }
        }

        let mut current_line = String::new();
        for word in section.body.split_whitespace() {
            if !current_line.is_empty() && current_line.len() + word.len() + 1 > MAX_CHARS_PER_LINE
            {
                if y <= BOTTOM_MARGIN_MM {
                    flush_page(&mut ops, &mut pages);
                    y = PAGE_HEIGHT_MM - TOP_MARGIN_MM;
                }
                push_text(
                    &mut ops,
                    &current_line,
                    BODY_FONT_SIZE,
                    LEFT_MARGIN_MM,
                    y,
                    BuiltinFont::Helvetica,
                );
                y -= BODY_LINE_HEIGHT_MM;
                current_line.clear();
            }

            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }

        if !current_line.is_empty() {
            if y <= BOTTOM_MARGIN_MM {
                flush_page(&mut ops, &mut pages);
                y = PAGE_HEIGHT_MM - TOP_MARGIN_MM;
            }
            push_text(
                &mut ops,
                &current_line,
                BODY_FONT_SIZE,
                LEFT_MARGIN_MM,
                y,
                BuiltinFont::Helvetica,
            );
            y -= BODY_LINE_HEIGHT_MM;
        }

        y -= 4.0;
    }

    flush_page(&mut ops, &mut pages);
    doc.with_pages(pages);

    let mut warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    let file = std::fs::File::create(output_path)
        .map_err(|error| format!("Failed to create output file: {}", error))?;
    let mut writer = BufWriter::new(file);
    std::io::Write::write_all(&mut writer, &bytes)
        .map_err(|error| format!("Failed to save PDF: {}", error))?;

    Ok(output_path.to_string_lossy().to_string())
}

fn extract_pdf_text(path: &PathBuf, max_pages: usize) -> Result<Vec<serde_json::Value>, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("Failed to read PDF: {}", error))?;
    let text = pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|error| format!("PDF text extraction failed: {}", error))?;

    Ok(text
        .split('\x0C')
        .enumerate()
        .take(max_pages)
        .map(|(index, page_text)| {
            serde_json::json!({
                "page": index + 1,
                "text": page_text.trim()
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn pdf_create_writes_file() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("test.pdf");
        let sections = vec![
            PdfSection {
                heading: Some("Introduction".to_string()),
                body: "This is a test document created by Rainy MaTE IRONMILL.".to_string(),
            },
            PdfSection {
                heading: None,
                body: "Second section body content without a heading.".to_string(),
            },
        ];
        let result = build_pdf("Test Document", &sections, &output);
        assert!(result.is_ok(), "PDF build failed: {:?}", result.err());
        assert!(output.exists(), "PDF file was not created");
        assert!(output.metadata().unwrap().len() > 0, "PDF file is empty");
    }
}
