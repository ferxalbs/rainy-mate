use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[cfg(target_os = "macos")]
fn compile_swift_bridge(module_name: &str, swift_file: &str, extra_frameworks: &[&str]) {
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
    let staged_frameworks_dir = manifest_dir.join("gen").join("macos-frameworks");
    let dylib_name = format!("lib{module_name}.dylib");
    let debug_dylib_path = target_dir.join(&dylib_name);
    let debug_deps_dylib_path = target_deps_dir.join(&dylib_name);
    let framework_dylib_path = target_frameworks_dir.join(&dylib_name);
    let staged_dylib_path = staged_frameworks_dir.join(&dylib_name);

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
    fs::create_dir_all(&staged_frameworks_dir)
        .expect("failed to create staged macOS Frameworks dir");
    fs::copy(&dylib_path, &debug_dylib_path)
        .expect("failed to copy Swift bridge dylib to target dir");
    fs::copy(&dylib_path, &debug_deps_dylib_path)
        .expect("failed to copy Swift bridge dylib to target deps dir");
    fs::copy(&dylib_path, &framework_dylib_path)
        .expect("failed to copy Swift bridge dylib to target Frameworks dir");
    fs::copy(&dylib_path, &staged_dylib_path)
        .expect("failed to copy Swift bridge dylib to staged Frameworks dir");

    let _ = extra_frameworks;

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
    fs::copy(&whale_src, &resource_whale_path).expect("failed to copy whale icon to Resources dir");
    fs::copy(&whale_src, &framework_whale_path)
        .expect("failed to copy whale icon to Frameworks dir");
}

#[derive(Clone, Copy)]
struct BeamTemplateBuildSpec {
    id: &'static str,
    contract_name: &'static str,
    source_rel_path: &'static str,
}

const BEAM_TEMPLATE_SPECS: &[BeamTemplateBuildSpec] = &[
    BeamTemplateBuildSpec {
        id: "simple-erc20",
        contract_name: "BeamArcToken",
        source_rel_path: "../templates/beam/simple-erc20/Main.sol",
    },
    BeamTemplateBuildSpec {
        id: "nft-collection",
        contract_name: "BeamCollection",
        source_rel_path: "../templates/beam/nft-collection/Main.sol",
    },
    BeamTemplateBuildSpec {
        id: "basic-game",
        contract_name: "BeamArcadeArena",
        source_rel_path: "../templates/beam/basic-game/Main.sol",
    },
    BeamTemplateBuildSpec {
        id: "ai-oracle",
        contract_name: "BeamAiOracle",
        source_rel_path: "../templates/beam/ai-oracle/Main.sol",
    },
    BeamTemplateBuildSpec {
        id: "mini-indexer",
        contract_name: "BeamMiniIndexer",
        source_rel_path: "../templates/beam/mini-indexer/Main.sol",
    },
];

fn compile_beam_templates() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir missing"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR missing"));
    let repo_root = manifest_dir
        .parent()
        .expect("failed to resolve repo root")
        .to_path_buf();
    let solc_entry = repo_root
        .join("node_modules")
        .join(".pnpm")
        .join("solc@0.8.34")
        .join("node_modules")
        .join("solc")
        .join("solc.js");

    println!("cargo:rerun-if-changed={}", solc_entry.display());
    for spec in BEAM_TEMPLATE_SPECS {
        let source_path = manifest_dir.join(spec.source_rel_path);
        println!("cargo:rerun-if-changed={}", source_path.display());
    }

    if !solc_entry.is_file() {
        panic!(
            "Beam template build requires solc-js at {}. Run `pnpm install` before Cargo build.",
            solc_entry.display()
        );
    }

    let build_dir = out_dir.join("beam-template-artifacts");
    fs::create_dir_all(&build_dir).expect("failed to create beam artifact dir");

    let mut generated = String::from(
        "pub struct PrecompiledBeamTemplateArtifact {\n    pub id: &'static str,\n    pub contract_name: &'static str,\n    pub abi_json: &'static str,\n    pub bytecode: &'static str,\n    pub compiler_version: &'static str,\n}\n\npub const PRECOMPILED_BEAM_TEMPLATE_ARTIFACTS: &[PrecompiledBeamTemplateArtifact] = &[\n",
    );

    for spec in BEAM_TEMPLATE_SPECS {
        let source_path = manifest_dir.join(spec.source_rel_path);
        let template_out = build_dir.join(spec.id);
        fs::create_dir_all(&template_out).expect("failed to create template build dir");

        let status = Command::new("node")
            .arg(&solc_entry)
            .args([
                "--abi",
                "--bin",
                "--optimize",
                "--optimize-runs",
                "200",
                "-o",
            ])
            .arg(&template_out)
            .arg(&source_path)
            .status()
            .expect("failed to execute solcjs during build");

        if !status.success() {
            panic!("solcjs failed while compiling {}", spec.id);
        }

        let abi_path = find_solc_output(&template_out, spec.contract_name, "abi");
        let bin_path = find_solc_output(&template_out, spec.contract_name, "bin");
        let abi_json = fs::read_to_string(&abi_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", abi_path.display(), e));
        let bytecode = fs::read_to_string(&bin_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", bin_path.display(), e));

        generated.push_str("    PrecompiledBeamTemplateArtifact {\n");
        generated.push_str(&format!("        id: {:?},\n", spec.id));
        generated.push_str(&format!("        contract_name: {:?},\n", spec.contract_name));
        generated.push_str(&format!("        abi_json: {:?},\n", abi_json.trim()));
        generated.push_str(&format!("        bytecode: {:?},\n", bytecode.trim()));
        generated.push_str("        compiler_version: \"solcjs 0.8.34\",\n");
        generated.push_str("    },\n");
    }

    generated.push_str("];\n");
    fs::write(out_dir.join("beam_template_artifacts.rs"), generated)
        .expect("failed to write beam_template_artifacts.rs");
}

fn find_solc_output(dir: &PathBuf, contract_name: &str, extension: &str) -> PathBuf {
    let suffix = format!("_{}.{}", contract_name, extension);
    let entries = fs::read_dir(dir).unwrap_or_else(|e| {
        panic!("failed to read solc output dir {}: {}", dir.display(), e);
    });
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("failed to inspect solc output entry: {}", e))
            .path();
        let file_name = path.file_name().and_then(|value| value.to_str()).unwrap_or("");
        if file_name.ends_with(&suffix) {
            return path;
        }
    }
    panic!(
        "failed to find solc output matching *{} in {}",
        suffix,
        dir.display()
    );
}

fn main() {
    compile_beam_templates();

    #[cfg(target_os = "macos")]
    {
        compile_swift_bridge(
            "RainyAutoLaunch",
            "RainyAutoLaunch.swift",
            &["ServiceManagement", "AppKit"],
        );
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
