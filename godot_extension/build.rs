// build.rs - 안전한 빌드 정보 주입 (vergen 없이)
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Auto-cleanup for release builds to ensure fresh DLL
    if std::env::var("PROFILE").unwrap() == "release" {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();

        // Delete .godot cache folder
        let godot_cache = workspace_root.join(".godot");
        if godot_cache.exists() {
            let _ = fs::remove_dir_all(&godot_cache);
            println!("cargo:warning=Cleaned .godot cache for fresh DLL load");
        }

        // Delete old DLL files
        let dll_paths = [
            workspace_root.join("target/release/football_rust.dll"),
            workspace_root.join("bin/football_rust.windows.template_release.x86_64.dll"),
        ];
        for dll_path in &dll_paths {
            if dll_path.exists() {
                let _ = fs::remove_file(dll_path);
            }
        }
    }
    // 변경 트리거
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    println!("cargo:rerun-if-changed=src/");

    // git short hash
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".into());

    // 빌드 타임(UTC)
    let build_time = chrono::Utc::now().to_rfc3339();

    // 환경 변수로 주입
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=BUILD_TIME_UTC={}", build_time);
    println!("cargo:rustc-env=VERGEN_BUILD_TIMESTAMP={}", build_time); // 기존 코드 호환성

    // Platform-specific configurations (기존 코드 유지)
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    match target_os.as_str() {
        "windows" => {
            // cdylib doesn't need SUBSYSTEM:WINDOWS
            // println!("cargo:rustc-link-arg-bins=/SUBSYSTEM:WINDOWS");
        }
        "macos" => {
            println!("cargo:rustc-link-arg=-undefined");
            println!("cargo:rustc-link-arg=dynamic_lookup");
        }
        "linux" => {
            // Linux-specific configurations
        }
        "android" => {
            println!("cargo:rustc-link-lib=log");
        }
        "ios" => {
            println!("cargo:rustc-link-arg=-undefined");
            println!("cargo:rustc-link-arg=dynamic_lookup");
        }
        _ => {}
    }

    // Release build optimizations
    if std::env::var("PROFILE").unwrap() == "release" {
        println!("cargo:rustc-env=CARGO_CFG_OPTIMIZED=true");
    }
}
