use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

mod parser;
mod storage;
mod runtime;
mod cli;

use crate::parser::parser::parse_loop_file;
use crate::parser::checker::check_ast;
use crate::runtime::engine::Engine;
use crate::runtime::sandbox::ToolSandbox;
use crate::cli::provider::{GeminiProvider, ClaudeProvider, OllamaProvider, Provider};
use crate::cli::tui::TuiDashboard;
use crate::storage::{get_latest_snapshot_hash, get_snapshot};

#[derive(Parser)]
#[command(name = "loop")]
#[command(about = "State Ledger & Sandbox Isolated AI Agent CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile and run a .loop DSL program
    Run {
        /// Path to the .loop file
        file: PathBuf,

        /// Provider name (gemini, claude, ollama)
        #[arg(long, default_value = "gemini")]
        provider: String,

        /// Session ID to track state persistence
        #[arg(long, default_value = "default-session")]
        session_id: String,
    },

    /// Switch the active LLM provider for a session, restoring state snapshot
    Switch {
        /// Session ID to restore state from
        #[arg(long, default_value = "default-session")]
        session_id: String,

        /// New provider to switch to (gemini, claude, ollama)
        #[arg(long)]
        provider: String,

        /// Path to the original .loop file to reload AST constraints
        file: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file, provider, session_id } => {
            if let Err(e) = run_loop(file, &provider, &session_id).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Switch { session_id, provider, file } => {
            if let Err(e) = switch_provider(&session_id, &provider, file).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

async fn run_loop(file_path: PathBuf, provider_name: &str, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read .loop file: {}", e))?;

    // 1. Compile: Parse and static compile checking
    let ast = parse_loop_file(&content)?;
    check_ast(&ast)?;

    // 2. Initialize Sandbox
    let sandbox_dir = std::env::current_dir()?.join(".loop_sandbox");
    let sandbox = ToolSandbox::new(sandbox_dir);

    // 3. Initialize Engine
    let mut engine = Engine::new(ast, session_id.to_string(), sandbox);

    // 4. Initialize Provider
    let provider = match provider_name.to_lowercase().as_str() {
        "gemini" => Provider::Gemini(GeminiProvider::new()),
        "claude" => Provider::Claude(ClaudeProvider::new()),
        "ollama" => Provider::Ollama(OllamaProvider::new("mistral".to_string())),
        _ => return Err(format!("Unsupported provider: {}", provider_name).into()),
    };

    // 5. Initialize TUI and Run
    let mut tui = TuiDashboard::new()?;

    // Save initial snapshot
    storage::save_snapshot(session_id, &engine.state)?;

    loop {
        tui.draw(&engine, provider_name)?;

        match engine.step(&provider).await {
            Ok(true) => {
                break;
            }
            Ok(false) => {
                // Next step
            }
            Err(e) => {
                return Err(e.into());
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    println!("Until condition met! Agent finished execution.");
    Ok(())
}

async fn switch_provider(session_id: &str, provider_name: &str, file_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read .loop file: {}", e))?;

    let ast = parse_loop_file(&content)?;
    check_ast(&ast)?;

    // Retrieve latest snapshot from Sled
    let latest_hash = get_latest_snapshot_hash(session_id)?
        .ok_or_else(|| format!("No active session found for session ID: {}", session_id))?;
    let snapshot = get_snapshot(&latest_hash)?;

    // Initialize Engine with reloaded state variables
    let sandbox_dir = std::env::current_dir()?.join(".loop_sandbox");
    let sandbox = ToolSandbox::new(sandbox_dir);

    let mut engine = Engine::new(ast, session_id.to_string(), sandbox);
    engine.state = snapshot.variables;

    let provider = match provider_name.to_lowercase().as_str() {
        "gemini" => Provider::Gemini(GeminiProvider::new()),
        "claude" => Provider::Claude(ClaudeProvider::new()),
        "ollama" => Provider::Ollama(OllamaProvider::new("mistral".to_string())),
        _ => return Err(format!("Unsupported provider: {}", provider_name).into()),
    };

    let mut tui = TuiDashboard::new()?;
    loop {
        tui.draw(&engine, provider_name)?;

        match engine.step(&provider).await {
            Ok(true) => {
                break;
            }
            Ok(false) => {}
            Err(e) => {
                return Err(e.into());
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    println!("Until condition met! Agent finished execution.");
    Ok(())
}

