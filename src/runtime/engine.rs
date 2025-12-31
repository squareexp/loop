use crate::parser::ast::{LoopAst, Value, ToolDecl};
use crate::runtime::sandbox::ToolSandbox;
use crate::runtime::interceptor::check_and_rollback_if_needed;
use crate::runtime::interpreter::eval_expr;
use crate::storage::save_snapshot;
use std::collections::HashMap;

#[allow(dead_code)]
pub struct AgentResponse {
    pub reasoning: String,
    pub tool_call: Option<ToolCallRequest>,
}

pub struct ToolCallRequest {
    pub name: String,
    pub args: Vec<Value>,
}

pub trait AgentProvider: Send + Sync {
    async fn reason(
        &self,
        prompt: &str,
        tools: &[ToolDecl],
    ) -> Result<AgentResponse, String>;
}

pub struct Engine {
    pub ast: LoopAst,
    pub session_id: String,
    pub state: HashMap<String, Value>,
    pub sandbox: ToolSandbox,
    pub max_iterations: usize,
    pub iteration_count: usize,
    pub budget_usd: f64,
    pub max_budget_usd: f64,
    pub last_error: Option<String>,
}

impl Engine {
    pub fn new(ast: LoopAst, session_id: String, sandbox: ToolSandbox) -> Self {
        let state = ast.state.clone();
        Self {
            ast,
            session_id,
            state,
            sandbox,
            max_iterations: 20,
            iteration_count: 0,
            budget_usd: 0.0,
            max_budget_usd: 5.0,
            last_error: None,
        }
    }

    pub fn build_prompt(&self) -> String {
        let mut prompt = String::new();
        prompt.push_str(&format!("Task:\n{}\n\n", self.ast.task));
        prompt.push_str(&format!("Strategy:\n{}\n\n", self.ast.strategy));

        prompt.push_str("Current State variables:\n");
        for (k, v) in &self.state {
            match v {
                Value::String(s) => {
                    if s.starts_with("file://") || s.starts_with("db://") {
                        prompt.push_str(&format!("  {}: {} (lazy reference)\n", k, s));
                    } else if s.len() > 200 {
                        prompt.push_str(&format!("  {}: {}... (truncated)\n", k, &s[..200]));
                    } else {
                        prompt.push_str(&format!("  {}: {:?}\n", k, s));
                    }
                }
                _ => {
                    prompt.push_str(&format!("  {}: {:?}\n", k, v));
                }
            }
        }
        prompt.push_str("\n");

        if let Some(err) = &self.last_error {
            prompt.push_str(&format!("Immediate Previous Error:\n{}\n\n", err));
        }

        prompt.push_str("Available Tools:\n");
        for tool in &self.ast.tools {
            prompt.push_str(&format!("  tool {}(", tool.name));
            let params: Vec<String> = tool.params.iter().map(|p| format!("{}: {:?}", p.name, p.param_type)).collect();
            prompt.push_str(&params.join(", "));
            prompt.push_str(")");
            if let Some(ref ret) = tool.return_type {
                prompt.push_str(&format!(" -> {:?}", ret));
            }
            prompt.push_str("\n");
        }
        prompt.push_str("\n");

        prompt.push_str("Propose the next tool call matching the tools schema. Return a JSON structure:\n");
        prompt.push_str("{\n  \"reasoning\": \"string explaining strategy\",\n  \"tool_call\": { \"name\": \"tool_name\", \"arguments\": [...] }\n}\n");

        prompt
    }

    pub async fn step<P: AgentProvider>(&mut self, provider: &P) -> Result<bool, String> {
        self.iteration_count += 1;
        if self.iteration_count > self.max_iterations {
            return Err(format!("Max iterations ({}) exceeded. Fallback action: {}", self.max_iterations, self.ast.fallback));
        }
        if self.budget_usd > self.max_budget_usd {
            return Err(format!("Max budget (${}) exceeded. Fallback action: {}", self.max_budget_usd, self.ast.fallback));
        }

        let prompt = self.build_prompt();
        let response = provider.reason(&prompt, &self.ast.tools).await
            .map_err(|e| format!("LLM reasoning failed: {}", e))?;

        if let Some(tool_call) = response.tool_call {
            let matched_tool = self.ast.tools.iter().find(|t| t.name == tool_call.name);
            if matched_tool.is_none() {
                let err_msg = format!("Error: Tool {} does not exist. Refer to compiled schema.", tool_call.name);
                self.last_error = Some(err_msg);
                return Ok(false);
            }

            match self.sandbox.execute_tool(&tool_call.name, tool_call.args) {
                Ok(output) => {
                    self.state.insert("last_tool_output".to_string(), output);
                    self.last_error = None;
                }
                Err(err) => {
                    let formatted_err = format!("Error: Tool {} failed: {}", tool_call.name, err);
                    self.last_error = Some(formatted_err);
                }
            }
        } else {
            self.last_error = Some("Error: No tool call was proposed by the model.".to_string());
        }

        if let Err(err) = check_and_rollback_if_needed(&self.session_id, &mut self.state, &self.ast.invariants, &self.sandbox) {
            return Err(format!("Invariant violation: {}. Triggering fallback: {}", err, self.ast.fallback));
        }

        let _snapshot_hash = save_snapshot(&self.session_id, &self.state)
            .map_err(|e| format!("Failed to save state snapshot: {}", e))?;

        let mut exec = |name: &str, args: Vec<Value>| -> Result<Value, String> {
            self.sandbox.execute_tool(name, args)
        };
        match eval_expr(&self.ast.until, &self.state, &mut exec) {
            Ok(Value::Boolean(true)) => Ok(true),
            Ok(Value::Boolean(false)) => Ok(false),
            _ => Err("Until expression must evaluate to boolean".to_string()),
        }
    }

    #[allow(dead_code)]
    pub async fn run<P: AgentProvider>(&mut self, provider: &P) -> Result<(), String> {
        let _ = save_snapshot(&self.session_id, &self.state)
            .map_err(|e| format!("Failed to save initial state snapshot: {}", e))?;

        loop {
            match self.step(provider).await {
                Ok(true) => {
                    println!("Until condition met! Loop executed successfully.");
                    break;
                }
                Ok(false) => {}
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(())
    }
}
