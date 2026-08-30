# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - ReleaseDate

## [0.3.0-rc.1] - 2026-08-29

> [!WARNING]
> **This is a release candidate for dogfooding only.** It is not a stable
> release and carries no stability or compatibility guarantee.
>
> **Known defect:** on 2026-08-29, `engram workspace-status` and indexed search
> timed out because the daemon did not reach Ready within 30 seconds. Cold open
> of a 135 MB Cozo database has been measured at approximately 7.5 minutes.
> Root-cause investigation is tracked by
> [`002-SP`](https://github.com/softwaresalt/agent-engram/blob/main/.backlogit/queue/002-SP.md).
> Raising the readiness timeout is explicitly not an accepted remediation.
> Stable `v0.3.0` remains blocked until the
> [reliability acceptance gates](https://github.com/softwaresalt/agent-engram/blob/main/docs/decisions/2026-08-29-v0.3.0-rc.1-rollback-and-observability.md#stable-v030-reliability-acceptance-gates)
> pass.
>
> Engram resolves `rmcp` 1.8.0. MCP 2026-07-28 modernization is planned and is
> not included in this RC. Apple Intel (`x86_64-apple-darwin`) is not a
> supported release artifact. See the
> [rollback, withdrawal, and observability contract](https://github.com/softwaresalt/agent-engram/blob/main/docs/decisions/2026-08-29-v0.3.0-rc.1-rollback-and-observability.md)
> before starting a dogfood rollout.

### Added

- Shared HCL parsing for Terraform-family files and reviewed SQL `CREATE PROCEDURE` support through an immutable grammar fork
- Deeper Power BI TMDL and DAX intelligence, plus Spark notebook table and path lineage
- Portable retrieval and graph-recall evaluation with deterministic coverage and agent-visible catalog oracles
- Per-call usage telemetry, measurement reports, indexing progress, and bounded embedding-backfill memory
- Canonical CLI-to-MCP parity documentation and drift coverage across the exposed tool catalog
- Version-generic packaged-archive smoke verification for release structure, CLI version/help, and MCP stdio behavior

### Changed

- Cross-file, method, Python namespace, qualified-caller, and shadowing-aware call resolution now fails closed when identity is ambiguous
- Code-graph revalidation, source reconciliation, and stale-edge cleanup now operate from versioned, single-snapshot evidence
- Daemon sync and indexing coordination now use single-authority generation, persistence, drain, and IPC response boundaries
- Workspace identity and Git admission are capability-rooted, no-follow, worktree-aware, and hardened against Windows object-identity races
- The MCP shim now serves `initialize` and the tool catalog before daemon readiness, tolerates Copilot `server/discover`, recovers after late readiness, and distinguishes transient from terminal health
- CLI diagnostics now expose self-identifying build versions, direct/full indexing controls, and corrected daemon and workspace status

### Fixed

- Windows stale-daemon recovery now validates process identity before reusing or replacing runtime state
- Power BI marker persistence, parser correctness, and write durability no longer leave partial or misleading state
- Cold CLI request IDs and final JSON response frames remain correlated across daemon IPC boundaries
- Dependency-audit and CI/release gates fail closed against known vulnerable or unverified release paths

## [0.2.0] - 2026-06-15

### Added

- Power BI PBIP project-definition indexing: dedicated `.pbip`/`.pbir`/`.pbism` source type and dispatch, project-definition file collector with deletion sweep, page/visual/report-identity extraction, `.pbism` descriptor + TMDL semantic-model assembly, and PBIP content records plus project graph edges
- Power BI code graph: `PowerBiNode`, `PowerBiEdge`, and `PowerBiEdgeType` models persisted and traversable via `powerbi_node`/`powerbi_edge` relations
- `powerbi-tmdl-parser` crate with hardened TMDL semantic-model extraction
- Jupyter notebook source support
- Telemetry instrumentation: telemetry envelope and indexing metrics
- Indexing progress visibility: streamed prewarm and indexing progress in the CLI and extension, plus direct prewarm startup readiness
- Markdown chunk-retrieval guardrails
- Oversized-file resilience: `oversized_files_skipped` counter and early metadata size check
- One-liner install scripts for engram

### Changed

- `build_model` borrows TMDL snapshot content instead of cloning, reducing peak memory during semantic-model rebuilds
- CI and release workflow hardening

### Fixed

- PBIP: avoid char-boundary panic in `report_display_name`; exclude skipped (oversized/non-UTF-8) files from ingested/unchanged counts; skip symlinked files and directories in the collector; emit `Relationship` and `DataSource` nodes from the Power BI graph builder
- Telemetry and health-report correctness; progress drain and non-blocking fixes; gate notebook tests on the cozo backend

## [0.1.0] - 2026-05-13

### Changed

- Storage backend migrated from SurrealDB to CozoDB

### Added

- Shim + daemon architecture: per-workspace MCP plugin model
- IPC transport: Unix domain sockets and Windows named pipes
- File watching with debounced change detection
- TTL-based daemon lifecycle with configurable idle timeout
- Plugin installer: `engram install`, `update`, `reinstall`, `uninstall`
- Plugin configuration via `.engram/config.toml`
- Security hardening: socket permissions, path traversal rejection, IPC size limits
- Workspace-moved detection with graceful shutdown
- 43 MCP tools for task management, code graph, and workspace operations

<!-- next-url -->
[Unreleased]: https://github.com/softwaresalt/agent-engram/compare/v0.3.0-rc.1...HEAD
[0.3.0-rc.1]: https://github.com/softwaresalt/agent-engram/compare/v0.2.0...v0.3.0-rc.1
[0.2.0]: https://github.com/softwaresalt/agent-engram/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/softwaresalt/agent-engram/releases/tag/v0.1.0
