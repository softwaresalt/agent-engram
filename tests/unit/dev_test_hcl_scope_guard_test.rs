//! Shipment 117-S scope-preservation guard (Plan unit U2, task 126.002-T).
//!
//! Shipment 117-S owns the six HCL parser test targets. This guard asserts
//! mechanically that the coverage oracle keeps those targets in the required
//! set for HCL source changes, and that their `[[test]]` definitions in
//! `Cargo.toml` are byte-identical (not merely name-matched, per plan-review
//! finding S1).
//!
//! Freeze-scope: this guard never edits anything under `tests/**/hcl_*`.
//!
//! Red phase: scenario one fails until the manifest and oracle (U3) map the
//! HCL source surface to the six HCL targets.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// The six Shipment 117-S HCL test targets.
const HCL_TARGETS: [&str; 6] = [
    "hcl_parser_contract_test",
    "hcl_grammar_abi_test",
    "hcl_parsing_test",
    "hcl_routing_test",
    "hcl_security_test",
    "hcl_indexing_test",
];

/// Frozen byte-identical `[[test]]` definitions for the six 117-S targets.
const HCL_TEST_BLOCKS: [&str; 6] = [
    "[[test]]\nname = \"hcl_parser_contract_test\"\npath = \"tests/contract/hcl_parser_contract_test.rs\"",
    "[[test]]\nname = \"hcl_grammar_abi_test\"\npath = \"tests/unit/hcl_grammar_abi_test.rs\"",
    "[[test]]\nname = \"hcl_parsing_test\"\npath = \"tests/unit/hcl_parsing_test.rs\"",
    "[[test]]\nname = \"hcl_routing_test\"\npath = \"tests/unit/hcl_routing_test.rs\"",
    "[[test]]\nname = \"hcl_security_test\"\npath = \"tests/unit/hcl_security_test.rs\"",
    "[[test]]\nname = \"hcl_indexing_test\"\npath = \"tests/integration/hcl_indexing_test.rs\"",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Run the coverage oracle with `args`, returning combined output and exit code.
fn run_oracle(args: &[&str]) -> (String, i32) {
    let root = repo_root();
    let (program, mut invocation): (&str, Vec<String>) = if cfg!(windows) {
        let script = root.join("scripts").join("test-coverage-oracle.ps1");
        (
            "powershell",
            vec![
                "-NoProfile".to_owned(),
                "-ExecutionPolicy".to_owned(),
                "Bypass".to_owned(),
                "-File".to_owned(),
                script.to_string_lossy().into_owned(),
            ],
        )
    } else {
        let script = root.join("scripts").join("test-coverage-oracle.sh");
        ("bash", vec![script.to_string_lossy().into_owned()])
    };
    for arg in args {
        invocation.push((*arg).to_owned());
    }
    let output = Command::new(program)
        .args(&invocation)
        .current_dir(&root)
        .output()
        .unwrap_or_else(|err| panic!("failed to spawn coverage oracle: {err}"));
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    (combined, output.status.code().unwrap_or(-1))
}

#[test]
fn hcl_targets_required_for_hcl_source_change() {
    let (out, code) = run_oracle(&[
        "--mode",
        "select",
        "--changed",
        "src/services/parsing/hcl.rs",
    ]);
    assert_eq!(code, 0, "oracle select must succeed for HCL source.\n{out}");
    for target in HCL_TARGETS {
        assert!(
            out.contains(&format!("TARGET={target}")),
            "HCL target {target} must be in the required set for an HCL source change.\n{out}"
        );
    }
}

#[test]
fn hcl_target_definitions_are_byte_identical() {
    let cargo = fs::read_to_string(repo_root().join("Cargo.toml"))
        .unwrap_or_else(|err| panic!("failed to read Cargo.toml: {err}"));
    let normalized = cargo.replace("\r\n", "\n");
    for block in HCL_TEST_BLOCKS {
        // Require the path line to terminate the block (blank line, next table,
        // or EOF). A bare substring match would still pass if a semantic field
        // such as `required-features` or `harness` were appended after `path`.
        let terminated = normalized.contains(&format!("{block}\n\n"))
            || normalized.contains(&format!("{block}\n["))
            || normalized.ends_with(block);
        assert!(
            terminated,
            "Shipment 117-S HCL [[test]] definition changed, gained an extra field, or is missing:\n{block}"
        );
    }
}
