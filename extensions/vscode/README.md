# Loop Engineering Language

VS Code support for `.loop` files — syntax highlighting, completions, and snippets
for the Loop Engineering Language.

## What it does

`.loop` files are structured task specifications for AI agents. Instead of a long
prompt in a chat window, you write a file that defines exactly what to build, what
tools the agent can use, and how to confirm it worked.

```loop
Goal [
    Build a REST API with JWT authentication and PostgreSQL persistence.
]

Task {
    "Create users table migration"
    "Implement POST /auth/register"
    "Implement POST /auth/login returning a JWT"
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
        "Run discovery — scan files and answer find questions"
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

## The seven blocks

| Block | Brackets | Required | Description |
|---|---|---|---|
| `Goal` | `[ ]` | yes | What you want — plain language |
| `Memory` | `{ }` | no | What the agent remembers across sessions |
| `Task` | `{ }` | no | Specific implementation items |
| `Discovery` | `{ }` | yes | What to find before writing code |
| `Planning` | `{ }` | yes | Ordered steps + iteration budget |
| `Execution` | `{ }` | yes | Available tools + strategy |
| `Verification` | `{ }` | yes | Success criteria |

**Bracket rules are strict.** `Goal` uses `[ ]`. Every other block uses `{ }`.
Tool parameters use `( )`. Using the wrong bracket is a syntax error.

## Completions

Type the start of any block name to get a full snippet:

| Type | Gets you |
|---|---|
| `loop` | Complete file with all seven blocks |
| `goal` | `Goal [ ... ]` |
| `memory` | `Memory { ... }` |
| `task` | `Task { ... }` |
| `discovery` | `Discovery { ... }` |
| `planning` | `Planning { ... }` |
| `execution` | `Execution { ... }` |
| `verification` | `Verification { ... }` |
| `tool` | Tool declaration with type placeholders |

Tab through the placeholders to fill in each field.

## Syntax highlighting

- Block keywords in distinct colors (`Goal`, `Memory`, `Task`, etc.)
- Goal content highlighted as an unquoted string
- Tool declarations with function-style coloring
- Field keywords (`scan`, `steps`, `tools`, `checks`, etc.)
- `retry` / `complete` as language constants
- Types (`string`, `int`, `bool`) in type color
- `->` return type arrows

## CLI

The extension pairs with the `loop` CLI:

```sh
npm install -g @loopeng/loop   # install the CLI
loop init                       # create .loop/skills, Memory/, Goal.loop
loop check Goal.loop            # validate your file
loop run Goal.loop              # run with an AI agent
loop verify Goal.loop           # check verification conditions
loop status                     # show current state
loop inspect Goal.loop          # print the parsed AST
```

Install the CLI: [npmjs.com/package/@loopeng/loop](https://www.npmjs.com/package/@loopeng/loop)

## Strict bracket rules (enforced by the compiler)

```loop
// These are correct:
Goal [ ... ]          // Goal uses [ ]
Memory { ... }        // Memory uses { }
Task { ... }          // Task uses { }
Verification { ... }  // All other blocks use { }

// These are syntax errors:
Goal { ... }          // ERROR: expected '[' after 'Goal'
Task [ ... ]          // ERROR: expected '{' after 'Task'
Execution ( ... )     // ERROR: expected '{' after 'Execution'
```

The compiler catches bracket mismatches and tells you exactly which block
got the wrong bracket and what was expected.

## Examples

### Minimal file

```loop
Goal [ Fix the failing auth test ]

Discovery { find: ["What does the test expect?"] }

Planning { steps: ["Read the test" "Fix the implementation"] }

Execution {
    tools: [
        read_file(path: string) -> string
        write_file(path: string, content: string) -> bool
    ]
    strategy: "Read the test first, then fix the source."
}

Verification {
    checks: ["cargo test auth passes"]
    on_fail: retry
    max_retries: 3
}
```

### With memory and tasks

```loop
Goal [
    Migrate the user auth system from sessions to JWT.
    All existing tests must pass after the migration.
]

Memory {
    project_path: "./myapp"
    framework: "Express"
    test_command: "npm test"
    migration_started: false
}

Task {
    "Remove express-session dependency"
    "Add jsonwebtoken dependency"
    "Rewrite POST /login to return a JWT"
    "Add JWT verification middleware"
    "Update all tests that check session state"
}

Discovery {
    scan: ["src/**/*.js", "tests/**/*.test.js", "package.json"]
    find: [
        "Where is session logic currently implemented?"
        "Which routes use session middleware?"
        "How many tests reference session state?"
    ]
}

Planning {
    steps: [
        "Run discovery and record findings in Memory"
        "Remove session, add JWT package"
        "Rewrite auth endpoints"
        "Update middleware"
        "Fix tests"
        "Run full test suite"
    ]
    max_iterations: 8
}

Execution {
    tools: [
        read_file(path: string) -> string
        write_file(path: string, content: string) -> bool
        run_command(cmd: string) -> string
        list_dir(path: string) -> string
    ]
    strategy: "Follow discovery findings. Update Memory after each step.
               Run npm test after every file change."
}

Verification {
    checks: [
        "npm test exits with code 0"
        "POST /login returns a token field in the response body"
        "GET /api/profile returns 401 without Authorization header"
    ]
    on_fail: retry
    max_retries: 5
}
```

## Requirements

- VS Code 1.60 or later
- `loop` CLI for running files (optional, but needed for `loop check`, `loop run`, etc.)

## Source

[github.com/squareexp/loop](https://github.com/squareexp/loop)
