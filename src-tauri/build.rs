use std::process::Command;

fn main() {
    // Release builds (CI) use the plain Cargo version; everything else is a dev build.
    let version = env!("CARGO_PKG_VERSION");
    let full = if std::env::var_os("JABBERWRITE_RELEASE").is_some() {
        version.to_string()
    } else {
        let sha = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".into());
        format!("{version}-dev+{sha}")
    };
    println!("cargo:rustc-env=JABBERWRITE_VERSION={full}");
    println!("cargo:rerun-if-env-changed=JABBERWRITE_RELEASE");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/heads");
    tauri_build::build()
}
