# **AI Agent Harness Evaluation: agent-engram**

## **Executive Summary**

The agent-engram repository demonstrates a highly advanced, file-system-backed agent harness that leans heavily into persistent state management and capability isolation. By externalizing agent roles (.github/agents/), skills (.github/skills/), and memory/tracking (.copilot-tracking/, .backlog/), the repository effectively treats the file system as its primary context and orchestration database.

However, when evaluated against emerging research on Compound AI Systems and **Irreducible Harness Primitives**, several structural gaps emerge—specifically around dynamic context compaction, task granularity, execution sandboxing, and automated evaluation loops. The model is currently vulnerable to "model drift" in long-horizon tasks due to the sheer volume of Markdown logs it must ingest and improperly sized task horizons.

## **1\. State & Context Management Primitive**

**Definition:** How the harness maintains durable state, manages the context window, and prevents token overflow or "context anxiety."

* **Current State:** Excellent. The harness uses a robust schema for memory (.copilot-tracking/memory/), checkpoints (.copilot-tracking/checkpoints/), and backlog hydration. The database itself is branch-aware (Task 009).  
* **Identified Gaps:** The system relies on append-only markdown tracking without an automated **Context Compaction** mechanism. As agents read large histories (e.g., 002-enhanced-task-management-phase-11-memory.md), the KV-cache hit rate drops, and the model's adherence to core instructions degrades (model drift). While agent-engram provides powerful retrieval, uncompacted histories dilute the semantic density of that retrieval.  
* **Proposed Changes:**  
  * **Implement a Compaction Hook:** Create an agent or workflow that monitors the size of .copilot-tracking logs. When a thread exceeds a token threshold, trigger a summarize-and-archive skill that distills the history into a dense, high-signal state file, archiving the verbose logs.  
  * **Context Chunking:** Enforce a rule in markdown.instructions.md where large specifications must be chunked into modular files (e.g., spec-part1, spec-part2) so the agent only loads the exact context required for the immediate task.

## **2\. Task Granularity & Horizon Scoping Primitive**

**Definition:** The methodology used to size, decompose, and sequence work chunks to prevent exponential error compounding and model drift.

* **Current State:** The backlog contains tasks of varying sizes (e.g., Feature-002-Enhanced-Task-Management encompasses 11 separate sub-phases). The decomposition relies heavily on human intuition during the planning phase.  
* **Identified Gaps:** Recent research (METR Time Horizons) demonstrates that agent reliability drops below 50% for tasks taking \>2 hours of human-equivalent effort, and approaches 0% for tasks \>4 hours. If the harness dispatches a multi-day feature specification to a single agent loop, failure is mathematically guaranteed due to sequential error compounding.  
* **Proposed Changes:**  
  * **The 2-Hour Rule:** Program the plan.agent.md and harness-architect.agent.md to strictly enforce "Extreme Decomposition." Any drafted task that is estimated to take a human longer than 2 hours must be rejected and split into smaller atomic milestones.  
  * **Width vs. Depth Isolation:** Ensure tasks are isolated by skill (Width). Do not bundle core Rust database migrations with documentation updates in the same sub-task. Route the database chunk to rust-engineer.agent.md and the documentation chunk to doc-ops.agent.md sequentially.  
  * **Atomic Milestone Validation:** Mandate that every decomposed sub-task must result in a verifiable state (e.g., a passing test or successful build). The agent must yield control to the test runner before proceeding to the next chunk.

## **3\. Orchestration & Routing Primitive**

**Definition:** How work is decomposed, delegated, and routed between specialized agents, including handoffs and stop conditions.

* **Current State:** The harness defines clear specialized roles (harness-architect, rust-engineer, build-orchestrator, pr-review).  
* **Identified Gaps:** The orchestration appears largely "flat" or heavily reliant on human-in-the-loop Copilot prompting to switch contexts. There is a lack of hard **Stop Conditions** or "Doom-Loop" prevention mechanisms within the prompt configurations.  
* **Proposed Changes:**  
  * **Explicit Supervisor Pattern:** Introduce a supervisor.agent.md whose sole job is to read the .backlog/tasks/ and assign discrete chunks to the rust-engineer or doc-ops agents. The supervisor must *not* write code, keeping its context clean to focus purely on state management.  
  * **Stop Conditions & Yielding:** Update ping-loop.prompt.md to include strict turn limits. For example: *"If CI tests fail 3 consecutive times, STOP execution and yield to build-orchestrator.agent.md for environmental analysis."*

## **4\. Tool Execution & Guardrails Primitive**

**Definition:** The mechanisms that allow agents to mutate the environment safely, including sandboxing, policy enforcement, and validation.

* **Current State:** Extremely strong native capability. agent-engram is itself an MCP server, providing rich graph traversal and workspace interactions. Task 009 isolated the database by git branch.  
* **Identified Gaps:** While branch isolation is excellent, there is limited "out-of-process" policy enforcement limiting *what* files an agent can edit. Without a strict sandboxing policy, an agent hallucination could overwrite core harness configurations.  
* **Proposed Changes:**  
  * **Policy Engine via MCP:** Restrict the write.rs tools based on the active agent. E.g., doc-ops.agent.md should only have write permissions for /docs and \*.md files.  
  * **Feature Flag Enforcement:** Bind the integration of new rust modules to strict feature flags, enforcing a rule in rust.instructions.md that all new agent-generated logic must be gated, preventing system-wide instability if the agent introduces a panic.

## **5\. Injection Points & Dynamic Reminders Primitive**

**Definition:** How the harness dynamically surfaces critical constraints, rules, and rules-of-engagement exactly when the agent needs them, rather than front-loading them in a massive system prompt.

* **Current State:** Relies on static global instructions (constitution.instructions.md, writing-style.instructions.md).  
* **Identified Gaps:** Static prompts suffer from the "lost in the middle" phenomenon. If an agent is executing a multi-step refactor, it may forget the constitution rules by step 5\.  
* **Proposed Changes:**  
  * **Tool-Bound Injections:** Modify the harness so that specific instructions are dynamically injected into the prompt *only* when relevant. For example, inject git-merge.instructions.md into the context window only when the agent stages a commit, rather than maintaining it in the global context at all times.  
  * **Definition of Done (DoD) Checks:** Add a pre-flight checklist hook that forces the agent to read the specific task-xxx.md file and output a self-reflection confirming all DoD criteria are met before invoking the final commit tool.

## **6\. Observability & Evaluation Primitive**

**Definition:** Tracking agent efficacy, token usage, failure modes, and implementing automated graders to verify output quality.

* **Current State:** Highly advanced on the telemetry side. Tasks 010 (Effectiveness Metrics) and query tracing (query\_tracing\_test.rs) prove the system is gathering rigorous data.  
* **Identified Gaps:** The evaluation loop is primarily human-driven via PR reviews. There is no automated "Model-Based Grader" operating synchronously to reject poor agent outputs before they reach the tracking/review state.  
* **Proposed Changes:**  
  * **Adversarial Evaluator Agent:** Elevate the role of rust-safety-reviewer.agent.md and architecture-strategist.agent.md to act as automated CI blockers. The harness should require an explicit "Approval" token from the rust-safety-reviewer agent before a branch can be merged or marked complete in the backlog.  
  * **Metrics-Driven Adaptation:** Utilize the metrics collected by get\_branch\_metrics to actively identify inefficient agents. If the Input-to-Output Token Ratio spikes for a specific task, the harness should automatically flag that skill (e.g., fix-ci/SKILL.md) for human review and prompt optimization.