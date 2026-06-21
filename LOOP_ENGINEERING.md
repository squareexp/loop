# The Primitives and Loops of Loop Engineering

This document details how the **Loop** language maps to the core concepts of **Loop Engineering** (specifically the 4 Loops from LangChain and the 6 Primitives from ReceiptRoller).

---

## Part 1: The Four Stacking Loops

Loop natively implements and manages the stack of agentic loops to automate and govern AI execution:

| Loop Level | Description | Implementation in Loop DSL & Runtime |
| :--- | :--- | :--- |
| **1. The Agent Loop** | The model calling tools in a loop to execute a task. | Enforced by the `task` and `tools` blocks. The runtime executes this cycle recursively until termination. |
| **2. The Verification Loop** | A grading harness that evaluates correctness and retries. | Handled by the `invariant` block. The runtime acts as the grader, rolling back state transactions in `sled` immediately if an invariant is violated. |
| **3. The Event-Driven Loop** | Connecting the agent to triggers, crons, or webhooks. | Managed by the `loop run` and `loop switch` commands. It runs headlessly in TUI/CLI modes and supports execution crons. |
| **4. The Hill-Climbing Loop** | Analyzing execution traces to optimize prompts and configuration. | Managed by session metrics logged locally. Trace outputs and cost statistics are structured to let optimizer agents rewrite the prompt strategy. |

---

## Part 2: The Six Primitives of Loop Engineering

If a loop is the engine, the six primitives are the component parts that make the engine durable and production-ready:

### 1. Automations (The Heartbeat)
- **Concept**: The scheduled triggers that wake the agent up and start the work.
- **Loop Mapping**: The Loop binary runs as an automated execution process, executing script workflows locally or in remote environments.

### 2. Worktrees (Isolation & Parallelism)
- **Concept**: Using isolated directories (like Git worktrees) to prevent file collisions between concurrent agent runs.
- **Loop Mapping**: The runtime sandbox broker restricts tool operations to a local execution sandbox (`.loop_sandbox`) with path traversal check verification.

### 3. Skills (Codified Knowledge)
- **Concept**: Structuring guidelines and conventions in a markdown specification so the model doesn't have to guess or relearn them.
- **Loop Mapping**: Enforced by the `strategy` block. It passes explicit context instructions directly to the LLM agent prompt.

### 4. Connectors (Standardized Interfaces)
- **Concept**: Universal connectors (like Model Context Protocol) to pull issues, databases, or third-party APIs.
- **Loop Mapping**: Configured by the `tools` declaration block. The VM maps tool signatures dynamically to isolated environment commands.

### 5. Sub-agents (The Maker/Checker Split)
- **Concept**: The agent generating code/text must not be the same agent grading it.
- **Loop Mapping**: Handled via the separation between the LLM generator (which operates within `strategy` and drafts mutations) and the compiler VM (which evaluates `invariant` checks independently using deterministic expressions).

### 6. Memory & State (The Glue)
- **Concept**: Keeping agent state outside the model's transient context window so it persists across runs.
- **Loop Mapping**: Managed by the **State Ledger**. Loop uses `sled` (a transactional embedded database) to save session logs and memory variables. A mid-session `loop switch` completely clears LLM prompt memory but restores the exact state variables from the database, mitigating context inflation.
