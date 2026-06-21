# @squareexp/loop

Command-line tool for the Loop Engineering Language. Write `.loop` files to give
AI agents a structured contract instead of a chat prompt.

## Install

```sh
npm install -g @squareexp/loop
```

This downloads a prebuilt binary for your platform. If no binary is available,
it falls back to building from source with `cargo install` (requires Rust).

## Quick start

```sh
loop init             # create workspace structure
loop check Goal.loop  # validate your .loop file
loop run Goal.loop    # run with an AI agent
loop verify Goal.loop # check if verification passes
loop status           # show current state
```

## What is a .loop file?

A `.loop` file describes a task for an AI agent. It defines seven blocks:

```loop
Goal [
    What you want, in plain language.
]

Memory {
    project_path: "./myapp"
    notes: []
}

Task {
    "First specific thing to build"
    "Second specific thing to build"
}

Discovery {
    scan: ["src/**/*.ts", "package.json"]
    find: [
        "What already exists?"
        "What's missing?"
    ]
}

Planning {
    steps: [
        "Run discovery"
        "Implement tasks"
        "Verify"
    ]
    max_iterations: 5
}

Execution {
    tools: [
        read_file(path: string) -> string
        write_file(path: string, content: string) -> bool
        run_command(cmd: string) -> string
    ]
    strategy: "Execute steps in order. Read before writing."
}

Verification {
    checks: [
        "npm test passes"
        "The feature described in Goal works as expected"
    ]
    on_fail: retry
    max_retries: 3
}
```

**Bracket rules are enforced by the compiler:**
- `Goal` uses `[ ]` — everything else uses `{ }`
- Tool parameters use `( )`
- Wrong brackets cause a syntax error with a clear message

## Commands

```
loop init [dir]           Create .loop/skills, Memory/, and Goal.loop
loop check <file>         Validate the file — shows per-block status
loop scaffold <file>      Generate Goal.md, Memory/, skills/, .loop/state.json
loop run <file>           Run with an AI agent (--provider claude|gemini|ollama)
loop verify <file>        Run verification checks, update state and memory
loop status [dir]         Show .loop/state.json (iteration, failures, history)
loop inspect <file>       Print the parsed AST
```

## How it works

1. Write a `.loop` file describing what you want built
2. Run `loop init` to create the workspace structure
3. Run `loop run Goal.loop --provider claude`
4. The agent reads the file, runs Discovery, executes Planning steps using
   the declared tools, and checks Verification after each step
5. Run `loop verify Goal.loop` when you think it's done
6. If all checks pass, Memory is updated and Goal.loop is marked complete

## Failure tracking

Every failed tool call and failed check is recorded in `.loop/state.json`.
The agent reads this on each iteration and avoids repeating the same failures.

```json
{
  "status": "running",
  "iteration": 3,
  "failed_tools": [
    { "tool": "write_file", "error": "permission denied: src/config.rs" }
  ],
  "failed_checks": [
    "POST /auth/login returns 200 with a JWT"
  ],
  "tool_history": [
    "[iter 1] read_file(src/main.rs) → fn main() {...",
    "[iter 2] write_file(src/auth.rs) → true"
  ]
}
```

## VS Code extension

Install the [Loop Engineering Language](https://marketplace.visualstudio.com/items?itemName=squareexp.loopagent)
extension for syntax highlighting, completions, and snippets.

## Source

[github.com/squareexp/loop](https://github.com/squareexp/loop)
