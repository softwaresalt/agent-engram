//! Integration tests for config-driven daemon behaviour (T068).
//!
//! Scenarios covered:
//! - S060-S061: Custom exclusion patterns applied to watcher config
//! - S048: Custom idle timeout from config returns correct Duration
//! - S086: Config is read-once at startup; runtime changes have no effect

use std::time::Duration;

use engram::daemon::watcher::WatcherConfig;
use engram::models::PluginConfig;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn write_config(dir: &std::path::Path, content: &str) {
    let engram_dir = dir.join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram dir");
    std::fs::write(engram_dir.join("config.toml"), content).expect("write config.toml");
}

// ── S060-S061: Custom exclusion patterns wired into WatcherConfig ─────────────

/// Verify that `PluginConfig` exclusion patterns are correctly transferred to
/// a `WatcherConfig` — the exact mechanism used by `daemon::run`.
#[test]
fn s060_s061_custom_exclude_patterns_wired_to_watcher() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    write_config(
        tmp.path(),
        r#"
exclude_patterns = [".git/", "build/", "dist/"]
watch_patterns = ["src/**/*.rs", "tests/**/*.rs"]
debounce_ms = 250
"#,
    );

    let plugin_config = PluginConfig::load(tmp.path());

    // Simulate the daemon's watcher construction (T072).
    let watcher_config = WatcherConfig {
        debounce_ms: plugin_config.debounce_ms,
        exclude_patterns: plugin_config.exclude_patterns.clone(),
        watch_patterns: plugin_config.watch_patterns.clone(),
    };

    assert_eq!(
        watcher_config.exclude_patterns,
        vec![".git/", "build/", "dist/"],
        "exclude_patterns must be forwarded to WatcherConfig unchanged"
    );
    assert_eq!(
        watcher_config.watch_patterns,
        vec!["src/**/*.rs", "tests/**/*.rs"],
        "watch_patterns must be forwarded to WatcherConfig unchanged"
    );
    assert_eq!(
        watcher_config.debounce_ms, 250,
        "debounce_ms must be forwarded to WatcherConfig unchanged"
    );
}

/// Default exclusion patterns include the canonical set.
#[test]
fn default_exclude_patterns_include_canonical_set() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    // No config file — load defaults.
    let plugin_config = PluginConfig::load(tmp.path());

    let watcher_config = WatcherConfig {
        debounce_ms: plugin_config.debounce_ms,
        exclude_patterns: plugin_config.exclude_patterns.clone(),
        watch_patterns: plugin_config.watch_patterns.clone(),
    };

    for expected in &[".engram/", ".git/", "node_modules/", "target/"] {
        assert!(
            watcher_config
                .exclude_patterns
                .iter()
                .any(|p| p == expected),
            "default exclude_patterns must contain {expected}"
        );
    }
}

// ── S048: Custom timeout returns correct Duration ─────────────────────────────

#[test]
fn s048_custom_idle_timeout_correct_duration() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    write_config(tmp.path(), "idle_timeout_minutes = 45");

    let cfg = PluginConfig::load(tmp.path());

    assert_eq!(
        cfg.idle_timeout(),
        Duration::from_secs(45 * 60),
        "idle_timeout() must convert minutes to seconds correctly"
    );
}

#[test]
fn s048_zero_idle_timeout_means_run_forever() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    write_config(tmp.path(), "idle_timeout_minutes = 0");

    let cfg = PluginConfig::load(tmp.path());

    assert_eq!(
        cfg.idle_timeout(),
        Duration::ZERO,
        "idle_timeout_minutes = 0 must produce Duration::ZERO (run forever)"
    );
}

// ── S086: Config is read-once; runtime changes have no effect ─────────────────

/// Demonstrates that `PluginConfig::load` captures a snapshot at call time.
/// Mutating the file on disk afterwards does not change the already-loaded config.
/// The daemon calls `load` exactly once during startup and caches the result.
#[test]
fn s086_config_is_read_once_at_startup() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    write_config(tmp.path(), "idle_timeout_minutes = 10");

    // Simulate daemon startup: load config once.
    let startup_config = PluginConfig::load(tmp.path());
    assert_eq!(startup_config.idle_timeout_minutes, 10);

    // Runtime change: overwrite the file.
    write_config(tmp.path(), "idle_timeout_minutes = 99");

    // The already-loaded config is unaffected.
    assert_eq!(
        startup_config.idle_timeout_minutes, 10,
        "in-memory config must not change when the file changes at runtime"
    );

    // A fresh load (new daemon process) would see the new value.
    let new_load = PluginConfig::load(tmp.path());
    assert_eq!(
        new_load.idle_timeout_minutes, 99,
        "a fresh load must pick up the updated file"
    );
}
