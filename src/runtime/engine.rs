use crate::parser::ast::{LoopFile, ToolDecl, Value};
use crate::runtime::sandbox::ToolSandbox;
use crate::scaffold::LoopState;

pub struct AgentResponse {
    pub reasoning: String,
    pub tool_call: Option<ToolCallRequest>,
}

pub struct ToolCallRequest {
    pub name: String,
    pub args: Vec<Value>,
}

pub trait AgentProvider: Send + Sync {
    async fn reason(&self, prompt: &str, tools: &[ToolDecl]) -> Result<AgentResponse, String>;
}

pub struct Engine {
    pub loop_file: LoopFile,
    pub session_id: String,
    pub loop_state: LoopState,
    pub sandbox: ToolSandbox,
    pub last_error: Option<String>,
    // legacy compat fields used by TUI
    pub iteration_count: u32,
    pub max_iterations: u32,
    pub budget_usd: f64,
    pub max_budget_usd: f64,
}

impl Engine {
    pub fn new(loop_file: LoopFile, session_id: String, sandbox: ToolSandbox) -> Self {
        let max = loop_file.planning.max_iterations;
        let loop_state = LoopState::new(&session_id, max);
        Self {
            loop_file,
            session_id,
            loop_state,
            sandbox,
            last_error: None,
            iteration_count: 0,
            max_iterations: max,
            budget_usd: 0.0,
            max_budget_usd: 10.0,
        }
    }

    pub fn build_prompt(&self) -> String {
        let lf = &self.loop_file;
        let ls = &self.loop_state;
        let mut out = String::new();

        // Goal
        out.push_str(&format!("# Goal\n{}\n\n", lf.goal.text.trim()));

        // Planning progress
        let steps = &lf.planning.steps;
        out.push_str(&format!(
            "# Planning — iteration {}/{} (step {}/{})\n",
            ls.iteration,
            ls.max_iterations,
            ls.current_step.saturating_add(1),
            steps.len()
        ));
        for (i, step) in steps.iter().enumerate() {
            let marker = if ls.completed_steps.contains(&i) {
                "✓"
            } else if i == ls.current_step {
                "→"
            } else {
                "○"
            };
            out.push_str(&format!("  {} {}. {}\n", marker, i + 1, step));
        }
        out.push('\n');

        // Strategy
        if !lf.execution.strategy.is_empty() {
            out.push_str(&format!("# Strategy\n{}\n\n", lf.execution.strategy));
        }

        // Discovery context
        if !lf.discovery.scan.is_empty() {
            out.push_str("# Files to Scan\n");
            for s in &lf.discovery.scan {
                out.push_str(&format!("  - {}\n", s));
            }
            out.push('\n');
        }
        if !lf.discovery.find.is_empty() {
            out.push_str("# Questions to Answer\n");
            for q in &lf.discovery.find {
                out.push_str(&format!("  - {}\n", q));
            }
            out.push('\n');
        }

        // Failed tools — tell the agent not to repeat them
        if !ls.failed_tools.is_empty() {
            out.push_str("# Failed Approaches — Do Not Repeat\n");
            for ft in &ls.failed_tools {
                out.push_str(&format!("  ✗ {} — {}\n", ft.tool, ft.error));
            }
            out.push('\n');
        }

        // Recent tool history (last 5)
        if !ls.tool_history.is_empty() {
            out.push_str("# Recent Tool Calls\n");
            let recent = ls.tool_history.iter().rev().take(5).collect::<Vec<_>>();
            for t in recent.iter().rev() {
                out.push_str(&format!("  {}\n", t));
            }
            out.push('\n');
        }

        // Last error
        if let Some(err) = &self.last_error {
            out.push_str(&format!("# Last Error\n{}\n\n", err));
        }

        // Available tools
        out.push_str("# Available Tools\n");
        for tool in &lf.execution.tools {
            out.push_str(&format!("  {}(", tool.name));
            let params: Vec<String> = tool
                .params
                .iter()
                .map(|p| format!("{}: {:?}", p.name, p.param_type))
                .collect();
            out.push_str(&params.join(", "));
            out.push(')');
            if let Some(ref ret) = tool.return_type {
                out.push_str(&format!(" -> {:?}", ret));
            }
            out.push('\n');
        }
        out.push('\n');

        // Verification checks
        out.push_str("# Verification Checks (all must pass to complete)\n");
        for check in &lf.verification.checks {
            let failed = ls.failed_checks.contains(check);
            let marker = if failed { "✗" } else { "○" };
            out.push_str(&format!("  {} {}\n", marker, check));
        }
        out.push('\n');

        out.push_str("Respond with the next tool call as JSON:\n");
        out.push_str(
            "{\n  \"reasoning\": \"why you are doing this\",\n  \"tool_call\": { \"name\": \"tool_name\", \"arguments\": [\"arg1\"] }\n}\n",
        );

        out
    }

    pub async fn step<P: AgentProvider>(&mut self, provider: &P) -> Result<bool, String> {
        if !self.loop_state.advance_iteration() {
            self.loop_state.mark_exhausted();
            return Err(format!(
                "Max iterations ({}) exceeded. Loop marked exhausted.",
                self.max_iterations
            ));
        }
        self.iteration_count = self.loop_state.iteration;

        let prompt = self.build_prompt();
        let response = provider
            .reason(&prompt, &self.loop_file.execution.tools)
            .await
            .map_err(|e| format!("LLM reasoning failed: {}", e))?;

        if let Some(tc) = response.tool_call {
            let tool_exists = self
                .loop_file
                .execution
                .tools
                .iter()
                .any(|t| t.name == tc.name);

            if !tool_exists {
                let msg = format!("Tool '{}' not declared in Execution.tools", tc.name);
                self.loop_state.record_tool_failure(&tc.name, &msg);
                self.last_error = Some(msg);
                return Ok(false);
            }

            let args_summary = tc
                .args
                .iter()
                .map(|a| match a {
                    Value::String(s) => format!("{:?}", &s[..s.len().min(40)]),
                    Value::Integer(i) => i.to_string(),
                    Value::Boolean(b) => b.to_string(),
                })
                .collect::<Vec<_>>()
                .join(", ");

            match self.sandbox.execute_tool(&tc.name, tc.args) {
                Ok(output) => {
                    let result_summary = match &output {
                        Value::String(s) => s[..s.len().min(80)].to_string(),
                        Value::Boolean(b) => b.to_string(),
                        Value::Integer(i) => i.to_string(),
                    };
                    self.loop_state
                        .record_tool_success(&tc.name, &args_summary, &result_summary);
                    self.last_error = None;
                }
                Err(err) => {
                    self.loop_state.record_tool_failure(&tc.name, &err);
                    self.last_error = Some(format!("Tool '{}' failed: {}", tc.name, err));
                    return Ok(false);
                }
            }
        } else {
            // AI returned no tool call — treat as a verification pass attempt
            // In a real integration the AI would signal "done" here
            self.last_error = Some("No tool call proposed — retrying".into());
        }

        // Check whether verification is satisfied (simple heuristic: no failed checks)
        // A full implementation would run each check as a command/LLM judge here.
        if self.loop_state.failed_checks.is_empty() && self.loop_state.iteration > 1 {
            self.loop_state.mark_complete();
            return Ok(true);
        }

        Ok(false)
    }
}

