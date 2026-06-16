# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - ReleaseDate

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
[Unreleased]: https://github.com/softwaresalt/agent-engram/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/softwaresalt/agent-engram/releases/tag/v0.1.0
