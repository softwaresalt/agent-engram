//! Unit tests for graph query model types (`TraversalDirection`).

use engram::models::TraversalDirection;

/// `TraversalDirection` deserializes from `"both"` and defaults to `Both`.
#[test]
fn direction_default_is_both() {
    let dir: TraversalDirection = serde_json::from_str("\"both\"").unwrap();
    assert_eq!(dir, TraversalDirection::Both);

    let default_dir = TraversalDirection::default();
    assert_eq!(default_dir, TraversalDirection::Both);
}

/// `TraversalDirection` deserializes `"outgoing"`.
#[test]
fn direction_outgoing_deserializes() {
    let dir: TraversalDirection = serde_json::from_str("\"outgoing\"").unwrap();
    assert_eq!(dir, TraversalDirection::Outgoing);
}

/// `TraversalDirection` deserializes `"incoming"`.
#[test]
fn direction_incoming_deserializes() {
    let dir: TraversalDirection = serde_json::from_str("\"incoming\"").unwrap();
    assert_eq!(dir, TraversalDirection::Incoming);
}

/// `TraversalDirection` roundtrips through serde.
#[test]
fn direction_roundtrips_serde() {
    for dir in [
        TraversalDirection::Both,
        TraversalDirection::Outgoing,
        TraversalDirection::Incoming,
    ] {
        let json = serde_json::to_string(&dir).unwrap();
        let back: TraversalDirection = serde_json::from_str(&json).unwrap();
        assert_eq!(dir, back);
    }
}
