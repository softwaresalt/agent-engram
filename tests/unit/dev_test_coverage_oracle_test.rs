//! Coverage-oracle harness (Plan unit U1, task 126.001-T).
//!
//! These scenarios exercise the canonical coverage oracle
//! (`scripts/test-coverage-oracle.ps1` / `.sh`) and its declared manifest
//! (`.cargo/test-coverage-manifest.toml`). The oracle computes the required
//! target set for a diff, compares it against the selected set, and fails when
//! any required target is omitted or a source surface is unmapped.
//!
//! Red phase: until U3/U4 land the manifest and oracle scripts, every scenario
//! fails because the oracle is absent.
//!
//! Scenarios:
//! * a — a changed source file whose required target is absent from the
//!   selected set fails, naming the omitted target.
//! * b — a changed source file with no manifest mapping fails as an unmapped
//!   surface.
//! * c — a fully covered diff passes with `omitted == 0`, and its bounded run
//!   plan stays within the configured concurrency cap (U4).

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Run the coverage oracle with `args`, returning combined output and exit code.
fn run_oracle(args: &[&str]) -> (String, i32) {
    let root = repo_root();
    let (program, mut invocation): (&str, Vec<String>) = if cfg!(windows) {
        let script = root.join("scripts").join("test-coverage-oracle.ps1");
        (
            "powershell",
            vec![
                "-NoProfile".to_owned(),
                "-ExecutionPolicy".to_owned(),
                "Bypass".to_owned(),
                "-File".to_owned(),
                script.to_string_lossy().into_owned(),
            ],
        )
    } else {
        let script = root.join("scripts").join("test-coverage-oracle.sh");
        ("bash", vec![script.to_string_lossy().into_owned()])
    };
    for arg in args {
        invocation.push((*arg).to_owned());
    }
    let output = Command::new(program)
        .args(&invocation)
        .current_dir(&root)
        .output()
        .unwrap_or_else(|err| panic!("failed to spawn coverage oracle: {err}"));
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    (combined, output.status.code().unwrap_or(-1))
}

/// Read the value of a `KEY=value` line from the oracle report.
fn field<'a>(out: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    out.lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()))
        .map(str::trim)
}

#[test]
fn scenario_a_omitted_required_target_is_reported_and_fails() {
    let (out, code) = run_oracle(&[
        "--mode",
        "report",
        "--changed",
        "src/db/workspace.rs",
        "--selected",
        "unit_query_tracing",
    ]);
    assert_ne!(code, 0, "oracle must exit non-zero on omission.\n{out}");
    assert_eq!(field(&out, "STATUS"), Some("FAIL"), "\n{out}");
    assert!(
        out.contains("contract_lifecycle"),
        "omitted list must name a required-but-omitted target.\n{out}"
    );
}

#[test]
fn scenario_b_unmapped_source_surface_fails() {
    let (out, code) = run_oracle(&[
        "--mode",
        "report",
        "--changed",
        "src/zzz_unmapped_surface/thing.rs",
    ]);
    assert_ne!(code, 0, "unmapped source surface must fail.\n{out}");
    assert_eq!(field(&out, "STATUS"), Some("FAIL"), "\n{out}");
    assert!(
        out.contains("src/zzz_unmapped_surface/thing.rs"),
        "unmapped surface must be named in the report.\n{out}"
    );
}

#[test]
fn scenario_c_fully_covered_diff_passes_with_zero_omitted() {
    let (out, code) = run_oracle(&["--mode", "report", "--changed", "src/db/workspace.rs"]);
    assert_eq!(code, 0, "fully covered diff must pass.\n{out}");
    assert_eq!(field(&out, "OMITTED_COUNT"), Some("0"), "\n{out}");
    assert_eq!(field(&out, "STATUS"), Some("PASS"), "\n{out}");
}

#[test]
fn scenario_c_run_plan_respects_concurrency_bound() {
    let (out, code) = run_oracle(&[
        "--mode",
        "run",
        "--dry-run",
        "--changed",
        "src/db/workspace.rs",
    ]);
    assert_eq!(code, 0, "bounded dry-run plan must succeed.\n{out}");
    let cap: usize = field(&out, "MAX_CONCURRENT_CAP")
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| panic!("MAX_CONCURRENT_CAP missing.\n{out}"));
    let peak: usize = field(&out, "PEAK_CONCURRENT")
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| panic!("PEAK_CONCURRENT missing.\n{out}"));
    assert!(cap >= 1, "concurrency cap must be at least 1.\n{out}");
    assert!(
        peak >= 1 && peak <= cap,
        "observed peak {peak} must stay within cap {cap}.\n{out}"
    );
}
