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
//! non-identifier segments, unknown roots) yields `None`. Non-default `mod`
//! mappings (`#[path=…]`) and conditional modules (`#[cfg(…)]`) are reported as
//! non-derivable so callers fail closed rather than guess (D1).

use std::path::Path;

/// A workspace crate: its Rust identifier name and the workspace-relative
/// directory that owns its `src/` tree.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, Default)]
pub struct WorkspaceCrates {
    crates: Vec<CrateRoot>,
}

impl WorkspaceCrates {
    /// Build a crate set, ordering roots for longest-prefix attribution.
    #[must_use]
    pub fn new(mut crates: Vec<CrateRoot>) -> Self {
        crates.sort_by_key(|c| std::cmp::Reverse(c.dir.len()));
        Self { crates }
    }

    /// Is `name` the identifier of a crate in this workspace? Used to classify a
    /// leading path segment as workspace-owned (resolvable) vs external
    /// (`std`/`tokio`/… — fail-closed). (D3)
    #[must_use]
    pub fn is_workspace_crate(&self, name: &str) -> bool {
        self.crates.iter().any(|c| c.name == name)
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
    let last = *comps.last().expect("non-empty after split");
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
fn is_module_ident(s: &str) -> bool {
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
    let mut candidate_dirs: Vec<String> = vec![String::new()];
    candidate_dirs.extend(read_workspace_member_dirs(&ws_root.join("Cargo.toml")));
    candidate_dirs.sort();
    candidate_dirs.dedup();

    let mut roots = Vec::new();
    for dir in candidate_dirs {
        let manifest = if dir.is_empty() {
            ws_root.join("Cargo.toml")
        } else {
            ws_root.join(&dir).join("Cargo.toml")
        };
        if let Some(name) = read_package_name(&manifest) {
            roots.push(CrateRoot {
                name: name.replace('-', "_"),
                dir,
            });
        }
    }
    WorkspaceCrates::new(roots)
}

/// Read `[workspace] members` from a manifest, returning workspace-relative
/// crate directories (`"."` → the empty string; a trailing `/*` glob expands one
/// filesystem level).
fn read_workspace_member_dirs(manifest: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(manifest) else {
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
    let base = manifest.parent().unwrap_or_else(|| Path::new("."));
    let mut dirs = Vec::new();
    for member in members {
        let Some(raw) = member.as_str() else { continue };
        let raw = raw.replace('\\', "/");
        if raw == "." {
            dirs.push(String::new());
        } else if let Some(prefix) = raw.strip_suffix("/*") {
            if let Ok(entries) = std::fs::read_dir(base.join(prefix)) {
                for entry in entries.flatten() {
                    if entry.path().join("Cargo.toml").is_file() {
                        dirs.push(format!("{prefix}/{}", entry.file_name().to_string_lossy()));
                    }
                }
            }
        } else {
            dirs.push(raw.trim_end_matches('/').to_owned());
        }
    }
    dirs
}

/// Read `[package] name` from a manifest, if present.
fn read_package_name(manifest: &Path) -> Option<String> {
    let text = std::fs::read_to_string(manifest).ok()?;
    let value = text.parse::<toml::Value>().ok()?;
    value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
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
}
