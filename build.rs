//! Build script: stamp the binary with a self-identifying build hash.
//!
//! Sets `ENGRAM_BUILD_HASH` (read via `option_env!` in
//! `crate::shim::version`) to `<version>+g<short-sha>[-dirty]` when built
//! inside a git checkout, so `engram --version` and the daemon handshake
//! identify the exact commit. An explicit `ENGRAM_BUILD_HASH` environment
//! variable (e.g. a release tag from CI) always wins. When neither git nor an
//! override is available, `option_env!` falls back to `CARGO_PKG_VERSION`.

use std::process::Command;

fn main() {
    // Re-run when the checked-out commit or the override env var changes.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    println!("cargo:rerun-if-env-changed=ENGRAM_BUILD_HASH");

    // An explicit override wins (release CI can pass a tag/version directly).
    if let Ok(explicit) = std::env::var("ENGRAM_BUILD_HASH") {
        let explicit = explicit.trim();
        if !explicit.is_empty() {
            println!("cargo:rustc-env=ENGRAM_BUILD_HASH={explicit}");
            return;
        }
    }

    // Otherwise derive an identifier from the git checkout, if present.
    if let Some(hash) = git_build_hash() {
        println!("cargo:rustc-env=ENGRAM_BUILD_HASH={hash}");
    }
}

/// Build `<version>+g<short-sha>[-dirty]` from the surrounding git checkout.
///
/// Returns `None` when `git` is unavailable or the directory is not a checkout,
/// leaving `option_env!` to fall back to `CARGO_PKG_VERSION`.
fn git_build_hash() -> Option<String> {
    let version = std::env::var("CARGO_PKG_VERSION").ok()?;
    let short = git(&["rev-parse", "--short", "HEAD"])?;
    let dirty = if git(&["status", "--porcelain", "--untracked-files=no"])
        .is_some_and(|status| !status.is_empty())
    {
        "-dirty"
    } else {
        ""
    };
    Some(format!("{version}+g{short}{dirty}"))
}

/// Run `git` with `args`, returning trimmed stdout on success.
fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        None
    }
}
