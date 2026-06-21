use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

mod parser;
mod storage;
mod runtime;
mod cli;
mod scaffold;

use crate::parser::parser::parse_loop_file;
use crate::parser::checker::{check_loop_file, format_check_output, audit};
use crate::runtime::engine::Engine;
use crate::runtime::sandbox::ToolSandbox;
use crate::cli::provider::{GeminiProvider, ClaudeProvider, OllamaProvider, Provider};
use crate::cli::tui::TuiDashboard;
use crate::scaffold::{scaffold as do_scaffold, print_scaffold_result, LoopState};

#[derive(Parser)]
#[command(name = "loop")]
#[command(about = "Loop Engineering Language — AI agent task runner")]
#[command(long_about = "
Loop is a structured language for AI agent execution.
Write a .loop file describing your Goal, Discovery, Planning, Execution,
and Verification — then let the agent do the work.
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and validate a .loop file — shows all errors clearly
    Check {
        /// Path to the .loop file
        file: PathBuf,
    },

    /// Generate the workspace folder structure from a .loop file
    ///
    /// Creates: Goal.md, Memory/memory.json, skills/, .loop/state.json
    Scaffold {
        /// Path to the .loop file
        file: PathBuf,

        /// Target directory (defaults to current directory)
        #[arg(long, default_value = ".")]
        dir: PathBuf,
    },

    /// Show the current loop execution state (.loop/state.json)
    Status {
        /// Workspace directory to read state from
        #[arg(default_value = ".")]
        dir: PathBuf,
    },

    /// Pretty-print the parsed AST of a .loop file
    Inspect {
        /// Path to the .loop file
        file: PathBuf,
    },

    /// Run a .loop file with an AI agent
    Run {
        /// Path to the .loop file
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
        Commands::Check { file } => cmd_check(file),
        Commands::Scaffold { file, dir } => cmd_scaffold(file, dir),
        Commands::Status { dir } => cmd_status(dir),
        Commands::Inspect { file } => cmd_inspect(file),
        Commands::Run { file, provider, session } => {
            if let Err(e) = cmd_run(file, &provider, &session).await {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

// ─── check ───────────────────────────────────────────────────────────────────

fn cmd_check(file: PathBuf) {
    let content = match fs::read_to_string(&file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", file.display(), e);
            std::process::exit(1);
        }
    };

    println!("\nChecking: {}\n", file.display());

    match parse_loop_file(&content) {
        Err(e) => {
            eprintln!("  ✗ Syntax error: {}", e);
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
    let content = match fs::read_to_string(&file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", file.display(), e);
            std::process::exit(1);
        }
    };

    let lf = match parse_loop_file(&content) {
        Ok(lf) => lf,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = check_loop_file(&lf) {
        eprintln!("error: loop file has errors — run 'loop check' first\n{}", e);
        std::process::exit(1);
    }

    let loop_file_name = file.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file.to_string_lossy().to_string());

    println!("\nScaffolding workspace from: {}\n", file.display());

    match do_scaffold(&lf, &loop_file_name, &dir) {
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
            println!("No loop state found in {}/.loop/state.json", dir.display());
            println!("Run 'loop scaffold <file.loop>' first.");
        }
    }
}

// ─── inspect ─────────────────────────────────────────────────────────────────

fn cmd_inspect(file: PathBuf) {
    let content = match fs::read_to_string(&file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: cannot read {}: {}", file.display(), e);
            std::process::exit(1);
        }
    };

    match parse_loop_file(&content) {
        Err(e) => {
            eprintln!("Syntax error: {}", e);
            std::process::exit(1);
        }
        Ok(lf) => {
            println!("\n.loop AST — {}\n", file.display());
            println!("Goal:\n  {}\n", lf.goal.text.trim());
            if let Some(mem) = &lf.memory {
                println!("Memory ({} fields):", mem.fields.len());
                for (k, v) in &mem.fields {
                    println!("  {}: {:?}", k, v);
                }
                println!();
            }
            println!("Discovery:");
            println!("  scan:  {:?}", lf.discovery.scan);
            println!("  find:  {:?}", lf.discovery.find);
            println!();
            println!("Planning ({} steps, max {} iterations):", lf.planning.steps.len(), lf.planning.max_iterations);
            for (i, step) in lf.planning.steps.iter().enumerate() {
                println!("  {}. {}", i + 1, step);
            }
            println!();
            println!("Execution ({} tools):", lf.execution.tools.len());
            for tool in &lf.execution.tools {
                let params: Vec<String> = tool.params.iter()
                    .map(|p| format!("{}: {:?}", p.name, p.param_type))
                    .collect();
                let ret = tool.return_type.as_ref()
                    .map(|r| format!(" -> {:?}", r))
                    .unwrap_or_default();
                println!("  {}({}){}", tool.name, params.join(", "), ret);
            }
            println!("  strategy: {}", lf.execution.strategy);
            println!();
            println!("Verification ({} checks, on_fail: {:?}, max_retries: {}):",
                lf.verification.checks.len(),
                lf.verification.on_fail,
                lf.verification.max_retries);
            for check in &lf.verification.checks {
                println!("  ○ {}", check);
            }
        }
    }
}

// ─── run ─────────────────────────────────────────────────────────────────────

async fn cmd_run(file: PathBuf, provider_name: &str, session: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(&file)
        .map_err(|e| format!("Cannot read {}: {}", file.display(), e))?;

    let lf = parse_loop_file(&content)
        .map_err(|e| format!("Syntax error: {}", e))?;

    check_loop_file(&lf)
        .map_err(|e| format!("Validation error:\n{}", e))?;

    let sandbox_dir = std::env::current_dir()?.join(".loop_sandbox");
    let sandbox = ToolSandbox::new(sandbox_dir);
    let mut engine = Engine::new(lf, session.to_string(), sandbox);

    let provider = match provider_name.to_lowercase().as_str() {
        "gemini" => Provider::Gemini(GeminiProvider::new()),
        "claude" => Provider::Claude(ClaudeProvider::new()),
        "ollama" => Provider::Ollama(OllamaProvider::new("mistral".to_string())),
        _ => return Err(format!("Unknown provider '{}'. Use: gemini, claude, ollama", provider_name).into()),
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
            Err(e) => {
                return Err(e.into());
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    Ok(())
}
