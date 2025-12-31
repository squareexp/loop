use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;
use crate::parser::ast::*;

#[derive(Parser)]
#[grammar = "parser/loop.pest"]
pub struct LoopParser;

#[derive(Debug, thiserror::Error)]
pub enum ParserError {
    #[error("Pest parsing error: {0}")]
    PestError(#[from] pest::error::Error<Rule>),
    #[error("Missing block: {0}")]
    MissingBlock(String),
    #[error("Duplicate block: {0}")]
    DuplicateBlock(String),
    #[error("Invalid value format: {0}")]
    InvalidValue(String),
}

pub fn parse_loop_file(content: &str) -> Result<LoopAst, ParserError> {
    let mut pairs = LoopParser::parse(Rule::file, content)?;
    let file_pair = pairs.next().ok_or_else(|| ParserError::MissingBlock("file".to_string()))?;

    let mut task: Option<String> = None;
    let mut state: Option<HashMap<String, Value>> = None;
    let mut tools: Option<Vec<ToolDecl>> = None;
    let mut invariants: Option<Vec<Expr>> = None;
    let mut strategy: Option<String> = None;
    let mut until: Option<Expr> = None;
    let mut fallback: Option<String> = None;

    for item in file_pair.into_inner() {
        match item.as_rule() {
            Rule::block => {
                let inner = item.into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::task_block => {
                        if task.is_some() {
                            return Err(ParserError::DuplicateBlock("task".to_string()));
                        }
                        let string_pair = inner.into_inner().next().unwrap();
                        task = Some(parse_string_literal(string_pair)?);
                    }
                    Rule::state_block => {
                        if state.is_some() {
                            return Err(ParserError::DuplicateBlock("state".to_string()));
                        }
                        let mut state_map = HashMap::new();
                        for field in inner.into_inner() {
                            if field.as_rule() == Rule::state_field {
                                let mut parts = field.into_inner();
                                let name = parts.next().unwrap().as_str().to_string();
                                let val_pair = parts.next().unwrap();
                                let val = match val_pair.as_rule() {
                                    Rule::string => Value::String(parse_string_literal(val_pair)?),
                                    Rule::integer => Value::Integer(val_pair.as_str().parse().map_err(|_| ParserError::InvalidValue("integer".to_string()))?),
                                    Rule::boolean => Value::Boolean(val_pair.as_str() == "true"),
                                    _ => unreachable!(),
                                };
                                state_map.insert(name, val);
                            }
                        }
                        state = Some(state_map);
                    }
                    Rule::tools_block => {
                        if tools.is_some() {
                            return Err(ParserError::DuplicateBlock("tools".to_string()));
                        }
                        let mut decls = Vec::new();
                        for decl in inner.into_inner() {
                            if decl.as_rule() == Rule::tool_decl {
                                let mut parts = decl.into_inner();
                                let name = parts.next().unwrap().as_str().to_string();
                                let mut params = Vec::new();
                                let mut return_type = None;

                                for part in parts {
                                    match part.as_rule() {
                                        Rule::tool_param => {
                                            let mut param_parts = part.into_inner();
                                            let p_name = param_parts.next().unwrap().as_str().to_string();
                                            let p_type_str = param_parts.next().unwrap().as_str();
                                            let p_type = match p_type_str {
                                                "string" => ParamType::String,
                                                "int" => ParamType::Int,
                                                "bool" => ParamType::Bool,
                                                _ => unreachable!(),
                                            };
                                            params.push(ToolParam { name: p_name, param_type: p_type });
                                        }
                                        Rule::param_type => {
                                            let p_type = match part.as_str() {
                                                "string" => ParamType::String,
                                                "int" => ParamType::Int,
                                                "bool" => ParamType::Bool,
                                                _ => unreachable!(),
                                            };
                                            return_type = Some(p_type);
                                        }
                                        _ => {}
                                    }
                                }
                                decls.push(ToolDecl { name, params, return_type });
                            }
                        }
                        tools = Some(decls);
                    }
                    Rule::invariant_block => {
                        if invariants.is_some() {
                            return Err(ParserError::DuplicateBlock("invariant".to_string()));
                        }
                        let mut exprs = Vec::new();
                        for expr_pair in inner.into_inner() {
                            if expr_pair.as_rule() == Rule::expr {
                                exprs.push(parse_expr(expr_pair)?);
                            }
                        }
                        invariants = Some(exprs);
                    }
                    Rule::strategy_block => {
                        if strategy.is_some() {
                            return Err(ParserError::DuplicateBlock("strategy".to_string()));
                        }
                        let string_pair = inner.into_inner().next().unwrap();
                        strategy = Some(parse_string_literal(string_pair)?);
                    }
                    Rule::until_block => {
                        if until.is_some() {
                            return Err(ParserError::DuplicateBlock("until".to_string()));
                        }
                        let expr_pair = inner.into_inner().next().unwrap();
                        until = Some(parse_expr(expr_pair)?);
                    }
                    Rule::fallback_block => {
                        if fallback.is_some() {
                            return Err(ParserError::DuplicateBlock("fallback".to_string()));
                        }
                        let string_pair = inner.into_inner().next().unwrap();
                        fallback = Some(parse_string_literal(string_pair)?);
                    }
                    _ => unreachable!(),
                }
            }
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }

    Ok(LoopAst {
        task: task.ok_or_else(|| ParserError::MissingBlock("task".to_string()))?,
        state: state.unwrap_or_default(),
        tools: tools.unwrap_or_default(),
        invariants: invariants.unwrap_or_default(),
        strategy: strategy.ok_or_else(|| ParserError::MissingBlock("strategy".to_string()))?,
        until: until.ok_or_else(|| ParserError::MissingBlock("until".to_string()))?,
        fallback: fallback.ok_or_else(|| ParserError::MissingBlock("fallback".to_string()))?,
    })
}

fn parse_string_literal(pair: pest::iterators::Pair<Rule>) -> Result<String, ParserError> {
    let inner = pair.into_inner().next().unwrap();
    let raw = inner.as_str();
    // basic unescaping could be added if needed, for now raw is fine
    Ok(raw.to_string())
}

fn parse_expr(pair: pest::iterators::Pair<Rule>) -> Result<Expr, ParserError> {
    let mut inner = pair.into_inner();
    let mut primary_expr = parse_primary(inner.next().unwrap())?;

    while let Some(op_pair) = inner.next() {
        let op = match op_pair.as_str() {
            "==" => BinOpKind::Eq,
            "!=" => BinOpKind::Ne,
            "<=" => BinOpKind::Le,
            ">=" => BinOpKind::Ge,
            "<" => BinOpKind::Lt,
            ">" => BinOpKind::Gt,
            "&&" => BinOpKind::And,
            "||" => BinOpKind::Or,
            _ => unreachable!(),
        };
        let rhs_pair = inner.next().unwrap();
        let rhs = parse_primary(rhs_pair)?;
        primary_expr = Expr::BinOp {
            lhs: Box::new(primary_expr),
            op,
            rhs: Box::new(rhs),
        };
    }

    Ok(primary_expr)
}

fn parse_primary(pair: pest::iterators::Pair<Rule>) -> Result<Expr, ParserError> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::boolean => Ok(Expr::Literal(Value::Boolean(inner.as_str() == "true"))),
        Rule::integer => Ok(Expr::Literal(Value::Integer(inner.as_str().parse().unwrap()))),
        Rule::string => Ok(Expr::Literal(Value::String(parse_string_literal(inner)?))),
        Rule::state_var => {
            let var_name = inner.into_inner().next().unwrap().as_str().to_string();
            Ok(Expr::StateVar(var_name))
        }
        Rule::tool_call => {
            let mut parts = inner.into_inner();
            let name = parts.next().unwrap().as_str().to_string();
            let mut args = Vec::new();
            for arg_pair in parts {
                args.push(parse_expr(arg_pair)?);
            }
            Ok(Expr::ToolCall { name, args })
        }
        Rule::expr => parse_expr(inner),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::checker::check_ast;

    #[test]
    fn test_parse_and_check_valid() {
        let content = r#"
            task {
                "Solve the bug"
            }
            state {
                counter: 0,
                filepath: "src/main.rs",
                is_done: false
            }
            tools {
                tool read_file(path: string) -> string
                tool write_file(path: string, content: string) -> bool
            }
            invariant {
                state.counter < 10
            }
            strategy {
                "Read and write."
            }
            until {
                state.is_done == true
            }
            fallback {
                "Failed."
            }
        "#;
        let ast = parse_loop_file(content).unwrap();
        assert_eq!(ast.task, "Solve the bug");
        assert_eq!(ast.strategy, "Read and write.");
        assert_eq!(ast.fallback, "Failed.");
        assert!(ast.state.contains_key("counter"));
        assert!(ast.state.contains_key("filepath"));
        assert!(ast.state.contains_key("is_done"));

        let check_res = check_ast(&ast);
        assert!(check_res.is_ok(), "Type check failed: {:?}", check_res);
    }

    #[test]
    fn test_check_invalid_var() {
        let content = r#"
            task { "Fix" }
            state { counter: 0 }
            tools {}
            invariant { state.nonexistent < 10 }
            strategy { "Fix" }
            until { state.counter == 0 }
            fallback { "Fail" }
        "#;
        let ast = parse_loop_file(content).unwrap();
        let check_res = check_ast(&ast);
        assert!(check_res.is_err());
        assert!(check_res.unwrap_err().to_string().contains("Uninitialized state variable"));
    }

    #[test]
    fn test_check_non_bool_until() {
        let content = r#"
            task { "Fix" }
            state { counter: 0 }
            tools {}
            invariant {}
            strategy { "Fix" }
            until { state.counter }
            fallback { "Fail" }
        "#;
        let ast = parse_loop_file(content).unwrap();
        let check_res = check_ast(&ast);
        assert!(check_res.is_err());
        assert!(check_res.unwrap_err().to_string().contains("Assertion must yield a boolean value"));
    }
}
