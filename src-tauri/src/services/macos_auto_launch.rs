#[cfg(target_os = "macos")]
use libloading::Library;
#[cfg(target_os = "macos")]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::sync::OnceLock;

#[cfg(target_os = "macos")]
static AUTO_LAUNCH_BRIDGE: OnceLock<Result<AutoLaunchBridgeSymbols, String>> = OnceLock::new();

#[cfg(target_os = "macos")]
struct AutoLaunchBridgeSymbols {
    _library: Library,
    runtime_supported: unsafe extern "C" fn() -> i32,
    status: unsafe extern "C" fn() -> i32,
    set_enabled: unsafe extern "C" fn(i32) -> i32,
    open_system_settings: unsafe extern "C" fn() -> i32,
}

#[cfg(target_os = "macos")]
fn auto_launch_bridge_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(macos_dir) = exe.parent() {
            paths.push(macos_dir.join("libRainyAutoLaunch.dylib"));
            if let Some(contents_dir) = macos_dir.parent() {
                paths.push(
                    contents_dir
                        .join("Frameworks")
                        .join("libRainyAutoLaunch.dylib"),
                );
            }
        }
    }
    paths
}

#[cfg(target_os = "macos")]
fn auto_launch_bridge() -> Result<&'static AutoLaunchBridgeSymbols, String> {
    let result = AUTO_LAUNCH_BRIDGE.get_or_init(|| unsafe {
        let mut tried = Vec::new();
        for path in auto_launch_bridge_candidates() {
            tried.push(path.display().to_string());
            if !path.exists() {
                continue;
            }

            let library = match Library::new(&path) {
                Ok(library) => library,
                Err(error) => {
                    return Err(format!(
                        "Failed to load libRainyAutoLaunch.dylib from {}: {}",
                        path.display(),
                        error
                    ));
                }
            };

            let runtime_supported = *library
                .get::<unsafe extern "C" fn() -> i32>(b"rainy_auto_launch_runtime_supported\0")
                .map_err(|e| e.to_string())?;
            let status = *library
                .get::<unsafe extern "C" fn() -> i32>(b"rainy_auto_launch_status\0")
                .map_err(|e| e.to_string())?;
            let set_enabled = *library
                .get::<unsafe extern "C" fn(i32) -> i32>(b"rainy_auto_launch_set_enabled\0")
                .map_err(|e| e.to_string())?;
            let open_system_settings = *library
                .get::<unsafe extern "C" fn() -> i32>(b"rainy_auto_launch_open_system_settings\0")
                .map_err(|e| e.to_string())?;

            return Ok(AutoLaunchBridgeSymbols {
                _library: library,
                runtime_supported,
                status,
                set_enabled,
                open_system_settings,
            });
        }

        Err(format!(
            "libRainyAutoLaunch.dylib not found. Tried: {}",
            tried.join(", ")
        ))
    });

    result.as_ref().map_err(|error| error.clone())
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AutoLaunchState {
    Enabled,
    Disabled,
    RequiresApproval,
    Unsupported,
    Error,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoLaunchStatus {
    pub supported: bool,
    pub enabled: bool,
    pub requires_approval: bool,
    pub status: String,
}

impl AutoLaunchStatus {
    fn from_state(state: AutoLaunchState) -> Self {
        match state {
            AutoLaunchState::Enabled => Self {
                supported: true,
                enabled: true,
                requires_approval: false,
                status: "enabled".to_string(),
            },
            AutoLaunchState::Disabled => Self {
                supported: true,
                enabled: false,
                requires_approval: false,
                status: "disabled".to_string(),
            },
            AutoLaunchState::RequiresApproval => Self {
                supported: true,
                enabled: false,
                requires_approval: true,
                status: "requiresApproval".to_string(),
            },
            AutoLaunchState::Unsupported => Self {
                supported: false,
                enabled: false,
                requires_approval: false,
                status: "unsupported".to_string(),
            },
            AutoLaunchState::Error => Self {
                supported: true,
                enabled: false,
                requires_approval: false,
                status: "error".to_string(),
            },
        }
    }
}

#[cfg(target_os = "macos")]
fn decode_state(code: i32) -> AutoLaunchState {
    match code {
        1 => AutoLaunchState::Enabled,
        2 => AutoLaunchState::RequiresApproval,
        0 => AutoLaunchState::Disabled,
        -2 => AutoLaunchState::Unsupported,
        _ => AutoLaunchState::Error,
    }
}

pub struct MacOSAutoLaunchBridge;

#[cfg(target_os = "macos")]
impl MacOSAutoLaunchBridge {
    pub fn is_runtime_supported() -> bool {
        match auto_launch_bridge() {
            Ok(bridge) => unsafe { (bridge.runtime_supported)() == 1 },
            Err(error) => {
                tracing::warn!("{}", error);
                false
            }
        }
    }

    pub fn status() -> AutoLaunchStatus {
        if !Self::is_runtime_supported() {
            return AutoLaunchStatus::from_state(AutoLaunchState::Unsupported);
        }

        auto_launch_bridge()
            .map(|bridge| unsafe { decode_state((bridge.status)()) })
            .map(AutoLaunchStatus::from_state)
            .unwrap_or_else(|error| {
                tracing::warn!("macOS auto-launch bridge status failed: {}", error);
                AutoLaunchStatus::from_state(AutoLaunchState::Error)
            })
    }

    pub fn set_enabled(enabled: bool) -> Result<AutoLaunchStatus, String> {
        if !Self::is_runtime_supported() {
            return Ok(AutoLaunchStatus::from_state(AutoLaunchState::Unsupported));
        }

        let bridge = auto_launch_bridge()?;
        let state = unsafe { decode_state((bridge.set_enabled)(if enabled { 1 } else { 0 })) };
        Ok(AutoLaunchStatus::from_state(state))
    }

    pub fn open_system_settings() -> Result<(), String> {
        if !Self::is_runtime_supported() {
            return Err(
                "Native macOS auto-launch is unavailable in the current runtime".to_string(),
            );
        }

        let bridge = auto_launch_bridge()?;
        let ok = unsafe { (bridge.open_system_settings)() };
        if ok == 1 {
            Ok(())
        } else {
            Err("Failed to open macOS login items settings".to_string())
        }
    }
}

#[cfg(not(target_os = "macos"))]
impl MacOSAutoLaunchBridge {
    pub fn is_runtime_supported() -> bool {
        false
    }

    pub fn status() -> AutoLaunchStatus {
        AutoLaunchStatus {
            supported: false,
            enabled: false,
            requires_approval: false,
            status: "unsupported".to_string(),
        }
    }

    pub fn set_enabled(_enabled: bool) -> Result<AutoLaunchStatus, String> {
        Ok(Self::status())
    }

    pub fn open_system_settings() -> Result<(), String> {
        Err("Auto-launch settings are unavailable on this platform".to_string())
    }
}
