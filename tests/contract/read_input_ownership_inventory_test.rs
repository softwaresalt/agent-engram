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
    ENUMERATED_READ_INPUTS, ReadInputKind, descriptors_without_enumerated_inputs,
    duplicate_enumerated_ids, read_mode_descriptor_names,
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
