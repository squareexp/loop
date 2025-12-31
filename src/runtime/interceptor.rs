use crate::parser::ast::{Expr, Value};
use crate::runtime::interpreter::eval_expr;
use crate::runtime::sandbox::ToolSandbox;
use crate::storage::{get_latest_snapshot_hash, get_snapshot};
use std::collections::HashMap;

pub fn evaluate_invariants(
    invariants: &[Expr],
    state: &HashMap<String, Value>,
    sandbox: &ToolSandbox,
) -> Result<(), String> {
    for inv in invariants {
        let mut exec = |name: &str, args: Vec<Value>| -> Result<Value, String> {
            sandbox.execute_tool(name, args)
        };
        match eval_expr(inv, state, &mut exec) {
            Ok(Value::Boolean(true)) => {}
            Ok(other) => {
                return Err(format!("Invariant assertion evaluated to non-true value: {:?}", other));
            }
            Err(e) => {
                return Err(format!("Invariant evaluation error: {:?}", e));
            }
        }
    }
    Ok(())
}

pub fn check_and_rollback_if_needed(
    session_id: &str,
    current_state: &mut HashMap<String, Value>,
    invariants: &[Expr],
    sandbox: &ToolSandbox,
) -> Result<(), String> {
    if let Err(err) = evaluate_invariants(invariants, current_state, sandbox) {
        // Attempt to roll back state variables from Sled
        if let Ok(Some(latest_hash)) = get_latest_snapshot_hash(session_id) {
            if let Ok(last_state) = get_snapshot(&latest_hash) {
                *current_state = last_state.variables;
            }
        }
        return Err(format!("Invariant Violated: {}. State rolled back.", err));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::BinOpKind;
    use crate::storage::save_snapshot;
    use tempfile::TempDir;

    #[test]
    fn test_invariant_success_and_rollback() {
        let temp = TempDir::new().unwrap();
        let sandbox = ToolSandbox::new(temp.path().to_path_buf());
        let session_id = "test-session-invariants";

        let mut state = HashMap::new();
        state.insert("counter".to_string(), Value::Integer(5));

        // Save initial valid snapshot
        let _ = save_snapshot(session_id, &state).unwrap();

        // Invariant: counter < 10
        let invariants = vec![Expr::BinOp {
            lhs: Box::new(Expr::StateVar("counter".to_string())),
            op: BinOpKind::Lt,
            rhs: Box::new(Expr::Literal(Value::Integer(10))),
        }];

        // This should pass
        assert!(check_and_rollback_if_needed(session_id, &mut state, &invariants, &sandbox).is_ok());

        // Now break the state: counter = 15
        state.insert("counter".to_string(), Value::Integer(15));

        // This should fail and roll state back to 5
        let res = check_and_rollback_if_needed(session_id, &mut state, &invariants, &sandbox);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Invariant Violated"));
        assert_eq!(state.get("counter"), Some(&Value::Integer(5)));
    }
}
