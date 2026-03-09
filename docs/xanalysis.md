# Comparative Analysis: Engram, Beads, and Backlog.md

This document provides a comparative analysis of three tools designed to aid in agentic software development: `agent-engram` (Engram), `beads`, and `backlog.md`. The analysis focuses on the needs they address, their relative strengths and weaknesses, and their roles as developer productivity tools.

## 1. Overview

All three tools address the problem of "agent amnesia" by providing a form of persistent memory for AI coding assistants, grounding their work within a project's Git repository. However, they do so with different philosophies, architectures, and capabilities.

*   **Agent Engram (Engram):** A high-performance, local-first MCP (Model Context Protocol) server that provides a rich, multi-modal memory for agents. It combines a structured task graph database with semantic search over the entire workspace, acting as a comprehensive "world model" for an agent.
*   **Beads:** A lightweight, agent-first, git-backed issue tracker. It focuses exclusively on managing a directed acyclic graph (DAG) of tasks to help agents identify and execute "ready work" efficiently.
*   **Backlog.md:** A developer-centric methodology and CLI tool for managing tasks and specifications as markdown files within the repository. It emphasizes human-readable documentation and spec-driven collaboration between developers and AI agents.

## 2. Core Philosophy and Approach

The fundamental difference lies in their primary focus:

*   **Engram** is a **holistic context server**. Its goal is to provide an agent with a rich, real-time understanding of both the *tasks to be done* and the *context in which to do them*. It's a backend for sophisticated agents that need to query and reason about the entire workspace.
*   **Beads** is a **pure execution tool**. It is laser-focused on the dependency graph of work. Its philosophy is to provide a clear, machine-readable "what's next" for an agent that is executing a pre-defined plan.
*   **Backlog.md** is a **collaborative planning tool**. It prioritizes a human-and-agent-readable format for defining and specifying work. It acts as the bridge between human intent and agent execution, focusing on the "spec" as the source of truth.

## 3. Comparative Analysis

| Feature                  | Agent Engram                                 | Beads                                                  | Backlog.md                                               |
| ------------------------ | ------------------------------------------------------ | ------------------------------------------------------ | -------------------------------------------------------- |
| **Primary User**         | Sophisticated AI Agent                                 | AI Agent                                               | Human Developer & AI Agent                               |
| **Storage Mechanism**    | Embedded SurrealDB, flushes to `.engram/` Markdown     | JSONL lines in a `.beads/` file within Git             | Plain Markdown files in a `backlog/` directory           |
| **Interface**            | Real-time HTTP SSE/JSON-RPC Server (MCP)               | Command-Line Interface (CLI) with `--json` output      | Command-Line Interface (CLI) & Web Viewer                |
| **Task Management Model**| Task Graph with dependencies (DAG)                     | Directed Acyclic Graph (DAG) of issues                 | Flat or nested lists within Markdown files               |
| **Semantic Search**      | **Yes**, hybrid vector + keyword search                | No                                                     | No                                                       |
| **Real-time Interaction**| **Yes**, via persistent SSE connection                 | No, requires invoking the CLI for each operation       | No, requires invoking the CLI for each operation         |
| **Strengths**            | - Rich, real-time context<br>- Semantic search<br>- High performance<br>- Structured + unstructured data | - Lightweight & simple<br>- Strong dependency tracking (DAG)<br>- Purely agent-focused | - Human-readable<br>- Great for specs & planning<br>- Low friction (just Markdown) |
| **Weaknesses**           | - Higher complexity (it's a server)<br>- Opinionated protocol (MCP)<br>- Requires a more sophisticated agent | - Execution only, not for planning<br>- No unstructured context search<br>- No real-time updates | - Less structured queries<br>- Weaker dependency tracking<br>- Potential for merge conflicts |

## 4. Use Cases and Scenarios

These tools are not mutually exclusive and could even be used together. Each shines in a different phase of the agentic development loop.

### When to use `backlog.md`:
Choose `backlog.md` for the **planning and specification phase**. It excels at:
*   Breaking down high-level goals into concrete tasks.
*   Writing detailed specifications that both humans and agents can understand.
*   Collaborating with an agent to define the scope of work.
*   A human developer managing the overall project backlog that an agent will consume.

**Example:** A developer uses `backlog.md` to create `task-123.md` with a detailed spec for a new feature. They then instruct their agent: "Implement the feature as described in `backlog/task-123.md`."

### When to use `beads`:
Choose `beads` for the **pure execution phase**, especially with autonomous agents. It is ideal for:
*   Managing complex work with many interdependent steps.
*   Enabling an agent to work autonomously, picking up the next "ready" task without human intervention.
*   Workflows where a plan has already been created (perhaps from `backlog.md`) and needs to be executed reliably.

**Example:** An agent, having been assigned a large feature, breaks it down into 20 sub-tasks and creates a dependency graph in `beads`. Over multiple sessions, it queries `beads` for ready work, executes it, and marks tasks as complete, ensuring it never works on a blocked task.

### When to use `agent-engram`:
Choose `engram` for building **advanced, context-aware agents**. It is the best fit for:
*   Agents that need to answer questions about the codebase ("Where is the authentication logic?").
*   Workflows requiring an agent to have a deep, persistent "memory" of previous actions, decisions, and code context.
*   Real-time applications where an agent needs to react to changes in the workspace.
*   Providing a single, unified backend for an agent that handles tasks, context, and search.

**Example:** An agent is tasked with a vague bug report: "The login page is sometimes slow." The agent uses Engram's `query_memory` tool with the query "login performance" to find relevant code, specs, and previous decisions. It then uses the task management tools to create and track sub-tasks for debugging and fixing the issue, all while maintaining a live connection to the Engram server.

## 5. Conclusion

`backlog.md`, `beads`, and `agent-engram` represent three different points on a spectrum of agentic tooling, from simple, human-centric planning to sophisticated, agent-centric execution and context management.

*   `backlog.md` is the **spec and planning layer**, facilitating human-agent collaboration.
*   `beads` is the **execution layer**, providing a robust task-tracking system for autonomous agents.
*   `engram` is the **memory and context layer**, providing a comprehensive world model that enables more intelligent and capable agents.

For a mature agentic software development workflow, one could imagine a process where `backlog.md` is used for initial planning, the output of which is used to populate a `beads` graph for execution, all while an `engram` server runs in the background to provide deep contextual understanding to the agent carrying out the work.
