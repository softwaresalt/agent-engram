---
title: Engram Quickstart
description: Install engram, initialize a workspace, connect a client, and run the first sync and query flow.
---

## Overview

This guide gets you from zero to a working Engram-backed workspace. It covers
both the release install path (recommended) and building from source, then walks
through workspace setup and the first query.

## Prerequisites

| Requirement | Notes |
|---|---|
| Git repository | Engram binds only to Git workspaces |
| Supported MCP client | Any client that can launch a stdio MCP server |
| Rust 1.85+ | Only required for building from source — install with [rustup](https://rustup.rs) |

## Install from release

The fastest path. A single command downloads the latest release binary and adds
it to your PATH.

**macOS (Apple Silicon)**

```sh
curl -fsSL https://raw.githubusercontent.com/softwaresalt/agent-engram/main/scripts/install.sh | sh
```

**Linux (x86_64)**

```sh
curl -fsSL https://raw.githubusercontent.com/softwaresalt/agent-engram/main/scripts/install.sh | sh
```

**Windows (x86_64, PowerShell)**

```powershell
irm https://raw.githubusercontent.com/softwaresalt/agent-engram/main/scripts/install.ps1 | iex
```

Override the install directory with the `ENGRAM_INSTALL_DIR` environment variable
if `~/.engram/bin` (Unix) or `%LOCALAPPDATA%\Programs\engram` (Windows) is not
suitable.

### Manual download

If you prefer not to pipe to a shell:

1. Go to [GitHub Releases](https://github.com/softwaresalt/agent-engram/releases/latest)
2. Download the archive for your platform:
   * Linux x86_64: `engram-<tag>-x86_64-unknown-linux-gnu.tar.gz`
   * macOS ARM: `engram-<tag>-aarch64-apple-darwin.tar.gz`
   * Windows x86_64: `engram-<tag>-x86_64-pc-windows-msvc.zip`
3. Extract the archive and place the `engram` binary on your PATH

## Build from source

```bash
git clone https://github.com/softwaresalt/agent-engram.git
cd agent-engram
cargo build --release
```

The resulting binary is `target/release/engram` on Unix-like systems and
`target\release\engram.exe` on Windows.

## Initialize the workspace

Run `engram install` from the repository you want Engram to manage:

```bash
cd /path/to/your/workspace
engram install
```

If you built from source instead of using the installer, use the full path to
the binary: `/path/to/agent-engram/target/release/engram install`

The installer prepares the workspace for Engram and writes the main runtime and
client artifacts below.

| Path | Purpose |
|---|---|
| `.engram/` | Workspace-local state, config, registry, logs, and runtime artifacts |
| `.vscode/mcp.json` | VS Code MCP stdio entry |
| `.cursor/mcp.json` | Cursor helper config |
| `.github/copilot-instructions.md` | GitHub Copilot helper content |
| `.claude/instructions.md` | Claude helper content |

> [!NOTE]
> If your client expects a plain stdio MCP entry, use the manual config below.
> Generated helper files vary by client and may include compatibility-oriented
> content that you should review before relying on it.

## Connect a client

The default entry point is the shim. Running `engram` with no subcommand is the
same as running `engram shim`.

Use a stdio MCP entry like this when you need to configure a client manually:

```json
{
  "mcpServers": {
    "engram": {
      "type": "stdio",
      "command": "/absolute/path/to/engram",
      "args": [],
      "cwd": "/absolute/path/to/your/workspace"
    }
  }
}
```

## Bind the workspace and build the first index

Bind once:

```bash
engram bind
```

Then build or refresh the code graph:

```bash
engram sync
```

Use these variants when you need them:

* `engram sync` increments an existing index and bootstraps a new one when needed
* `engram sync --full` forces a full rebuild through the daemon
* `engram index` is the CLI alias for a full rebuild
* `engram sync --direct` runs the indexing path in-process and exits when done

## Add a notebook content source

Jupyter notebooks belong on the content-ingestion path, not the code graph.
Add a `notebook` source to `.engram/registry.yaml` when you want `.ipynb`
files to appear in `query_memory` and `unified_search`.

```yaml
sources:
  - type: notebook
    path: notebooks
```

Then refresh the workspace:

```bash
engram sync
engram query-memory "SELECT region FROM sales" --format text
```

Notebook indexing in v1 emits one file-level notebook summary plus one
content record per author-written markdown or code cell. Stable cell ordinals
surface through `chunk_id` values such as `cell-0001`.

> [!IMPORTANT]
> Notebook v1 deliberately excludes outputs, execution state, arbitrary magic
> parsing, notebook graph edges, and code-graph symbol extraction. Treat
> notebooks as retrieval content, not as code-graph input.

> [!NOTE]
> The fixture-backed verification flow for notebook support lives in
> `tests/fixtures/notebooks/`, `tests/unit/notebook_extract_test.rs`, and
> `tests/integration/notebook_search_ingestion_test.rs`.

## Add a Power BI source

If your workspace includes PBIP or TMDL assets, register them in
`.engram/registry.yaml` so Engram indexes them through the Power BI ingestion
path instead of the code-symbol path.

```yaml
sources:
  - type: powerbi
    path: analytics
```

Power BI indexing currently covers:

* `report.json` report descriptors
* `model.bim` semantic model files
* `definition/**/*.tmdl` semantic model assets

TMDL support is structural. Tables, columns, measures, relationships, and data
sources are indexed. Full DAX lineage is not.

## Verify Power BI indexing

After adding the registry source, run:

```bash
engram sync
engram search "Total Sales" --content-type powerbi --format text
engram query-graph --format text
```

Healthy verification looks like:

* `engram sync` completes without skipping the Power BI source
* `engram search` returns page, visual, table, or measure records with
  `content_type = powerbi`
* `query_graph` can traverse report, page, visual, table, measure, or
  relationship nodes for the indexed workspace

## Add a PBIP project-definition source

The `powerbi` source above targets the legacy, file-at-a-time layout
(`report.json`, `model.bim`, loose `*.tmdl`). Workspaces saved in the newer
**PBIP project-definition** format — split `.pbip` / `.pbir` / `.pbism`
descriptors with per-report/page/visual JSON under `definition/` and a
folder-based TMDL semantic model — use the dedicated `pbip` source type:

```yaml
sources:
  - type: pbip
    path: analytics
```

`pbip` indexing assembles each `.pbip` project as a whole rather than treating
files in isolation. From one project it emits:

* a workspace record per `.pbip` entry and a report-link record per `.pbir`
* report, page, and visual entities from the `definition/**` JSON
* semantic-model, table, column, measure, and expression entities merged from
  the `<Model>.SemanticModel/definition/**/*.tmdl` files
* a project graph: report → page → visual (`contains`), report → semantic model
  (`depends_on_model`), the model subgraph (`contains` / `relates_to_table`),
  and visual → measure/column (`uses_field`)

### `pbip` vs `powerbi`: which to use

| Use `pbip` when… | Use `powerbi` when… |
|---|---|
| The workspace is saved as a PBIP project (`*.pbip` plus `.Report/` and `.SemanticModel/` folders with `definition/`) | The workspace is a flat folder of `report.json` / `model.bim` / loose `*.tmdl` files |
| You want report→page→visual→field graph linkage across descriptors | You only need per-file entity extraction |

The two source types are independent. Records are scoped by `content_type`
(`pbip` vs `powerbi`) and source path, so a repository can register both without
collision. **Migration of existing `powerbi` sources to `pbip` is intentionally
deferred** — the legacy path remains fully supported and is not deprecated by
this work. Choose the source type that matches each workspace's on-disk layout.

## Verify PBIP indexing

After registering a `pbip` source, run:

```bash
engram sync
engram search "Total Sales" --content-type pbip --format text
engram query-graph --format text
```

Healthy verification looks like:

* `engram sync` completes without skipping the `pbip` source
* `engram search` returns workspace, report, page, visual, table, or measure
  records with `content_type = pbip`
* `query_graph` can walk report → page → visual via `pbi_contains`, report →
  semantic model via `pbi_depends_on_model`, and visual → measure/column via
  `pbi_uses_field` for the indexed project

The project-definition fixture under `tmp/ILSOS-VehicleServices.*` (and the
contract test `tests/contract/pbip_graph_query_test.rs`) is the reference shape:
a single `.pbip` workspace, one `.Report` with ordered pages and visuals, and a
`.SemanticModel` whose `definition/**/*.tmdl` files merge into one model. A
correct run surfaces a `Total Sales`-style measure both as a searchable
`pbip` record and as a `uses_field` graph target reachable from the binding
visual.

## Try a few commands

```bash
engram workspace-status --format text
engram search "daemon lock" --format text
engram symbols --type function --limit 10 --format text
engram map-code run --depth 2 --format text
engram health --format text
```

Those commands confirm that the workspace is bound, the index exists, and the
main search and diagnostics surfaces are reachable.

## Next steps

After the first successful run:

* move to [docs/workflows.md](workflows.md) for task-oriented command recipes
* read [docs/configuration.md](configuration.md) before changing runtime or workspace settings
* use [docs/mcp-tool-reference.md](mcp-tool-reference.md) when you need the current tool catalog
