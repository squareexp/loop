# Loop Project Roadmap

## Phase 1: Core DSL Improvements
- Add support for floating-point primitive types.
- Support complex object definitions in the state block.
- Enhance PEG grammar parsing error diagnostics.

## Phase 2: WASM & Container Isolation
- Integrate a lightweight WASM sandboxed runner for arbitrary code execution.
- Implement Docker container worktree isolation for shell command security.
- Add granular resource limits (CPU, memory, disk) to the sandboxed runtime.

## Phase 3: Advanced Optimization & Metrics
- Add interactive debugger support (breakpoint, step-by-step state inspection).
- Expose token expenditures and budget burn metrics via a JSON endpoint.
- Develop a web interface dashboard to monitor active sessions and execution logs.
