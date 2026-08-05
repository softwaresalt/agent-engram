//! Integration tests for the cross-file calls resolution post-pass (082.008-T).
//!
//! The post-pass resolves staged cross-file calls (082.002-T) against a
//! workspace-global `name -> [function_id]` index and creates a `calls_edge`
//! ONLY for unambiguous (exactly-one-definition) names, tagged with the
//! canonical provenance `calls_resolved_singleton`. It runs in the full /
//! `--force` index path only — never on incremental sync.
//!
//! Scenarios (8):
//!   1. unique cross-file name -> one edge tagged `calls_resolved_singleton`
//!   2. ambiguous name (2 defs) -> skipped (no edge)
//!   3. name with no matching def -> no edge (no false edge)
//!   4. incremental sync path -> post-pass NOT invoked (no singleton edges)
//!   5. a singleton that became ambiguous (2nd same-named def added) is
//!      retracted by re-resolution even when the caller file is unchanged
//!   6. with EMPTY staging (post-rehydration / fresh upgrade) the post-pass
//!      PRESERVES existing singleton edges instead of destroying them
//!   7. re-running the post-pass on an UNCHANGED workspace reports zero newly
//!      created edges (`resolved == 0`) — pre-existing singletons are not
//!      recounted even though staged rows persist and are re-upserted
//!   8. ignored/opt-in: one owned daemon preserves the known-GREEN singleton
//!      across its index, flush, same-endpoint query, timeout, and
//!      response-frame boundaries

#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::doc_markdown)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use engram::daemon::protocol::{IpcRequest, IpcResponse};
use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::errors::{EngramError, IpcError};
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;
use engram::shim::ipc_client::{probe, send_request};
use engram::shim::pidfile::PidFile;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[path = "../helpers/mod.rs"]
mod helpers;

fn write_sample_file(dir: &Path, rel_path: &str, content: &str) {
    let full = dir.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create dirs");
    }
    fs::write(full, content).expect("write file");
}

fn test_db_params(path: &Path) -> (std::path::PathBuf, String) {
    use sha2::{Digest, Sha256};
    let canon = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase();
    let branch = format!("{:x}", Sha256::digest(canon.as_bytes()));
    (std::env::temp_dir().join("engram-test"), branch)
}

async fn queries_for(data_dir: &Path, branch: &str) -> CodeGraphQueries {
    let db = connect_db(data_dir, branch).await.expect("connect_db");
    CodeGraphQueries::new(db)
}

async fn singleton_count(q: &CodeGraphQueries) -> u64 {
    q.count_calls_edges_by_resolution()
        .await
        .expect("count_calls_edges_by_resolution")
        .get("calls_resolved_singleton")
        .copied()
        .unwrap_or(0)
}

// Scenario 1: an unambiguous cross-file callee yields exactly one
// calls_resolved_singleton edge.
#[test]
async fn unique_cross_file_name_resolves_to_singleton_edge() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "src/a.rs", "pub fn caller() {\n    helper();\n}\n");
    write_sample_file(ws, "src/b.rs", "pub fn helper() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");

    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(
        singleton_count(&q).await,
        1,
        "unambiguous cross-file call must create one singleton edge"
    );
    let singletons = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singletons");
    assert_eq!(
        singletons.len(),
        1,
        "exactly one singleton edge, {singletons:?}"
    );
}

// Scenario 2: an ambiguous name (two definitions) is skipped — no edge.
#[test]
async fn ambiguous_name_is_skipped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "src/a.rs", "pub fn caller() {\n    dup();\n}\n");
    write_sample_file(ws, "src/b.rs", "pub fn dup() {}\n");
    write_sample_file(ws, "src/c.rs", "pub fn dup() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");

    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "ambiguous name (2 defs) must not create a singleton edge"
    );
}

// Scenario 3: a call to a name with no definition creates no false edge.
#[test]
async fn unmatched_name_creates_no_false_edge() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(
        ws,
        "src/a.rs",
        "pub fn caller() {\n    absent_target_fn();\n}\n",
    );

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");

    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "unmatched callee name must not create a false singleton edge"
    );
}

// Scenario 4: the incremental sync path does NOT invoke the post-pass.
#[test]
async fn sync_path_does_not_invoke_postpass() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "src/a.rs", "pub fn caller() {\n    helper();\n}\n");
    write_sample_file(ws, "src/b.rs", "pub fn helper() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    // First-time sync exercises the incremental path (all files "added").
    code_graph::sync_workspace(ws, &data_dir, &branch, &config)
        .await
        .expect("sync");

    let q = queries_for(&data_dir, &branch).await;
    // The call is staged (082.002-T) but never promoted to a singleton edge,
    // because the post-pass runs on full index only.
    assert_eq!(
        singleton_count(&q).await,
        0,
        "sync path must not create singleton edges"
    );
    let staged = q.list_staged_calls().await.expect("list_staged_calls");
    assert!(
        staged.iter().any(|s| s.callee_name == "helper"),
        "sync path must still stage the unresolved cross-file call, got {staged:?}"
    );
}

// Scenario 5: a singleton resolved on a prior index must be RETRACTED by
// re-resolution once its callee name becomes ambiguous (a second same-named
// definition is added). The caller file is unchanged, so a non-forced full
// index skips it; the post-pass alone must revalidate and drop the now-invalid
// singleton rather than leaving it stale.
#[test]
async fn reresolution_retracts_now_ambiguous_singleton() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "src/a.rs", "pub fn caller() {\n    helper();\n}\n");
    write_sample_file(ws, "src/b.rs", "pub fn helper() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index 1");
    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(singleton_count(&q).await, 1, "baseline singleton edge");

    // Add a SECOND `helper` definition -> the name is now ambiguous. The caller
    // file is unchanged, so a non-forced index skips it and the post-pass must
    // retract the stale singleton on its own.
    write_sample_file(ws, "src/c.rs", "pub fn helper() {}\n");
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index 2");

    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(
        singleton_count(&q).await,
        0,
        "a singleton that became ambiguous must be retracted by re-resolution"
    );
}

// Scenario 6: the post-pass must PRESERVE existing singleton edges when the
// staging relation is empty. This models JSONL rehydration and a fresh upgrade,
// where singleton edges are restored but `staged_call` rows are not — a
// destructive global retract-then-rebuild would wrongly delete every singleton.
#[test]
async fn postpass_preserves_singletons_when_staging_empty() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "src/a.rs", "pub fn caller() {\n    helper();\n}\n");
    write_sample_file(ws, "src/b.rs", "pub fn helper() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");
    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(singleton_count(&q).await, 1, "baseline singleton edge");

    // Model rehydration: the singleton edge exists but staging is empty.
    q.clear_staged_calls_for_file("src/a.rs")
        .await
        .expect("clear staging");
    assert!(
        q.list_staged_calls()
            .await
            .expect("list_staged_calls")
            .is_empty(),
        "staging must be empty to model the rehydration case"
    );

    // Re-running the post-pass must NOT destroy the existing singleton.
    q.reresolve_calls_edges().await.expect("reresolve");
    assert_eq!(
        singleton_count(&q).await,
        1,
        "post-pass must preserve singletons when staging is empty"
    );
}

// Scenario 7: re-running the post-pass on an unchanged workspace must report
// zero NEWLY created edges. Staged rows persist across a non-forced re-index, so
// the post-pass re-upserts every prior singleton; `resolved` must count only
// genuinely new provenance, otherwise `IndexResult.edges_created` over-reports on
// every no-op re-index.
#[test]
async fn reresolve_reports_zero_new_edges_on_unchanged_rerun() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_sample_file(ws, "src/a.rs", "pub fn caller() {\n    helper();\n}\n");
    write_sample_file(ws, "src/b.rs", "pub fn helper() {}\n");

    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);
    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("index");
    let q = queries_for(&data_dir, &branch).await;
    assert_eq!(singleton_count(&q).await, 1, "baseline singleton edge");

    // Staging persists after indexing, so the direct post-pass re-upserts the
    // same singleton — but it already exists, so nothing new is created.
    assert!(
        !q.list_staged_calls()
            .await
            .expect("list_staged_calls")
            .is_empty(),
        "staged rows must persist to exercise the re-upsert path"
    );
    let result = q.reresolve_calls_edges().await.expect("reresolve");
    assert_eq!(
        result.resolved, 0,
        "re-resolving an unchanged workspace must report zero new edges, got {result:?}"
    );
    assert_eq!(
        singleton_count(&q).await,
        1,
        "the singleton edge count is unchanged after a no-op re-resolution"
    );
}

const PROBE_LIMIT: Duration = Duration::from_secs(300);
const CLEANUP_RESERVE: Duration = Duration::from_secs(12);
const READY_LIMIT: Duration = Duration::from_secs(60);
const IPC_LIMIT: Duration = Duration::from_secs(30);
const FORCED_REAP_LIMIT: Duration = Duration::from_secs(5);
const SHORT_TIMEOUT: Duration = Duration::from_millis(10);
const CORPUS: [(&str, &str); 2] = [
    ("src/a.rs", "pub fn alpha() {\n    beta();\n}\n"),
    ("src/b.rs", "pub fn beta() {}\n"),
];

#[derive(Clone, Copy)]
struct EndToEndDeadline {
    start: Instant,
    work: Instant,
    aggregate: Instant,
}

impl EndToEndDeadline {
    fn start(limit: Duration) -> Self {
        assert!(
            limit > CLEANUP_RESERVE,
            "probe limit must preserve a bounded child-cleanup reserve"
        );
        let start = Instant::now();
        let aggregate = start
            .checked_add(limit)
            .unwrap_or_else(|| panic!("cannot represent aggregate probe deadline"));
        let work = aggregate
            .checked_sub(CLEANUP_RESERVE)
            .unwrap_or_else(|| panic!("cannot represent probe cleanup reserve"));
        Self {
            start,
            work,
            aggregate,
        }
    }

    fn remaining_work(self, cap: Duration, phase: &str) -> Duration {
        let remaining = self.work.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "five-minute aggregate probe budget exhausted before {phase}"
        );
        remaining.min(cap)
    }

    fn remaining_cleanup(self, cap: Duration, phase: &str) -> Duration {
        let remaining = self.aggregate.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "five-minute aggregate probe budget exhausted during {phase}"
        );
        remaining.min(cap)
    }

    fn phase_deadline(self, cap: Duration, phase: &str) -> Instant {
        Instant::now()
            .checked_add(self.remaining_work(cap, phase))
            .unwrap_or(self.work)
    }

    fn remaining_in_phase(self, phase_deadline: Instant, cap: Duration, phase: &str) -> Duration {
        let phase_remaining = phase_deadline.saturating_duration_since(Instant::now());
        assert!(!phase_remaining.is_zero(), "{phase} deadline exhausted");
        self.remaining_work(cap, phase).min(phase_remaining)
    }

    fn elapsed_ms(self) -> u64 {
        elapsed_ms(self.start)
    }
}

fn init_git_workspace(root: &Path) {
    let git = root.join(".git");
    fs::create_dir_all(&git).expect("create isolated .git");
    fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("write isolated HEAD");
}

fn write_frozen_corpus(root: &Path) {
    for (path, content) in CORPUS {
        write_sample_file(root, path, content);
    }
}

fn corpus_hashes(root: &Path) -> (String, BTreeMap<String, String>) {
    let mut aggregate = Sha256::new();
    let files = CORPUS
        .iter()
        .map(|(relative, _)| {
            let bytes = fs::read(root.join(relative)).expect("read frozen corpus file");
            aggregate.update(relative.as_bytes());
            aggregate.update([0]);
            aggregate.update(&bytes);
            aggregate.update([0xff]);
            ((*relative).to_owned(), hex::encode(Sha256::digest(&bytes)))
        })
        .collect();
    (hex::encode(aggregate.finalize()), files)
}

fn now_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn normalized_path_text(path: &str) -> String {
    path.strip_prefix(r"\\?\")
        .unwrap_or(path)
        .replace('\\', "/")
        .to_lowercase()
}

fn elapsed_ms(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn correlated_request(id: &str, method: &str, mut params: Value) -> IpcRequest {
    params
        .as_object_mut()
        .expect("correlated request params must be an object")
        .insert("_meta".to_owned(), json!({ "correlation_id": id }));
    IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::String(id.to_owned())),
        method: method.to_owned(),
        params: Some(params),
    }
}

fn internal_request(id: &str, method: &str) -> IpcRequest {
    IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::String(id.to_owned())),
        method: method.to_owned(),
        params: None,
    }
}

fn assert_response_id(request: &IpcRequest, response: &IpcResponse) {
    assert_eq!(
        request.id.as_ref(),
        Some(&response.id),
        "daemon must echo the correlated JSON-RPC request id"
    );
}

async fn send_ok(
    endpoint: &str,
    request: &IpcRequest,
    timeout: Duration,
    deadline: EndToEndDeadline,
) -> Value {
    let request_timeout = deadline.remaining_work(timeout, &request.method);
    let response = send_request(endpoint, request, request_timeout)
        .await
        .unwrap_or_else(|error| panic!("{} IPC request failed: {error}", request.method));
    assert_response_id(request, &response);
    assert!(
        response.error.is_none(),
        "{} returned an error: {:?}",
        request.method,
        response.error
    );
    response
        .result
        .unwrap_or_else(|| panic!("{} returned no result", request.method))
}

fn empty_settled(status: &Value) -> bool {
    let graph = &status["code_graph"];
    let graph_empty = ["code_files", "functions", "classes", "interfaces", "edges"]
        .iter()
        .all(|field| graph[*field].as_u64() == Some(0));
    let scan_running = status
        .pointer("/scan_status/running")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    graph_empty && !scan_running
}

async fn wait_for_empty_daemon(endpoint: &str, deadline: EndToEndDeadline) -> (Value, u32) {
    let phase_deadline = deadline.phase_deadline(READY_LIMIT, "empty-daemon settlement");
    let mut prior_completion = None;
    let mut stable_observations = 0_u8;
    let mut attempt = 0_u32;

    loop {
        attempt = attempt.saturating_add(1);
        let id = format!("107s-settle-{attempt:02}");
        let status = send_ok(
            endpoint,
            &correlated_request(&id, "get_workspace_status", json!({})),
            deadline.remaining_in_phase(phase_deadline, IPC_LIMIT, "empty-daemon status request"),
            deadline,
        )
        .await;
        let completion = status
            .pointer("/scan_status/last_completed_at")
            .cloned()
            .unwrap_or(Value::Null);

        if empty_settled(&status) && prior_completion.as_ref() == Some(&completion) {
            stable_observations = stable_observations.saturating_add(1);
        } else if empty_settled(&status) {
            stable_observations = 1;
        } else {
            stable_observations = 0;
        }
        prior_completion = Some(completion);

        if stable_observations >= 2 {
            return (status, attempt);
        }
        assert!(
            Instant::now() < phase_deadline,
            "owned daemon did not settle on an empty graph within {READY_LIMIT:?}"
        );
        tokio::time::sleep(deadline.remaining_in_phase(
            phase_deadline,
            Duration::from_millis(100),
            "empty-daemon settlement delay",
        ))
        .await;
    }
}

fn watcher_event_count(daemon_status: &Value) -> u64 {
    let message = daemon_status
        .pointer("/health/checks")
        .and_then(Value::as_array)
        .and_then(|checks| {
            checks
                .iter()
                .find(|check| check["name"] == "telemetry_health")
        })
        .and_then(|check| check["message"].as_str())
        .expect("daemon status must expose telemetry health");
    if message == "no telemetry recorded yet (daemon just started)" {
        return 0;
    }
    message
        .split(',')
        .nth(1)
        .and_then(|part| part.split_whitespace().next())
        .and_then(|count| count.parse().ok())
        .unwrap_or_else(|| panic!("cannot parse watcher count from `{message}`"))
}

async fn wait_for_usage_record(
    root: &Path,
    correlation_id: &str,
    deadline: EndToEndDeadline,
) -> Value {
    let usage_path = root
        .join(".engram")
        .join("metrics")
        .join("main")
        .join("usage.jsonl");
    let phase_deadline = deadline.phase_deadline(READY_LIMIT, "usage telemetry persistence");

    loop {
        if let Ok(content) = tokio::fs::read_to_string(&usage_path).await {
            for line in content.lines().filter(|line| !line.trim().is_empty()) {
                if let Ok(record) = serde_json::from_str::<Value>(line)
                    && record["correlation_id"] == correlation_id
                {
                    return record;
                }
            }
        }
        assert!(
            Instant::now() < phase_deadline,
            "usage telemetry `{correlation_id}` did not persist within {READY_LIMIT:?}"
        );
        tokio::time::sleep(deadline.remaining_in_phase(
            phase_deadline,
            Duration::from_millis(25),
            "usage telemetry poll delay",
        ))
        .await;
    }
}

fn assert_singleton_visible(map: &Value) -> usize {
    assert_eq!(
        map.pointer("/root/name").and_then(Value::as_str),
        Some("beta")
    );
    assert_eq!(map["fallback_used"], false);
    let alpha = map["neighbors"]
        .as_array()
        .expect("map_code neighbors")
        .iter()
        .find(|node| node["name"] == "alpha")
        .expect("alpha must be an incoming beta neighbor");
    let alpha_id = alpha["id"].as_str().expect("alpha id");
    let beta_id = map
        .pointer("/root/id")
        .and_then(Value::as_str)
        .expect("beta id");
    let calls: Vec<&Value> = map["edges"]
        .as_array()
        .expect("map_code edges")
        .iter()
        .filter(|edge| edge["type"] == "calls")
        .collect();
    assert_eq!(
        calls.len(),
        1,
        "bare cross-file fixture must expose exactly one calls edge: {map}"
    );
    assert_eq!(calls[0]["from"], alpha_id);
    assert_eq!(calls[0]["to"], beta_id);
    calls.len()
}

fn singleton_rows(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read persisted edges mirror {}: {error}", path.display()))
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Value>(line).expect("valid persisted edge JSONL"))
        .filter(|edge| edge["resolution"] == "calls_resolved_singleton")
        .collect()
}

fn strip_ansi(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut in_escape = false;
    for character in line.chars() {
        if in_escape {
            if character == 'm' {
                in_escape = false;
            }
        } else if character == '\u{1b}' {
            in_escape = true;
        } else {
            output.push(character);
        }
    }
    output
}

fn trace_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read daemon trace {}: {error}", path.display()))
        .lines()
        .map(strip_ansi)
        .collect()
}

fn trace_event_indices(lines: &[String], marker: &str) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| line.contains(marker).then_some(index))
        .collect()
}

fn trace_timestamp(lines: &[String], event_index: usize) -> String {
    let start = event_index.saturating_sub(2);
    lines[start..=event_index]
        .iter()
        .rev()
        .flat_map(|line| line.split_whitespace())
        .find_map(|token| {
            let candidate = token.trim_matches(|character: char| {
                !(character.is_ascii_alphanumeric() || matches!(character, '-' | ':' | '.' | '+'))
            });
            chrono::DateTime::parse_from_rfc3339(candidate)
                .is_ok()
                .then(|| candidate.to_owned())
        })
        .expect("pretty trace event must carry an RFC3339 timestamp")
}

fn frame_boundary(lines: &[String], completion_index: usize) -> (usize, Option<usize>) {
    let close_index = lines
        .iter()
        .enumerate()
        .skip(completion_index + 1)
        .find_map(|(index, line)| line.contains("ipc_connection_closed").then_some(index))
        .expect("index request must reach a response-frame close boundary");
    let error_index = lines[completion_index + 1..close_index]
        .iter()
        .position(|line| {
            line.contains("failed to write IPC response")
                || line.contains("failed to flush IPC response")
        })
        .map(|relative| completion_index + 1 + relative);
    (close_index, error_index)
}

async fn wait_for_child_exit(
    harness: &mut helpers::HarnessWithoutOwnership,
    timeout: Duration,
) -> Option<ExitStatus> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .unwrap_or_else(|| panic!("cannot represent child-exit deadline"));
    loop {
        match harness.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {}
            Err(error) => panic!("cannot poll owned daemon child: {error}"),
        }
        if Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[cfg(test)]
mod trace_parser_fixture_tests {
    use super::{frame_boundary, strip_ansi, trace_event_indices, trace_timestamp};

    #[test]
    fn synthetic_fixture_preserves_all_events_and_pretty_timestamps() {
        // Synthetic cardinality exercises parser mechanics only; it is not
        // runtime evidence about how many post-pass events a daemon emits.
        let lines = [
            "\u{1b}[2m2026-08-05T01:02:03.004Z\u{1b}[0m",
            "code graph: resolved cross-file singleton calls edges",
            "2026-08-05T01:02:04.005Z code graph: indexing complete",
            "2026-08-05T01:02:05.006Z code graph: resolved cross-file singleton calls edges",
        ]
        .map(strip_ansi);

        assert_eq!(
            trace_event_indices(
                &lines,
                "code graph: resolved cross-file singleton calls edges"
            ),
            vec![1, 3]
        );
        assert_eq!(trace_timestamp(&lines, 1), "2026-08-05T01:02:03.004Z");
        assert_eq!(trace_timestamp(&lines, 2), "2026-08-05T01:02:04.005Z");
    }

    #[test]
    fn synthetic_fixture_bounds_frame_errors_before_connection_close() {
        let lines = [
            "2026-08-05T01:02:03.004Z code graph: indexing complete",
            "2026-08-05T01:02:03.005Z failed to flush IPC response",
            "2026-08-05T01:02:03.006Z ipc_connection_closed",
            "2026-08-05T01:02:03.007Z failed to write IPC response",
        ]
        .map(str::to_owned);

        let (close_index, error_index) = frame_boundary(&lines, 0);
        assert_eq!(close_index, 2);
        assert_eq!(error_index, Some(1));
        assert_eq!(
            trace_timestamp(&lines, close_index),
            "2026-08-05T01:02:03.006Z"
        );
    }
}

// Scenario 8 / 107-S: characterize, rather than repair, the controlled daemon
// persistence and IPC boundaries that were confounded in deliberation 015-D.
#[test]
#[ignore = "107-S PARTIAL: opt-in only; two-run circuit cap exhausted before cold CLI request-ID/frame correlation"]
async fn daemon_index_runtime_boundaries_characterized() {
    let deadline = EndToEndDeadline::start(PROBE_LIMIT);
    tokio::time::timeout(
        deadline.remaining_work(PROBE_LIMIT, "controlled runtime characterization"),
        characterize_daemon_index_runtime_boundaries(deadline),
    )
    .await
    .expect("controlled runtime characterization exceeded the five-minute aggregate cap");
}

#[allow(clippy::too_many_lines)]
async fn characterize_daemon_index_runtime_boundaries(deadline: EndToEndDeadline) {
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .expect("repository canonical path");
    let repository_data_identity = repository_root.join(".engram");
    let repository_endpoint = engram::daemon::ipc_server::ipc_endpoint(&repository_root)
        .expect("repository IPC endpoint");
    let repository_pid = PidFile::read(&repository_root).map(|pid| pid.pid);

    let baseline = tempfile::tempdir().expect("baseline tempdir");
    let baseline_root = baseline
        .path()
        .canonicalize()
        .expect("baseline canonical path");
    helpers::verify_workspace_isolated_from_repository(&baseline_root)
        .expect("baseline workspace must be disjoint from repository identities");
    init_git_workspace(&baseline_root);
    write_frozen_corpus(&baseline_root);
    let (baseline_hash, baseline_file_hashes) = corpus_hashes(&baseline_root);
    let baseline_data = baseline_root.join(".engram");
    let baseline_db = baseline_data.join("cozo").join("main").join("engram.db");

    let baseline_started_at = now_timestamp();
    let baseline_started = Instant::now();
    let baseline_result = tokio::time::timeout(
        deadline.remaining_work(PROBE_LIMIT, "known-GREEN baseline index"),
        code_graph::index_workspace(
            &baseline_root,
            &baseline_data,
            "main",
            &CodeGraphConfig::default(),
            false,
        ),
    )
    .await
    .expect("baseline index exceeded five-minute circuit breaker")
    .expect("known-GREEN baseline index");
    let baseline_elapsed_ms = elapsed_ms(baseline_started);
    let baseline_completed_at = now_timestamp();
    let baseline_queries = queries_for(&baseline_data, "main").await;
    let baseline_singletons = singleton_count(&baseline_queries).await;
    assert_eq!(
        baseline_singletons, 1,
        "known-GREEN in-process fixture must persist one singleton"
    );
    assert!(baseline_db.is_file(), "baseline database must persist");
    drop(baseline_queries);

    let daemon = tempfile::tempdir().expect("daemon tempdir");
    let daemon_root = daemon.path().canonicalize().expect("daemon canonical path");
    helpers::verify_workspace_isolated_from_repository(&daemon_root)
        .expect("daemon workspace must be disjoint from repository identities");
    assert_ne!(
        baseline_root, daemon_root,
        "baseline and daemon workspaces must be distinct"
    );
    init_git_workspace(&daemon_root);
    let daemon_engram = daemon_root.join(".engram");
    assert_ne!(
        daemon_engram, repository_data_identity,
        "owned daemon data identity must differ from repository-owned data"
    );
    assert_ne!(
        baseline_data, repository_data_identity,
        "baseline data identity must differ from repository-owned data"
    );
    let expected_daemon_endpoint =
        engram::daemon::ipc_server::ipc_endpoint(&daemon_root).expect("owned daemon IPC endpoint");
    assert_ne!(
        normalized_path_text(&expected_daemon_endpoint),
        normalized_path_text(&repository_endpoint),
        "owned endpoint must differ from the repository-owned endpoint"
    );
    fs::create_dir_all(&daemon_engram).expect("create daemon data directory");
    fs::write(
        daemon_engram.join("config.toml"),
        r#"idle_timeout_minutes = 0
debounce_ms = 50
watch_patterns = ["**/*"]
exclude_patterns = [".engram/", ".git/", "node_modules/", "target/", "src/"]
log_level = "debug"
log_format = "json"

[metrics]
enabled = true
buffer_size = 1024
"#,
    )
    .expect("write watcher-hold config");
    let trace_path = daemon_engram.join("107s-daemon-trace.log");
    let stderr_path = daemon_engram.join("107s-daemon-stderr.log");

    let cold_started_at = now_timestamp();
    let cold_started = Instant::now();
    let mut harness = helpers::DaemonHarness::spawn_for_workspace_with_trace_log(
        &daemon_root,
        &trace_path,
        &stderr_path,
        deadline.remaining_work(READY_LIMIT, "cold daemon readiness"),
    )
    .await
    .expect("one owned daemon must become ready");
    let cold_ready_ms = elapsed_ms(cold_started);
    let cold_ready_at = now_timestamp();
    let endpoint = harness
        .ipc_path()
        .to_str()
        .expect("UTF-8 endpoint")
        .to_owned();
    assert_eq!(
        normalized_path_text(&endpoint),
        normalized_path_text(&expected_daemon_endpoint),
        "owned child must serve the endpoint derived for its isolated workspace"
    );
    assert_ne!(
        normalized_path_text(&endpoint),
        normalized_path_text(&repository_endpoint),
        "owned child endpoint must not alias the repository daemon"
    );
    let owned_pid = harness.pid();
    if let Some(repository_pid) = repository_pid {
        assert_ne!(
            owned_pid, repository_pid,
            "owned child PID must differ from the repository daemon PID"
        );
    }

    let health = send_ok(
        &endpoint,
        &internal_request("107s-health-ready", "_health"),
        IPC_LIMIT,
        deadline,
    )
    .await;
    assert_eq!(health["status"], "ready");
    assert_eq!(
        normalized_path_text(health["workspace"].as_str().expect("health workspace path")),
        normalized_path_text(&daemon_root.display().to_string())
    );
    let build_hash = health["build_hash"]
        .as_str()
        .expect("health build hash")
        .to_owned();
    let protocol_version = health["protocol_version"]
        .as_u64()
        .expect("health protocol version");
    let pid_file = PidFile::read(&daemon_root).expect("owned daemon PID file");
    assert_eq!(
        pid_file.pid, owned_pid,
        "circuit breaker: endpoint and child PID identities must agree"
    );
    assert!(
        pid_file.verify_alive().expect("verify owned daemon PID"),
        "owned daemon PID must be live"
    );

    let (settled_status, settle_attempts) = wait_for_empty_daemon(&endpoint, deadline).await;
    assert!(empty_settled(&settled_status));
    let settled_scan_completion = settled_status
        .pointer("/scan_status/last_completed_at")
        .cloned()
        .unwrap_or(Value::Null);
    let daemon_db = PathBuf::from(
        settled_status["db_path"]
            .as_str()
            .expect("daemon database path"),
    );
    assert_ne!(
        baseline_db, daemon_db,
        "baseline and daemon databases must be distinct"
    );
    let daemon_before = send_ok(
        &endpoint,
        &correlated_request("107s-daemon-before", "get_daemon_status", json!({})),
        IPC_LIMIT,
        deadline,
    )
    .await;
    assert_eq!(daemon_before["active_workspaces"].as_u64(), Some(1));
    assert_eq!(
        daemon_before.pointer("/telemetry/duplicate_daemon_detected"),
        Some(&Value::from(0_u64)),
        "circuit breaker: a second daemon identity was detected"
    );
    assert_eq!(watcher_event_count(&daemon_before), 0);
    let model_loaded_before = daemon_before["model_loaded"].as_bool().unwrap_or(false);

    write_frozen_corpus(&daemon_root);
    let (daemon_hash, daemon_file_hashes) = corpus_hashes(&daemon_root);
    assert_eq!(daemon_hash, baseline_hash, "aggregate corpus bytes drifted");
    assert_eq!(
        daemon_file_hashes, baseline_file_hashes,
        "per-file corpus bytes drifted"
    );
    tokio::time::sleep(
        deadline.remaining_work(Duration::from_millis(250), "post-seed watcher hold"),
    )
    .await;
    let seeded_status = send_ok(
        &endpoint,
        &correlated_request("107s-seeded-status", "get_workspace_status", json!({})),
        IPC_LIMIT,
        deadline,
    )
    .await;
    assert!(
        empty_settled(&seeded_status),
        "watcher or background ingestion populated the graph before explicit index: {seeded_status}"
    );
    assert_eq!(
        seeded_status
            .pointer("/scan_status/last_completed_at")
            .cloned()
            .unwrap_or(Value::Null),
        settled_scan_completion,
        "a background scan began after corpus seeding"
    );
    let seeded_daemon_status = send_ok(
        &endpoint,
        &correlated_request("107s-seeded-daemon-status", "get_daemon_status", json!({})),
        IPC_LIMIT,
        deadline,
    )
    .await;
    assert_eq!(
        watcher_event_count(&seeded_daemon_status),
        0,
        "watcher hold must suppress all corpus ingestion"
    );

    let primary_request = correlated_request(
        "107s-index-primary",
        "index_workspace",
        json!({ "force": false }),
    );
    let primary_client_started_at = now_timestamp();
    let primary_client_started = Instant::now();
    let primary_result = send_ok(&endpoint, &primary_request, PROBE_LIMIT, deadline).await;
    let primary_client_elapsed_ms = elapsed_ms(primary_client_started);
    let primary_client_received_at = now_timestamp();
    assert_eq!(primary_result["files_parsed"].as_u64(), Some(2));
    assert_eq!(primary_result["functions_indexed"].as_u64(), Some(2));
    assert_eq!(primary_result["errors"], json!([]));
    assert_eq!(
        primary_result["edges_created"].as_u64(),
        Some(3),
        "two defines edges plus the singleton calls edge must be reported"
    );
    let primary_usage = wait_for_usage_record(&daemon_root, "107s-index-primary", deadline).await;
    assert_eq!(primary_usage["tool_name"], "index_workspace");
    assert_eq!(primary_usage["outcome"], "success");
    assert_eq!(
        normalized_path_text(
            primary_usage["workspace"]
                .as_str()
                .expect("usage workspace path")
        ),
        normalized_path_text(&daemon_root.display().to_string())
    );

    let pre_flush_map = send_ok(
        &endpoint,
        &correlated_request(
            "107s-query-before-flush",
            "map_code",
            json!({ "symbol_name": "beta", "depth": 1, "max_nodes": 10 }),
        ),
        IPC_LIMIT,
        deadline,
    )
    .await;
    let calls_before_flush = assert_singleton_visible(&pre_flush_map);
    let indexed_status = send_ok(
        &endpoint,
        &correlated_request("107s-indexed-status", "get_workspace_status", json!({})),
        IPC_LIMIT,
        deadline,
    )
    .await;
    assert_eq!(
        indexed_status.pointer("/code_graph/code_files"),
        Some(&json!(2))
    );
    assert_eq!(
        indexed_status.pointer("/code_graph/functions"),
        Some(&json!(2))
    );
    assert_eq!(indexed_status.pointer("/code_graph/edges"), Some(&json!(3)));
    assert_eq!(
        indexed_status
            .pointer("/scan_status/running")
            .and_then(Value::as_bool),
        Some(false)
    );

    let flush_result = send_ok(
        &endpoint,
        &correlated_request("107s-flush-primary", "flush_state", json!({})),
        IPC_LIMIT,
        deadline,
    )
    .await;
    let flush_timestamp = flush_result["flush_timestamp"]
        .as_str()
        .expect("flush timestamp")
        .to_owned();
    let post_flush_map = send_ok(
        &endpoint,
        &correlated_request(
            "107s-query-after-flush",
            "map_code",
            json!({ "symbol_name": "beta", "depth": 1, "max_nodes": 10 }),
        ),
        IPC_LIMIT,
        deadline,
    )
    .await;
    let calls_after_flush = assert_singleton_visible(&post_flush_map);
    let post_flush_status = send_ok(
        &endpoint,
        &correlated_request("107s-status-after-flush", "get_workspace_status", json!({})),
        IPC_LIMIT,
        deadline,
    )
    .await;
    assert!(
        post_flush_status["last_flush"].is_string(),
        "finalization must publish last_flush after explicit flush"
    );
    let edges_path = daemon_engram
        .join("code-graph")
        .join("main")
        .join("edges.jsonl");
    assert_eq!(
        singleton_rows(&edges_path).len(),
        1,
        "explicit flush must persist singleton provenance"
    );

    let daemon_after_primary = send_ok(
        &endpoint,
        &correlated_request("107s-daemon-after-primary", "get_daemon_status", json!({})),
        IPC_LIMIT,
        deadline,
    )
    .await;
    let model_loaded_after = daemon_after_primary["model_loaded"]
        .as_bool()
        .unwrap_or(false);
    if primary_result["embeddings_generated"].as_u64().unwrap_or(0) > 0 {
        assert!(
            model_loaded_after,
            "generated embeddings require the owned daemon model to be ready"
        );
    }
    assert_eq!(watcher_event_count(&daemon_after_primary), 0);
    assert_eq!(
        daemon_after_primary.pointer("/telemetry/duplicate_daemon_detected"),
        Some(&Value::from(0_u64))
    );

    let negative_request = correlated_request(
        "107s-index-short-timeout",
        "index_workspace",
        json!({ "force": true }),
    );
    let negative_client_started_at = now_timestamp();
    let negative_client_started = Instant::now();
    let negative_result = send_request(
        &endpoint,
        &negative_request,
        deadline.remaining_work(SHORT_TIMEOUT, "short-timeout index request"),
    )
    .await;
    let negative_client_elapsed_ms = elapsed_ms(negative_client_started);
    let negative_client_finished_at = now_timestamp();
    let negative_outcome = match negative_result {
        Err(EngramError::Ipc(IpcError::Timeout { timeout_ms })) => {
            assert_eq!(timeout_ms, 10);
            "client_timeout"
        }
        Ok(response) => {
            assert_response_id(&negative_request, &response);
            assert!(
                response.error.is_none(),
                "short-timeout request returned a tool error: {:?}",
                response.error
            );
            "completed_within_deadline"
        }
        Err(error) => panic!("unexpected short-timeout IPC result: {error}"),
    };
    let negative_usage =
        wait_for_usage_record(&daemon_root, "107s-index-short-timeout", deadline).await;
    assert_eq!(negative_usage["tool_name"], "index_workspace");
    assert_eq!(negative_usage["outcome"], "success");
    tokio::time::sleep(
        deadline.remaining_work(Duration::from_millis(100), "post-timeout settlement"),
    )
    .await;

    let final_map = send_ok(
        &endpoint,
        &correlated_request(
            "107s-query-after-timeout",
            "map_code",
            json!({ "symbol_name": "beta", "depth": 1, "max_nodes": 10 }),
        ),
        IPC_LIMIT,
        deadline,
    )
    .await;
    let calls_after_timeout = assert_singleton_visible(&final_map);
    let final_flush = send_ok(
        &endpoint,
        &correlated_request("107s-flush-final", "flush_state", json!({})),
        IPC_LIMIT,
        deadline,
    )
    .await;
    assert!(final_flush["flush_timestamp"].is_string());
    let final_daemon_status = send_ok(
        &endpoint,
        &correlated_request("107s-daemon-final", "get_daemon_status", json!({})),
        IPC_LIMIT,
        deadline,
    )
    .await;
    let final_watcher_events = watcher_event_count(&final_daemon_status);
    assert_eq!(final_watcher_events, 0);
    assert_eq!(
        final_daemon_status.pointer("/telemetry/duplicate_daemon_detected"),
        Some(&Value::from(0_u64))
    );

    let shutdown_started = Instant::now();
    let shutdown_result = send_ok(
        &endpoint,
        &internal_request("107s-owned-shutdown", "_shutdown"),
        IPC_LIMIT,
        deadline,
    )
    .await;
    assert_eq!(shutdown_result["status"], "shutting_down");
    let graceful_exit_limit =
        deadline.remaining_cleanup(FORCED_REAP_LIMIT, "graceful child shutdown");
    let Some(exit_status) = wait_for_child_exit(&mut harness, graceful_exit_limit).await else {
        let forced_reap_limit = deadline.remaining_cleanup(FORCED_REAP_LIMIT, "forced child reap");
        harness
            .kill_and_wait_bounded(forced_reap_limit)
            .unwrap_or_else(|error| panic!("failed to reap owned daemon: {error}"));
        panic!("owned daemon cleanup exceeded {IPC_LIMIT:?}");
    };
    let shutdown_elapsed_ms = elapsed_ms(shutdown_started);
    assert!(exit_status.success(), "owned daemon exited unsuccessfully");
    assert!(
        !pid_file.verify_alive().expect("verify reaped owned PID"),
        "owned daemon PID remained alive after cleanup"
    );
    assert!(
        probe(
            &endpoint,
            deadline.remaining_work(
                Duration::from_millis(100),
                "post-reap endpoint verification",
            ),
        )
        .await
        .is_err(),
        "owned endpoint remained reachable after cleanup"
    );
    drop(harness);

    let persisted_singletons = singleton_rows(&edges_path);
    assert_eq!(
        persisted_singletons.len(),
        1,
        "singleton must survive the negative case and graceful shutdown"
    );
    let lines = trace_lines(&trace_path);
    let stderr_bytes = fs::read(&stderr_path)
        .unwrap_or_else(|error| panic!("read daemon stderr {}: {error}", stderr_path.display()))
        .len();
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.contains("watcher_event_sent"))
            .count(),
        0,
        "trace confirms watcher ingestion remained held"
    );
    let postpass_events = trace_event_indices(
        &lines,
        "code graph: resolved cross-file singleton calls edges",
    );
    assert_eq!(
        postpass_events.len(),
        1,
        "post-pass must resolve exactly once"
    );
    let completions = trace_event_indices(&lines, "code graph: indexing complete");
    assert_eq!(
        completions.len(),
        2,
        "primary and short-timeout probes are the only daemon index executions"
    );
    let (primary_frame_index, primary_frame_error) = frame_boundary(&lines, completions[0]);
    assert!(
        primary_frame_error.is_none(),
        "normal response frame must write and flush without error"
    );
    let (negative_frame_index, negative_frame_error) = frame_boundary(&lines, completions[1]);
    let postpass_at = trace_timestamp(&lines, postpass_events[0]);
    let primary_service_completed_at = trace_timestamp(&lines, completions[0]);
    let primary_frame_closed_at = trace_timestamp(&lines, primary_frame_index);
    let negative_service_completed_at = trace_timestamp(&lines, completions[1]);
    let negative_frame_closed_at = trace_timestamp(&lines, negative_frame_index);
    let negative_frame_error_at = negative_frame_error.map(|index| trace_timestamp(&lines, index));

    let primary_dispatch_at = primary_usage["timestamp"]
        .as_str()
        .expect("primary dispatch timestamp")
        .to_owned();
    let primary_dispatch_latency_ms = primary_usage["latency_ms"]
        .as_u64()
        .expect("primary dispatch latency");
    let negative_dispatch_at = negative_usage["timestamp"]
        .as_str()
        .expect("negative dispatch timestamp")
        .to_owned();
    let negative_dispatch_latency_ms = negative_usage["latency_ms"]
        .as_u64()
        .expect("negative dispatch latency");
    let index_duration_ms = primary_result["duration_ms"]
        .as_u64()
        .expect("index duration");

    let baseline_path = baseline_root.clone();
    let daemon_path = daemon_root.clone();
    baseline.close().expect("remove owned baseline temp state");
    daemon.close().expect("remove owned daemon temp state");
    assert!(!baseline_path.exists(), "baseline temp state remained");
    assert!(!daemon_path.exists(), "daemon temp state remained");

    let evidence = json!({
        "revision_and_binary": {
            "build_hash": build_hash,
            "package_version": env!("CARGO_PKG_VERSION"),
            "binary": env!("CARGO_BIN_EXE_engram"),
            "protocol_version": protocol_version,
        },
        "corpus": {
            "aggregate_sha256": baseline_hash,
            "file_sha256": baseline_file_hashes,
            "baseline_workspace": baseline_path,
            "daemon_workspace": daemon_path,
            "baseline_database": baseline_db,
            "daemon_database": daemon_db,
        },
        "baseline": {
            "started_at": baseline_started_at,
            "completed_at": baseline_completed_at,
            "elapsed_ms": baseline_elapsed_ms,
            "files_parsed": baseline_result.files_parsed,
            "functions_indexed": baseline_result.functions_indexed,
            "edges_created": baseline_result.edges_created,
            "singletons": baseline_singletons,
        },
        "daemon_identity": {
            "pid": owned_pid,
            "endpoint": endpoint,
            "repository_endpoint_distinct": true,
            "repository_data_identity_distinct": true,
            "repository_pid_checked_when_available": repository_pid.is_some(),
            "settle_attempts": settle_attempts,
            "active_workspaces": 1,
            "duplicate_daemon_detected": 0,
            "watcher_events": final_watcher_events,
        },
        "aggregate_deadline": {
            "limit_ms": u64::try_from(PROBE_LIMIT.as_millis()).unwrap_or(u64::MAX),
            "cleanup_reserve_ms": u64::try_from(CLEANUP_RESERVE.as_millis()).unwrap_or(u64::MAX),
            "elapsed_ms": deadline.elapsed_ms(),
        },
        "diagnostics": {
            "stdout_trace_captured": true,
            "stderr_captured_separately": true,
            "stderr_bytes": stderr_bytes,
        },
        "u2": {
            "classification": "no current defect",
            "postpass_resolved_at": postpass_at,
            "postpass_resolved_count": 1,
            "calls_before_flush": calls_before_flush,
            "flush_timestamp": flush_timestamp,
            "calls_after_flush": calls_after_flush,
            "calls_after_timeout": calls_after_timeout,
            "persisted_singletons_after_shutdown": persisted_singletons.len(),
        },
        "u3": {
            "classification": "static contract finding: startup outside user request deadline",
            "runtime_blocker": "missing cold CLI timeout/end-to-end request-ID frame correlation",
            "cold_start": {
                "started_at": cold_started_at,
                "ready_at": cold_ready_at,
                "elapsed_ms": cold_ready_ms,
            },
            "model_readiness": {
                "loaded_before_index": model_loaded_before,
                "loaded_after_index": model_loaded_after,
                "upper_bound_ms": primary_client_elapsed_ms,
            },
            "primary_request": {
                "id": "107s-index-primary",
                "client_started_at": primary_client_started_at,
                "service_completed_at": primary_service_completed_at,
                "dispatch_completed_at": primary_dispatch_at,
                "dispatch_latency_ms": primary_dispatch_latency_ms,
                "index_duration_ms": index_duration_ms,
                "frame_closed_at": primary_frame_closed_at,
                "client_received_at": primary_client_received_at,
                "client_elapsed_ms": primary_client_elapsed_ms,
                "frame_error": Value::Null,
            },
            "short_timeout": {
                "id": "107s-index-short-timeout",
                "deadline_ms": 10,
                "client_started_at": negative_client_started_at,
                "client_finished_at": negative_client_finished_at,
                "client_elapsed_ms": negative_client_elapsed_ms,
                "outcome": negative_outcome,
                "service_completed_at": negative_service_completed_at,
                "dispatch_completed_at": negative_dispatch_at,
                "dispatch_latency_ms": negative_dispatch_latency_ms,
                "frame_closed_at": negative_frame_closed_at,
                "frame_error_at": negative_frame_error_at,
            },
            "smallest_future_contract_surface": "src/cli/runner.rs::run_tool_dispatch deadline envelope around health/startup and send_request",
        },
        "cleanup": {
            "graceful_shutdown": true,
            "shutdown_elapsed_ms": shutdown_elapsed_ms,
            "pid_reaped": true,
            "endpoint_unreachable": true,
            "baseline_temp_removed": true,
            "daemon_temp_removed": true,
            "repository_workspace_touched": false,
        },
    });
    assert!(
        deadline.elapsed_ms() <= u64::try_from(PROBE_LIMIT.as_millis()).unwrap_or(u64::MAX),
        "controlled probe exceeded its five-minute aggregate deadline"
    );
    println!(
        "107S_EVIDENCE={}",
        serde_json::to_string(&evidence).expect("serialize 107-S evidence")
    );
}
