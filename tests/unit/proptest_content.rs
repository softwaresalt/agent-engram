//! Proptest serialization round-trip tests for new models (T012).
//!
//! Verifies that all workspace content intelligence models
//! survive JSON serialization + deserialization without data loss.

use chrono::Utc;
use proptest::prelude::*;

use engram::models::commit::{ChangeRecord, ChangeType, CommitNode};
use engram::models::content::ContentRecord;
use engram::models::registry::{ContentSource, ContentSourceStatus, RegistryConfig};

fn arb_content_source_status() -> impl Strategy<Value = ContentSourceStatus> {
    prop_oneof![
        Just(ContentSourceStatus::Unknown),
        Just(ContentSourceStatus::Active),
        Just(ContentSourceStatus::Missing),
        Just(ContentSourceStatus::Error),
    ]
}

fn arb_change_type() -> impl Strategy<Value = ChangeType> {
    prop_oneof![
        Just(ChangeType::Add),
        Just(ChangeType::Modify),
        Just(ChangeType::Delete),
        Just(ChangeType::Rename),
    ]
}

fn arb_content_source() -> impl Strategy<Value = ContentSource> {
    (
        "[a-z]{3,10}",
        prop::option::of("[a-z]{2,8}"),
        "[a-z/]{1,20}",
    )
        .prop_map(|(content_type, language, path)| ContentSource {
            content_type,
            language,
            path,
            status: ContentSourceStatus::Unknown,
        })
}

fn arb_registry_config() -> impl Strategy<Value = RegistryConfig> {
    (
        prop::collection::vec(arb_content_source(), 0..5),
        1..=104_857_600_u64,
        1..=500_usize,
    )
        .prop_map(
            |(sources, max_file_size_bytes, batch_size)| RegistryConfig {
                sources,
                max_file_size_bytes,
                batch_size,
            },
        )
}

fn arb_content_record() -> impl Strategy<Value = ContentRecord> {
    (
        "[a-z0-9]{8}",
        "[a-z]{3,8}",
        "[a-z/\\.]{5,30}",
        "[a-f0-9]{64}",
        ".{0,100}",
        "[a-z/]{3,15}",
        0..2_000_000_u64,
    )
        .prop_map(
            |(id, content_type, file_path, content_hash, content, source_path, file_size_bytes)| {
                ContentRecord {
                    id,
                    content_type,
                    file_path,
                    content_hash,
                    content,
                    embedding: None,
                    source_path,
                    file_size_bytes,
                    ingested_at: Utc::now(),
                }
            },
        )
}

fn arb_change_record() -> impl Strategy<Value = ChangeRecord> {
    (
        "[a-z/\\.]{5,30}",
        arb_change_type(),
        ".{0,80}",
        prop::option::of(0..1000_u32),
        prop::option::of(0..1000_u32),
        0..100_u32,
        0..100_u32,
    )
        .prop_map(
            |(
                file_path,
                change_type,
                diff_snippet,
                old_line_start,
                new_line_start,
                lines_added,
                lines_removed,
            )| {
                ChangeRecord {
                    file_path,
                    change_type,
                    diff_snippet,
                    old_line_start,
                    new_line_start,
                    lines_added,
                    lines_removed,
                }
            },
        )
}

fn arb_commit_node() -> impl Strategy<Value = CommitNode> {
    (
        "[a-z0-9]{8}",
        "[a-f0-9]{40}",
        "[a-f0-9]{7}",
        "[A-Za-z ]{3,20}",
        "[a-z@.]{5,20}",
        ".{0,60}",
        prop::collection::vec("[a-f0-9]{40}", 0..3),
        prop::collection::vec(arb_change_record(), 0..4),
    )
        .prop_map(
            |(id, hash, short_hash, author_name, author_email, message, parent_hashes, changes)| {
                CommitNode {
                    id,
                    hash,
                    short_hash,
                    author_name,
                    author_email,
                    timestamp: Utc::now(),
                    message,
                    parent_hashes,
                    changes,
                }
            },
        )
}

proptest! {
    #[test]
    fn content_source_roundtrip(src in arb_content_source()) {
        let json = serde_json::to_string(&src).unwrap();
        let decoded: ContentSource = serde_json::from_str(&json).unwrap();
        // status is #[serde(skip)] so will be default (Unknown)
        prop_assert_eq!(src.content_type, decoded.content_type);
        prop_assert_eq!(src.language, decoded.language);
        prop_assert_eq!(src.path, decoded.path);
    }

    #[test]
    fn registry_config_roundtrip(config in arb_registry_config()) {
        let json = serde_json::to_string(&config).unwrap();
        let decoded: RegistryConfig = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(config.sources.len(), decoded.sources.len());
        prop_assert_eq!(config.max_file_size_bytes, decoded.max_file_size_bytes);
        prop_assert_eq!(config.batch_size, decoded.batch_size);
    }

    #[test]
    fn content_record_roundtrip(record in arb_content_record()) {
        let json = serde_json::to_string(&record).unwrap();
        let decoded: ContentRecord = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(record.id, decoded.id);
        prop_assert_eq!(record.content_type, decoded.content_type);
        prop_assert_eq!(record.file_path, decoded.file_path);
        prop_assert_eq!(record.content_hash, decoded.content_hash);
        prop_assert_eq!(record.file_size_bytes, decoded.file_size_bytes);
    }

    #[test]
    fn commit_node_roundtrip(node in arb_commit_node()) {
        let json = serde_json::to_string(&node).unwrap();
        let decoded: CommitNode = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(node.hash, decoded.hash);
        prop_assert_eq!(node.short_hash, decoded.short_hash);
        prop_assert_eq!(node.author_name, decoded.author_name);
        prop_assert_eq!(node.changes.len(), decoded.changes.len());
        prop_assert_eq!(node.parent_hashes, decoded.parent_hashes);
    }

    #[test]
    fn content_source_status_roundtrip(status in arb_content_source_status()) {
        let json = serde_json::to_string(&status).unwrap();
        let decoded: ContentSourceStatus = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(status, decoded);
    }

    #[test]
    fn change_type_roundtrip(ct in arb_change_type()) {
        let json = serde_json::to_string(&ct).unwrap();
        let decoded: ChangeType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(ct, decoded);
    }
}
