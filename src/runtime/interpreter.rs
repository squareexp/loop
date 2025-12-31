use crate::parser::ast::{Expr, Value, BinOpKind};
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("Variable not found in state: {0}")]
    VariableNotFound(String),
    #[error("Tool execution failed: {0}")]
    ToolError(String),
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        expected: String,
        actual: String,
    },
}

pub fn eval_expr<F>(
    expr: &Expr,
    state: &HashMap<String, Value>,
    exec_tool: &mut F,
) -> Result<Value, EvalError>
where
    F: FnMut(&str, Vec<Value>) -> Result<Value, String>
{
    match expr {
        Expr::Literal(val) => Ok(val.clone()),
        Expr::StateVar(name) => state
            .get(name)
            .cloned()
            .ok_or_else(|| EvalError::VariableNotFound(name.clone())),
        Expr::ToolCall { name, args } => {
            let mut evaluated_args = Vec::new();
            for arg in args {
                evaluated_args.push(eval_expr(arg, state, exec_tool)?);
            }
            exec_tool(name, evaluated_args).map_err(EvalError::ToolError)
        }
        Expr::BinOp { lhs, op, rhs } => {
            let lhs_val = eval_expr(lhs, state, exec_tool)?;
            let rhs_val = eval_expr(rhs, state, exec_tool)?;

            match op {
                BinOpKind::Eq => Ok(Value::Boolean(lhs_val == rhs_val)),
                BinOpKind::Ne => Ok(Value::Boolean(lhs_val != rhs_val)),
                BinOpKind::Le | BinOpKind::Ge | BinOpKind::Lt | BinOpKind::Gt => {
                    let lhs_int = match lhs_val {
                        Value::Integer(i) => i,
                        _ => return Err(EvalError::TypeMismatch { expected: "Integer".to_string(), actual: format!("{:?}", lhs_val) }),
                    };
                    let rhs_int = match rhs_val {
                        Value::Integer(i) => i,
                        _ => return Err(EvalError::TypeMismatch { expected: "Integer".to_string(), actual: format!("{:?}", rhs_val) }),
                    };
                    let res = match op {
                        BinOpKind::Lt => lhs_int < rhs_int,
                        BinOpKind::Gt => lhs_int > rhs_int,
                        BinOpKind::Le => lhs_int <= rhs_int,
                        BinOpKind::Ge => lhs_int >= rhs_int,
                        _ => unreachable!(),
                    };
                    Ok(Value::Boolean(res))
                }
                BinOpKind::And | BinOpKind::Or => {
                    let lhs_bool = match lhs_val {
                        Value::Boolean(b) => b,
                        _ => return Err(EvalError::TypeMismatch { expected: "Boolean".to_string(), actual: format!("{:?}", lhs_val) }),
                    };
                    let rhs_bool = match rhs_val {
                        Value::Boolean(b) => b,
                        _ => return Err(EvalError::TypeMismatch { expected: "Boolean".to_string(), actual: format!("{:?}", rhs_val) }),
                    };
                    let res = match op {
                        BinOpKind::And => lhs_bool && rhs_bool,
                        BinOpKind::Or => lhs_bool || rhs_bool,
                        _ => unreachable!(),
                    };
                    Ok(Value::Boolean(res))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_simple() {
        let mut state = HashMap::new();
        state.insert("count".to_string(), Value::Integer(5));

        let expr = Expr::BinOp {
            lhs: Box::new(Expr::StateVar("count".to_string())),
            op: BinOpKind::Lt,
            rhs: Box::new(Expr::Literal(Value::Integer(10))),
        };

        let mut mock_tool = |_: &str, _: Vec<Value>| -> Result<Value, String> {
            unreachable!()
        };

        let res = eval_expr(&expr, &state, &mut mock_tool).unwrap();
        assert_eq!(res, Value::Boolean(true));
    }
}
