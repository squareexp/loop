use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod parser;
mod storage;
mod runtime;
mod cli;
mod scaffold;
mod init;

use crate::parser::parser::parse_loop_file;
use crate::parser::checker::{check_loop_file, format_check_output, audit};
use crate::parser::ast::MemoryValue;
use crate::runtime::engine::Engine;
use crate::runtime::sandbox::ToolSandbox;
use crate::cli::provider::{GeminiProvider, ClaudeProvider, OllamaProvider, Provider};
use crate::cli::tui::TuiDashboard;
use crate::scaffold::{scaffold as do_scaffold, print_scaffold_result, LoopState, LoopStatus};
use crate::init::{init as do_init, print_init_result};

#[derive(Parser)]
#[command(name = "loop")]
#[command(about = "Loop Engineering Language — structured AI agent task runner")]
#[command(long_about = "
Loop lets you write a .loop file that tells an AI agent exactly what to build,
how to build it, and how to confirm it worked. The agent follows the file like
a contract, tracks every failure, and only stops when verification passes.

Blocks:
  Goal []          what you want (natural language, uses [ ])
  Memory {}        what the agent remembers across sessions (uses { })
  Task {}          specific implementation items (uses { })
  Discovery {}     what to find before coding (uses { })
  Planning {}      ordered steps + iteration budget (uses { })
  Execution {}     available tools + strategy (uses { })
  Verification {}  success criteria (uses { })

Quick start:
  loop init            create workspace (.loop/skills, Memory/, Goal.loop)
  loop check file.loop validate the file
  loop run file.loop   run with an AI agent
  loop verify file.loop check if verification passes
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create workspace: .loop/skills/, Memory/, and Goal.loop template
    Init {
        /// Target directory (defaults to current directory)
        #[arg(default_value = ".")]
        dir: PathBuf,
    },

    /// Parse and validate a .loop file — shows per-block status and all errors
    Check {
        file: PathBuf,
    },

    /// Generate workspace folder structure from a .loop file
    ///
    /// Creates: Goal.md, Memory/memory.json, skills/, .loop/state.json
    Scaffold {
        file: PathBuf,

        /// Target directory
        #[arg(long, default_value = ".")]
        dir: PathBuf,
    },

    /// Run each Verification check and update .loop/state.json
    ///
    /// Shell-like checks (starting with cargo, npm, python, curl, etc.)
    /// are executed automatically. Others are listed for manual review.
    /// When all checks pass, Memory and Goal.loop are updated.
    Verify {
        file: PathBuf,

        /// Workspace directory containing .loop/state.json
        #[arg(long, default_value = ".")]
        dir: PathBuf,
    },

    /// Show the current loop execution state (.loop/state.json)
    Status {
        #[arg(default_value = ".")]
        dir: PathBuf,
    },

    /// Pretty-print the parsed AST of a .loop file
    Inspect {
        file: PathBuf,
    },

    /// Run a .loop file with an AI agent
    Run {
        file: PathBuf,

        /// LLM provider: gemini, claude, ollama
        #[arg(long, default_value = "claude")]
        provider: String,

        /// Session identifier
        #[arg(long, default_value = "default")]
        session: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { dir }                    => cmd_init(dir),
        Commands::Check { file }                  => cmd_check(file),
        Commands::Scaffold { file, dir }          => cmd_scaffold(file, dir),
        Commands::Verify { file, dir }            => cmd_verify(file, dir),
        Commands::Status { dir }                  => cmd_status(dir),
        Commands::Inspect { file }                => cmd_inspect(file),
        Commands::Run { file, provider, session } => {
            if let Err(e) = cmd_run(file, &provider, &session).await {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

// ─── init ────────────────────────────────────────────────────────────────────

fn cmd_init(dir: PathBuf) {
    println!("\nInitializing Loop workspace in: {}\n", dir.display());

    match do_init(&dir) {
        Ok(result) => {
            print_init_result(&result);
            println!("\nWorkspace ready.");
            println!("  1. Edit Goal.loop with your goal");
            println!("  2. Run: loop check Goal.loop");
            println!("  3. Run: loop run Goal.loop --provider claude");
            println!("\nAgent skills are in .loop/skills/loop.md");
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

// ─── check ───────────────────────────────────────────────────────────────────

fn cmd_check(file: PathBuf) {
    let content = read_file_or_exit(&file);
    println!("\nChecking: {}\n", file.display());

    match parse_loop_file(&content) {
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
        Ok(lf) => {
            print!("{}", format_check_output(&lf));
            let report = audit(&lf);
            if !report.is_ok() {
                std::process::exit(1);
            }
        }
    }
}

// ─── scaffold ────────────────────────────────────────────────────────────────

fn cmd_scaffold(file: PathBuf, dir: PathBuf) {
    let content = read_file_or_exit(&file);
    let lf = parse_or_exit(&content);

    if let Err(e) = check_loop_file(&lf) {
        eprintln!("error: .loop file has validation errors — run 'loop check' first\n{}", e);
        std::process::exit(1);
    }

    let name = file.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file.to_string_lossy().to_string());

    println!("\nScaffolding workspace from: {}\n", file.display());

    match do_scaffold(&lf, &name, &dir) {
        Ok(result) => {
            print_scaffold_result(&result);
            println!("\nWorkspace ready. Run 'loop run {}' to execute.", file.display());
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

// ─── verify ──────────────────────────────────────────────────────────────────

fn cmd_verify(file: PathBuf, dir: PathBuf) {
    let content = read_file_or_exit(&file);
    let lf = parse_or_exit(&content);

    println!("\nVerifying: {}\n", file.display());

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut manual = 0usize;
    let mut failed_checks: Vec<String> = vec![];

    for check in &lf.verification.checks {
        let runnable = is_runnable_check(check);
        if runnable {
            print!("  running  {}", check);
            match run_check(check) {
                Ok(true) => {
                    println!("  ✓");
                    passed += 1;
                }
                Ok(false) => {
                    println!("  ✗");
                    failed += 1;
                    failed_checks.push(check.clone());
                }
                Err(e) => {
                    println!("  ✗ (error: {})", e);
                    failed += 1;
                    failed_checks.push(check.clone());
                }
            }
        } else {
            println!("  manual   ○ {}", check);
            manual += 1;
        }
    }

    // Update .loop/state.json
    let state_path = dir.join(".loop").join("state.json");
    if let Some(mut state) = LoopState::load(&dir) {
        state.failed_checks = failed_checks.clone();
        if failed == 0 && manual == 0 {
            state.mark_complete();
        }
        if let Err(e) = state.save(&dir) {
            eprintln!("warning: could not update state: {}", e);
        }
    }

    println!();
    if manual > 0 {
        println!("  {} automated, {} manual (confirm manually above)", passed + failed, manual);
    }

    if failed == 0 && manual == 0 {
        println!("  All {} checks passed — loop complete!", passed);
        update_memory_on_complete(&dir, &lf.verification.checks.len());
        mark_goal_complete(&file, lf.verification.checks.len());
    } else if failed > 0 {
        println!("  {} passed, {} failed", passed, failed);
        for c in &failed_checks {
            println!("    ✗ {}", c);
        }
        std::process::exit(1);
    } else {
        println!("  {} automated passed, {} need manual confirmation", passed, manual);
    }
}

/// Returns true if the check looks like it can be run as a shell command.
fn is_runnable_check(check: &str) -> bool {
    let cmd = check.trim().split_whitespace().next().unwrap_or("");
    matches!(
        cmd,
        "cargo" | "npm" | "npx" | "yarn" | "pnpm" | "python" | "python3" |
        "pytest" | "go" | "make" | "curl" | "sh" | "bash" | "zsh" | "node" |
        "deno" | "bun" | "jest" | "vitest" | "gradle" | "mvn" | "dotnet" |
        "echo" | "ls" | "cat" | "grep" | "find" | "test" | "ruby" | "swift" |
        "php" | "java" | "docker" | "kubectl" | "terraform" | "ansible"
    )
}

fn run_check(check: &str) -> Result<bool, String> {
    let parts: Vec<&str> = check.trim().splitn(2, ' ').collect();
    let (cmd, args_str) = if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        (parts[0], "")
    };

    let mut c = Command::new(cmd);
    for arg in args_str.split_whitespace() {
        c.arg(arg);
    }

    let status = c.status().map_err(|e| e.to_string())?;
    Ok(status.success())
}

fn update_memory_on_complete(dir: &Path, checks_count: &usize) {
    let memory_path = dir.join("Memory").join("memory.json");
    let existing: serde_json::Value = fs::read_to_string(&memory_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    let mut map = match existing {
        serde_json::Value::Object(m) => m,
        _ => serde_json::Map::new(),
    };

    map.insert("completed".into(), serde_json::Value::Bool(true));
    map.insert(
        "completed_at".into(),
        serde_json::Value::String(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
        ),
    );
    map.insert("passing_checks".into(), serde_json::Value::Number((*checks_count).into()));

    if let Ok(json) = serde_json::to_string_pretty(&serde_json::Value::Object(map)) {
        let _ = fs::create_dir_all(dir.join("Memory"));
        let _ = fs::write(&memory_path, json);
        println!("  updated  Memory/memory.json (completed: true)");
    }
}

fn mark_goal_complete(file: &PathBuf, checks_passed: usize) {
    let Ok(content) = fs::read_to_string(file) else { return };
    if content.starts_with("// Completed") { return }

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let header = format!(
        "// Completed: {} ({} checks passed)\n",
        ts, checks_passed
    );
    let updated = header + &content;
    let _ = fs::write(file, updated);
    println!("  updated  {} (completion marker added)", file.display());
}

// ─── status ──────────────────────────────────────────────────────────────────

fn cmd_status(dir: PathBuf) {
    match LoopState::load(&dir) {
        Some(state) => {
            println!("\nLoop State — {}\n", state.loop_file);
            println!("  Status:      {:?}", state.status);
            println!("  Iteration:   {} / {}", state.iteration, state.max_iterations);
            println!("  Step:        {}", state.current_step + 1);
            if !state.failed_tools.is_empty() {
                println!("\n  Failed Tools:");
                for ft in &state.failed_tools {
                    println!("    ✗ {}: {}", ft.tool, ft.error);
                }
            }
            if !state.failed_checks.is_empty() {
                println!("\n  Failed Checks:");
                for c in &state.failed_checks {
                    println!("    ✗ {}", c);
                }
            }
            if !state.tool_history.is_empty() {
                println!("\n  Tool History (last 5):");
                for t in state.tool_history.iter().rev().take(5) {
                    println!("    {}", t);
                }
            }
        }
        None => {
            eprintln!("No .loop/state.json found in {}", dir.display());
            eprintln!("Run 'loop init' or 'loop scaffold <file.loop>' first.");
            std::process::exit(1);
        }
    }
}

// ─── inspect ─────────────────────────────────────────────────────────────────

fn cmd_inspect(file: PathBuf) {
    let content = read_file_or_exit(&file);
    let lf = parse_or_exit(&content);

    println!("\n.loop AST — {}\n", file.display());
    println!("Goal:\n  {}\n", lf.goal.text.trim());

    if let Some(mem) = &lf.memory {
        println!("Memory ({} fields):", mem.fields.len());
        for (k, v) in &mem.fields {
            println!("  {}: {:?}", k, v);
        }
        println!();
    }

    if let Some(task) = &lf.task {
        println!("Task ({} items):", task.items.len());
        for (i, item) in task.items.iter().enumerate() {
            println!("  {}. {}", i + 1, item);
        }
        println!();
    }

    println!("Discovery:");
    println!("  scan: {:?}", lf.discovery.scan);
    println!("  find: {:?}", lf.discovery.find);
    println!();

    println!(
        "Planning ({} steps, max {} iterations):",
        lf.planning.steps.len(),
        lf.planning.max_iterations
    );
    for (i, step) in lf.planning.steps.iter().enumerate() {
        println!("  {}. {}", i + 1, step);
    }
    println!();

    println!("Execution ({} tools):", lf.execution.tools.len());
    for tool in &lf.execution.tools {
        let params: Vec<String> = tool
            .params
            .iter()
            .map(|p| format!("{}: {:?}", p.name, p.param_type))
            .collect();
        let ret = tool
            .return_type
            .as_ref()
            .map(|r| format!(" -> {:?}", r))
            .unwrap_or_default();
        println!("  {}({}){}", tool.name, params.join(", "), ret);
    }
    println!("  strategy: {}", lf.execution.strategy);
    println!();

    println!(
        "Verification ({} checks, on_fail: {:?}, max_retries: {}):",
        lf.verification.checks.len(),
        lf.verification.on_fail,
        lf.verification.max_retries
    );
    for check in &lf.verification.checks {
        println!("  ○ {}", check);
    }
}

// ─── run ─────────────────────────────────────────────────────────────────────

async fn cmd_run(
    file: PathBuf,
    provider_name: &str,
    session: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(&file)
        .map_err(|e| format!("Cannot read {}: {}", file.display(), e))?;

    let lf = parse_loop_file(&content).map_err(|e| format!("{}", e))?;
    check_loop_file(&lf).map_err(|e| format!("Validation error:\n{}", e))?;

    let sandbox_dir = std::env::current_dir()?.join(".loop_sandbox");
    let sandbox = ToolSandbox::new(sandbox_dir);
    let mut engine = Engine::new(lf, session.to_string(), sandbox);

    let provider = match provider_name.to_lowercase().as_str() {
        "gemini" => Provider::Gemini(GeminiProvider::new()),
        "claude" => Provider::Claude(ClaudeProvider::new()),
        "ollama" => Provider::Ollama(OllamaProvider::new("mistral".to_string())),
        _ => {
            return Err(
                format!("Unknown provider '{}'. Use: gemini, claude, ollama", provider_name).into(),
            )
        }
    };

    let mut tui = TuiDashboard::new()?;

    loop {
        tui.draw(&engine, provider_name)?;

        match engine.step(&provider).await {
            Ok(true) => {
                println!("\nVerification passed — loop complete.");
                break;
            }
            Ok(false) => {}
            Err(e) => return Err(e.into()),
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn read_file_or_exit(file: &PathBuf) -> String {
    match fs::read_to_string(file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", file.display(), e);
            std::process::exit(1);
        }
    }
}

fn parse_or_exit(content: &str) -> crate::parser::ast::LoopFile {
    match parse_loop_file(content) {
        Ok(lf) => lf,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
