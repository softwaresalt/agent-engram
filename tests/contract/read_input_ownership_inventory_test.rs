//! Contract coverage for the Read-mode input ownership inventory (plan unit
//! F24, 142.033-T).
//!
//! Two obligations live here:
//!
//! * **Breadth** (142.033.001-ST) — every input a Read descriptor can observe
//!   is enumerated, across every trust boundary a read can cross.
//! * **Ownership** (142.033.002-ST) — every enumerated input is classified as
//!   generation-owned, pinned-operational (individually justified), or
//!   disallowed in `ReadServer` mode, and an unclassified input makes this
//!   suite RED rather than passing by default.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use engram::services::generations::{
    CLASSIFIED_READ_INPUTS, ENUMERATED_READ_INPUTS, ReadInputClass, ReadInputKind,
    classification_of, descriptors_without_enumerated_inputs, duplicate_enumerated_ids,
    read_mode_descriptor_names, unclassified_inputs, unenumerated_classifications,
    unjustified_classifications,
};

// ── Breadth pass (142.033.001-ST) ────────────────────────────────────────────

#[test]
fn the_enumeration_is_not_empty_and_has_no_duplicate_identifiers() {
    assert!(
        !ENUMERATED_READ_INPUTS.is_empty(),
        "the read-input inventory must enumerate something"
    );
    assert_eq!(
        duplicate_enumerated_ids(),
        Vec::<&str>::new(),
        "each enumerated input must appear exactly once"
    );
}

#[test]
fn every_trust_boundary_kind_is_represented() {
    // A missing kind means a whole class of input was never looked at, which
    // is precisely the failure mode the inventory exists to prevent.
    let present: BTreeSet<ReadInputKind> = ENUMERATED_READ_INPUTS
        .iter()
        .map(|entry| entry.kind)
        .collect();

    for kind in ReadInputKind::all() {
        assert!(
            present.contains(kind),
            "no enumerated input covers the '{kind}' trust boundary"
        );
    }
}

#[test]
fn every_read_mode_descriptor_reaches_at_least_one_enumerated_input() {
    let orphans = descriptors_without_enumerated_inputs();
    assert_eq!(
        orphans,
        Vec::<&str>::new(),
        "these read descriptors reach no enumerated input, so their inputs were never inventoried"
    );
}

#[test]
fn the_enumeration_is_anchored_to_the_descriptor_registry() {
    // The inventory's obligation is derived from F19, not from a parallel
    // hand-maintained list, so a newly declared read method widens it
    // automatically.
    let names = read_mode_descriptor_names();
    assert!(
        !names.is_empty(),
        "the descriptor registry must declare read-server methods for the inventory to anchor to"
    );
    assert!(
        names.contains(&"query_graph"),
        "query_graph must be part of the anchored surface: it accepts caller-authored read \
         queries, so its reachable input set is the whole relation surface"
    );
}

#[test]
fn every_enumerated_input_records_how_a_read_reaches_it() {
    for entry in ENUMERATED_READ_INPUTS {
        assert!(
            !entry.reached_via.trim().is_empty(),
            "input '{}' does not record how a read request reaches it",
            entry.id
        );
        assert!(
            entry.id.contains('.'),
            "input '{}' must use a namespaced identifier so the boundary is legible",
            entry.id
        );
    }
}

#[test]
fn the_database_relation_surface_covers_the_declared_schema() {
    // Spot-check the relations a read path cannot avoid. If the schema grows
    // a relation and the inventory does not, the ownership question for that
    // relation was never asked.
    let ids: BTreeSet<&str> = ENUMERATED_READ_INPUTS
        .iter()
        .map(|entry| entry.id)
        .collect();
    for relation in [
        "db.function_meta",
        "db.calls_edge",
        "db.file_node",
        "db.content_record",
        "db.commit_node",
        "db.index_canonical_workspace_snapshot",
    ] {
        assert!(
            ids.contains(relation),
            "relation '{relation}' is reachable from Read dispatch but is not enumerated"
        );
    }
}

// ── Ownership pass (142.033.002-ST) ──────────────────────────────────────────

#[test]
fn no_enumerated_input_is_left_unclassified() {
    // The guard. If this fires, an input was discovered and its ownership
    // question was never answered -- which is a RED state by design, not a
    // formality.
    assert_eq!(
        unclassified_inputs(),
        Vec::<&str>::new(),
        "these enumerated inputs carry no ownership verdict"
    );
}

#[test]
fn no_classification_describes_an_input_that_no_longer_exists() {
    assert_eq!(
        unenumerated_classifications(),
        Vec::<&str>::new(),
        "these verdicts describe inputs that are not enumerated"
    );
}

#[test]
fn the_two_tables_are_the_same_set() {
    let enumerated: BTreeSet<&str> = ENUMERATED_READ_INPUTS
        .iter()
        .map(|entry| entry.id)
        .collect();
    let classified: BTreeSet<&str> = CLASSIFIED_READ_INPUTS
        .iter()
        .map(|entry| entry.id)
        .collect();
    assert_eq!(
        enumerated, classified,
        "the enumeration set and the classification set must be identical"
    );
}

#[test]
fn every_pinned_operational_entry_is_individually_justified() {
    // A pinned-operational entry is an exception to the single-generation
    // rule. An unexplained exception is indistinguishable from an oversight.
    assert_eq!(
        unjustified_classifications(),
        Vec::<&str>::new(),
        "these non-generation-owned verdicts do not say why they are safe"
    );

    for entry in CLASSIFIED_READ_INPUTS {
        if entry.class == ReadInputClass::PinnedOperational {
            assert!(
                entry.justification.len() > 40,
                "input '{}' has a justification too terse to review: {:?}",
                entry.id,
                entry.justification
            );
        }
    }
}

#[test]
fn the_generation_database_relations_are_all_generation_owned() {
    for entry in ENUMERATED_READ_INPUTS {
        if entry.kind != ReadInputKind::DatabaseRelation {
            continue;
        }
        let verdict = classification_of(entry.id)
            .unwrap_or_else(|| panic!("relation '{}' must be classified", entry.id));
        assert_eq!(
            verdict.class,
            ReadInputClass::GenerationOwned,
            "relation '{}' lives inside the sealed generation and must be generation-owned",
            entry.id
        );
    }
}

#[test]
fn live_workspace_tree_inputs_are_disallowed_in_read_server_mode() {
    // The mutable workspace tree is precisely what generations exist to keep
    // off the read path; none of it may be pinned-operational.
    for entry in ENUMERATED_READ_INPUTS {
        if entry.kind != ReadInputKind::WorkspaceRootPath {
            continue;
        }
        let verdict = classification_of(entry.id)
            .unwrap_or_else(|| panic!("input '{}' must be classified", entry.id));
        assert_eq!(
            verdict.class,
            ReadInputClass::DisallowedInReadServerMode,
            "input '{}' reads the live workspace tree and must be disallowed",
            entry.id
        );
    }
}

#[test]
fn connection_count_is_disallowed_not_pinned_operational() {
    // `snapshot.connection_count` is a live transport counter that changes as
    // clients connect and disconnect -- it is not stable across every
    // request, so `PinnedOperational`'s own invariant rules it out even
    // though it describes the daemon process rather than workspace data.
    let verdict = classification_of("snapshot.connection_count")
        .expect("snapshot.connection_count must be classified");
    assert_eq!(
        verdict.class,
        ReadInputClass::DisallowedInReadServerMode,
        "snapshot.connection_count varies per request and must not be pinned-operational"
    );
}

#[test]
fn every_verdict_is_one_of_the_three_reviewed_classes() {
    // Exhaustive match with no wildcard arm: a new class cannot slip through
    // as "probably fine".
    for entry in CLASSIFIED_READ_INPUTS {
        let label = match entry.class {
            ReadInputClass::GenerationOwned => "generation_owned",
            ReadInputClass::PinnedOperational => "pinned_operational",
            ReadInputClass::DisallowedInReadServerMode => "disallowed_in_read_server_mode",
        };
        assert_eq!(
            label,
            entry.class.as_str(),
            "class labelling for '{}' disagrees with its canonical string",
            entry.id
        );
    }
}

#[test]
fn mutable_indexer_watermarks_never_reach_a_read() {
    for id in [
        "snapshot.last_flush",
        "snapshot.stale_files",
        "snapshot.file_mtimes",
        "engram.legacy_managed_database",
        "env.ENGRAM_AUTO_REINDEX",
    ] {
        let verdict =
            classification_of(id).unwrap_or_else(|| panic!("input '{id}' must be classified"));
        assert_eq!(
            verdict.class,
            ReadInputClass::DisallowedInReadServerMode,
            "input '{id}' is indexer-mutable and must be disallowed in ReadServer mode"
        );
    }
}

#[test]
fn read_mode_descriptors_remain_the_anchor_for_the_classified_surface() {
    // Re-assert the anchor after classification so the two passes cannot be
    // satisfied by different descriptor sets.
    assert!(!read_mode_descriptor_names().is_empty());
    assert_eq!(descriptors_without_enumerated_inputs(), Vec::<&str>::new());
    assert_eq!(duplicate_enumerated_ids(), Vec::<&str>::new());
}
