# Loop: State-Ledger & Sandbox-Isolated Agent DSL

Loop is a programming language, execution engine, and CLI framework designed to define, type-check, and run agentic workflow loops safely. It integrates a transactional State Ledger (using Sled) and synchronous Invariant Interceptors to isolate agents and guarantee technical guardrails.

## Features

- **Pest-Based PEG Grammar Frontend**: Complete AST representation and compilation phase with strict static check guarantees.
- **Embedded State Ledger**: Binary-level content-addressable storage using `sled` and `bincode` serialization with sub-millisecond overhead.
- **Synchronous Invariant Check & Rollback**: Rules evaluated immediately after every tool output. Automatic rollback to previous valid state snapshots on invariant violation.
- **Sandboxed Execution Broker**: Secure directory restriction with path traversal validation to isolate system commands.
- **Provider-Agnostic Adapters**: Native support for Google Gemini, Anthropic Claude, and Ollama interfaces.
- **Metrics Tracking Control Panel**: Interactive TUI (using Ratatui) and clean CLI text output fallback to monitor token burn, budget, iteration depth, and invariants.

## Installation

Ensure you have Rust installed (v1.85+ recommended).

```bash
cargo build --release
```

The resulting binary will be located at `target/release/loop`.

## DSL Syntax Example

```
task {
    "Write something to src/main.rs"
}
state {
    is_done: false,
    last_tool_output: false
}
tools {
    tool write_file(path: string, content: string) -> bool
}
invariant {
    state.is_done == false
}
strategy {
    "Run write_file to set content."
}
until {
    state.last_tool_output == true
}
fallback {
    "Report failure."
}
```

## CLI Usage

### Running a DSL Script
```bash
loop run <file.loop> --provider gemini --session-id <session_id>
```

### Switching Providers Mid-Session
Wipes the provider prompt memory (mitigating context inflation) while restoring the State Ledger variables from Sled:
```bash
loop switch --provider claude --session-id <session_id> <file.loop>
```
