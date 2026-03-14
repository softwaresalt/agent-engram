//! Property-based round-trip serialization tests for Event and Collection models.
//!
//! Tests verify that Event and Collection can be serialized to JSON and
//! deserialized back without loss of information.

use chrono::{DateTime, TimeZone, Utc};
use engram::models::{Collection, Event, EventKind};
use proptest::prelude::*;

fn arb_event_kind() -> impl Strategy<Value = EventKind> {
    prop_oneof![
        Just(EventKind::TaskCreated),
        Just(EventKind::TaskUpdated),
        Just(EventKind::TaskDeleted),
        Just(EventKind::EdgeCreated),
        Just(EventKind::EdgeDeleted),
        Just(EventKind::ContextCreated),
        Just(EventKind::CollectionCreated),
        Just(EventKind::CollectionUpdated),
        Just(EventKind::CollectionMembershipChanged),
        Just(EventKind::RollbackApplied),
    ]
}

fn arb_datetime() -> impl Strategy<Value = DateTime<Utc>> {
    (0i64..1_800_000_000i64).prop_map(|secs| Utc.timestamp_opt(secs, 0).unwrap())
}

fn arb_event() -> impl Strategy<Value = Event> {
    (
        "event:[a-z0-9]{8}",
        arb_event_kind(),
        prop_oneof![Just("task"), Just("collection"), Just("depends_on")],
        "[a-z0-9_:]{4,20}",
        prop::option::of(prop::bool::ANY.prop_map(|b| serde_json::json!({ "active": b }))),
        prop::option::of(prop::bool::ANY.prop_map(|b| serde_json::json!({ "active": b }))),
        "[a-z0-9_-]{3,16}",
        arb_datetime(),
    )
        .prop_map(
            |(
                id,
                kind,
                entity_table,
                entity_id,
                previous_value,
                new_value,
                source_client,
                created_at,
            )| {
                Event {
                    id,
                    kind,
                    entity_table: entity_table.to_string(),
                    entity_id,
                    previous_value,
                    new_value,
                    source_client,
                    created_at,
                }
            },
        )
}

fn arb_collection() -> impl Strategy<Value = Collection> {
    (
        "collection:[a-z0-9]{8}",
        "[A-Za-z][A-Za-z0-9 _-]{1,30}",
        prop::option::of(".{1,80}"),
        arb_datetime(),
        arb_datetime(),
    )
        .prop_map(
            |(id, name, description, created_at, updated_at)| Collection {
                id,
                name,
                description,
                created_at,
                updated_at,
            },
        )
}

proptest! {
    #[test]
    fn event_roundtrip(event in arb_event()) {
        let json = serde_json::to_string(&event).expect("Event serialization failed");
        let back: Event = serde_json::from_str(&json).expect("Event deserialization failed");
        prop_assert_eq!(event.id, back.id);
        prop_assert_eq!(event.kind, back.kind);
        prop_assert_eq!(event.entity_table, back.entity_table);
        prop_assert_eq!(event.entity_id, back.entity_id);
        prop_assert_eq!(event.source_client, back.source_client);
    }

    #[test]
    fn collection_roundtrip(collection in arb_collection()) {
        let json = serde_json::to_string(&collection).expect("Collection serialization failed");
        let back: Collection = serde_json::from_str(&json).expect("Collection deserialization failed");
        prop_assert_eq!(collection.id, back.id);
        prop_assert_eq!(collection.name, back.name);
        prop_assert_eq!(collection.description, back.description);
    }

    #[test]
    fn event_kind_roundtrip(kind in arb_event_kind()) {
        let json = serde_json::to_string(&kind).expect("EventKind serialization failed");
        let back: EventKind = serde_json::from_str(&json).expect("EventKind deserialization failed");
        prop_assert_eq!(format!("{kind:?}"), format!("{back:?}"));
    }
}
