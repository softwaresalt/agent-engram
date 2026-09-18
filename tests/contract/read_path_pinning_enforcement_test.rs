//! Contract guard for read-path pinning enforcement (plan unit F37, 142.046-T).
//!
//! This is the closing static gate for the pinned-read migration. It does not
//! try to prove semantic correctness of every handler; instead it guards the
//! small set of structural escape hatches that would let a supposedly pinned
//! read re-derive live inputs after request entry:
//!
//! * a fresh `connect_db(` open,
//! * a fresh workspace snapshot read,
//! * live workspace-root threading beyond the pinning seam, or
//! * a direct `.engram` path read from the read path.
//!
//! Each allowed exception is anchored back to the F24 inventory in
//! `src/services/generations/read_inputs.rs`: if an exception is not classified
//! there as `PinnedOperational`, this suite treats it as a regression.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use engram::services::generations::{ReadInputClass, classification_of};

#[derive(Clone, Copy)]
enum FileScope {
    AllProductionFunctions,
    OnlyFunctions(&'static [&'static str]),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Occurrence {
    file: &'static str,
    function: String,
    line: usize,
    text: String,
}

impl Occurrence {
    fn key(&self) -> String {
        format!("{}::{}", self.file, self.function)
    }

    fn render(&self) -> String {
        format!(
            "{}:{} in {} => {}",
            self.file, self.line, self.function, self.text
        )
    }
}

fn manifest_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn production_lines(relative_path: &'static str) -> Vec<String> {
    let path = manifest_dir().join(relative_path);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));

    let mut lines = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed == "#[cfg(test)]" || trimmed.starts_with("mod tests") {
            break;
        }
        lines.push(line.to_owned());
    }
    lines
}

fn function_name_from_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with("#[") {
        return None;
    }

    let fn_offset = trimmed.find("fn ")?;
    let after_fn = &trimmed[fn_offset + 3..];
    let name: String = after_fn
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect();
    if name.is_empty() { None } else { Some(name) }
}

fn enclosing_function(lines: &[String], line_index: usize) -> Option<String> {
    for candidate in (0..=line_index).rev() {
        if let Some(name) = function_name_from_line(&lines[candidate]) {
            return Some(name);
        }
    }
    None
}

fn scope_allows(scope: FileScope, function: &str) -> bool {
    match scope {
        FileScope::AllProductionFunctions => true,
        FileScope::OnlyFunctions(functions) => functions.contains(&function),
    }
}

fn collect_occurrences(targets: &[(&'static str, FileScope)], needles: &[&str]) -> Vec<Occurrence> {
    let mut occurrences = Vec::new();

    for (file, scope) in targets {
        let lines = production_lines(file);
        for (line_index, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || !needles.iter().any(|needle| line.contains(needle)) {
                continue;
            }

            let Some(function) = enclosing_function(&lines, line_index) else {
                continue;
            };
            if !scope_allows(*scope, &function) {
                continue;
            }

            occurrences.push(Occurrence {
                file,
                function,
                line: line_index + 1,
                text: line.trim().to_owned(),
            });
        }
    }

    occurrences.sort();
    occurrences
}

fn counts_by_function(occurrences: &[Occurrence]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for occurrence in occurrences {
        *counts.entry(occurrence.key()).or_insert(0) += 1;
    }
    counts
}

fn expected_counts(entries: &[(&str, usize)]) -> BTreeMap<String, usize> {
    entries
        .iter()
        .map(|(key, count)| ((*key).to_owned(), *count))
        .collect()
}

fn render_occurrences(occurrences: &[Occurrence]) -> String {
    if occurrences.is_empty() {
        return "(none)".to_owned();
    }

    occurrences
        .iter()
        .map(Occurrence::render)
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_pinned_operational(ids: &[&str], reason: &str) {
    for id in ids {
        let classification = classification_of(id)
            .unwrap_or_else(|| panic!("F24 inventory entry '{id}' must exist for this guard"));
        assert_eq!(
            classification.class,
            ReadInputClass::PinnedOperational,
            "F24 inventory entry '{id}' must be pinned-operational for this guard: {reason}"
        );
    }
}

#[test]
fn connect_db_calls_stay_inside_the_pinned_query_seams() {
    assert_pinned_operational(
        &["snapshot.data_dir", "snapshot.branch"],
        "a read-path DB open is only allowed when both the data dir and branch come from the \
         already-pinned request context",
    );

    let occurrences = collect_occurrences(
        &[
            ("src/tools/read.rs", FileScope::AllProductionFunctions),
            (
                "src/tools/eval.rs",
                FileScope::OnlyFunctions(&[
                    "snapshot_parts",
                    "open_queries",
                    "get_retrieval_eval_report",
                ]),
            ),
        ],
        &["connect_db("],
    );

    let actual = counts_by_function(&occurrences);
    let expected = expected_counts(&[
        ("src/tools/read.rs::queries_from_read_context", 1),
        ("src/tools/read.rs::pinned_queries", 1),
        ("src/tools/read.rs::queries_from_context", 1),
        ("src/tools/eval.rs::open_queries", 1),
    ]);

    assert_eq!(
        actual,
        expected,
        "read-path connect_db calls must remain confined to the established pinned-query seams. \
         A new location means a read started opening storage from a fresh, potentially unpinned \
         source.\nactual occurrences:\n{}",
        render_occurrences(&occurrences)
    );
}

#[test]
fn workspace_snapshot_reads_stay_inside_the_pinning_seams() {
    assert_pinned_operational(
        &[
            "snapshot.workspace_id",
            "snapshot.branch",
            "snapshot.data_dir",
            "snapshot.path",
        ],
        "the only permitted live snapshot reads are the seam functions that materialize the \
         caller-pinned request view",
    );

    let occurrences = collect_occurrences(
        &[
            (
                "src/server/state.rs",
                FileScope::OnlyFunctions(&["from_managed_state"]),
            ),
            ("src/tools/read.rs", FileScope::AllProductionFunctions),
            (
                "src/tools/eval.rs",
                FileScope::OnlyFunctions(&[
                    "snapshot_parts",
                    "open_queries",
                    "get_retrieval_eval_report",
                ]),
            ),
            ("src/tools/lint.rs", FileScope::AllProductionFunctions),
        ],
        &[
            "snapshot_workspace()",
            "snapshot_dispatch_context()",
            "ReadRequestContext::from_managed_state(",
        ],
    );

    let actual = counts_by_function(&occurrences);
    let expected = expected_counts(&[
        ("src/server/state.rs::from_managed_state", 1),
        ("src/tools/read.rs::pinned_dispatch_context", 1),
        ("src/tools/read.rs::maybe_pinned_dispatch_context", 1),
        ("src/tools/read.rs::pinned_read_request_context", 1),
        ("src/tools/eval.rs::snapshot_parts", 1),
        ("src/tools/lint.rs::pinned_lint_context", 1),
    ]);

    assert_eq!(
        actual,
        expected,
        "read-path workspace snapshot reads must remain confined to the established pinning \
         seams. A new location means a handler started re-reading live AppState after request \
         entry instead of consuming the already-pinned context.\nactual occurrences:\n{}",
        render_occurrences(&occurrences)
    );
}

#[test]
fn workspace_root_inputs_do_not_escape_the_path_pinning_seams() {
    assert_pinned_operational(
        &["snapshot.path"],
        "the live workspace root is only allowed to appear at the two seam functions that pin \
         the path itself; every downstream use is a fresh live-root dependency",
    );

    let occurrences = collect_occurrences(
        &[
            (
                "src/tools/eval.rs",
                FileScope::OnlyFunctions(&["snapshot_parts", "get_retrieval_eval_report"]),
            ),
            ("src/tools/lint.rs", FileScope::AllProductionFunctions),
            (
                "src/services/dax_lint.rs",
                FileScope::AllProductionFunctions,
            ),
        ],
        &["workspace_path", "workspace_root"],
    );

    let (allowed, disallowed): (Vec<_>, Vec<_>) = occurrences.into_iter().partition(|occurrence| {
        matches!(
            (occurrence.file, occurrence.function.as_str()),
            ("src/tools/eval.rs", "snapshot_parts") | ("src/tools/lint.rs", "pinned_lint_context")
        )
    });

    assert!(
        !allowed.is_empty(),
        "the workspace-root guard lost its seam anchors; it must still see snapshot_parts and \
         pinned_lint_context"
    );
    assert!(
        disallowed.is_empty(),
        "a read-path function is threading or consuming a live workspace root outside the two \
         approved pinning seams. F24 classifies only snapshot.path as pinned-operational here; \
         every downstream root use is therefore a regression until explicitly inventoried and \
         justified.\ndisallowed occurrences:\n{}",
        render_occurrences(&disallowed)
    );
}

#[test]
fn direct_dot_engram_reads_do_not_escape_into_the_read_path() {
    let occurrences = collect_occurrences(
        &[
            (
                "src/tools/eval.rs",
                FileScope::OnlyFunctions(&["snapshot_parts", "get_retrieval_eval_report"]),
            ),
            ("src/tools/lint.rs", FileScope::AllProductionFunctions),
            (
                "src/services/dax_lint.rs",
                FileScope::AllProductionFunctions,
            ),
        ],
        &["join(\".engram\")"],
    );

    assert!(
        occurrences.is_empty(),
        "the read path must not construct live `.engram` paths directly. The F24 inventory's \
         pinned-operational `.engram` exceptions live at startup/activation seams, not inside \
         the request handlers or read services this guard scans.\noccurrences:\n{}",
        render_occurrences(&occurrences)
    );
}
