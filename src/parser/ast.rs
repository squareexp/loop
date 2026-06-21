use std::collections::HashMap;
use serde::{Serialize, Deserialize};

// Value is kept for storage compatibility (sled snapshots, provider args)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    String(String),
    Integer(i64),
    Boolean(bool),
}

// ─── Tool Declaration (used by execution + providers) ────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParamType {
    String,
    Int,
    Bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParam {
    pub name: String,
    pub param_type: ParamType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDecl {
    pub name: String,
    pub params: Vec<ToolParam>,
    pub return_type: Option<ParamType>,
}

// ─── Loop Blocks ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalBlock {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    StringArray(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBlock {
    pub fields: HashMap<String, MemoryValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryBlock {
    pub scan: Vec<String>,
    pub find: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningBlock {
    pub steps: Vec<String>,
    pub max_iterations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionBlock {
    pub tools: Vec<ToolDecl>,
    pub strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OnFail {
    Retry,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationBlock {
    pub checks: Vec<String>,
    pub on_fail: OnFail,
    pub max_retries: u32,
}

// ─── Task Block ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBlock {
    pub items: Vec<String>,
}

// ─── Root AST Node ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopFile {
    pub goal: GoalBlock,
    pub memory: Option<MemoryBlock>,
    pub task: Option<TaskBlock>,
    pub discovery: DiscoveryBlock,
    pub planning: PlanningBlock,
    pub execution: ExecutionBlock,
    pub verification: VerificationBlock,
}
