//! Tests for the supervisor invocation contract and dispatch.

use std::cell::Cell;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

use engram_indexer::preflight::{StageKind, VerificationError, Verifier};
use engram_indexer::preflight_invocation::{
    Invocation, PreflightOutput, SetupError, UsageError, deadline_from, main_with, parse_invocation,
};
use engram_indexer::preflight_verdict::{EXIT_USAGE, stage_name};

struct Mock {
    calls: Rc<Cell<usize>>,
}

fn mock() -> (Mock, Rc<Cell<usize>>) {
    let calls = Rc::new(Cell::new(0));
    (
        Mock {
            calls: Rc::clone(&calls),
        },
        calls,
    )
}

impl Verifier for Mock {
    fn verify(
        &mut self,
        _stage: StageKind,
        _deadline: Instant,
        _expected_generation: &str,
    ) -> Result<(), VerificationError> {
        self.calls.set(self.calls.get() + 1);
        Ok(())
    }
}

fn engram_exe() -> PathBuf {
    std::env::current_exe().expect("current exe")
}

fn argv(workspace: &Path, timeout: &str, engram: &Path) -> Vec<OsString> {
    vec![
        "--workspace".into(),
        workspace.as_os_str().to_owned(),
        "--timeout-ms".into(),
        timeout.into(),
        "--engram".into(),
        engram.as_os_str().to_owned(),
    ]
}

fn os(items: &[&str]) -> Vec<OsString> {
    items.iter().map(OsString::from).collect()
}

fn usage_via_main(args: Vec<OsString>) -> PreflightOutput {
    let made = Cell::new(false);
    let output = main_with(args, |_inv: &Invocation, _deadline| {
        made.set(true);
        Ok((mock().0, "gen".to_string()))
    });
    assert!(!made.get(), "factory must not run on a usage error");
    output
}

#[cfg(unix)]
fn non_utf8_case(exe: &Path) -> Vec<(&'static str, Vec<OsString>)> {
    use std::os::unix::ffi::OsStringExt;
    let bad = OsString::from_vec(vec![b'/', b't', 0xFF, 0xFE]);
    vec![(
        "non-utf8 workspace",
        vec![
            "--workspace".into(),
            bad,
            "--timeout-ms".into(),
            "5".into(),
            "--engram".into(),
            exe.as_os_str().to_owned(),
        ],
    )]
}

#[cfg(not(unix))]
fn non_utf8_case(_exe: &Path) -> Vec<(&'static str, Vec<OsString>)> {
    Vec::new()
}

fn assert_stderr_line(stderr: &str, stage: &str, cause: &str) {
    assert!(stderr.starts_with("engram-preflight "), "{stderr}");
    assert!(!stderr.starts_with("engram-preflight-relay "), "{stderr}");
    assert_eq!(stderr.trim_end_matches('\n').lines().count(), 1, "{stderr}");
    let line = stderr.trim_end_matches('\n');
    let prefix = format!("engram-preflight stage={stage} cause={cause} elapsed_ms=");
    let elapsed = line
        .strip_prefix(prefix.as_str())
        .unwrap_or_else(|| panic!("unexpected stderr line: {line}"));
    assert!(elapsed.parse::<u64>().is_ok(), "{line}");
}

#[test]
fn valid_spaced_invocation_parses_as_given() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let spaced = tmp.path().join("my work space");
    std::fs::create_dir(&spaced).expect("mkdir");
    let exe = engram_exe();

    let invocation = parse_invocation(argv(&spaced, "1500", &exe)).expect("valid invocation");
    assert_eq!(
        invocation.workspace,
        spaced.to_str().expect("utf-8 workspace")
    );
    assert_eq!(invocation.timeout_ms, 1500);
    assert!(invocation.engram.is_absolute());
    assert!(invocation.engram.is_file());

    let relative = os(&[
        "--workspace",
        spaced.to_str().expect("utf-8"),
        "--timeout-ms",
        "1",
        "--engram",
        "engram-relative",
    ]);
    assert_eq!(parse_invocation(relative), Err(UsageError::InvalidEngram));

    let missing = os(&[
        "--workspace",
        spaced.to_str().expect("utf-8"),
        "--timeout-ms",
        "1",
    ]);
    assert_eq!(parse_invocation(missing), Err(UsageError::MissingFlag));
}

#[test]
fn strict_parser_table_exits_two_without_stdout() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path().join("ws dir");
    std::fs::create_dir(&ws).expect("mkdir");
    let file = tmp.path().join("plain.txt");
    std::fs::write(&file, b"x").expect("write");
    let exe = engram_exe();
    let ws_s = ws.to_str().expect("utf-8").to_string();
    let exe_s = exe.to_str().expect("utf-8").to_string();

    assert!(deadline_from(Instant::now(), u64::MAX).is_err());
    let max_ms = u64::from(u32::MAX) * 1000;
    assert!(deadline_from(Instant::now(), max_ms).is_ok());
    assert_eq!(
        deadline_from(Instant::now(), max_ms + 1),
        Err(UsageError::DeadlineOverflow)
    );

    let base: Vec<(&str, Vec<OsString>)> = vec![
        ("timeout overflow", argv(&ws, "18446744073709551615", &exe)),
        (
            "absolute nonexistent engram",
            argv(&ws, "5", &tmp.path().join("no-engram")),
        ),
        ("directory as engram", argv(&ws, "5", tmp.path())),
        (
            "missing flag",
            os(&["--workspace", &ws_s, "--timeout-ms", "5"]),
        ),
        (
            "duplicate flag",
            os(&[
                "--workspace",
                &ws_s,
                "--workspace",
                &ws_s,
                "--timeout-ms",
                "5",
                "--engram",
                &exe_s,
            ]),
        ),
        (
            "unknown flag",
            os(&[
                "--workspace",
                &ws_s,
                "--timeout-ms",
                "5",
                "--engram",
                &exe_s,
                "--bogus",
                "1",
            ]),
        ),
        (
            "positional extra",
            os(&[
                "--workspace",
                &ws_s,
                "--timeout-ms",
                "5",
                "--engram",
                &exe_s,
                "extra",
            ]),
        ),
        (
            "non-numeric value",
            os(&[
                "--workspace",
                &ws_s,
                "--timeout-ms",
                "soon",
                "--engram",
                &exe_s,
            ]),
        ),
        (
            "flag used as value",
            os(&["--workspace", "--timeout-ms", "5", "--engram", &exe_s]),
        ),
        (
            "nonexistent workspace",
            argv(&tmp.path().join("absent"), "5", &exe),
        ),
        ("workspace is a file", argv(&file, "5", &exe)),
    ];

    for (name, args) in base.into_iter().chain(non_utf8_case(&exe)) {
        if name != "timeout overflow" {
            assert!(parse_invocation(args.clone()).is_err(), "{name}");
        }
        let output = usage_via_main(args);
        assert_eq!(output.exit_code, EXIT_USAGE, "{name}");
        assert_eq!(output.exit_code, 2, "{name}");
        assert_eq!(output.stdout, None, "{name}");
        assert_stderr_line(&output.stderr, "none", "usage");
        assert!(!output.stderr.contains(&ws_s), "{name}");
        assert!(!output.stderr.contains(&exe_s), "{name}");
    }
}

#[test]
fn main_with_maps_outcomes_to_verdict_and_stderr() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path().join("ws dir");
    std::fs::create_dir(&ws).expect("mkdir");
    let exe = engram_exe();
    let ws_s = ws.to_str().expect("utf-8").to_string();

    let (ok_mock, ok_calls) = mock();
    let calls = Cell::new(0usize);
    let ok_mock = Cell::new(Some(ok_mock));
    let ok = main_with(argv(&ws, "60000", &exe), |_inv: &Invocation, _deadline| {
        calls.set(calls.get() + 1);
        let verifier = ok_mock.take().expect("single make call");
        Ok((verifier, "gen-1".to_string()))
    });
    assert_eq!(calls.get(), 1);
    assert_eq!(ok_calls.get(), 7);
    assert_eq!(ok.stdout, Some(r#"{"state":"Succeeded"}"#));
    assert_eq!(ok.exit_code, 0);
    assert_stderr_line(&ok.stderr, "none", "ok");

    let setup = main_with(
        argv(&ws, "60000", &exe),
        |_inv: &Invocation, _deadline| -> Result<(Mock, String), SetupError> {
            Err(SetupError::Layout)
        },
    );
    assert_eq!(setup.stdout, Some(r#"{"state":"Failed","stage":"Build"}"#));
    assert_eq!(setup.exit_code, 1);
    let build = stage_name(engram_indexer::preflight::Failure::Build);
    assert_stderr_line(&setup.stderr, build, "setup");

    let setup_expired = main_with(
        argv(&ws, "0", &exe),
        |_inv: &Invocation, _deadline| -> Result<(Mock, String), SetupError> {
            Err(SetupError::Layout)
        },
    );
    assert_eq!(setup_expired.exit_code, 1);
    assert_stderr_line(&setup_expired.stderr, build, "setup");

    let (expired_mock, expired_calls) = mock();
    let expired_mock = Cell::new(Some(expired_mock));
    let expired = main_with(argv(&ws, "0", &exe), |_inv: &Invocation, _deadline| {
        let verifier = expired_mock.take().expect("single make call");
        Ok((verifier, "gen-2".to_string()))
    });
    assert_eq!(expired_calls.get(), 0);
    assert_eq!(
        expired.stdout,
        Some(r#"{"state":"Failed","stage":"Build"}"#)
    );
    assert_eq!(expired.exit_code, 1);
    assert_stderr_line(&expired.stderr, build, "deadline");

    for output in [&ok, &setup, &setup_expired, &expired] {
        assert!(!output.stderr.contains(&ws_s));
        assert!(!output.stderr.contains(exe.to_str().expect("utf-8")));
    }
}
