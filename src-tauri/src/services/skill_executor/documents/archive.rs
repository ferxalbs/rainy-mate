use super::limits::{ensure_output_extension, normalize_archive_entries, validate_archive_create};
use super::super::args::ArchiveCreateArgs;
use super::super::SkillExecutor;
use crate::models::neural::CommandResult;
use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;

impl SkillExecutor {
    pub(super) async fn handle_archive_create(
        &self,
        workspace_id: String,
        params: &Value,
        allowed_paths: &[String],
        blocked_paths: &[String],
    ) -> CommandResult {
        let args: ArchiveCreateArgs = match serde_json::from_value(params.clone()) {
            Ok(value) => value,
            Err(error) => return self.error(&format!("Invalid parameters: {}", error)),
        };

        if let Err(error) = validate_archive_create(&args) {
            return self.error(&error);
        }

        let output_path = match self
            .resolve_path(workspace_id.clone(), &args.filename, allowed_paths, blocked_paths)
            .await
        {
            Ok(path) => path,
            Err(error) => return self.error(&error),
        };

        if let Err(error) = ensure_output_extension(&output_path, "zip") {
            return self.error(&error);
        }

        if let Some(parent) = output_path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                return self.error(&format!("Failed to create output directory: {}", error));
            }
        }

        let mut resolved_files = Vec::with_capacity(args.files.len());
        for file in args.files {
            match self
                .resolve_path(workspace_id.clone(), &file, allowed_paths, blocked_paths)
                .await
            {
                Ok(path) => resolved_files.push(path),
                Err(error) => return self.error(&error),
            }
        }

        match tokio::task::spawn_blocking(move || build_zip(&resolved_files, &output_path)).await {
            Ok(Ok((path, count))) => CommandResult {
                success: true,
                output: Some(
                    serde_json::json!({
                        "path": path,
                        "files_added": count,
                        "message": "Archive created successfully"
                    })
                    .to_string(),
                ),
                error: None,
                exit_code: Some(0),
            },
            Ok(Err(error)) => self.error(&format!("Archive creation failed: {}", error)),
            Err(error) => self.error(&format!("Archive task panicked: {}", error)),
        }
    }
}

fn build_zip(files: &[PathBuf], output_path: &PathBuf) -> Result<(String, usize), String> {
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    let file =
        std::fs::File::create(output_path).map_err(|error| format!("Failed to create archive file: {}", error))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    let normalized_files = normalize_archive_entries(files)?;
    let mut count = 0usize;

    for (file_path, entry_name) in normalized_files {
        zip.start_file(&entry_name, options)
            .map_err(|error| format!("Failed to start zip entry '{}': {}", entry_name, error))?;

        let content =
            std::fs::read(&file_path).map_err(|error| format!("Failed to read '{}': {}", file_path.to_string_lossy(), error))?;

        zip.write_all(&content).map_err(|error| {
            format!("Failed to write '{}' to archive: {}", entry_name, error)
        })?;

        count += 1;
    }

    zip.finish()
        .map_err(|error| format!("Failed to finalize archive: {}", error))?;

    Ok((output_path.to_string_lossy().to_string(), count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn archive_create_bundles_files() {
        let dir = tempdir().unwrap();
        let file_a = dir.path().join("a.txt");
        let file_b = dir.path().join("b.txt");
        std::fs::write(&file_a, "content a").unwrap();
        std::fs::write(&file_b, "content b").unwrap();

        let output = dir.path().join("bundle.zip");
        let result = build_zip(&[file_a.clone(), file_b.clone()], &output);
        assert!(result.is_ok(), "ZIP build failed: {:?}", result.err());
        let (_, count) = result.unwrap();
        assert_eq!(count, 2);
        assert!(output.metadata().unwrap().len() > 0);
    }
}
