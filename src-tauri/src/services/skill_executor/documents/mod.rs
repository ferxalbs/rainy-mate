/// IRONMILL — Document generation and reading handlers.
///
/// This module keeps PHASE 1 document logic in Rust and limits TypeScript to
/// UI-only concerns.
mod archive;
mod docx;
mod excel;
mod limits;
mod pdf;

use super::SkillExecutor;
use crate::models::neural::CommandResult;
use serde_json::Value;

impl SkillExecutor {
    pub(super) async fn execute_documents(
        &self,
        workspace_id: String,
        method: &str,
        params: &Option<Value>,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let params = match params {
            Some(p) => p,
            None => return self.error("Missing parameters"),
        };

        match method {
            "pdf_create" => {
                self.handle_pdf_create(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "pdf_read" => {
                self.handle_pdf_read(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "excel_write" => {
                self.handle_excel_write(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "excel_read" => {
                self.handle_excel_read(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "docx_create" => {
                self.handle_docx_create(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "docx_read" => {
                self.handle_docx_read(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            "archive_create" => {
                self.handle_archive_create(workspace_id, params, allowed_paths, blocked_paths)
                    .await
            }
            _ => CommandResult {
                success: false,
                output: None,
                error: Some(format!("Unknown documents method: {}", method)),
                exit_code: Some(1),
            },
        }
    }
}
