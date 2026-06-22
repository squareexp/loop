# Loop Engineering Language

Write `.loop` files — structured task contracts that tell AI agents exactly what to build, what tools they can use, and how to confirm it worked.

Unlike a chat prompt, a `.loop` file is a **compiler-checked specification**. Wrong brackets cause a syntax error. Missing required blocks are caught before the agent starts.

---

## Features

- **Syntax highlighting** — block keywords, tool declarations, field names, types, and return arrows all highlighted distinctly
- **Snippets** — type `goal`, `task`, `loop`, etc. to expand full blocks with tab stops
- **Bracket enforcement** — `Goal` uses `[ ]`, everything else uses `{ }`, wrong bracket = error
- **File icon** — `.loop` files get their own icon in the Explorer

---

## Quick look

```loop
Goal [
    Build a REST API with JWT authentication and PostgreSQL persistence.
]

Task {
    "Create users table migration"
    "Implement POST /auth/register and POST /auth/login"
    "Protect GET /api/tasks with auth middleware"
}

Discovery {
    scan: ["src/**/*.rs", "migrations/**/*.sql"]
    find: [
        "Does an auth module already exist?"
        "What database connection setup is in place?"
    ]
}

Planning {
    steps: [
        "Run discovery"
        "Create database schema"
        "Implement auth endpoints"
        "Write integration tests"
    ]
    max_iterations: 6
}

Execution {
    tools: [
        read_file(path: string) -> string
        write_file(path: string, content: string) -> bool
        run_command(cmd: string) -> string
    ]
    strategy: "Execute steps in order. Read before writing. Verify after each step."
}

Verification {
    checks: [
        "cargo test passes with exit code 0"
        "POST /auth/login returns 200 with a JWT"
        "GET /api/tasks returns 401 without Authorization header"
    ]
    on_fail: retry
    max_retries: 4
}
```

---

## The seven blocks

| Block | Brackets | Required | Purpose |
|---|---|---|---|
| `Goal` | `[ ]` | yes | What you want, in plain language |
| `Memory` | `{ }` | no | State the agent carries between sessions |
| `Task` | `{ }` | no | Specific implementation items |
| `Discovery` | `{ }` | yes | What to read and understand before writing anything |
| `Planning` | `{ }` | yes | Ordered steps and iteration budget |
| `Execution` | `{ }` | yes | Tools the agent can call and how to use them |
| `Verification` | `{ }` | yes | Success criteria — what "done" actually means |

---

## Snippets

Type the start of a block name and press Tab:

| Trigger | Expands to |
|---|---|
| `loop` or `lf` | Complete file with all seven blocks |
| `goal` or `go` | `Goal [ ... ]` |
| `memory` or `me` | `Memory { ... }` |
| `task` or `ta` | `Task { ... }` |
| `discovery` or `di` | `Discovery { ... }` |
| `planning` or `pl` | `Planning { ... }` |
| `execution` or `ex` | `Execution { ... }` |
| `verification` or `ve` | `Verification { ... }` |
| `tool` | Tool declaration with typed parameters |

Tab through each placeholder to fill in the details.

---

## Bracket rules

`Goal` is the only block that uses square brackets. Everything else uses curly braces. Tool parameters use parentheses. The compiler rejects wrong brackets with a clear message:

```loop
// Correct
Goal [ describe the task here ]
Memory { project_path: "./myapp" }
Task { "do this" "then this" }

// Syntax errors
Goal { describe the task }   // ERROR: Goal requires [ ] not { }
Task [ "item" ]              // ERROR: Task requires { } not [ ]
Execution ( ... )            // ERROR: Execution requires { } not ( )
```

---

## CLI

The extension pairs with the `loop` CLI for validation and execution:

```sh
npm install -g @squareexperience/loop   # install

loop init                         # create .loop/skills, Memory/, Goal.loop
loop check Goal.loop              # validate the file — shows per-block status
loop run Goal.loop                # run with an AI agent
loop verify Goal.loop             # run verification checks
loop status                       # show current state and failure history
loop inspect Goal.loop            # print the parsed AST
```

Install: [npmjs.com/package/@squareexperience/loop](https://www.npmjs.com/package/@squareexperience/loop)

---

## How it works

1. Write a `.loop` file describing what you want built
2. Run `loop init` — creates the workspace structure and drops a skill document that explains the language to the agent
3. Give the `.loop` file to an AI agent (Claude, Gemini, GPT-4, etc.)
4. The agent reads Discovery, runs Planning steps with the declared tools, and checks Verification after each iteration
5. Run `loop verify Goal.loop` when you think it's done — if all checks pass, state is updated and the file is marked complete

Every failed tool call and failed check is recorded in `.loop/state.json`. The agent reads this on each iteration and doesn't repeat the same mistakes.

---

## Requirements

- VS Code 1.60 or later
- `loop` CLI for running and validating files (optional but recommended)

---

## Source

[github.com/squareexp/loop](https://github.com/squareexp/loop)
