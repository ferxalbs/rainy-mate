#[cfg(target_os = "macos")]
fn compile_swift_bridge(module_name: &str, swift_file: &str, extra_frameworks: &[&str]) {
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR missing"));
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir missing"));
    let swift_src = manifest_dir.join("macos").join(swift_file);
    let dylib_path = out_dir.join(format!("lib{module_name}.dylib"));
    let module_cache = out_dir.join("swift-module-cache");
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").expect("target arch missing");
    let swift_target = match target_arch.as_str() {
        "x86_64" => "x86_64-apple-macos15.0",
        "aarch64" => "arm64-apple-macos15.0",
        other => panic!("unsupported macOS target arch for Swift bridge: {other}"),
    };
    let target_dir = out_dir
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .expect("failed to resolve Cargo target dir")
        .to_path_buf();
    let target_deps_dir = target_dir.join("deps");
    let target_frameworks_dir = target_dir.join("Frameworks");
    let dylib_name = format!("lib{module_name}.dylib");
    let debug_dylib_path = target_dir.join(&dylib_name);
    let debug_deps_dylib_path = target_deps_dir.join(&dylib_name);
    let framework_dylib_path = target_frameworks_dir.join(&dylib_name);

    println!("cargo:rerun-if-changed={}", swift_src.display());

    let status = Command::new("swiftc")
        .args([
            "-target",
            swift_target,
            "-parse-as-library",
            "-emit-library",
            "-module-name",
            module_name,
            "-module-cache-path",
        ])
        .arg(&module_cache)
        .args([
            "-Xlinker",
            "-install_name",
            "-Xlinker",
            &format!("@rpath/{dylib_name}"),
        ])
        .arg(&swift_src)
        .arg("-o")
        .arg(&dylib_path)
        .status()
        .expect("failed to run swiftc");

    if !status.success() {
        panic!("swiftc failed to compile {swift_file}");
    }

    fs::create_dir_all(&target_deps_dir).expect("failed to create Cargo deps dir");
    fs::create_dir_all(&target_frameworks_dir).expect("failed to create Cargo Frameworks dir");
    fs::copy(&dylib_path, &debug_dylib_path)
        .expect("failed to copy Swift bridge dylib to target dir");
    fs::copy(&dylib_path, &debug_deps_dylib_path)
        .expect("failed to copy Swift bridge dylib to target deps dir");
    fs::copy(&dylib_path, &framework_dylib_path)
        .expect("failed to copy Swift bridge dylib to target Frameworks dir");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=dylib={module_name}");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=AppKit");
    for framework in extra_frameworks {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");

    let whale_src = manifest_dir
        .parent()
        .expect("failed to resolve repo root")
        .join("public")
        .join("whale-dnf.png");
    let target_resources_dir = target_dir.join("Resources");
    let debug_whale_path = target_dir.join("whale-dnf.png");
    let resource_whale_path = target_resources_dir.join("whale-dnf.png");
    let framework_whale_path = target_frameworks_dir.join("whale-dnf.png");

    println!("cargo:rerun-if-changed={}", whale_src.display());

    fs::create_dir_all(&target_resources_dir).expect("failed to create Cargo Resources dir");
    fs::copy(&whale_src, &debug_whale_path).expect("failed to copy whale icon to target dir");
    fs::copy(&whale_src, &resource_whale_path)
        .expect("failed to copy whale icon to Resources dir");
    fs::copy(&whale_src, &framework_whale_path)
        .expect("failed to copy whale icon to Frameworks dir");
}

fn main() {
    #[cfg(target_os = "macos")]
    {
        compile_swift_bridge(
            "RainyNativeNotifications",
            "RainyNativeNotifications.swift",
            &["UserNotifications"],
        );
        compile_swift_bridge(
            "RainyQuickDelegate",
            "RainyQuickDelegate.swift",
            &["Carbon"],
        );
    }

    tauri_build::build()
}
