use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChatArtifactKind {
    Image,
    Pdf,
    Docx,
    Xlsx,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChatArtifactOpenMode {
    Inline,
    Preview,
    SystemDefault,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChatArtifactAction {
    Open,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatArtifact {
    pub id: String,
    pub path: String,
    pub filename: String,
    pub kind: ChatArtifactKind,
    pub mime_type: String,
    pub open_mode: ChatArtifactOpenMode,
    pub available_actions: Vec<ChatArtifactAction>,
    pub origin_tool: String,
}

pub fn artifact_from_tool_result(
    tool_name: &str,
    args_json: Option<&str>,
    result: &str,
) -> Option<ChatArtifact> {
    let path = extract_path_from_result(result).or_else(|| extract_path_from_args(args_json))?;
    artifact_from_path(&path, tool_name)
}

pub fn artifact_from_path(path: &str, origin_tool: &str) -> Option<ChatArtifact> {
    let path_buf = PathBuf::from(path);
    if !path_buf.is_absolute() {
        return None;
    }
    let filename = path_buf.file_name()?.to_string_lossy().to_string();
    let extension = path_buf
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())?;

    let (kind, mime_type, open_mode) = match extension.as_str() {
        "png" => (
            ChatArtifactKind::Image,
            "image/png",
            ChatArtifactOpenMode::Inline,
        ),
        "jpg" | "jpeg" => (
            ChatArtifactKind::Image,
            "image/jpeg",
            ChatArtifactOpenMode::Inline,
        ),
        "gif" => (
            ChatArtifactKind::Image,
            "image/gif",
            ChatArtifactOpenMode::Inline,
        ),
        "webp" => (
            ChatArtifactKind::Image,
            "image/webp",
            ChatArtifactOpenMode::Inline,
        ),
        "pdf" => (
            ChatArtifactKind::Pdf,
            "application/pdf",
            ChatArtifactOpenMode::Preview,
        ),
        "docx" => (
            ChatArtifactKind::Docx,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            ChatArtifactOpenMode::SystemDefault,
        ),
        "xlsx" => (
            ChatArtifactKind::Xlsx,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            ChatArtifactOpenMode::SystemDefault,
        ),
        _ => return None,
    };

    Some(ChatArtifact {
        id: format!("{}:{}", origin_tool, path_buf.to_string_lossy()),
        path: path_buf.to_string_lossy().to_string(),
        filename,
        kind,
        mime_type: mime_type.to_string(),
        open_mode,
        available_actions: vec![ChatArtifactAction::Open],
        origin_tool: origin_tool.to_string(),
    })
}

pub fn push_unique_artifact(artifacts: &mut Vec<ChatArtifact>, artifact: ChatArtifact) {
    let normalized = normalize_path(&artifact.path);
    if artifacts
        .iter()
        .any(|existing| normalize_path(&existing.path) == normalized)
    {
        return;
    }
    artifacts.push(artifact);
}

pub fn ensure_openable_artifact_path(path: &str) -> Result<ChatArtifact, String> {
    let normalized = Path::new(path);
    if !normalized.is_absolute() {
        return Err("Artifact path must be absolute".to_string());
    }
    if !normalized.exists() {
        return Err("Artifact file does not exist".to_string());
    }
    artifact_from_path(path, "chat_artifact")
        .ok_or_else(|| "Unsupported artifact type".to_string())
}

fn extract_path_from_result(result: &str) -> Option<String> {
    let json = serde_json::from_str::<serde_json::Value>(result).ok()?;
    json.get("path")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn extract_path_from_args(args_json: Option<&str>) -> Option<String> {
    let json = serde_json::from_str::<serde_json::Value>(args_json?).ok()?;
    let path = json
        .get("path")
        .or_else(|| json.get("filename"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())?;

    Path::new(&path).is_absolute().then_some(path)
}

fn normalize_path(path: &str) -> String {
    PathBuf::from(path).to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_pdf_artifact_from_structured_result() {
        let artifact = artifact_from_tool_result(
            "pdf_create",
            None,
            r#"{"path":"/tmp/report.pdf","message":"ok"}"#,
        )
        .expect("artifact");

        assert_eq!(artifact.kind, ChatArtifactKind::Pdf);
        assert_eq!(artifact.open_mode, ChatArtifactOpenMode::Preview);
        assert_eq!(artifact.filename, "report.pdf");
    }

    #[test]
    fn extracts_docx_artifact_from_args_when_result_has_no_path() {
        let artifact = artifact_from_tool_result(
            "write_file",
            Some(r#"{"path":"/tmp/notes.docx"}"#),
            "File written successfully",
        )
        .expect("artifact");

        assert_eq!(artifact.kind, ChatArtifactKind::Docx);
        assert_eq!(artifact.open_mode, ChatArtifactOpenMode::SystemDefault);
    }

    #[test]
    fn ignores_unsupported_extensions() {
        let artifact = artifact_from_tool_result(
            "write_file",
            Some(r#"{"path":"/tmp/notes.txt"}"#),
            "File written successfully",
        );

        assert!(artifact.is_none());
    }
}
