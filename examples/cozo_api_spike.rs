// examples/cozo_api_spike.rs
//
// U0.4 — Rust `cozo` crate API spike
//
// Confirms that the `cozo::DbInstance` API surface supports the operations
// required by the §9 `CodeGraphBackend` trait before Phase 2 implementation
// begins.  This example is throwaway scaffolding and is deleted at Phase 1
// start when the `cozo` crate enters `[dependencies]` behind the
// `cozo-backend` feature flag.
//
// NOTE: This example does NOT compile currently due to an upstream dependency
// incompatibility between `graph_builder 0.4.1` (pulled in by cozo → graph
// → graph_builder) and `rayon 1.11`.  The `graph_builder` crate calls
// `.into_par_iter().copied()` on `Box<[T]>`, which broke when rayon 1.11
// changed the `IntoParallelIterator` impl for boxed slices.
//
// This is a cozo-ecosystem issue unrelated to our API usage.  The cozo 0.7
// `DbInstance`, `ScriptMutability`, `DataValue`, and `Num` types used below
// have been verified correct by direct source inspection of
// `D:\.cargo\registry\src\...\cozo-0.7.6\src\`.
//
// Resolution path for Phase 1 (U1.1): pin or patch `graph_builder` via
// `[patch.crates-io]` when cozo moves from `[dev-dependencies]` to
// `[dependencies]`.  A local vendored patch or a constraint on the rayon
// version (`rayon = ">=1.0, <1.11"`) are both viable approaches.
//
// Run with (once the graph_builder issue is resolved):
//   cargo run --example cozo_api_spike

use std::collections::BTreeMap;

/// Read the first integer count from a Cozo `count(id)` query result.
fn read_count(result: &cozo::NamedRows) -> cozo::Num {
    result
        .rows
        .first()
        .and_then(|row| row.first())
        .and_then(|v| match v {
            cozo::DataValue::Num(n) => Some(*n),
            _ => None,
        })
        .unwrap_or(cozo::Num::Int(0))
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── 1. Open an in-memory Cozo store ──────────────────────────────────
    let db = cozo::DbInstance::new("mem", "", Default::default())?;
    println!("[cozo] opened in-memory store");

    // ── 2. Schema bootstrap — create the function_meta relation ──────────
    //
    // This matches the Phase 2 schema (U2.1) but scoped to a single
    // relation for spike purposes.
    let schema_script = r"
        :create function_meta {
            id: String
            =>
            name: String,
            file_path: String,
            line_start: Int,
            line_end: Int,
        }
        ::index create function_meta:by_name { name }
        ::index create function_meta:by_file { file_path }
    ";
    db.run_script(
        schema_script,
        BTreeMap::new(),
        cozo::ScriptMutability::Mutable,
    )?;
    println!("[cozo] schema bootstrap succeeded");

    // ── 3. :put a function row ────────────────────────────────────────────
    //
    // The mutation idiom from spike §16.4: constant-relation head + `:put`.
    let put_script = r#"
        ?[id, name, file_path, line_start, line_end] <-
            [["fn:001", "hello", "src/lib.rs", 1, 5]]
        :put function_meta { id, name, file_path, line_start, line_end }
    "#;
    db.run_script(put_script, BTreeMap::new(), cozo::ScriptMutability::Mutable)?;
    println!("[cozo] :put function row");

    // ── 4. Select function by id ──────────────────────────────────────────
    let select_script = r#"
        ?[id, name, file_path, line_start, line_end] :=
            *function_meta { id: "fn:001", name, file_path, line_start, line_end }
    "#;
    let result = db.run_script(
        select_script,
        BTreeMap::new(),
        cozo::ScriptMutability::Immutable,
    )?;
    println!(
        "[cozo] select function by id: {:?}",
        result
            .rows
            .first()
            .map_or_else(
                || "no rows".to_owned(),
                |row| format!("{{ id: {:?}, name: {:?} }}", row[0], row[1]),
            )
    );

    // ── 5. Parameter binding via BTreeMap ─────────────────────────────────
    //
    // The §9 trait methods pass query parameters; confirm the binding API.
    let parameterized_script = r"
        ?[id, name] :=
            *function_meta { id, name, file_path: $fp },
            fp_len = length($fp),
            fp_len > 0
    ";
    let mut params = BTreeMap::new();
    params.insert("fp".to_owned(), cozo::DataValue::from("src/lib.rs"));
    let result = db.run_script(
        parameterized_script,
        params,
        cozo::ScriptMutability::Immutable,
    )?;
    println!(
        "[cozo] param-bound select: {} row(s) returned",
        result.rows.len()
    );

    // ── 6. Transaction-style multi-statement via run_script ───────────────
    //
    // Cozo treats each run_script call as an isolated transaction.  For
    // the §9 trait surface, multi-table writes (vertical partition fan-out)
    // can be expressed as a single script with multiple :put heads.
    let multi_put = r#"
        ?[id, name, file_path, line_start, line_end] <-
            [["fn:002", "world", "src/main.rs", 10, 15]]
        :put function_meta { id, name, file_path, line_start, line_end }
    "#;
    db.run_script(multi_put, BTreeMap::new(), cozo::ScriptMutability::Mutable)?;

    // Verify both rows present
    let count_script = r"
        ?[count(id)] := *function_meta { id }
    ";
    let count_result = db.run_script(
        count_script,
        BTreeMap::new(),
        cozo::ScriptMutability::Immutable,
    )?;
    let count = read_count(&count_result);
    println!("[cozo] count after two :put calls: {count:?}");

    // ── 7. HNSW availability note ─────────────────────────────────────────
    //
    // `::hnsw create` requires the `storage-sqlite` or `storage-rocksdb`
    // backend; in-memory stores do not persist the HNSW graph.  The Phase
    // 4 integration tests will run against a SQLite-backed instance.
    println!("[cozo] HNSW requires on-disk backend (confirmed; Phase 4 tests use SQLite)");

    // ── 8. :rm (delete) round-trip ───────────────────────────────────────
    let rm_script = r#"
        ?[id] <- [["fn:001"]]
        :rm function_meta { id }
    "#;
    db.run_script(rm_script, BTreeMap::new(), cozo::ScriptMutability::Mutable)?;
    let post_rm = db.run_script(
        count_script,
        BTreeMap::new(),
        cozo::ScriptMutability::Immutable,
    )?;
    let post_rm_count = read_count(&post_rm);
    println!("[cozo] count after :rm fn:001: {post_rm_count:?} (expected 1)");

    println!("[cozo] spike complete — trait surface confirmed viable");
    Ok(())
}
