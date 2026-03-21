use std::fs;
use std::path::PathBuf;

pub const CURRENT_BUNDLE_ID: &str = "com.enosislabs.rainymate";
pub const CURRENT_APP_DIR_NAME: &str = "rainy-mate";

pub fn resolve_namespaced_data_dir(base_dir: PathBuf) -> Result<PathBuf, String> {
    let dir = base_dir.join(CURRENT_BUNDLE_ID);
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create namespaced app dir: {}", e))?;
    Ok(dir)
}

pub fn resolve_app_dir(base_dir: PathBuf) -> Result<PathBuf, String> {
    let dir = base_dir.join(CURRENT_APP_DIR_NAME);
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create app dir: {}", e))?;
    Ok(dir)
}

pub fn resolve_child_dir(base_dir: PathBuf, child: &str) -> Result<PathBuf, String> {
    let dir = resolve_app_dir(base_dir)?.join(child);
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create {} dir: {}", child, e))?;
    Ok(dir)
}

pub fn resolve_namespaced_child_dir(base_dir: PathBuf, child: &str) -> Result<PathBuf, String> {
    let dir = resolve_namespaced_data_dir(base_dir)?.join(child);
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create {} dir: {}", child, e))?;
    Ok(dir)
}

pub fn resolve_child_file(base_dir: PathBuf, child: &str) -> Result<PathBuf, String> {
    let path = resolve_app_dir(base_dir)?.join(child);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent for {}: {}", child, e))?;
    }
    Ok(path)
}
