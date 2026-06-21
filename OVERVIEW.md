# Project Overview: Loop DSL

## What We Want to Fix

Autonomous AI agents running general-purpose code (like Python or Node.js) suffer from critical engineering challenges when deployed in production:
1. **Unconstrained Reasoning Loops**: Agents get trapped in recursive cycles or repetitive tool executions, burning token budgets and API costs with no built-in terminal prevention.
2. **State Corruption & Drift**: Agent memory and variables drift over long contexts. There is no structured "ledger" of state mutations, making debuggability and deterministic rollbacks impossible.
3. **Runaway Side-Effects**: LLMs calling raw shell commands or editing code without synchronous sandboxed safety checks can break the host system or execute malicious code.
4. **Context Inflation & Memory Loss**: Passing entire tool execution histories back to the LLM degrades its reasoning capacity over time. Swapping model providers mid-task is impossible without wiping active state.

---

## What We Want This Language to Be

**Loop** is a domain-specific agentic language (DSL) and sandbox-isolated runtime designed to define, type-check, and run agent workflows safely. It represents the transition from ad-hoc prompting to **principled compiler-level loop engineering**.

We want Loop to be:
- **Declarative**: All agent workflows must be declared with a strict schema (`task`, `state`, `tools`, `invariant`, `strategy`, `until`, `fallback`).
- **Provably Safe**: Synchronous assertions (`invariant`) are enforced at the compiler and runtime level, running immediately after any tool executes.
- **State-Ledgered**: All memory mutations are logged into a transactional key-value store, enabling instant state recovery and rollback to the last valid state if an invariant is violated.
- **Provider-Neutral**: Mid-session provider swaps (e.g., from Gemini to Claude) are natively supported by compiling the same state ledger into different provider prompt schemas.

---

## How It Should Work

Loop operates in three core phases managed entirely by the compiled binary:

### 1. Compilation & Checking
- **Parser (Pest)**: Compiles the `.loop` file into an Abstract Syntax Tree (AST).
- **Type Checker**: Statically validates that:
  - All expressions in the `invariant` and `until` blocks yield booleans.
  - All referenced variables exist in the `state` schema.
  - All executed tools match their declared parameter signatures.

### 2. Sandbox VM Execution
- **State Initialization**: The VM creates a session entry in the local `sled` transactional database (`~/.loop/store/`).
- **Reasoning Step**: The VM compiles the active state and task into a provider-specific prompt.
- **Action Step**: The LLM returns a tool execution request conforming to the JSON mutation schema.
- **Observation Step**: The VM executes the tool inside a restricted directory sandbox with path traversal protection.

### 3. Invariant Evaluation & Rollback
- Immediately after the tool output is received, the VM evaluates the `invariant` expressions against the updated state ledger.
- **Success**: The VM saves a new state snapshot to `sled` and proceeds to the next iteration.
- **Failure**: The VM discards the mutation, rolls back the state ledger to the previous valid snapshot, logs the violation, and prompts the agent to try a different strategy.
- **Completion**: Iteration continues until the `until` expression evaluates to `true`, or it runs out of budget, triggering the `fallback` strategy.
