//! A1 — canonical module-path derivation for Rust files.
//!
//! Derives the **workspace-global** canonical module path for a Rust source
//! file from the crate's filesystem layout, and enumerates the workspace
//! crate-name set used to classify a path root as workspace-owned vs external
//! (adversarial remediations D1/D3).
//!
//! Canonical paths are **crate-name rooted** (`engram::services::parsing::rust`)
//! rather than `crate::`-rooted, so identities are unique across a multi-crate
//! workspace — a prerequisite for the fail-closed singleton guarantee.
//!
//! Deterministic and **fail-closed**: any layout that cannot be mapped
//! unambiguously (binaries, examples, integration tests, `build.rs`,
//! non-identifier segments, unknown roots) yields `None`.
//!
//! Limitation (deferred to Unit B): [`module_path_for_file`] derives the path
//! purely from the crate's **filesystem layout** — it does not inspect `mod`
//! attributes. The [`mod_mapping_is_non_default`] helper lets a caller detect
//! `#[path=…]` / `#[cfg(…)]` mappings, but it is not yet wired into file→path
//! derivation, so a file selected through such a mapping is still assigned its
//! filesystem-derived path. Full non-default-`mod` rigor lands with Unit B (D1).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A workspace crate: its Rust identifier name and the workspace-relative
/// directory that owns its `src/` tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateRoot {
    /// Crate identifier with hyphens normalised to underscores, e.g.
    /// `powerbi_tmdl_parser`.
    pub name: String,
    /// Workspace-relative crate directory (forward slashes, no trailing slash).
    /// The crate's sources live under `<dir>/src/`; the empty string denotes the
    /// workspace-root crate (sources under `src/`).
    pub dir: String,
}

impl CrateRoot {
    /// The `src/` prefix (with trailing slash) that a workspace-relative file of
    /// this crate must start with.
    fn src_prefix(&self) -> String {
        if self.dir.is_empty() {
            "src/".to_owned()
        } else {
            format!("{}/src/", self.dir)
        }
    }
}

/// The set of crates that make up the current workspace.
///
/// Crate roots are kept sorted by descending `dir` length so that a file is
/// attributed to the **most specific** (longest-prefix) crate — a member crate
/// wins over the workspace-root crate.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceCrates {
    crates: Vec<CrateRoot>,
    /// Dependency keys rebound to an EXTERNAL package via Cargo's `package = "…"`
    /// rename in any workspace manifest (C9-1). Such a key shadows a same-named
    /// workspace member from the renaming crate's perspective, so the
    /// workspace-crate fast path must fail closed for it rather than forge an
    /// edge to the colliding member. Sorted + deduped.
    renamed_dep_keys: Vec<String>,
}

/// Canonical Rust workspace context captured at graph-index time.
///
/// Retrieval eval uses this whole snapshot as one unit when reconciling
/// denominator spellings against resolved edge identities. Loading the same
/// crate roots, dependency rename set, and unsafe module prefixes that produced
/// the indexed edges prevents live-manifest or live-remap drift from turning a
/// genuine miss into a collapsed hit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalWorkspace {
    /// Workspace crate roots and dependency-renamed keys.
    pub crates: WorkspaceCrates,
    /// Module prefixes whose canonical resolution is unsafe.
    pub unsafe_prefixes: HashSet<String>,
    /// 096-F/C6-1: workspace-relative directories that are provable Python
    /// regular packages (each contains an `__init__.py`). Persisted so an
    /// incremental sync can detect a package-topology change (an `__init__.py`
    /// added or removed) and recompute affected descendant `canonical_path`
    /// values the content-hash skip would otherwise leave stale — even for an
    /// empty `__init__.py`, which sync never persists as a code file.
    /// `#[serde(default)]` keeps snapshots written before this field loadable
    /// (fail-closed empty, forcing a conservative recompute).
    #[serde(default)]
    pub python_packages: HashSet<String>,
}

impl WorkspaceCrates {
    /// Build a crate set, ordering roots for longest-prefix attribution.
    #[must_use]
    pub fn new(mut crates: Vec<CrateRoot>) -> Self {
        crates.sort_by_key(|c| std::cmp::Reverse(c.dir.len()));
        Self {
            crates,
            renamed_dep_keys: Vec::new(),
        }
    }

    /// Is `name` the identifier of a crate in this workspace? Used to classify a
    /// leading path segment as workspace-owned (resolvable) vs external
    /// (`std`/`tokio`/… — fail-closed). (D3)
    #[must_use]
    pub fn is_workspace_crate(&self, name: &str) -> bool {
        self.crates.iter().any(|c| c.name == name)
    }

    /// Attach the set of dependency keys rebound to an external package by a
    /// Cargo `package = "…"` rename (C9-1). Deduplicated on assignment.
    #[must_use]
    pub fn with_renamed_dep_keys(mut self, mut keys: Vec<String>) -> Self {
        keys.sort();
        keys.dedup();
        self.renamed_dep_keys = keys;
        self
    }

    /// Is `name` a dependency key that some workspace manifest rebinds to an
    /// EXTERNAL package via `package = "…"`? Such a rename shadows a same-named
    /// workspace member, so the workspace-crate fast path must fail closed for
    /// `name` (C9-1, no-false-edge invariant). Workspace-wide over-approximation
    /// is safe: it only ever drops an edge, never invents one.
    #[must_use]
    pub fn is_dependency_renamed(&self, name: &str) -> bool {
        self.renamed_dep_keys.iter().any(|k| k == name)
    }

    /// Number of crates enumerated.
    #[must_use]
    pub fn len(&self) -> usize {
        self.crates.len()
    }

    /// Whether the crate set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.crates.is_empty()
    }
}

/// A resolved, crate-name-rooted module path (e.g. `engram::services::parsing`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModulePath {
    /// The owning crate identifier (root of the canonical path).
    pub crate_name: String,
    /// Module segments beneath the crate root; empty for the crate root module.
    pub segments: Vec<String>,
}

impl ModulePath {
    /// Render the canonical `crate_name::seg::seg` string.
    #[must_use]
    pub fn to_canonical(&self) -> String {
        if self.segments.is_empty() {
            self.crate_name.clone()
        } else {
            format!("{}::{}", self.crate_name, self.segments.join("::"))
        }
    }

    /// A child module path extended by one inline-`mod` segment.
    #[must_use]
    pub fn child(&self, segment: &str) -> ModulePath {
        let mut segments = self.segments.clone();
        segments.push(segment.to_owned());
        ModulePath {
            crate_name: self.crate_name.clone(),
            segments,
        }
    }

    /// The parent module path, or `None` at the crate root (`super` off-root).
    #[must_use]
    pub fn parent(&self) -> Option<ModulePath> {
        if self.segments.is_empty() {
            None
        } else {
            let mut segments = self.segments.clone();
            segments.pop();
            Some(ModulePath {
                crate_name: self.crate_name.clone(),
                segments,
            })
        }
    }
}

/// Derive the file-level canonical module path from the default Rust filesystem
/// layout, or `None` (fail-closed) when the layout is not deterministically
/// mappable.
///
/// Mapping rules (crate-name rooted):
/// - `<crate>/src/lib.rs` / `main.rs` → the crate root module (no segments).
/// - `<crate>/src/foo.rs` and `<crate>/src/foo/mod.rs` → `crate::foo`.
/// - `<crate>/src/foo/bar.rs` → `crate::foo::bar`.
///
/// Fail-closed for: binaries (`src/bin/…`), examples, benches, integration
/// tests, `build.rs`, files outside any crate `src/`, and any non-identifier
/// path segment.
#[must_use]
pub fn module_path_for_file(crates: &WorkspaceCrates, rel_path: &str) -> Option<ModulePath> {
    let rel = rel_path.replace('\\', "/");
    // Crate roots are sorted by descending `dir` length, so the first prefix
    // match is the most specific (member crate wins over the workspace root).
    let owner = crates
        .crates
        .iter()
        .find(|c| rel.starts_with(&c.src_prefix()))?;
    let stem = rel[owner.src_prefix().len()..].strip_suffix(".rs")?;
    if stem.is_empty() {
        return None;
    }
    let mut comps: Vec<&str> = stem.split('/').collect();
    if comps.iter().any(|c| c.is_empty()) {
        return None;
    }
    // `src/bin/*`, `src/examples/*`, `src/benches/*` are separate crate roots.
    if matches!(comps[0], "bin" | "examples" | "benches") {
        return None;
    }
    let last = *comps.last()?;
    let segments: Vec<String> = match last {
        "lib" | "main" => {
            // A crate root file must be directly under `src/`.
            if comps.len() == 1 {
                Vec::new()
            } else {
                return None;
            }
        }
        "mod" => {
            comps.pop();
            if comps.is_empty() {
                // `src/mod.rs` is not a valid module file.
                return None;
            }
            comps.iter().map(|s| (*s).to_owned()).collect()
        }
        _ => comps.iter().map(|s| (*s).to_owned()).collect(),
    };
    if segments.iter().any(|s| !is_module_ident(s)) {
        return None;
    }
    Some(ModulePath {
        crate_name: owner.name.clone(),
        segments,
    })
}

/// Whether `s` is a plain ASCII Rust module identifier (`[A-Za-z_][A-Za-z0-9_]*`).
/// Conservative: any non-ASCII or unusual segment fails closed.
pub(super) fn is_module_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    s.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// Whether a `mod` declaration's attributes make its module mapping
/// non-derivable from the default layout (`#[path=…]`) or conditional
/// (`#[cfg(…)]`/`#[cfg_attr(…)]`), in which case canonical identity must fail
/// closed (D1).
#[must_use]
pub fn mod_mapping_is_non_default(attr_texts: &[String]) -> bool {
    attr_texts.iter().any(|a| {
        let inner = a
            .trim()
            .trim_start_matches("#!")
            .trim_start_matches('#')
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim();
        inner.starts_with("path") || inner.starts_with("cfg")
    })
}

/// Enumerate the workspace crates by reading the root `Cargo.toml` workspace
/// members plus any root package. Crate names are normalised to their Rust
/// identifier form (hyphens → underscores).
#[must_use]
pub fn discover_workspace_crates(ws_root: &Path) -> WorkspaceCrates {
    // Symlink-aware containment (Constitution III/IV): resolve the workspace root
    // once; every manifest we read must canonicalize to a real path under it, so a
    // member (or `crates/*` entry) that symlinks outside ws_root is rejected
    // rather than followed. Fail closed if the root itself cannot be resolved.
    let Ok(canonical_root) = ws_root.canonicalize() else {
        return WorkspaceCrates::new(Vec::new());
    };

    let read_manifest = |relative: &Path| {
        let manifest = ws_root.join(relative);
        if !manifest_within_root(&manifest, &canonical_root) {
            return None;
        }
        std::fs::read_to_string(manifest).ok()
    };
    let list_manifest_children = |relative: &Path| {
        let mut children = Vec::new();
        let Ok(entries) = std::fs::read_dir(ws_root.join(relative)) else {
            return children;
        };
        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };
            let candidate = entry.path().join("Cargo.toml");
            if candidate.is_file() && manifest_within_root(&candidate, &canonical_root) {
                if let Some(name) = entry.file_name().to_str() {
                    children.push(name.to_owned());
                }
            }
        }
        children
    };
    discover_workspace_crates_with(&read_manifest, &list_manifest_children)
}

/// Enumerate workspace crates from caller-provided, workspace-relative accessors.
///
/// Code-graph indexing supplies capability-rooted accessors so manifest reads
/// share the same retained workspace authority as source reads.
pub(crate) fn discover_workspace_crates_with(
    read_manifest: &impl Fn(&Path) -> Option<String>,
    list_manifest_children: &impl Fn(&Path) -> Vec<String>,
) -> WorkspaceCrates {
    let root_manifest = PathBuf::from("Cargo.toml");
    let mut candidate_dirs: Vec<String> = vec![String::new()];
    candidate_dirs.extend(read_workspace_member_dirs(
        &root_manifest,
        read_manifest,
        list_manifest_children,
    ));
    // Honour `[workspace] exclude`: Cargo drops an excluded path from the
    // workspace even when a `members` glob would otherwise match it, so an
    // excluded package must not be classified as workspace-owned (fail closed —
    // never treat external code as workspace).
    let excludes = read_workspace_exclude_dirs(&root_manifest, read_manifest);
    candidate_dirs.retain(|dir| dir.is_empty() || !is_excluded_dir(dir, &excludes));
    candidate_dirs.sort();
    candidate_dirs.dedup();

    let mut roots = Vec::new();
    let mut renamed_dep_keys = Vec::new();
    for dir in candidate_dirs {
        let manifest = if dir.is_empty() {
            PathBuf::from("Cargo.toml")
        } else {
            Path::new(&dir).join("Cargo.toml")
        };
        // Collect dependency renames from EVERY in-workspace manifest, including
        // the virtual-workspace root (which carries no `[package]` but may carry
        // `[workspace.dependencies]`), so a rename shadows the colliding member
        // regardless of whether that manifest is itself a crate root (C9-1).
        renamed_dep_keys.extend(read_dependency_rename_keys(&manifest, read_manifest));
        if let Some(name) = read_crate_name(&manifest, read_manifest) {
            roots.push(CrateRoot {
                name: name.replace('-', "_"),
                dir,
            });
        }
    }
    WorkspaceCrates::new(roots).with_renamed_dep_keys(renamed_dep_keys)
}

/// Whether `manifest` canonicalizes to a real path contained within
/// `canonical_root`. Rejects symlinked members that escape the workspace and any
/// manifest that cannot be resolved (Constitution III/IV — fail closed).
fn manifest_within_root(manifest: &Path, canonical_root: &Path) -> bool {
    matches!(manifest.canonicalize(), Ok(real) if real.starts_with(canonical_root))
}

/// Read `[workspace] members` from a manifest, returning workspace-relative
/// crate directories (`"."` → the empty string; a trailing `/*` glob expands one
/// filesystem level).
fn read_workspace_member_dirs(
    manifest: &Path,
    read_manifest: &impl Fn(&Path) -> Option<String>,
    list_manifest_children: &impl Fn(&Path) -> Vec<String>,
) -> Vec<String> {
    let Some(text) = read_manifest(manifest) else {
        return Vec::new();
    };
    let Ok(value) = text.parse::<toml::Value>() else {
        return Vec::new();
    };
    let Some(members) = value
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(toml::Value::as_array)
    else {
        return Vec::new();
    };
    let mut dirs = Vec::new();
    for member in members {
        let Some(raw) = member.as_str() else { continue };
        let raw = raw.replace('\\', "/");
        // Workspace containment (Constitution III/IV): a member spelling must stay
        // within the workspace root. Reject absolute paths (`/x`, `C:/x`, UNC) and
        // any `..` traversal so indexing never reads a `Cargo.toml` outside the
        // workspace via a crafted root manifest.
        if !is_contained_member(&raw) {
            continue;
        }
        if raw == "." {
            dirs.push(String::new());
        } else if let Some(prefix) = raw.strip_suffix("/*") {
            for child in list_manifest_children(Path::new(prefix)) {
                dirs.push(format!("{prefix}/{child}"));
            }
        } else {
            dirs.push(raw.trim_end_matches('/').to_owned());
        }
    }
    dirs
}

/// Collect dependency KEYS that a manifest rebinds to a DIFFERENT external
/// package via Cargo's `package = "…"` rename (C9-1). Scans `[dependencies]`,
/// `[dev-dependencies]`, `[build-dependencies]`, every `[target.*.dependencies]`
/// variant, and the root `[workspace.dependencies]` table. A key is a rename
/// only when it carries an explicit `package` field whose (identifier-normalised)
/// value differs from the (identifier-normalised) key — a plain string/path/version
/// dependency or an identity `package` is NOT a rename. Returned keys are
/// identifier-normalised (`-` → `_`). Fails closed (empty) on any read/parse error.
fn read_dependency_rename_keys(
    manifest: &Path,
    read_manifest: &impl Fn(&Path) -> Option<String>,
) -> Vec<String> {
    let Some(text) = read_manifest(manifest) else {
        return Vec::new();
    };
    let Ok(value) = text.parse::<toml::Value>() else {
        return Vec::new();
    };
    let mut keys = Vec::new();
    collect_dep_renames(value.get("dependencies"), &mut keys);
    collect_dep_renames(value.get("dev-dependencies"), &mut keys);
    collect_dep_renames(value.get("build-dependencies"), &mut keys);
    // `[target.<cfg>.{dependencies,dev-dependencies,build-dependencies}]`
    if let Some(targets) = value.get("target").and_then(toml::Value::as_table) {
        for cfg in targets.values() {
            collect_dep_renames(cfg.get("dependencies"), &mut keys);
            collect_dep_renames(cfg.get("dev-dependencies"), &mut keys);
            collect_dep_renames(cfg.get("build-dependencies"), &mut keys);
        }
    }
    // A root `[workspace.dependencies]` rename is inherited by members via
    // `dep = { workspace = true }`, so it shadows the member from every
    // inheriting crate — collect it too.
    if let Some(ws) = value.get("workspace") {
        collect_dep_renames(ws.get("dependencies"), &mut keys);
    }
    keys
}

/// Push every rename KEY from a Cargo dependency `table` into `out`: a key whose
/// value is an inline table carrying a `package` field whose identifier-normalised
/// value differs from the identifier-normalised key. Collected keys are
/// identifier-normalised (`-` → `_`).
fn collect_dep_renames(table: Option<&toml::Value>, out: &mut Vec<String>) {
    let Some(table) = table.and_then(toml::Value::as_table) else {
        return;
    };
    for (key, spec) in table {
        let Some(pkg) = spec
            .as_table()
            .and_then(|t| t.get("package"))
            .and_then(toml::Value::as_str)
        else {
            continue;
        };
        let norm_key = key.replace('-', "_");
        if norm_key != pkg.replace('-', "_") {
            out.push(norm_key);
        }
    }
}

/// Read `[workspace] exclude` from a manifest, returning normalized
/// workspace-relative directory spellings. Cargo excludes these paths from the
/// workspace even when a `members` glob would otherwise match them; malformed or
/// empty entries are ignored.
fn read_workspace_exclude_dirs(
    manifest: &Path,
    read_manifest: &impl Fn(&Path) -> Option<String>,
) -> Vec<String> {
    let Some(text) = read_manifest(manifest) else {
        return Vec::new();
    };
    let Ok(value) = text.parse::<toml::Value>() else {
        return Vec::new();
    };
    let Some(excludes) = value
        .get("workspace")
        .and_then(|w| w.get("exclude"))
        .and_then(toml::Value::as_array)
    else {
        return Vec::new();
    };
    excludes
        .iter()
        .filter_map(toml::Value::as_str)
        .map(|raw| raw.replace('\\', "/").trim_end_matches('/').to_owned())
        .filter(|dir| !dir.is_empty())
        .collect()
}

/// Whether `dir` is excluded by a `[workspace] exclude` entry — an exact match or
/// any directory nested beneath an excluded root (Cargo's exclude semantics).
fn is_excluded_dir(dir: &str, excludes: &[String]) -> bool {
    excludes
        .iter()
        .any(|excl| dir == excl || dir.starts_with(&format!("{excl}/")))
}

/// Whether a workspace-member spelling stays within the workspace root: rejects
/// absolute paths (`/x`, `C:/x`, UNC `//host`) and any `..` traversal component
/// (Constitution III/IV — indexing must not read manifests outside ws_root).
fn is_contained_member(raw: &str) -> bool {
    if raw.starts_with('/') {
        return false; // POSIX-absolute or UNC (`//host/share`)
    }
    // Windows drive-absolute (`C:/…`).
    let bytes = raw.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        return false;
    }
    !raw.split('/').any(|c| c == "..")
}

/// Read the Rust crate-root name from a manifest: `[lib] name` when configured,
/// otherwise `[package] name`. A package such as `[package] name = "foo-bar"`
/// with `[lib] name = "api"` canonicalizes under `api` (the real crate root),
/// so cross-crate paths converge on the compiler's crate identity.
fn read_crate_name(
    manifest: &Path,
    read_manifest: &impl Fn(&Path) -> Option<String>,
) -> Option<String> {
    let text = read_manifest(manifest)?;
    let value = text.parse::<toml::Value>().ok()?;
    let lib_name = value
        .get("lib")
        .and_then(|l| l.get("name"))
        .and_then(toml::Value::as_str);
    let package_name = value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(toml::Value::as_str);
    lib_name.or(package_name).map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engram_workspace() -> WorkspaceCrates {
        WorkspaceCrates::new(vec![
            CrateRoot {
                name: "engram".to_owned(),
                dir: String::new(),
            },
            CrateRoot {
                name: "powerbi_tmdl_parser".to_owned(),
                dir: "crates/powerbi-tmdl-parser".to_owned(),
            },
        ])
    }

    fn canon(crates: &WorkspaceCrates, rel: &str) -> Option<String> {
        module_path_for_file(crates, rel).map(|m| m.to_canonical())
    }

    #[test]
    fn crate_root_lib_is_bare_crate_name() {
        assert_eq!(
            canon(&engram_workspace(), "src/lib.rs").as_deref(),
            Some("engram")
        );
    }

    #[test]
    fn crate_root_main_is_bare_crate_name() {
        assert_eq!(
            canon(&engram_workspace(), "src/main.rs").as_deref(),
            Some("engram")
        );
    }

    #[test]
    fn single_file_module() {
        assert_eq!(
            canon(&engram_workspace(), "src/config.rs").as_deref(),
            Some("engram::config")
        );
    }

    #[test]
    fn mod_rs_module_equals_directory() {
        assert_eq!(
            canon(&engram_workspace(), "src/services/mod.rs").as_deref(),
            Some("engram::services")
        );
    }

    #[test]
    fn nested_module_from_layout() {
        assert_eq!(
            canon(&engram_workspace(), "src/services/parsing/rust.rs").as_deref(),
            Some("engram::services::parsing::rust")
        );
    }

    #[test]
    fn member_crate_root_uses_crate_name() {
        assert_eq!(
            canon(&engram_workspace(), "crates/powerbi-tmdl-parser/src/lib.rs").as_deref(),
            Some("powerbi_tmdl_parser")
        );
    }

    #[test]
    fn member_crate_nested_module() {
        assert_eq!(
            canon(
                &engram_workspace(),
                "crates/powerbi-tmdl-parser/src/tmdl/lexer.rs"
            )
            .as_deref(),
            Some("powerbi_tmdl_parser::tmdl::lexer")
        );
    }

    #[test]
    fn longest_prefix_attributes_to_member_crate() {
        // A file under the member crate must NOT be attributed to the root crate.
        let m = module_path_for_file(
            &engram_workspace(),
            "crates/powerbi-tmdl-parser/src/parser.rs",
        )
        .expect("member file maps");
        assert_eq!(m.crate_name, "powerbi_tmdl_parser");
    }

    #[test]
    fn windows_separators_normalised() {
        assert_eq!(
            canon(&engram_workspace(), r"src\services\parsing\rust.rs").as_deref(),
            Some("engram::services::parsing::rust")
        );
    }

    #[test]
    fn binary_crate_is_fail_closed() {
        assert_eq!(canon(&engram_workspace(), "src/bin/engram.rs"), None);
    }

    #[test]
    fn integration_tests_dir_is_fail_closed() {
        assert_eq!(canon(&engram_workspace(), "tests/foo.rs"), None);
    }

    #[test]
    fn examples_dir_is_fail_closed() {
        assert_eq!(canon(&engram_workspace(), "examples/demo.rs"), None);
    }

    #[test]
    fn build_script_outside_src_is_fail_closed() {
        assert_eq!(canon(&engram_workspace(), "build.rs"), None);
    }

    #[test]
    fn non_identifier_segment_is_fail_closed() {
        // `foo-bar` is not a valid module identifier under the default layout.
        assert_eq!(canon(&engram_workspace(), "src/foo-bar.rs"), None);
    }

    #[test]
    fn unknown_root_is_fail_closed() {
        let only_member = WorkspaceCrates::new(vec![CrateRoot {
            name: "powerbi_tmdl_parser".to_owned(),
            dir: "crates/powerbi-tmdl-parser".to_owned(),
        }]);
        assert_eq!(canon(&only_member, "src/config.rs"), None);
    }

    #[test]
    fn is_workspace_crate_classifies_roots() {
        let wc = engram_workspace();
        assert!(wc.is_workspace_crate("engram"));
        assert!(wc.is_workspace_crate("powerbi_tmdl_parser"));
        assert!(!wc.is_workspace_crate("std"));
        assert!(!wc.is_workspace_crate("tokio"));
    }

    #[test]
    fn module_path_child_and_parent() {
        let base = ModulePath {
            crate_name: "engram".to_owned(),
            segments: vec!["a".to_owned()],
        };
        let child = base.child("inner");
        assert_eq!(child.to_canonical(), "engram::a::inner");
        assert_eq!(
            child.parent().map(|p| p.to_canonical()).as_deref(),
            Some("engram::a")
        );
        let root = ModulePath {
            crate_name: "engram".to_owned(),
            segments: vec![],
        };
        assert_eq!(root.parent(), None);
    }

    #[test]
    fn mod_mapping_path_attr_is_non_default() {
        assert!(mod_mapping_is_non_default(&[
            "#[path = \"custom.rs\"]".to_owned()
        ]));
    }

    #[test]
    fn mod_mapping_cfg_attr_is_non_default() {
        assert!(mod_mapping_is_non_default(&["#[cfg(test)]".to_owned()]));
        assert!(mod_mapping_is_non_default(&[
            "#[cfg_attr(unix, path = \"u.rs\")]".to_owned()
        ]));
    }

    #[test]
    fn mod_mapping_ordinary_attrs_are_default() {
        assert!(!mod_mapping_is_non_default(
            &["#[derive(Debug)]".to_owned()]
        ));
        assert!(!mod_mapping_is_non_default(&[
            "#[allow(dead_code)]".to_owned()
        ]));
        assert!(!mod_mapping_is_non_default(&[]));
    }

    #[test]
    fn discover_reads_workspace_and_member_crates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\".\", \"crates/powerbi-tmdl-parser\"]\n\n[package]\nname = \"engram\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let member = root.join("crates").join("powerbi-tmdl-parser");
        std::fs::create_dir_all(&member).unwrap();
        std::fs::write(
            member.join("Cargo.toml"),
            "[package]\nname = \"powerbi-tmdl-parser\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let wc = discover_workspace_crates(root);
        assert!(wc.is_workspace_crate("engram"), "root package discovered");
        assert!(
            wc.is_workspace_crate("powerbi_tmdl_parser"),
            "member crate discovered with hyphen->underscore normalisation"
        );
        assert!(!wc.is_workspace_crate("std"));
    }

    #[test]
    fn member_containment_rejects_traversal_and_absolute() {
        // Contained relative members are accepted.
        assert!(is_contained_member("."));
        assert!(is_contained_member("crates/foo"));
        assert!(is_contained_member("a/b/c"));
        // Traversal and absolute spellings escape ws_root → rejected (P1 #4).
        assert!(!is_contained_member("../evil"));
        assert!(!is_contained_member("crates/../../evil"));
        assert!(!is_contained_member("/etc/passwd"));
        assert!(!is_contained_member("C:/Windows"));
        assert!(!is_contained_member("//host/share"));
    }

    #[test]
    fn discover_prefers_lib_name_over_package_name() {
        // A crate whose `[lib] name` differs from `[package] name` canonicalizes
        // under the Rust crate root (`api`), not the package name (Copilot #3).
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"foo-package\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"api\"\n",
        )
        .unwrap();

        let wc = discover_workspace_crates(root);
        assert!(wc.is_workspace_crate("api"), "lib name is the crate root");
        assert!(
            !wc.is_workspace_crate("foo_package"),
            "package name is not the crate root when [lib] name is set"
        );
    }

    #[test]
    fn discover_collects_dependency_rename_keys() {
        // A member that renames a dependency to a name colliding with another
        // workspace member (`util = { package = "external-util" }`) must mark
        // `util` as dependency-renamed so the resolver fails closed for it, while
        // plain (unrenamed) dependencies stay resolvable (C9-1).
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"app\", \"util\"]\n",
        )
        .unwrap();
        let app = root.join("app");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nutil = { package = \"external-util\", version = \"1\" }\nserde = \"1\"\n",
        )
        .unwrap();
        let util = root.join("util");
        std::fs::create_dir_all(&util).unwrap();
        std::fs::write(
            util.join("Cargo.toml"),
            "[package]\nname = \"util\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let wc = discover_workspace_crates(root);
        assert!(wc.is_workspace_crate("util"), "util is a workspace member");
        assert!(
            wc.is_dependency_renamed("util"),
            "util is rebound to an external package by app's dependency rename"
        );
        assert!(
            !wc.is_dependency_renamed("serde"),
            "a plain (unrenamed) dependency is not a rename"
        );
        assert!(
            !wc.is_dependency_renamed("app"),
            "an unrelated member name is not a dependency rename"
        );
    }

    #[test]
    fn read_dependency_rename_keys_scans_all_dependency_tables() {
        // Renames in dev/build/target tables are collected; an identity `package`
        // and a plain string dependency are not (C9-1).
        let dir = tempfile::tempdir().expect("tempdir");
        let manifest = dir.path().join("Cargo.toml");
        std::fs::write(
            &manifest,
            concat!(
                "[package]\nname = \"m\"\nversion = \"0.1.0\"\n\n",
                "[dependencies]\nplain = \"1\"\nident = { package = \"ident\", version = \"1\" }\n",
                "renamed_dep = { package = \"upstream\" }\n\n",
                "[dev-dependencies]\ndev_alias = { package = \"dev-real\" }\n\n",
                "[build-dependencies]\nbuild_alias = { package = \"build-real\" }\n\n",
                "[target.'cfg(unix)'.dependencies]\ntgt_alias = { package = \"tgt-real\" }\n",
            ),
        )
        .unwrap();

        let keys =
            read_dependency_rename_keys(&manifest, &|path| std::fs::read_to_string(path).ok());
        assert!(keys.contains(&"renamed_dep".to_owned()));
        assert!(keys.contains(&"dev_alias".to_owned()));
        assert!(keys.contains(&"build_alias".to_owned()));
        assert!(keys.contains(&"tgt_alias".to_owned()));
        assert!(
            !keys.contains(&"plain".to_owned()),
            "a plain string dependency is not a rename"
        );
        assert!(
            !keys.contains(&"ident".to_owned()),
            "an identity `package` is not a rename"
        );
    }

    #[cfg(unix)]
    #[test]
    fn discover_rejects_symlinked_member_escaping_workspace() {
        // A lexically-contained member that symlinks to a manifest outside the
        // workspace must be rejected, not followed (Constitution III/IV — the
        // symlink-aware canonicalization gate; Copilot #1).
        let outside = tempfile::tempdir().expect("outside tempdir");
        let evil = outside.path().join("evil");
        std::fs::create_dir_all(&evil).unwrap();
        std::fs::write(
            evil.join("Cargo.toml"),
            "[package]\nname = \"evil\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let ws = tempfile::tempdir().expect("ws tempdir");
        let root = ws.path();
        std::fs::create_dir_all(root.join("crates")).unwrap();
        std::os::unix::fs::symlink(&evil, root.join("crates").join("evil")).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/evil\"]\n\n[package]\nname = \"host\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let wc = discover_workspace_crates(root);
        assert!(wc.is_workspace_crate("host"), "root package discovered");
        assert!(
            !wc.is_workspace_crate("evil"),
            "a symlinked member escaping ws_root must not be read"
        );
    }

    #[test]
    fn discover_applies_workspace_exclude() {
        // A package matched by a `crates/*` members glob but listed in
        // `[workspace] exclude` must not be classified as a workspace crate — its
        // root stays external (Cargo exclude semantics; Copilot round-2 #3, fail
        // closed on excluded packages).
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"crates/experimental\"]\n\n[package]\nname = \"engram\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let keep = root.join("crates").join("keep");
        std::fs::create_dir_all(&keep).unwrap();
        std::fs::write(
            keep.join("Cargo.toml"),
            "[package]\nname = \"keep\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let excluded = root.join("crates").join("experimental");
        std::fs::create_dir_all(&excluded).unwrap();
        std::fs::write(
            excluded.join("Cargo.toml"),
            "[package]\nname = \"experimental\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let wc = discover_workspace_crates(root);
        assert!(wc.is_workspace_crate("engram"), "root package discovered");
        assert!(
            wc.is_workspace_crate("keep"),
            "a non-excluded glob member is discovered"
        );
        assert!(
            !wc.is_workspace_crate("experimental"),
            "a member listed in [workspace] exclude must not be workspace-owned"
        );
    }
}
