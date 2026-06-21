/// scaffold.rs — generates the workspace folder structure from a .loop file.
///
/// Running `loop scaffold my.loop` creates:
///   Goal.md
///   Memory/memory.json
///   skills/
///   .loop/state.json
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use crate::parser::ast::{LoopFile, MemoryValue, OnFail};

// ─── Loop State (persisted in .loop/state.json) ───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedToolRecord {
    pub tool: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LoopStatus {
    Pending,
    Running,
    Complete,
    Exhausted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopState {
    pub loop_file: String,
    pub status: LoopStatus,
    pub iteration: u32,
    pub max_iterations: u32,
    pub current_step: usize,
    pub completed_steps: Vec<usize>,
    pub failed_tools: Vec<FailedToolRecord>,
    pub failed_checks: Vec<String>,
    pub tool_history: Vec<String>,
}

impl LoopState {
    pub fn new(loop_file: &str, max_iterations: u32) -> Self {
        Self {
            loop_file: loop_file.to_string(),
            status: LoopStatus::Pending,
            iteration: 0,
            max_iterations,
            current_step: 0,
            completed_steps: vec![],
            failed_tools: vec![],
            failed_checks: vec![],
            tool_history: vec![],
        }
    }

    pub fn load(dir: &Path) -> Option<Self> {
        let path = dir.join(".loop").join("state.json");
        let text = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn save(&self, dir: &Path) -> Result<(), String> {
        let path = dir.join(".loop").join("state.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize state: {}", e))?;
        fs::write(&path, json).map_err(|e| format!("Failed to write state: {}", e))
    }

    pub fn record_tool_success(&mut self, tool: &str, args_summary: &str, result_summary: &str) {
        self.tool_history.push(format!(
            "[iter {}] {}({}) → {}",
            self.iteration, tool, args_summary, result_summary
        ));
    }

    pub fn record_tool_failure(&mut self, tool: &str, error: &str) {
        self.failed_tools.push(FailedToolRecord {
            tool: tool.to_string(),
            error: error.to_string(),
        });
    }

    pub fn record_check_failure(&mut self, check: &str) {
        if !self.failed_checks.contains(&check.to_string()) {
            self.failed_checks.push(check.to_string());
        }
    }

    pub fn advance_iteration(&mut self) -> bool {
        self.iteration += 1;
        self.status = LoopStatus::Running;
        if self.iteration > self.max_iterations {
            self.status = LoopStatus::Exhausted;
            return false;
        }
        true
    }

    pub fn mark_complete(&mut self) {
        self.status = LoopStatus::Complete;
    }

    pub fn mark_exhausted(&mut self) {
        self.status = LoopStatus::Exhausted;
    }

    pub fn on_fail_action(&self, on_fail: &OnFail) -> bool {
        match on_fail {
            OnFail::Retry => self.iteration < self.max_iterations,
            OnFail::Complete => false,
        }
    }
}

// ─── Scaffold ─────────────────────────────────────────────────────────────────

pub struct ScaffoldResult {
    pub created: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
}

pub fn scaffold(loop_file: &LoopFile, loop_file_name: &str, workspace: &Path) -> Result<ScaffoldResult, String> {
    let mut result = ScaffoldResult { created: vec![], skipped: vec![] };

    let write = |path: &PathBuf, content: &str, res: &mut ScaffoldResult| -> Result<(), String> {
        if path.exists() {
            res.skipped.push(path.clone());
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create {}: {}", parent.display(), e))?;
        }
        fs::write(path, content).map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
        res.created.push(path.clone());
        Ok(())
    };

    let mkdir = |path: &PathBuf, res: &mut ScaffoldResult| -> Result<(), String> {
        if path.exists() {
            res.skipped.push(path.clone());
            return Ok(());
        }
        fs::create_dir_all(path).map_err(|e| format!("Cannot create dir {}: {}", path.display(), e))?;
        res.created.push(path.clone());
        Ok(())
    };

    // Goal.md
    let goal_md = format!(
        "# Goal\n\n{}\n",
        loop_file.goal.text.trim()
    );
    write(&workspace.join("Goal.md"), &goal_md, &mut result)?;

    // Memory/memory.json
    mkdir(&workspace.join("Memory"), &mut result)?;
    let memory_json = if let Some(mem) = &loop_file.memory {
        let mut map = serde_json::Map::new();
        for (k, v) in &mem.fields {
            let jv = match v {
                MemoryValue::String(s) => serde_json::Value::String(s.clone()),
                MemoryValue::Integer(i) => serde_json::Value::Number((*i).into()),
                MemoryValue::Boolean(b) => serde_json::Value::Bool(*b),
                MemoryValue::StringArray(arr) => {
                    serde_json::Value::Array(arr.iter().map(|s| serde_json::Value::String(s.clone())).collect())
                }
            };
            map.insert(k.clone(), jv);
        }
        serde_json::to_string_pretty(&serde_json::Value::Object(map))
            .unwrap_or_else(|_| "{}".to_string())
    } else {
        "{}".to_string()
    };
    write(&workspace.join("Memory").join("memory.json"), &memory_json, &mut result)?;

    // skills/
    mkdir(&workspace.join("skills"), &mut result)?;
    write(
        &workspace.join("skills").join(".gitkeep"),
        "",
        &mut result,
    )?;

    // .loop/ state
    mkdir(&workspace.join(".loop"), &mut result)?;
    let state = LoopState::new(loop_file_name, loop_file.planning.max_iterations);
    let state_json = serde_json::to_string_pretty(&state)
        .map_err(|e| format!("Cannot serialize state: {}", e))?;
    write(&workspace.join(".loop").join("state.json"), &state_json, &mut result)?;

    Ok(result)
}

pub fn print_scaffold_result(result: &ScaffoldResult) {
    for path in &result.created {
        println!("  created  {}", path.display());
    }
    for path in &result.skipped {
        println!("  skipped  {} (already exists)", path.display());
    }
}
