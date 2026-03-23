use super::super::args::{
    ArchiveCreateArgs, DocxCreateArgs, ExcelCell, ExcelReadArgs, ExcelWriteArgs, PdfCreateArgs,
    PdfReadArgs,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(super) const PDF_CREATE_MAX_SECTIONS: usize = 100;
pub(super) const PDF_CREATE_MAX_TEXT_BYTES: usize = 1_000_000;
pub(super) const PDF_READ_DEFAULT_MAX_PAGES: usize = 50;
pub(super) const PDF_READ_MAX_PAGES: usize = 200;
pub(super) const EXCEL_WRITE_MAX_SHEETS: usize = 20;
pub(super) const EXCEL_MAX_ROWS: usize = 10_000;
pub(super) const EXCEL_MAX_COLUMNS: usize = 100;
pub(super) const EXCEL_MAX_TEXT_CELL_BYTES: usize = 32 * 1024;
pub(super) const EXCEL_READ_DEFAULT_MAX_ROWS: usize = 1_000;
pub(super) const DOCX_CREATE_MAX_PARAGRAPHS: usize = 200;
pub(super) const ARCHIVE_CREATE_MAX_FILES: usize = 100;

pub(super) fn ensure_output_extension(path: &Path, expected: &str) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("Output file must end with .{}", expected))?;

    if ext.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!("Output file must end with .{}", expected))
    }
}

pub(super) fn validate_pdf_create(args: &PdfCreateArgs) -> Result<(), String> {
    if args.sections.is_empty() {
        return Err("pdf_create requires at least one section".to_string());
    }
    if args.sections.len() > PDF_CREATE_MAX_SECTIONS {
        return Err(format!(
            "pdf_create supports at most {} sections",
            PDF_CREATE_MAX_SECTIONS
        ));
    }

    let title_bytes = args.title.as_deref().unwrap_or_default().len();
    let section_bytes = args
        .sections
        .iter()
        .map(|section| section.body.len() + section.heading.as_deref().unwrap_or_default().len())
        .sum::<usize>();

    if title_bytes + section_bytes > PDF_CREATE_MAX_TEXT_BYTES {
        return Err(format!(
            "pdf_create input exceeds {} bytes",
            PDF_CREATE_MAX_TEXT_BYTES
        ));
    }

    Ok(())
}

pub(super) fn normalized_pdf_read_max_pages(args: &PdfReadArgs) -> usize {
    args.max_pages
        .unwrap_or(PDF_READ_DEFAULT_MAX_PAGES)
        .clamp(1, PDF_READ_MAX_PAGES)
}

pub(super) fn validate_excel_write(args: &ExcelWriteArgs) -> Result<(), String> {
    if args.sheets.is_empty() {
        return Err("excel_write requires at least one sheet".to_string());
    }
    if args.sheets.len() > EXCEL_WRITE_MAX_SHEETS {
        return Err(format!(
            "excel_write supports at most {} sheets",
            EXCEL_WRITE_MAX_SHEETS
        ));
    }

    for sheet in &args.sheets {
        if sheet.rows.len() > EXCEL_MAX_ROWS {
            return Err(format!(
                "Sheet '{}' exceeds {} rows",
                sheet.name, EXCEL_MAX_ROWS
            ));
        }

        if let Some(headers) = &sheet.headers {
            if headers.len() > EXCEL_MAX_COLUMNS {
                return Err(format!(
                    "Sheet '{}' exceeds {} header columns",
                    sheet.name, EXCEL_MAX_COLUMNS
                ));
            }
        }

        for row in &sheet.rows {
            if row.len() > EXCEL_MAX_COLUMNS {
                return Err(format!(
                    "Sheet '{}' exceeds {} columns in a row",
                    sheet.name, EXCEL_MAX_COLUMNS
                ));
            }

            for cell in row {
                match cell {
                    ExcelCell::Text(value) | ExcelCell::Formula(value)
                        if value.len() > EXCEL_MAX_TEXT_CELL_BYTES =>
                    {
                        return Err(format!(
                            "Sheet '{}' contains a cell larger than {} bytes",
                            sheet.name, EXCEL_MAX_TEXT_CELL_BYTES
                        ));
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

pub(super) fn normalized_excel_read_max_rows(args: &ExcelReadArgs) -> usize {
    args.max_rows
        .unwrap_or(EXCEL_READ_DEFAULT_MAX_ROWS)
        .clamp(1, EXCEL_MAX_ROWS)
}

pub(super) fn validate_docx_create(args: &DocxCreateArgs) -> Result<(), String> {
    if args.paragraphs.is_empty() {
        return Err("docx_create requires at least one paragraph".to_string());
    }
    if args.paragraphs.len() > DOCX_CREATE_MAX_PARAGRAPHS {
        return Err(format!(
            "docx_create supports at most {} paragraphs",
            DOCX_CREATE_MAX_PARAGRAPHS
        ));
    }

    for paragraph in &args.paragraphs {
        if let Some(level) = paragraph.heading_level {
            if !(1..=6).contains(&level) {
                return Err("docx_create heading_level must be between 1 and 6".to_string());
            }
        }
    }

    Ok(())
}

pub(super) fn validate_archive_create(args: &ArchiveCreateArgs) -> Result<(), String> {
    if args.files.is_empty() {
        return Err("archive_create requires at least one file".to_string());
    }
    if args.files.len() > ARCHIVE_CREATE_MAX_FILES {
        return Err(format!(
            "archive_create supports at most {} files",
            ARCHIVE_CREATE_MAX_FILES
        ));
    }
    Ok(())
}

pub(super) fn normalize_archive_entries(paths: &[PathBuf]) -> Result<Vec<(PathBuf, String)>, String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(paths.len());

    for path in paths {
        if !path.is_file() {
            return Err(format!(
                "Archive source must be a file: {}",
                path.to_string_lossy()
            ));
        }

        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                format!(
                    "Archive source has an invalid file name: {}",
                    path.to_string_lossy()
                )
            })?
            .to_string();

        if !seen.insert(name.clone()) {
            return Err(format!("Duplicate archive entry name: {}", name));
        }

        normalized.push((path.clone(), name));
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::args::{DocxParagraph, ExcelSheet, PdfSection};

    #[test]
    fn rejects_wrong_output_extension() {
        let path = Path::new("report.txt");
        let result = ensure_output_extension(path, "pdf");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_excessive_pdf_sections() {
        let args = PdfCreateArgs {
            filename: "test.pdf".to_string(),
            title: None,
            sections: (0..=PDF_CREATE_MAX_SECTIONS)
                .map(|_| PdfSection {
                    heading: None,
                    body: "body".to_string(),
                })
                .collect(),
        };

        let result = validate_pdf_create(&args);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_docx_heading_level() {
        let args = DocxCreateArgs {
            filename: "test.docx".to_string(),
            paragraphs: vec![DocxParagraph {
                text: "Bad".to_string(),
                heading_level: Some(7),
                bold: None,
                italic: None,
            }],
        };

        let result = validate_docx_create(&args);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_too_many_excel_sheets() {
        let args = ExcelWriteArgs {
            filename: "test.xlsx".to_string(),
            sheets: (0..=EXCEL_WRITE_MAX_SHEETS)
                .map(|index| ExcelSheet {
                    name: format!("Sheet{}", index),
                    headers: None,
                    rows: vec![],
                })
                .collect(),
        };

        let result = validate_excel_write(&args);
        assert!(result.is_err());
    }

    #[test]
    fn clamps_pdf_pages_to_hard_cap() {
        let args = PdfReadArgs {
            path: "test.pdf".to_string(),
            max_pages: Some(PDF_READ_MAX_PAGES + 50),
        };

        assert_eq!(normalized_pdf_read_max_pages(&args), PDF_READ_MAX_PAGES);
    }

    #[test]
    fn clamps_excel_rows_to_hard_cap() {
        let args = ExcelReadArgs {
            path: "test.xlsx".to_string(),
            max_rows: Some(EXCEL_MAX_ROWS + 500),
        };

        assert_eq!(normalized_excel_read_max_rows(&args), EXCEL_MAX_ROWS);
    }

    #[test]
    fn rejects_oversized_excel_text_cells() {
        let args = ExcelWriteArgs {
            filename: "test.xlsx".to_string(),
            sheets: vec![ExcelSheet {
                name: "Sheet1".to_string(),
                headers: None,
                rows: vec![vec![ExcelCell::Text("a".repeat(EXCEL_MAX_TEXT_CELL_BYTES + 1))]],
            }],
        };

        let result = validate_excel_write(&args);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_duplicate_archive_entry_names() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("report.txt");
        let second_dir = dir.path().join("nested");
        std::fs::create_dir_all(&second_dir).unwrap();
        let second = second_dir.join("report.txt");
        std::fs::write(&first, "a").unwrap();
        std::fs::write(&second, "b").unwrap();

        let result = normalize_archive_entries(&[first, second]);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_directories_from_archive_entries() {
        let dir = tempfile::tempdir().unwrap();
        let result = normalize_archive_entries(&[dir.path().to_path_buf()]);
        assert!(result.is_err());
    }
}
