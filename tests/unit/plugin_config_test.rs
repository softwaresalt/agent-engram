//! Unit tests for `PluginConfig` parsing (T067).
//!
//! Scenarios covered:
//! - S079: No config file → defaults
//! - S080: Valid config with `idle_timeout_minutes = 30` → custom timeout
//! - S081: Config with all fields set → all custom values applied
//! - S082: Config with unknown field → ignored, daemon uses file values
//! - S083: Malformed TOML → error logged, fallback to defaults
//! - S084: Negative `idle_timeout_minutes` → u64 parse error → defaults
//! - S085: Very large `debounce_ms` → accepted as-is
//! - S087: Zero `debounce_ms` → accepted
//!
//! Plan unit F02 (142.002-T) — strict `DaemonMode` parsing:
//! - mode setting absent → resolves to `DaemonMode::Managed`
//! - mode setting `"managed"` → resolves to `DaemonMode::Managed`
//! - mode setting `"read_server"` → resolves to `DaemonMode::ReadServer`
//! - mode setting present but unrecognized → hard parse error, never a
//!   silent fallback to managed
//! - round-trip parse/serialize is stable for both valid modes

use std::time::Duration;

use engram::models::{DaemonMode, DaemonModeParseError, PluginConfig};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn write_config(dir: &std::path::Path, content: &str) {
    let engram_dir = dir.join(".engram");
    std::fs::create_dir_all(&engram_dir).expect("create .engram dir");
    std::fs::write(engram_dir.join("config.toml"), content).expect("write config.toml");
}

// ── S079: No config file → defaults ──────────────────────────────────────────

#[test]
fn s079_no_config_uses_defaults() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    let cfg = PluginConfig::load(tmp.path());

    assert_eq!(cfg.idle_timeout_minutes, 240);
    assert_eq!(cfg.debounce_ms, 500);
    assert_eq!(cfg.watch_patterns, vec!["**/*"]);
    assert!(
        cfg.exclude_patterns.contains(&".engram/".to_string()),
        "exclude_patterns must contain .engram/"
    );
    assert!(
        cfg.exclude_patterns.contains(&".git/".to_string()),
        "exclude_patterns must contain .git/"
    );
    assert!(
        cfg.exclude_patterns.contains(&"node_modules/".to_string()),
        "exclude_patterns must contain node_modules/"
    );
    assert!(
        cfg.exclude_patterns.contains(&"target/".to_string()),
        "exclude_patterns must contain target/"
    );
    assert_eq!(cfg.log_level, "info");
    assert_eq!(cfg.log_format, "pretty");
}

// ── S080: Valid config with custom timeout ────────────────────────────────────

#[test]
fn s080_valid_config_custom_timeout() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    write_config(tmp.path(), "idle_timeout_minutes = 30");

    let cfg = PluginConfig::load(tmp.path());

    assert_eq!(cfg.idle_timeout_minutes, 30);
    assert_eq!(cfg.idle_timeout(), Duration::from_secs(30 * 60));
    // Other fields fall back to defaults.
    assert_eq!(cfg.debounce_ms, 500);
}

// ── S081: All fields set → all custom values ──────────────────────────────────

#[test]
fn s081_all_fields_set() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    write_config(
        tmp.path(),
        r#"
idle_timeout_minutes = 60
debounce_ms = 250
watch_patterns = ["src/**/*.rs"]
exclude_patterns = [".git/", "build/"]
log_level = "debug"
log_format = "json"
"#,
    );

    let cfg = PluginConfig::load(tmp.path());

    assert_eq!(cfg.idle_timeout_minutes, 60);
    assert_eq!(cfg.debounce_ms, 250);
    assert_eq!(cfg.watch_patterns, vec!["src/**/*.rs"]);
    assert_eq!(cfg.exclude_patterns, vec![".git/", "build/"]);
    assert_eq!(cfg.log_level, "debug");
    assert_eq!(cfg.log_format, "json");
    assert_eq!(cfg.idle_timeout(), Duration::from_secs(60 * 60));
}

// ── S082: Unknown field → ignored ────────────────────────────────────────────

#[test]
fn s082_unknown_field_ignored() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    write_config(
        tmp.path(),
        r#"
idle_timeout_minutes = 15
unknown_field = true
another_unknown = "hello"
"#,
    );

    // Should not fall back to defaults — the known field value is used.
    let cfg = PluginConfig::load(tmp.path());

    assert_eq!(
        cfg.idle_timeout_minutes, 15,
        "known field must be parsed even when unknown fields are present"
    );
}

// ── S083: Malformed TOML → fallback to defaults ───────────────────────────────

#[test]
fn s083_malformed_toml_uses_defaults() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    write_config(tmp.path(), "this is not [ valid toml !!!{{");

    let cfg = PluginConfig::load(tmp.path());

    // Must be identical to the default.
    assert_eq!(cfg, PluginConfig::default());
}

// ── S084: Negative idle_timeout_minutes → u64 parse error → defaults ─────────

#[test]
fn s084_negative_idle_timeout_uses_defaults() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    // TOML u64 cannot represent -1; `toml` will return a deserialisation error.
    write_config(tmp.path(), "idle_timeout_minutes = -1");

    let cfg = PluginConfig::load(tmp.path());

    assert_eq!(
        cfg,
        PluginConfig::default(),
        "negative u64 value must trigger parse error and fall back to defaults"
    );
}

// ── S085: Very large debounce_ms → accepted as-is ────────────────────────────

#[test]
fn s085_very_large_debounce_accepted() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    write_config(tmp.path(), "debounce_ms = 999999999");

    let cfg = PluginConfig::load(tmp.path());

    assert_eq!(
        cfg.debounce_ms, 999_999_999,
        "very large debounce_ms must be accepted without clamping"
    );
}

// ── S087: Zero debounce_ms → accepted ────────────────────────────────────────

#[test]
fn s087_zero_debounce_accepted() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    write_config(tmp.path(), "debounce_ms = 0");

    let cfg = PluginConfig::load(tmp.path());

    assert_eq!(cfg.debounce_ms, 0, "zero debounce_ms must be accepted");
}

// ── idle_timeout() with zero minutes = Duration::ZERO ────────────────────────

#[test]
fn idle_timeout_zero_minutes_returns_zero_duration() {
    let cfg = PluginConfig {
        idle_timeout_minutes: 0,
        ..PluginConfig::default()
    };
    assert_eq!(
        cfg.idle_timeout(),
        Duration::ZERO,
        "0 minutes must produce Duration::ZERO (run forever)"
    );
}

// ── Default impl matches field-level defaults ─────────────────────────────────

#[test]
fn default_impl_is_consistent() {
    let from_default = PluginConfig::default();
    let tmp = tempfile::tempdir().expect("tmp dir");
    // Empty config file — every field uses its serde default.
    write_config(tmp.path(), "");
    let from_empty_file = PluginConfig::load(tmp.path());

    assert_eq!(
        from_default, from_empty_file,
        "Default::default() and loading an empty config.toml must produce identical structs"
    );
}

// ── F02 (142.002-T): strict DaemonMode parsing ───────────────────────────────

// Absent setting → managed default.
#[test]
fn daemon_mode_resolve_absent_yields_managed_default() {
    assert_eq!(DaemonMode::resolve(None), Ok(DaemonMode::Managed));
}

// Valid "managed" → DaemonMode::Managed.
#[test]
fn daemon_mode_resolve_valid_managed() {
    assert_eq!(
        DaemonMode::resolve(Some("managed")),
        Ok(DaemonMode::Managed)
    );
}

// Valid "read_server" → DaemonMode::ReadServer.
#[test]
fn daemon_mode_resolve_valid_read_server() {
    assert_eq!(
        DaemonMode::resolve(Some("read_server")),
        Ok(DaemonMode::ReadServer)
    );
}

// Present-but-unrecognized value → typed error, never a silent fallback to
// managed.
#[test]
fn daemon_mode_resolve_invalid_value_is_hard_error() {
    let err = DaemonMode::resolve(Some("bogus")).expect_err("unrecognized value must error");
    assert_eq!(err, DaemonModeParseError::Unrecognized("bogus".to_owned()));
}

// Empty string is present-but-unrecognized, not treated as absent.
#[test]
fn daemon_mode_resolve_empty_string_is_hard_error() {
    let err = DaemonMode::resolve(Some(""))
        .expect_err("empty string is present-but-unrecognized, not absent");
    assert_eq!(err, DaemonModeParseError::Unrecognized(String::new()));
}

// Round-trip parse/serialize is stable for both valid modes.
#[test]
fn daemon_mode_round_trip_is_stable_for_both_modes() {
    for mode in [DaemonMode::Managed, DaemonMode::ReadServer] {
        let round_tripped =
            DaemonMode::resolve(Some(mode.as_str())).expect("canonical string must resolve");
        assert_eq!(round_tripped, mode);
        assert_eq!(round_tripped.as_str(), mode.as_str());
        assert_eq!(round_tripped.to_string(), mode.as_str());
    }
}

// PluginConfig carries the raw mode setting unresolved; the file itself
// stays permissive (a malformed mode value does not abort parsing of the
// rest of the config) while the single shared resolver enforces strictness.
#[test]
fn plugin_config_mode_field_absent_is_none() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    write_config(tmp.path(), "");
    let cfg = PluginConfig::load(tmp.path());

    assert_eq!(cfg.mode, None);
    assert_eq!(
        DaemonMode::resolve(cfg.mode.as_deref()),
        Ok(DaemonMode::Managed)
    );
}

#[test]
fn plugin_config_mode_field_valid_read_server_round_trips_through_resolver() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    write_config(tmp.path(), r#"mode = "read_server""#);
    let cfg = PluginConfig::load(tmp.path());

    assert_eq!(cfg.mode.as_deref(), Some("read_server"));
    assert_eq!(
        DaemonMode::resolve(cfg.mode.as_deref()),
        Ok(DaemonMode::ReadServer)
    );
}

#[test]
fn plugin_config_mode_field_invalid_value_is_carried_raw_and_fails_at_resolver() {
    let tmp = tempfile::tempdir().expect("tmp dir");
    write_config(tmp.path(), r#"mode = "bogus""#);
    let cfg = PluginConfig::load(tmp.path());

    // The permissive raw field still parses (file-level parsing is
    // unaffected); strictness is enforced only by the single shared
    // resolver, never by a duplicate parser at the config-file layer.
    assert_eq!(cfg.mode.as_deref(), Some("bogus"));
    assert_eq!(
        DaemonMode::resolve(cfg.mode.as_deref()),
        Err(DaemonModeParseError::Unrecognized("bogus".to_owned()))
    );
}
