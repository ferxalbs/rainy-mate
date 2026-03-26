use super::super::args::{ExcelCell, ExcelReadArgs, ExcelSheet, ExcelWriteArgs};
use super::super::SkillExecutor;
use super::limits::{
    ensure_output_extension, normalized_excel_read_max_rows, validate_excel_write,
};
use crate::models::neural::CommandResult;
use serde_json::Value;
use std::path::PathBuf;

impl SkillExecutor {
    pub(super) async fn handle_excel_write(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: ExcelWriteArgs = match serde_json::from_value(params.clone()) {
            Ok(value) => value,
            Err(error) => return self.error(&format!("Invalid parameters: {}", error)),
        };

        if let Err(error) = validate_excel_write(&args) {
            return self.error(&error);
        }

        let output_path = match self
            .resolve_path(workspace_id, &args.filename, allowed_paths, blocked_paths)
            .await
        {
            Ok(path) => path,
            Err(error) => return self.error(&error),
        };

        if let Err(error) = ensure_output_extension(&output_path, "xlsx") {
            return self.error(&error);
        }

        if let Some(parent) = output_path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                return self.error(&format!("Failed to create output directory: {}", error));
            }
        }

        let sheet_count = args.sheets.len();
        match tokio::task::spawn_blocking(move || build_excel(&args.sheets, &output_path)).await {
            Ok(Ok(path)) => CommandResult {
                success: true,
                output: Some(
                    serde_json::json!({
                        "path": path,
                        "sheets": sheet_count,
                        "message": "Excel file created successfully"
                    })
                    .to_string(),
                ),
                error: None,
                exit_code: Some(0),
            },
            Ok(Err(error)) => self.error(&format!("Excel generation failed: {}", error)),
            Err(error) => self.error(&format!("Excel task panicked: {}", error)),
        }
    }

    pub(super) async fn handle_excel_read(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: ExcelReadArgs = match serde_json::from_value(params.clone()) {
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

        let max_rows = normalized_excel_read_max_rows(&args);

        match tokio::task::spawn_blocking(move || read_excel(&path, max_rows)).await {
            Ok(Ok(sheets)) => CommandResult {
                success: true,
                output: Some(
                    serde_json::json!({
                        "sheet_count": sheets.len(),
                        "sheets": sheets
                    })
                    .to_string(),
                ),
                error: None,
                exit_code: Some(0),
            },
            Ok(Err(error)) => self.error(&format!("Excel read failed: {}", error)),
            Err(error) => self.error(&format!("Excel task panicked: {}", error)),
        }
    }
}

fn build_excel(sheets: &[ExcelSheet], output_path: &PathBuf) -> Result<String, String> {
    use rust_xlsxwriter::{Format, Formula, Workbook};

    let mut workbook = Workbook::new();
    let bold_format = Format::new().set_bold();

    for sheet_def in sheets {
        let worksheet = workbook
            .add_worksheet()
            .set_name(&sheet_def.name)
            .map_err(|error| format!("Failed to set sheet name '{}': {}", sheet_def.name, error))?;

        let mut row_offset = 0u32;

        if let Some(headers) = &sheet_def.headers {
            for (col, header) in headers.iter().enumerate() {
                worksheet
                    .write_with_format(0, col as u16, header.as_str(), &bold_format)
                    .map_err(|error| format!("Failed to write header: {}", error))?;
            }
            row_offset = 1;
        }

        for (row_idx, row) in sheet_def.rows.iter().enumerate() {
            let xlsx_row = row_offset + row_idx as u32;
            for (col_idx, cell) in row.iter().enumerate() {
                let xlsx_col = col_idx as u16;
                match cell {
                    ExcelCell::Text(value) => {
                        worksheet
                            .write(xlsx_row, xlsx_col, value.as_str())
                            .map_err(|error| format!("Failed to write text cell: {}", error))?;
                    }
                    ExcelCell::Number(value) => {
                        worksheet
                            .write(xlsx_row, xlsx_col, *value)
                            .map_err(|error| format!("Failed to write number cell: {}", error))?;
                    }
                    ExcelCell::Bool(value) => {
                        worksheet
                            .write(xlsx_row, xlsx_col, *value)
                            .map_err(|error| format!("Failed to write bool cell: {}", error))?;
                    }
                    ExcelCell::Formula(value) => {
                        worksheet
                            .write(xlsx_row, xlsx_col, Formula::new(value))
                            .map_err(|error| format!("Failed to write formula cell: {}", error))?;
                    }
                }
            }
        }
    }

    workbook
        .save(output_path)
        .map_err(|error| format!("Failed to save Excel file: {}", error))?;

    Ok(output_path.to_string_lossy().to_string())
}

fn read_excel(path: &PathBuf, max_rows: usize) -> Result<Vec<serde_json::Value>, String> {
    use calamine::{open_workbook_auto, Data, Reader};

    let mut workbook = open_workbook_auto(path)
        .map_err(|error| format!("Failed to open Excel file: {}", error))?;

    let mut result = Vec::new();
    for name in workbook.sheet_names().to_vec() {
        match workbook.worksheet_range(&name) {
            Ok(range) => {
                let rows = range
                    .rows()
                    .take(max_rows)
                    .map(|row| {
                        row.iter()
                            .map(|cell| match cell {
                                Data::String(value) => serde_json::Value::String(value.clone()),
                                Data::Float(value) => serde_json::json!(*value),
                                Data::Int(value) => serde_json::json!(*value),
                                Data::Bool(value) => serde_json::Value::Bool(*value),
                                Data::Empty => serde_json::Value::Null,
                                Data::Error(error) => {
                                    serde_json::Value::String(format!("#ERR:{:?}", error))
                                }
                                _ => serde_json::Value::String(cell.to_string()),
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();

                result.push(serde_json::json!({
                    "name": name,
                    "row_count": rows.len(),
                    "rows": rows
                }));
            }
            Err(_) => {
                result.push(serde_json::json!({
                    "name": name,
                    "row_count": 0,
                    "rows": [],
                    "error": "Sheet could not be parsed"
                }));
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn excel_write_reads_back() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("test.xlsx");
        let sheets = vec![ExcelSheet {
            name: "Sheet1".to_string(),
            headers: Some(vec!["Name".to_string(), "Value".to_string()]),
            rows: vec![
                vec![ExcelCell::Text("Alpha".to_string()), ExcelCell::Number(1.0)],
                vec![ExcelCell::Text("Beta".to_string()), ExcelCell::Number(2.5)],
            ],
        }];
        let write_result = build_excel(&sheets, &output);
        assert!(
            write_result.is_ok(),
            "Excel build failed: {:?}",
            write_result.err()
        );
        assert!(output.exists());

        let read_result = read_excel(&output, 100);
        assert!(
            read_result.is_ok(),
            "Excel read failed: {:?}",
            read_result.err()
        );
        assert_eq!(read_result.unwrap().len(), 1);
    }
}
