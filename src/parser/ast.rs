use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    String(String),
    Integer(i64),
    Boolean(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParamType {
    String,
    Int,
    Bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolParam {
    pub name: String,
    pub param_type: ParamType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDecl {
    pub name: String,
    pub params: Vec<ToolParam>,
    pub return_type: Option<ParamType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BinOpKind {
    Eq,
    Ne,
    Le,
    Ge,
    Lt,
    Gt,
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Expr {
    Literal(Value),
    StateVar(String),
    ToolCall {
        name: String,
        args: Vec<Expr>,
    },
    BinOp {
        lhs: Box<Expr>,
        op: BinOpKind,
        rhs: Box<Expr>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopAst {
    pub task: String,
    pub state: HashMap<String, Value>,
    pub tools: Vec<ToolDecl>,
    pub invariants: Vec<Expr>,
    pub strategy: String,
    pub until: Expr,
    pub fallback: String,
}
