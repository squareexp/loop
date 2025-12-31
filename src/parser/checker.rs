use crate::parser::ast::*;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum TypeError {
    #[error("Uninitialized state variable: {0}")]
    UninitializedVariable(String),
    #[error("Unknown tool: {0}")]
    UnknownTool(String),
    #[error("Tool {tool_name} expects {expected} arguments, but got {actual}")]
    ArgCountMismatch {
        tool_name: String,
        expected: usize,
        actual: usize,
    },
    #[error("Type mismatch: expected {expected:?}, but got {actual:?}")]
    TypeMismatch {
        expected: ParamType,
        actual: ParamType,
    },
    #[error("Cannot apply binary operator {op:?} to types {lhs:?} and {rhs:?}")]
    InvalidBinaryOp {
        op: BinOpKind,
        lhs: ParamType,
        rhs: ParamType,
    },
    #[error("Assertion must yield a boolean value, but got {0:?}")]
    NonBooleanAssertion(ParamType),
    #[error("Tool {0} has no return type and cannot be used in expression")]
    VoidToolInExpression(String),
}

pub fn check_ast(ast: &LoopAst) -> Result<(), TypeError> {
    let mut state_types = HashMap::new();
    for (name, val) in &ast.state {
        let t = match val {
            Value::String(_) => ParamType::String,
            Value::Integer(_) => ParamType::Int,
            Value::Boolean(_) => ParamType::Bool,
        };
        state_types.insert(name.clone(), t);
    }

    let mut tool_decls = HashMap::new();
    for tool in &ast.tools {
        tool_decls.insert(tool.name.clone(), tool.clone());
    }

    // Check invariants
    for inv in &ast.invariants {
        let t = infer_type(inv, &state_types, &tool_decls)?;
        if t != ParamType::Bool {
            return Err(TypeError::NonBooleanAssertion(t));
        }
    }

    // Check until condition
    let until_type = infer_type(&ast.until, &state_types, &tool_decls)?;
    if until_type != ParamType::Bool {
        return Err(TypeError::NonBooleanAssertion(until_type));
    }

    Ok(())
}

fn infer_type(
    expr: &Expr,
    state_types: &HashMap<String, ParamType>,
    tool_decls: &HashMap<String, ToolDecl>,
) -> Result<ParamType, TypeError> {
    match expr {
        Expr::Literal(val) => match val {
            Value::String(_) => Ok(ParamType::String),
            Value::Integer(_) => Ok(ParamType::Int),
            Value::Boolean(_) => Ok(ParamType::Bool),
        },
        Expr::StateVar(name) => state_types
            .get(name)
            .cloned()
            .ok_or_else(|| TypeError::UninitializedVariable(name.clone())),
        Expr::ToolCall { name, args } => {
            let tool = tool_decls
                .get(name)
                .ok_or_else(|| TypeError::UnknownTool(name.clone()))?;

            if tool.params.len() != args.len() {
                return Err(TypeError::ArgCountMismatch {
                    tool_name: name.clone(),
                    expected: tool.params.len(),
                    actual: args.len(),
                });
            }

            for (param, arg) in tool.params.iter().zip(args.iter()) {
                let arg_type = infer_type(arg, state_types, tool_decls)?;
                if param.param_type != arg_type {
                    return Err(TypeError::TypeMismatch {
                        expected: param.param_type.clone(),
                        actual: arg_type,
                    });
                }
            }

            tool.return_type
                .clone()
                .ok_or_else(|| TypeError::VoidToolInExpression(name.clone()))
        }
        Expr::BinOp { lhs, op, rhs } => {
            let lhs_t = infer_type(lhs, state_types, tool_decls)?;
            let rhs_t = infer_type(rhs, state_types, tool_decls)?;

            match op {
                BinOpKind::Eq | BinOpKind::Ne => {
                    if lhs_t != rhs_t {
                        return Err(TypeError::InvalidBinaryOp {
                            op: op.clone(),
                            lhs: lhs_t,
                            rhs: rhs_t,
                        });
                    }
                    Ok(ParamType::Bool)
                }
                BinOpKind::Le | BinOpKind::Ge | BinOpKind::Lt | BinOpKind::Gt => {
                    if lhs_t != ParamType::Int || rhs_t != ParamType::Int {
                        return Err(TypeError::InvalidBinaryOp {
                            op: op.clone(),
                            lhs: lhs_t,
                            rhs: rhs_t,
                        });
                    }
                    Ok(ParamType::Bool)
                }
                BinOpKind::And | BinOpKind::Or => {
                    if lhs_t != ParamType::Bool || rhs_t != ParamType::Bool {
                        return Err(TypeError::InvalidBinaryOp {
                            op: op.clone(),
                            lhs: lhs_t,
                            rhs: rhs_t,
                        });
                    }
                    Ok(ParamType::Bool)
                }
            }
        }
    }
}
