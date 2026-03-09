//! Integration tests for the file watcher pipeline (T035–T038).
//!
//! These tests exercise [`engram::daemon::watcher::start_watcher`] end-to-end
//! with real file system operations and verify that [`WatcherEvent`] values
//! are emitted with the correct kind and path after debouncing.
//!
//! All tests use real wall-clock time (no `tokio::time::pause`) because the
//! underlying `notify-debouncer-full` uses `std::thread::sleep`.

use std::path::PathBuf;
use std::time::Duration;

use engram::daemon::watcher::{WatcherConfig, start_watcher};
use engram::models::{WatchEventKind, WatcherEvent};
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Maximum time to wait for a single watcher event.
const EVENT_TIMEOUT: Duration = Duration::from_secs(3);

/// Start a watcher on `dir`, returning the handle and the event receiver.
macro_rules! watch_dir {
    ($dir:expr) => {{
        let (tx, rx) = mpsc::unbounded_channel::<WatcherEvent>();
        let handle = start_watcher($dir.path(), WatcherConfig::default(), tx)
            .expect("start_watcher must succeed")
            .expect("watcher handle must be Some (watcher initialisation succeeded)");
        (handle, rx)
    }};
}

/// Drain all events currently pending in `rx` and return them.
fn drain(rx: &mut mpsc::UnboundedReceiver<WatcherEvent>) -> Vec<WatcherEvent> {
    let mut events = Vec::new();
    while let Ok(e) = rx.try_recv() {
        events.push(e);
    }
    events
}

/// Wait up to `EVENT_TIMEOUT` for a single event satisfying `pred`.
async fn wait_for(
    rx: &mut mpsc::UnboundedReceiver<WatcherEvent>,
    pred: impl Fn(&WatcherEvent) -> bool,
) -> Option<WatcherEvent> {
    timeout(EVENT_TIMEOUT, async {
        loop {
            if let Some(e) = rx.recv().await {
                if pred(&e) {
                    return e;
                }
            }
        }
    })
    .await
    .ok()
}

// ── T035: Basic create / modify / delete ─────────────────────────────────────

/// S052: A file created in the workspace root emits a `Created` event.
#[tokio::test]
async fn s052_file_created_emits_created_event() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (_handle, mut rx) = watch_dir!(dir);

    let file_path = dir.path().join("hello.txt");
    std::fs::write(&file_path, b"hello").expect("write file");

    let event = wait_for(&mut rx, |e| e.kind == WatchEventKind::Created).await;
    assert!(
        event.is_some(),
        "expected Created event within {EVENT_TIMEOUT:?}"
    );

    let event = event.unwrap();
    assert_eq!(event.kind, WatchEventKind::Created);
    assert_eq!(event.path, PathBuf::from("hello.txt"));
    assert!(event.old_path.is_none());
}

/// S053: Writing to an existing file emits a `Modified` event.
#[tokio::test]
async fn s053_file_modified_emits_modified_event() {
    let dir = tempfile::tempdir().expect("tempdir");

    // Create the file before starting the watcher so the first write is a Modify.
    let file_path = dir.path().join("data.txt");
    std::fs::write(&file_path, b"initial").expect("write initial");

    let (_handle, mut rx) = watch_dir!(dir);

    // Give the watcher time to settle before writing.
    sleep(Duration::from_millis(100)).await;

    std::fs::write(&file_path, b"updated").expect("write update");

    let event = wait_for(&mut rx, |e| e.kind == WatchEventKind::Modified).await;
    assert!(
        event.is_some(),
        "expected Modified event within {EVENT_TIMEOUT:?}"
    );

    let event = event.unwrap();
    assert_eq!(event.kind, WatchEventKind::Modified);
    assert_eq!(event.path, PathBuf::from("data.txt"));
}

/// S054: Deleting a file emits a `Deleted` event.
#[tokio::test]
async fn s054_file_deleted_emits_deleted_event() {
    let dir = tempfile::tempdir().expect("tempdir");

    let file_path = dir.path().join("gone.txt");
    std::fs::write(&file_path, b"bye").expect("write file");

    let (_handle, mut rx) = watch_dir!(dir);

    // Let any initial event from the file creation settle.
    sleep(Duration::from_millis(200)).await;
    drain(&mut rx);

    std::fs::remove_file(&file_path).expect("remove file");

    let event = wait_for(&mut rx, |e| e.kind == WatchEventKind::Deleted).await;
    assert!(
        event.is_some(),
        "expected Deleted event within {EVENT_TIMEOUT:?}"
    );

    let event = event.unwrap();
    assert_eq!(event.kind, WatchEventKind::Deleted);
    assert_eq!(event.path, PathBuf::from("gone.txt"));
}

// ── T036: Debounce ────────────────────────────────────────────────────────────

/// S055: 10 rapid saves within 200 ms collapse to a single debounced event.
#[tokio::test]
async fn s055_rapid_saves_debounced_to_single_event() {
    let dir = tempfile::tempdir().expect("tempdir");

    // Pre-create the file so all subsequent writes are Modify events.
    let file_path = dir.path().join("rapid.txt");
    std::fs::write(&file_path, b"v0").expect("write initial");

    let (_handle, mut rx) = watch_dir!(dir);

    // Allow watcher to settle.
    sleep(Duration::from_millis(200)).await;
    drain(&mut rx);

    // Write 10 times in rapid succession (~20 ms per write = ~200 ms total).
    for i in 1u8..=10 {
        std::fs::write(&file_path, [i]).expect("rapid write");
        // Minimal sleep to allow OS to register the write, but within debounce window.
        sleep(Duration::from_millis(20)).await;
    }

    // Wait for the debounce window to expire and all events to be emitted.
    sleep(Duration::from_millis(800)).await;

    let events = drain(&mut rx);

    // Filter to Modify events for our file.
    let modify_events: Vec<_> = events
        .iter()
        .filter(|e| {
            e.kind == WatchEventKind::Modified && e.path == std::path::Path::new("rapid.txt")
        })
        .collect();

    assert!(
        modify_events.len() <= 3,
        "expected debounce to collapse 10 writes; got {} Modified events: {modify_events:?}",
        modify_events.len()
    );
    assert!(
        !modify_events.is_empty(),
        "expected at least 1 Modified event after 10 writes"
    );
}

// ── T037: Exclusion patterns ──────────────────────────────────────────────────

/// Asserts that no event for a file inside an excluded directory is received.
async fn assert_no_event_in_excluded_dir(subdir: &str) {
    let dir = tempfile::tempdir().expect("tempdir");

    let excluded = dir.path().join(subdir);
    std::fs::create_dir_all(&excluded).expect("create excluded dir");

    let (_handle, mut rx) = watch_dir!(dir);

    // Allow the watcher to fully settle and any directory-creation events to drain.
    sleep(Duration::from_millis(400)).await;
    drain(&mut rx);

    // Create a file inside the excluded directory.
    std::fs::write(excluded.join("secret.txt"), b"ignored").expect("write in excluded dir");

    // Wait longer than the debounce window; no event should arrive for paths
    // inside the excluded directory (including directory-level modify events).
    let result = timeout(Duration::from_millis(1500), async {
        loop {
            if let Some(evt) = rx.recv().await {
                // Accept only events that are truly NOT for the excluded subtree.
                // (The watcher may still emit events for the workspace root itself.)
                let path_str = evt.path.to_string_lossy().replace('\\', "/");
                let stem = subdir.trim_end_matches('/');
                let is_for_excluded = path_str == stem || path_str.starts_with(&format!("{stem}/"));
                if is_for_excluded {
                    return Some(evt);
                }
            }
        }
    })
    .await;

    assert!(
        result.is_err() || result.unwrap().is_none(),
        "expected NO event for file inside '{subdir}'"
    );
}

/// S056: Changes inside `.engram/` are excluded.
#[tokio::test]
async fn s056_engram_dir_excluded() {
    assert_no_event_in_excluded_dir(".engram").await;
}

/// S057: Changes inside `.git/` are excluded.
#[tokio::test]
async fn s057_git_dir_excluded() {
    assert_no_event_in_excluded_dir(".git").await;
}

/// S058: Changes inside `node_modules/` are excluded.
#[tokio::test]
async fn s058_node_modules_excluded() {
    assert_no_event_in_excluded_dir("node_modules").await;
}

/// S059: Changes inside `target/` are excluded.
#[tokio::test]
async fn s059_target_dir_excluded() {
    assert_no_event_in_excluded_dir("target").await;
}

// ── T038: Edge cases ──────────────────────────────────────────────────────────

/// S062: Renaming a file emits a `Renamed` event with both old and new paths.
#[tokio::test]
async fn s062_file_renamed_emits_renamed_event() {
    let dir = tempfile::tempdir().expect("tempdir");

    let old_path = dir.path().join("before.txt");
    let new_path = dir.path().join("after.txt");
    std::fs::write(&old_path, b"content").expect("write file");

    let (_handle, mut rx) = watch_dir!(dir);

    // Let initial create event drain.
    sleep(Duration::from_millis(800)).await;
    drain(&mut rx);

    std::fs::rename(&old_path, &new_path).expect("rename file");

    // Wait for a Renamed event; fall back to accepting Created/Deleted pair
    // since some OS backends may not produce RenameMode::Both.
    let result = timeout(EVENT_TIMEOUT, async {
        let mut events = Vec::new();
        loop {
            if let Some(e) = rx.recv().await {
                let done = e.kind == WatchEventKind::Renamed || events.len() >= 3;
                events.push(e);
                if done {
                    break;
                }
            }
        }
        events
    })
    .await;

    assert!(
        result.is_ok(),
        "expected rename-related event within {EVENT_TIMEOUT:?}"
    );
    let events = result.unwrap();

    let has_rename = events.iter().any(|e| e.kind == WatchEventKind::Renamed);
    let has_delete_and_create = events.iter().any(|e| e.kind == WatchEventKind::Deleted)
        && events.iter().any(|e| e.kind == WatchEventKind::Created);

    assert!(
        has_rename || has_delete_and_create,
        "expected Renamed (or Created+Deleted pair) event for file rename; got: {events:?}"
    );

    if has_rename {
        let rename_event = events
            .iter()
            .find(|e| e.kind == WatchEventKind::Renamed)
            .unwrap();
        assert_eq!(
            rename_event.path,
            PathBuf::from("after.txt"),
            "new path mismatch"
        );
        assert_eq!(
            rename_event.old_path.as_deref(),
            Some(PathBuf::from("before.txt").as_path()),
            "old_path mismatch"
        );
    }
}

/// S063: 500 file batch creates are debounced and processed without crash.
///
/// This test verifies stability, not exact event counts. We just confirm
/// some events arrive and the watcher does not panic or deadlock.
#[tokio::test]
async fn s063_large_batch_creates_processed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (_handle, mut rx) = watch_dir!(dir);

    sleep(Duration::from_millis(100)).await;

    // Create 500 files in batches to avoid overwhelming the OS.
    let batch_dir = dir.path().join("batch");
    std::fs::create_dir_all(&batch_dir).expect("create batch dir");

    for i in 0..500u32 {
        std::fs::write(batch_dir.join(format!("file_{i:04}.txt")), b"data")
            .expect("write batch file");
    }

    // Wait for debounce to flush.
    sleep(Duration::from_millis(1500)).await;

    let events = drain(&mut rx);
    let file_events: Vec<_> = events
        .iter()
        .filter(|e| e.path.starts_with("batch"))
        .collect();

    assert!(
        !file_events.is_empty(),
        "expected at least one event from 500-file batch create; got none"
    );
    // The test passes as long as no panic occurred and at least one event arrived.
}

/// S066: Modifying a binary file emits a `Modified` event.
///
/// Binary detection is the caller's concern; the watcher emits unconditionally.
#[tokio::test]
async fn s066_binary_file_modified_emits_event() {
    let dir = tempfile::tempdir().expect("tempdir");

    let bin_path = dir.path().join("data.bin");
    // Write initial binary content.
    let binary: Vec<u8> = (0u8..=255).collect();
    std::fs::write(&bin_path, &binary).expect("write binary initial");

    let (_handle, mut rx) = watch_dir!(dir);

    sleep(Duration::from_millis(200)).await;
    drain(&mut rx);

    // Write modified binary content.
    let modified: Vec<u8> = (255u8..=255).chain(0u8..=254).collect();
    std::fs::write(&bin_path, &modified).expect("write binary modified");

    let event = wait_for(&mut rx, |e| e.kind == WatchEventKind::Modified).await;
    assert!(
        event.is_some(),
        "expected Modified event for binary file within {EVENT_TIMEOUT:?}"
    );

    let event = event.unwrap();
    assert_eq!(event.path, PathBuf::from("data.bin"));
}
