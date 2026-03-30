use std::path::{Path, PathBuf};

use tokio::fs;

const MEMORY_FILE_NAME: &str = "MEMORY.md";
const GUARDRAILS_FILE_NAME: &str = "GUARDRAILS.md";
const WORKSTATE_FILE_NAME: &str = "WORKSTATE.md";
const MAX_FILE_BYTES: usize = 4096;

#[derive(Debug, Clone, Default)]
pub struct WorkspaceMemoryBootstrap {
    pub root: Option<String>,
    pub enabled: bool,
    pub context_block: Option<String>,
}

pub struct WorkspaceMemoryFiles;

impl WorkspaceMemoryFiles {
    pub async fn bootstrap(
        workspace_id: &str,
        allowed_paths: Option<&[String]>,
    ) -> Result<WorkspaceMemoryBootstrap, String> {
        let Some(root) = Self::resolve_root(workspace_id, allowed_paths) else {
            return Ok(WorkspaceMemoryBootstrap::default());
        };

        let metadata = match fs::metadata(&root).await {
            Ok(metadata) => metadata,
            Err(_) => return Ok(WorkspaceMemoryBootstrap::default()),
        };

        let root_dir = if metadata.is_dir() {
            root
        } else if let Some(parent) = root.parent() {
            parent.to_path_buf()
        } else {
            return Ok(WorkspaceMemoryBootstrap::default());
        };

        let memory_path = root_dir.join(MEMORY_FILE_NAME);
        let guardrails_path = root_dir.join(GUARDRAILS_FILE_NAME);
        let workstate_path = root_dir.join(WORKSTATE_FILE_NAME);

        Self::ensure_file(
            &memory_path,
            "# MEMORY\n\nCapture durable business context, preferences, and facts worth remembering across sessions.\n",
        )
        .await?;
        Self::ensure_file(
            &guardrails_path,
            "# GUARDRAILS\n\nList non-negotiable rules, risks, and mistakes the agent must not repeat.\n",
        )
        .await?;
        Self::ensure_file(
            &workstate_path,
            "# WORKSTATE\n\nTrack the latest active task, current state, and next recommended action.\n",
        )
        .await?;

        let mut sections = Vec::new();
        if let Some(text) = Self::read_trimmed(&memory_path).await? {
            sections.push(format!("[{}]\n{}", MEMORY_FILE_NAME, text));
        }
        if let Some(text) = Self::read_trimmed(&guardrails_path).await? {
            sections.push(format!("[{}]\n{}", GUARDRAILS_FILE_NAME, text));
        }
        if let Some(text) = Self::read_trimmed(&workstate_path).await? {
            sections.push(format!("[{}]\n{}", WORKSTATE_FILE_NAME, text));
        }

        let context_block = if sections.is_empty() {
            None
        } else {
            Some(format!(
                "\n\nWorkspace Memory Files:\n{}\n",
                sections.join("\n\n")
            ))
        };

        Ok(WorkspaceMemoryBootstrap {
            root: Some(root_dir.to_string_lossy().to_string()),
            enabled: true,
            context_block,
        })
    }

    pub async fn update_workstate(
        root: Option<&str>,
        prompt: &str,
        response: &str,
    ) -> Result<(), String> {
        let Some(root) = root else {
            return Ok(());
        };

        let workstate_path = Path::new(root).join(WORKSTATE_FILE_NAME);
        let prompt = truncate_chars(prompt.trim(), 1400);
        let response = truncate_chars(response.trim(), 2200);
        let now = chrono::Utc::now().to_rfc3339();
        let content = format!(
            "# WORKSTATE\n\n## Last Updated\n{}\n\n## Active Task\n{}\n\n## Latest Outcome\n{}\n",
            now, prompt, response
        );
        fs::write(workstate_path, content)
            .await
            .map_err(|e| format!("Failed to update WORKSTATE.md: {}", e))
    }

    fn resolve_root(workspace_id: &str, allowed_paths: Option<&[String]>) -> Option<PathBuf> {
        if let Some(path) = allowed_paths
            .unwrap_or(&[])
            .iter()
            .find(|value| Path::new(value).is_absolute())
        {
            return Some(PathBuf::from(path));
        }

        let path = Path::new(workspace_id);
        if path.is_absolute() {
            Some(path.to_path_buf())
        } else {
            None
        }
    }

    async fn ensure_file(path: &Path, default_content: &str) -> Result<(), String> {
        match fs::metadata(path).await {
            Ok(metadata) if metadata.is_file() => Ok(()),
            Ok(_) => Err(format!(
                "Workspace memory path is not a file: {}",
                path.display()
            )),
            Err(_) => fs::write(path, default_content)
                .await
                .map_err(|e| format!("Failed to create {}: {}", path.display(), e)),
        }
    }

    async fn read_trimmed(path: &Path) -> Result<Option<String>, String> {
        let bytes = fs::read(path)
            .await
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        if bytes.is_empty() {
            return Ok(None);
        }
        let text = String::from_utf8_lossy(&bytes);
        let trimmed = truncate_chars(text.trim(), MAX_FILE_BYTES);
        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(trimmed))
        }
    }
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let mut value = input.chars().take(max_chars).collect::<String>();
    value.push_str("\n[TRUNCATED]");
    value
}

#[cfg(test)]
mod tests {
    use super::WorkspaceMemoryFiles;

    #[tokio::test]
    async fn bootstrap_creates_memory_files_for_absolute_workspace() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let bootstrap =
            WorkspaceMemoryFiles::bootstrap(tempdir.path().to_string_lossy().as_ref(), None)
                .await
                .expect("bootstrap");

        assert!(bootstrap.enabled);
        assert!(tempdir.path().join("MEMORY.md").exists());
        assert!(tempdir.path().join("GUARDRAILS.md").exists());
        assert!(tempdir.path().join("WORKSTATE.md").exists());
        assert!(bootstrap.context_block.is_some());
    }

    #[tokio::test]
    async fn update_workstate_overwrites_latest_state() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        WorkspaceMemoryFiles::bootstrap(tempdir.path().to_string_lossy().as_ref(), None)
            .await
            .expect("bootstrap");

        WorkspaceMemoryFiles::update_workstate(
            Some(tempdir.path().to_string_lossy().as_ref()),
            "Investigate invoice folder",
            "Indexed PDFs and wrote summary.csv",
        )
        .await
        .expect("update workstate");

        let workstate =
            std::fs::read_to_string(tempdir.path().join("WORKSTATE.md")).expect("read workstate");
        assert!(workstate.contains("Investigate invoice folder"));
        assert!(workstate.contains("Indexed PDFs and wrote summary.csv"));
    }
}
