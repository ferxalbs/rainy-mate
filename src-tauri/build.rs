#[cfg(target_os = "macos")]
fn compile_swift_bridge() {
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR missing"));
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir missing"));
    let swift_src = manifest_dir.join("macos").join("RainyNativeNotifications.swift");
    let dylib_path = out_dir.join("libRainyNativeNotifications.dylib");
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
    let debug_dylib_path = target_dir.join("libRainyNativeNotifications.dylib");
    let debug_deps_dylib_path = target_deps_dir.join("libRainyNativeNotifications.dylib");
    let framework_dylib_path = target_frameworks_dir.join("libRainyNativeNotifications.dylib");

    println!("cargo:rerun-if-changed={}", swift_src.display());

    let status = Command::new("swiftc")
        .args([
            "-target",
            swift_target,
            "-parse-as-library",
            "-emit-library",
            "-module-name",
            "RainyNativeNotifications",
            "-module-cache-path",
        ])
        .arg(&module_cache)
        .args([
            "-Xlinker",
            "-install_name",
            "-Xlinker",
            "@rpath/libRainyNativeNotifications.dylib",
        ])
        .arg(&swift_src)
        .arg("-o")
        .arg(&dylib_path)
        .status()
        .expect("failed to run swiftc");

    if !status.success() {
        panic!("swiftc failed to compile RainyNativeNotifications.swift");
    }

    fs::create_dir_all(&target_deps_dir).expect("failed to create Cargo deps dir");
    fs::create_dir_all(&target_frameworks_dir).expect("failed to create Cargo Frameworks dir");
    fs::copy(&dylib_path, &debug_dylib_path).expect("failed to copy Swift bridge dylib to target dir");
    fs::copy(&dylib_path, &debug_deps_dylib_path)
        .expect("failed to copy Swift bridge dylib to target deps dir");
    fs::copy(&dylib_path, &framework_dylib_path)
        .expect("failed to copy Swift bridge dylib to target Frameworks dir");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=dylib=RainyNativeNotifications");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=AppKit");
    println!("cargo:rustc-link-lib=framework=UserNotifications");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
}

fn main() {
    #[cfg(target_os = "macos")]
    compile_swift_bridge();

    tauri_build::build()
}
