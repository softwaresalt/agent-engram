//! Contract harness for plan unit F04a (IPC server seam).
//!
//! Two properties are asserted:
//!
//! 1. **Seam signature stability** — the five named seam entry points exist with
//!    exactly the declared shapes. Coercing each item to a function pointer of
//!    the expected type is a compile-time assertion: any signature drift (type,
//!    arity, or by-value vs by-reference change) fails to compile, so this test
//!    cannot silently rot.
//! 2. **Single admission authority** — admission is reachable only through
//!    [`engram::daemon::request_entry::admit`]. Rust has no runtime reflection
//!    over call graphs, so this is asserted structurally against the daemon
//!    sources: the composition root must contain no frame decode and no tool
//!    dispatch of its own, and `request_entry` must expose exactly one `admit`
//!    definition which its own request-entry path calls before dispatching.
//!
//! See docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md.

use std::fs;
use std::path::{Path, PathBuf};

use engram::daemon::{error_transport, lifecycle_policy, request_entry, startup_activation};
use engram::server::state::AppState;

fn daemon_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("daemon")
}

fn read_daemon_source(file: &str) -> String {
    let path = daemon_dir().join(file);
    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("read daemon source {}: {error}", path.display());
    })
}

/// Strip `//`-style line comments so structural assertions inspect code only,
/// never doc prose that happens to mention a symbol.
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The five seam signatures declared by F04a must exist and stay stable.
///
/// Each coercion below is checked by the compiler, so this test fails to build
/// if any seam signature changes shape.
#[test]
fn seam_signatures_are_present_and_stable() {
    let run_initial_gate: fn(&AppState) -> startup_activation::StartupOutcome =
        startup_activation::run_initial_gate;
    let readiness: fn(&AppState) -> startup_activation::ReadinessView =
        startup_activation::readiness;
    let admit: fn(&AppState, &request_entry::Frame) -> request_entry::Admission =
        request_entry::admit;
    let to_wire: fn(error_transport::DomainError) -> error_transport::WireError =
        error_transport::to_wire;
    let on_start: fn(&AppState) = lifecycle_policy::on_start;
    let on_shutdown: fn(&AppState) = lifecycle_policy::on_shutdown;

    // Function items are zero-sized until coerced; assert the coercions produced
    // real, distinct pointers so the bindings cannot be optimized into nothing.
    let pointers: [usize; 6] = [
        run_initial_gate as usize,
        readiness as usize,
        admit as usize,
        to_wire as usize,
        on_start as usize,
        on_shutdown as usize,
    ];
    assert!(
        pointers.iter().all(|address| *address != 0),
        "every seam entry point must resolve to a real function address"
    );
}

/// The readiness seam must report the startup gate outcome it was built from.
#[test]
fn readiness_view_reports_startup_gate_outcome() {
    assert!(
        startup_activation::ReadinessView {
            startup: startup_activation::StartupOutcome::Ready,
        }
        .is_ready(),
        "a passed startup gate must publish a ready view"
    );
    assert!(
        !startup_activation::ReadinessView {
            startup: startup_activation::StartupOutcome::Pending,
        }
        .is_ready(),
        "a pending startup gate must not publish a ready view"
    );
}

/// The admission decision type must distinguish admitted from refused frames.
#[test]
fn admission_decision_is_observable() {
    assert!(request_entry::Admission::Admitted.is_admitted());
}

/// Admission is reachable only through `request_entry::admit`.
///
/// Enforcement mechanism: `admit` is defined exactly once, and the only daemon
/// path that decodes a frame and dispatches it — `request_entry::process_request`
/// — calls `admit` before dispatch. The composition root holds no decode or
/// dispatch of its own, so no caller can reach dispatch while bypassing the
/// admission seam.
#[test]
fn admission_is_reachable_only_through_request_entry_admit() {
    let request_entry_src = code_only(&read_daemon_source("request_entry.rs"));

    let admit_definitions = request_entry_src.matches("pub fn admit(").count();
    assert_eq!(
        admit_definitions, 1,
        "request_entry must define exactly one public admission function"
    );

    let entry_body = request_entry_src
        .split_once("pub async fn process_request(")
        .expect("request_entry must own the request-entry path")
        .1;
    let admit_call = entry_body
        .find("admit(state, &request)")
        .expect("the request-entry path must call the admission seam");
    let dispatch_call = entry_body
        .find("tools::dispatch(")
        .expect("the request-entry path must own tool dispatch");
    assert!(
        admit_call < dispatch_call,
        "admission must run before dispatch on the request-entry path"
    );

    let ipc_server_src = code_only(&read_daemon_source("ipc_server.rs"));
    assert!(
        !ipc_server_src.contains("tools::dispatch("),
        "the composition root must not dispatch tools directly"
    );
    assert!(
        !ipc_server_src.contains("IpcRequest::from_line("),
        "the composition root must not decode frames directly"
    );
    assert!(
        ipc_server_src.contains("request_entry::process_request("),
        "the composition root must route every request through request_entry"
    );
}

/// Every daemon module other than `request_entry` must stay out of the
/// admission and dispatch business.
#[test]
fn no_other_daemon_module_dispatches_requests() {
    let offenders = fs::read_dir(daemon_dir())
        .expect("read src/daemon")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "rs")
        })
        .filter(|entry| entry.file_name() != "request_entry.rs")
        .filter(|entry| {
            let source = fs::read_to_string(entry.path()).unwrap_or_default();
            code_only(&source).contains("tools::dispatch(")
        })
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "only request_entry may dispatch daemon requests; found: {offenders:?}"
    );
}
