# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - ReleaseDate

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
[Unreleased]: https://github.com/softwaresalt/agent-engram/compare/v0.0.1...HEAD
