/// init.rs — creates the workspace structure for a new Loop project.
///
/// Running `loop init [dir]` creates:
///   .loop/
///     skills/
///       loop.md               ← complete agent instruction document
///       loop-strategies.md    ← pattern library for loop engineering
///     state.json              ← empty execution state
///   Memory/
///     memory.json             ← empty memory
///   Goal.loop                 ← template to fill in
use std::fs;
use std::path::{Path, PathBuf};

pub struct InitResult {
    pub created: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
}

pub fn init(workspace: &Path) -> Result<InitResult, String> {
    let mut result = InitResult { created: vec![], skipped: vec![] };

    let write = |path: &PathBuf, content: &str, res: &mut InitResult| -> Result<(), String> {
        if path.exists() {
            res.skipped.push(path.clone());
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create {}: {}", parent.display(), e))?;
        }
        fs::write(path, content)
            .map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
        res.created.push(path.clone());
        Ok(())
    };

    let mkdir = |path: &PathBuf, res: &mut InitResult| -> Result<(), String> {
        if path.exists() {
            res.skipped.push(path.clone());
            return Ok(());
        }
        fs::create_dir_all(path)
            .map_err(|e| format!("Cannot create dir {}: {}", path.display(), e))?;
        res.created.push(path.clone());
        Ok(())
    };

    // .loop/skills/
    mkdir(&workspace.join(".loop").join("skills"), &mut result)?;

    // .loop/skills/loop.md
    write(
        &workspace.join(".loop").join("skills").join("loop.md"),
        SKILL_LOOP_MD,
        &mut result,
    )?;

    // .loop/skills/loop-strategies.md
    write(
        &workspace.join(".loop").join("skills").join("loop-strategies.md"),
        SKILL_STRATEGIES_MD,
        &mut result,
    )?;

    // .loop/state.json
    write(
        &workspace.join(".loop").join("state.json"),
        "{\n  \"status\": \"pending\",\n  \"iteration\": 0,\n  \"failed_tools\": [],\n  \"failed_checks\": [],\n  \"tool_history\": []\n}\n",
        &mut result,
    )?;

    // Memory/
    mkdir(&workspace.join("Memory"), &mut result)?;
    write(
        &workspace.join("Memory").join("memory.json"),
        "{}\n",
        &mut result,
    )?;

    // Goal.loop
    write(
        &workspace.join("Goal.loop"),
        GOAL_LOOP_TEMPLATE,
        &mut result,
    )?;

    Ok(result)
}

pub fn print_init_result(result: &InitResult) {
    for path in &result.created {
        println!("  created  {}", path.display());
    }
    for path in &result.skipped {
        println!("  skipped  {} (already exists)", path.display());
    }
}

// ─── Skill: loop.md ───────────────────────────────────────────────────────────

const SKILL_LOOP_MD: &str = r#"# Loop Engineering Language — Agent Reference

## What this is

A `.loop` file tells you exactly what to build, how to build it, and how to
know you're done. Every time you work on a `.loop` file, read this document
first. It defines the contract between the human who wrote the file and you.

## The seven blocks

### Goal []

The human's intent in plain language. No code. No structure.

```
Goal [
    Build a REST API for task management with JWT auth and Postgres.
]
```

`Goal` uses square brackets `[ ]`. Using `{ }` or `( )` is a syntax error.

Before you touch anything, read Goal and write it out in your own words.
If you can't restate it clearly, ask for clarification before starting.

---

### Memory {}

Persistent facts about the project. Memory lives in `Memory/memory.json`.
It starts empty. You fill it as you learn things.

```
Memory {
    project_path: "./api"
    tech_stack: ["Rust", "Axum", "Postgres"]
    completed: false
}
```

Rules for Memory:
- Read it at the start of every iteration.
- Write to it after Discovery (what you found).
- Write to it after Verification passes (what changed, what worked).
- Never delete entries — add new ones or update existing values.
- If `completed` is `true`, the loop is done. Don't start another iteration.

---

### Task {}

The specific implementation items. More granular than Planning steps.

```
Task {
    "Create users table migration"
    "Implement POST /auth/register"
    "Implement POST /auth/login with JWT"
    "Add auth middleware for protected routes"
}
```

`Task` uses curly braces `{ }`. Using `[ ]` or `( )` is a syntax error.

Work through tasks in order. After completing each one, run verification.
If verification fails, record what failed and move to the next task.
Do not skip tasks unless a dependency makes them impossible.

---

### Discovery {}

Before writing any code, answer the questions in `find:` and scan the
patterns in `scan:`. Write every finding to `Memory/memory.json`.

```
Discovery {
    scan: ["src/**/*.rs", "Cargo.toml", "migrations/**/*.sql"]
    find: [
        "Does an auth module already exist?"
        "What database connection setup is already in place?"
        "Are there any existing tests for the API?"
    ]
}
```

Discovery is not optional. Skipping it causes wasted iterations.
Do not start Planning until you have answers to every `find:` question.

---

### Planning {}

High-level steps to achieve the Goal. `max_iterations` is your budget.

```
Planning {
    steps: [
        "Run discovery and record findings in Memory"
        "Create database schema for users"
        "Implement auth endpoints"
        "Write integration tests"
        "Run verification"
    ]
    max_iterations: 6
}
```

Each step maps to one or more tool calls. If a step needs more than five
tool calls to complete, it is too large — break it down further yourself.

When `iteration` in `.loop/state.json` reaches `max_iterations`, the loop
is marked exhausted and stops regardless of completion status.

---

### Execution {}

The tools you have. Only call tools declared here.

```
Execution {
    tools: [
        read_file(path: string) -> string
        write_file(path: string, content: string) -> bool
        run_command(cmd: string) -> string
        list_dir(path: string) -> string
    ]
    strategy: "Execute planning steps in order. After each write, run
               a quick sanity check. Track what you did in Memory."
}
```

Tool signatures use parentheses `( )`. Parameters use `name: type` format.
Types are `string`, `int`, or `bool`. The return type follows `->`.

Read `strategy:` carefully. It tells you how to use the tools, not just
which tools exist.

---

### Verification {}

The success criteria. Run these after every major step.

```
Verification {
    checks: [
        "cargo test passes with exit code 0"
        "POST /auth/register returns 201 with a user object"
        "POST /auth/login returns 200 with a JWT token"
        "GET /api/tasks returns 401 without Authorization header"
    ]
    on_fail: retry
    max_retries: 4
}
```

After running checks, run `loop verify` to update `.loop/state.json`.

`on_fail: retry` means fix the failure and try again.
`on_fail: complete` means accept the current state and stop.

When all checks pass, `loop verify` will:
1. Update `.loop/state.json` status to `"complete"`
2. Update `Memory/memory.json` with completion info
3. Update `Goal.loop` with a completion marker

---

## How to work a .loop file

1. Read `Goal []` — understand what done looks like.
2. Read `.loop/state.json` — check iteration count and failed history.
3. Read `Memory {}` and `Memory/memory.json` — load prior knowledge.
4. Run `Discovery {}` — scan files, answer find questions, write to Memory.
5. Read `Task {}` — know the specific items to build.
6. Execute `Planning {}` steps using `Execution {}` tools.
7. After each meaningful change, run `loop verify`.
8. If verification fails: read `failed_checks` in state.json, fix the root
   cause, and retry. Do not repeat the same approach that already failed.
9. When all checks pass: `loop verify` closes the loop.

---

## Failure tracking

`.loop/state.json` records everything that went wrong:

```json
{
  "failed_tools": [
    { "tool": "write_file", "error": "permission denied: src/config.rs" }
  ],
  "failed_checks": [
    "POST /auth/login returns 200 with a JWT token"
  ],
  "tool_history": [
    "[iter 1] read_file(src/main.rs) → fn main() { ..."
  ]
}
```

Read this before every iteration. Never call a tool with the same arguments
that already failed. Never retry a check the same way it failed before.

If `failed_tools` has an entry for `write_file` with a specific path, either
fix the permission issue first or choose a different approach.

---

## Updating Memory

Memory is the source of truth across sessions. Update it at these moments:

**After Discovery:**
```json
{
  "auth_module_exists": false,
  "db_pool_setup": "sqlx::PgPool in src/db.rs",
  "existing_tests": 0
}
```

**After a planning step completes:**
```json
{
  "schema_created": true,
  "schema_path": "migrations/001_create_users.sql"
}
```

**After Verification passes:**
```json
{
  "completed": true,
  "completed_at": "2025-01-15",
  "passing_checks": 4,
  "iterations_used": 3
}
```

---

## Running CLI commands

```sh
loop check Goal.loop          # validate the file before running
loop init                     # create workspace structure (this)
loop scaffold Goal.loop       # generate folder structure from .loop file
loop run Goal.loop            # run with an AI provider
loop verify Goal.loop         # check verification conditions, update state
loop status                   # show current .loop/state.json
loop inspect Goal.loop        # print the parsed AST
```
"#;

// ─── Skill: loop-strategies.md ────────────────────────────────────────────────

const SKILL_STRATEGIES_MD: &str = r#"# Loop Engineering Strategies

## The four loops

Every `.loop` execution moves through four phases. These aren't named blocks
in the file — they're the natural rhythm of the agent working:

```
MEMORY (load)
    ↓
DISCOVERY (find)
    ↓
PLANNING + EXECUTION (build)
    ↓
VERIFICATION (check)
    ↓ fails
ITERATION (fix and retry)
    ↓ passes
COMPLETE
```

---

## Discovery-first

Run Discovery before touching any files. Scan everything in `scan:`, answer
every question in `find:`, write findings to Memory.

Bad pattern: start writing code, hit an error, realize the setup was different
than expected, waste two iterations.

Good pattern: spend one iteration on Discovery, confirm assumptions, then
build with full context.

---

## Read before write

Always read a file before writing to it. A write without a prior read is
likely to overwrite something important.

```
read_file("src/auth/mod.rs")   # understand the current state
write_file("src/auth/mod.rs", updated_content)
```

---

## Small writes, fast verify

Don't make five file changes then verify once at the end.
Make one change, verify it works, make the next change.

This keeps `failed_checks` short and makes the root cause obvious.

---

## Never repeat a failed approach

Before each tool call, check `failed_tools` in `.loop/state.json`.
If you were going to call `write_file("src/main.rs", ...)` and that same
call is already in `failed_tools`, choose a different path.

Same for verification: if a check failed because of a specific error,
fix that specific error before re-running the check.

---

## Update Memory after each step

Don't wait until the end to update Memory. After each completed planning
step, add a short entry:

```json
{
  "step_1_done": true,
  "users_table": "created at migrations/001_users.sql"
}
```

If the session is interrupted, the next iteration starts with accurate context.

---

## Fail fast, fail clearly

If a tool call fails, stop and record the error in Memory before trying
anything else. A clear error record is more valuable than a rushed retry.

---

## Max iterations is a budget

`max_iterations` is not a suggestion. When the loop hits that number, it
stops. Spend early iterations on Discovery and setup. Don't burn your budget
on repeated failed attempts at the same approach.

If you're on iteration 4 of 5 and still failing, change your approach
entirely rather than retrying the same thing.

---

## Task items are checkboxes

Work through `Task {}` items in order. After each one passes verification,
mark it in Memory:

```json
{
  "task_create_users_table": "done",
  "task_post_auth_register": "in_progress"
}
```

Don't jump ahead. Dependencies matter.

---

## When to use `on_fail: complete`

Use `on_fail: complete` when:
- The task is best-effort (partial completion is acceptable)
- External dependencies may be unavailable (third-party APIs, services)
- You're doing research or exploration with no strict success condition

Use `on_fail: retry` (default) when:
- The checks must all pass for the work to be useful
- You have a clear max_retries budget and want automatic retry
"#;

// ─── Goal.loop template ───────────────────────────────────────────────────────

const GOAL_LOOP_TEMPLATE: &str = r#"// Goal.loop — replace the placeholders and run: loop check Goal.loop

Goal [
    Describe your goal here in plain language. No code, no structure.
    Write what done looks like from the user's perspective.
]

// Optional: what the agent already knows about the project
Memory {
    project_path: "."
    notes: []
}

// Optional: specific tasks to implement
// Task {
//     "First concrete task"
//     "Second concrete task"
// }

Discovery {
    scan: ["./**/*"]
    find: [
        "What already exists in this project?"
        "What needs to be built or changed?"
    ]
}

Planning {
    steps: [
        "Run discovery and record findings"
        "Implement the first task"
        "Verify and complete"
    ]
    max_iterations: 5
}

Execution {
    tools: [
        read_file(path: string) -> string
        write_file(path: string, content: string) -> bool
        run_command(cmd: string) -> string
        list_dir(path: string) -> string
    ]
    strategy: "Execute steps in order. Read before writing. Verify after each step."
}

Verification {
    checks: [
        "The goal described above is met"
    ]
    on_fail: retry
    max_retries: 3
}
"#;
