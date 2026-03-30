use crate::services::chat_artifacts::{ensure_openable_artifact_path, ChatArtifactOpenMode};
use std::process::Command;

#[tauri::command]
pub async fn open_chat_artifact(path: String) -> Result<(), String> {
    let artifact = ensure_openable_artifact_path(&path)?;

    #[cfg(target_os = "macos")]
    {
        let status = match artifact.open_mode {
            ChatArtifactOpenMode::Preview => Command::new("open")
                .args(["-a", "Preview", &artifact.path])
                .status()
                .map_err(|error| format!("Failed to launch Preview: {}", error))?,
            ChatArtifactOpenMode::Inline | ChatArtifactOpenMode::SystemDefault => {
                Command::new("open")
                    .arg(&artifact.path)
                    .status()
                    .map_err(|error| format!("Failed to open artifact: {}", error))?
            }
        };

        if status.success() {
            return Ok(());
        }

        return Err(format!("Open command exited with status {}", status));
    }

    #[cfg(target_os = "linux")]
    {
        let status = Command::new("xdg-open")
            .arg(&artifact.path)
            .status()
            .map_err(|error| format!("Failed to open artifact: {}", error))?;
        if status.success() {
            return Ok(());
        }
        return Err(format!("Open command exited with status {}", status));
    }

    #[cfg(target_os = "windows")]
    {
        let status = Command::new("cmd")
            .args(["/C", "start", "", &artifact.path])
            .status()
            .map_err(|error| format!("Failed to open artifact: {}", error))?;
        if status.success() {
            return Ok(());
        }
        return Err(format!("Open command exited with status {}", status));
    }

    #[allow(unreachable_code)]
    Err("Unsupported platform".to_string())
}
