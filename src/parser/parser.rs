use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;
use crate::parser::ast::*;

#[derive(Parser)]
#[grammar = "parser/loop.pest"]
pub struct LoopParser;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("{0}")]
    Pest(String),
    #[error("Duplicate block: '{0}' appears more than once")]
    DuplicateBlock(String),
    #[error("Invalid integer: {0}")]
    InvalidInteger(String),
}

impl From<pest::error::Error<Rule>> for ParseError {
    fn from(e: pest::error::Error<Rule>) -> Self {
        ParseError::Pest(format_pest_error(&e, &e.to_string()))
    }
}

/// Detects common bracket mismatches and rewrites the pest error message
/// into something actionable (e.g. "Goal requires [ ] not { }").
fn format_pest_error(e: &pest::error::Error<Rule>, raw: &str) -> String {
    let src = match &e.location {
        pest::error::InputLocation::Pos(p) => *p,
        pest::error::InputLocation::Span((s, _)) => *s,
    };
    // Walk backwards to find the nearest block keyword before the error
    let before = &raw[..src.min(raw.len())];
    let keyword = ["Goal", "Memory", "Task", "Discovery", "Planning", "Execution", "Verification"]
        .iter()
        .filter_map(|k| before.rfind(k).map(|pos| (*k, pos)))
        .max_by_key(|(_, pos)| *pos);

    if let Some((kw, _)) = keyword {
        let (expected_open, expected_close) = if kw == "Goal" { ("[", "]") } else { ("{", "}") };
        let wrong = if expected_open == "[" {
            "{ } or ( )"
        } else {
            "[ ] or ( )"
        };
        // Check if the char at error position is a wrong bracket
        let ch = raw[src..].chars().next().unwrap_or(' ');
        if (ch == '{' || ch == '(' || ch == '[') && ch.to_string() != expected_open {
            return format!(
                "Syntax error near '{kw}': expected {expected_open} ... {expected_close} \
                 but found '{ch}'\n  \
                 hint: {kw} uses {expected_open} {expected_close}, not {wrong}\n\n{raw}"
            );
        }
    }
    raw.to_string()
}

/// Pre-scan for common bracket mistakes before running the PEG parser.
/// Gives clearer errors than letting pest fail on an unrecognized rule.
fn pre_check_brackets(content: &str) -> Option<ParseError> {
    let stripped: String = content
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    // Goal must use [ ]
    if let Some(pos) = stripped.find("Goal") {
        let after = stripped[pos + 4..].trim_start();
        if after.starts_with('{') || after.starts_with('(') {
            return Some(ParseError::Pest(
                "Syntax error: 'Goal' requires [ ] not { } or ( )\n  hint: Goal [ your goal here ]".into()
            ));
        }
    }

    // All other named blocks must use { }
    for kw in &["Task", "Memory", "Discovery", "Planning", "Execution", "Verification"] {
        if let Some(pos) = stripped.find(kw) {
            let after = stripped[pos + kw.len()..].trim_start();
            if after.starts_with('[') || after.starts_with('(') {
                let wrong = if after.starts_with('[') { "[ ]" } else { "( )" };
                return Some(ParseError::Pest(format!(
                    "Syntax error: '{}' requires {{ }} not {}\n  hint: {} {{ ... }}",
                    kw, wrong, kw
                )));
            }
        }
    }

    None
}

pub fn parse_loop_file(content: &str) -> Result<LoopFile, ParseError> {
    if let Some(err) = pre_check_brackets(content) {
        return Err(err);
    }

    let mut pairs = LoopParser::parse(Rule::file, content)
        .map_err(|e| {
            let raw = e.to_string();
            ParseError::Pest(format_pest_error(&e, &raw))
        })?;
    let file_pair = pairs.next().unwrap();

    let mut goal: Option<GoalBlock> = None;
    let mut memory: Option<MemoryBlock> = None;
    let mut task: Option<TaskBlock> = None;
    let mut discovery: Option<DiscoveryBlock> = None;
    let mut planning: Option<PlanningBlock> = None;
    let mut execution: Option<ExecutionBlock> = None;
    let mut verification: Option<VerificationBlock> = None;

    for block in file_pair.into_inner() {
        match block.as_rule() {
            Rule::goal_block => {
                if goal.is_some() {
                    return Err(ParseError::DuplicateBlock("Goal".into()));
                }
                let text = block.into_inner().next().unwrap().as_str().trim().to_string();
                goal = Some(GoalBlock { text });
            }

            Rule::memory_block => {
                if memory.is_some() {
                    return Err(ParseError::DuplicateBlock("Memory".into()));
                }
                let mut fields = HashMap::new();
                for field in block.into_inner() {
                    if field.as_rule() == Rule::memory_field {
                        let mut parts = field.into_inner();
                        let key = parts.next().unwrap().as_str().to_string();
                        let val_pair = parts.next().unwrap();
                        let val = parse_memory_value(val_pair)?;
                        fields.insert(key, val);
                    }
                }
                memory = Some(MemoryBlock { fields });
            }

            Rule::task_block => {
                if task.is_some() {
                    return Err(ParseError::DuplicateBlock("Task".into()));
                }
                let items = block
                    .into_inner()
                    .filter(|p| p.as_rule() == Rule::string)
                    .map(parse_string_literal)
                    .collect();
                task = Some(TaskBlock { items });
            }

            Rule::discovery_block => {
                if discovery.is_some() {
                    return Err(ParseError::DuplicateBlock("Discovery".into()));
                }
                let mut scan = Vec::new();
                let mut find = Vec::new();
                for child in block.into_inner() {
                    match child.as_rule() {
                        Rule::scan_field => {
                            scan = parse_string_array(child.into_inner().next().unwrap());
                        }
                        Rule::find_field => {
                            find = parse_string_array(child.into_inner().next().unwrap());
                        }
                        _ => {}
                    }
                }
                discovery = Some(DiscoveryBlock { scan, find });
            }

            Rule::planning_block => {
                if planning.is_some() {
                    return Err(ParseError::DuplicateBlock("Planning".into()));
                }
                let mut steps = Vec::new();
                let mut max_iterations: u32 = 5;
                for child in block.into_inner() {
                    match child.as_rule() {
                        Rule::steps_field => {
                            steps = parse_string_array(child.into_inner().next().unwrap());
                        }
                        Rule::max_iterations_field => {
                            let n = child.into_inner().next().unwrap().as_str();
                            max_iterations = n.parse().map_err(|_| ParseError::InvalidInteger(n.to_string()))?;
                        }
                        _ => {}
                    }
                }
                planning = Some(PlanningBlock { steps, max_iterations });
            }

            Rule::execution_block => {
                if execution.is_some() {
                    return Err(ParseError::DuplicateBlock("Execution".into()));
                }
                let mut tools = Vec::new();
                let mut strategy = String::new();
                for child in block.into_inner() {
                    match child.as_rule() {
                        Rule::tools_field => {
                            for decl in child.into_inner() {
                                if decl.as_rule() == Rule::tool_decl {
                                    tools.push(parse_tool_decl(decl)?);
                                }
                            }
                        }
                        Rule::strategy_field => {
                            strategy = parse_string_literal(child.into_inner().next().unwrap());
                        }
                        _ => {}
                    }
                }
                execution = Some(ExecutionBlock { tools, strategy });
            }

            Rule::verification_block => {
                if verification.is_some() {
                    return Err(ParseError::DuplicateBlock("Verification".into()));
                }
                let mut checks = Vec::new();
                let mut on_fail = OnFail::Retry;
                let mut max_retries: u32 = 3;
                for child in block.into_inner() {
                    match child.as_rule() {
                        Rule::checks_field => {
                            checks = parse_string_array(child.into_inner().next().unwrap());
                        }
                        Rule::on_fail_field => {
                            let v = child.into_inner().next().unwrap().as_str();
                            on_fail = if v == "complete" { OnFail::Complete } else { OnFail::Retry };
                        }
                        Rule::max_retries_field => {
                            let n = child.into_inner().next().unwrap().as_str();
                            max_retries = n.parse().map_err(|_| ParseError::InvalidInteger(n.to_string()))?;
                        }
                        _ => {}
                    }
                }
                verification = Some(VerificationBlock { checks, on_fail, max_retries });
            }

            Rule::EOI => {}
            _ => {}
        }
    }

    Ok(LoopFile {
        goal: goal.unwrap_or(GoalBlock { text: String::new() }),
        memory,
        task,
        discovery: discovery.unwrap_or(DiscoveryBlock { scan: vec![], find: vec![] }),
        planning: planning.unwrap_or(PlanningBlock { steps: vec![], max_iterations: 5 }),
        execution: execution.unwrap_or(ExecutionBlock { tools: vec![], strategy: String::new() }),
        verification: verification.unwrap_or(VerificationBlock {
            checks: vec![],
            on_fail: OnFail::Retry,
            max_retries: 3,
        }),
    })
}

fn parse_string_literal(pair: pest::iterators::Pair<Rule>) -> String {
    pair.into_inner().next().unwrap().as_str().to_string()
}

fn parse_string_array(pair: pest::iterators::Pair<Rule>) -> Vec<String> {
    pair.into_inner()
        .filter(|p| p.as_rule() == Rule::string)
        .map(parse_string_literal)
        .collect()
}

fn parse_memory_value(pair: pest::iterators::Pair<Rule>) -> Result<MemoryValue, ParseError> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::string => Ok(MemoryValue::String(parse_string_literal(inner))),
        Rule::integer => {
            let n = inner.as_str();
            Ok(MemoryValue::Integer(n.parse().map_err(|_| ParseError::InvalidInteger(n.to_string()))?))
        }
        Rule::boolean => Ok(MemoryValue::Boolean(inner.as_str() == "true")),
        Rule::string_array => Ok(MemoryValue::StringArray(parse_string_array(inner))),
        _ => unreachable!(),
    }
}

fn parse_tool_decl(pair: pest::iterators::Pair<Rule>) -> Result<ToolDecl, ParseError> {
    let mut parts = pair.into_inner();
    let name = parts.next().unwrap().as_str().to_string();
    let mut params = Vec::new();
    let mut return_type = None;

    for part in parts {
        match part.as_rule() {
            Rule::tool_param => {
                let mut pp = part.into_inner();
                let p_name = pp.next().unwrap().as_str().to_string();
                let p_type = parse_param_type(pp.next().unwrap().as_str());
                params.push(ToolParam { name: p_name, param_type: p_type });
            }
            Rule::param_type => {
                return_type = Some(parse_param_type(part.as_str()));
            }
            _ => {}
        }
    }

    Ok(ToolDecl { name, params, return_type })
}

fn parse_param_type(s: &str) -> ParamType {
    match s {
        "int"  => ParamType::Int,
        "bool" => ParamType::Bool,
        _      => ParamType::String,
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::checker::check_loop_file;

    fn minimal_valid(extra: &str) -> String {
        format!(r#"
            Goal [ Fix the thing ]
            {extra}
            Discovery {{ find: ["What broke?"] }}
            Planning {{ steps: ["Investigate"] }}
            Execution {{
                tools: [ read_file(path: string) -> string ]
                strategy: "read files"
            }}
            Verification {{ checks: ["tests pass"] on_fail: retry }}
        "#)
    }

    #[test]
    fn test_parse_valid_full() {
        let content = r#"
            Goal [
                Build a REST API with authentication
            ]
            Memory {
                project_path: "./api"
                stack: ["Rust", "Axum"]
            }
            Task {
                "Add JWT middleware"
                "Write auth tests"
                "Document endpoints"
            }
            Discovery {
                scan: ["src/**/*.rs", "Cargo.toml"]
                find: ["What routes exist?"]
            }
            Planning {
                steps: ["Read source" "Add middleware" "Write tests"]
                max_iterations: 4
            }
            Execution {
                tools: [
                    read_file(path: string) -> string
                    write_file(path: string, content: string) -> bool
                ]
                strategy: "Execute in order."
            }
            Verification {
                checks: ["cargo test passes"]
                on_fail: retry
                max_retries: 3
            }
        "#;
        let lf = parse_loop_file(content).expect("parse failed");
        assert!(lf.goal.text.contains("REST API"));
        assert!(lf.task.is_some());
        assert_eq!(lf.task.as_ref().unwrap().items.len(), 3);
        assert_eq!(lf.planning.max_iterations, 4);
        assert_eq!(lf.execution.tools.len(), 2);
        assert!(check_loop_file(&lf).is_ok());
    }

    #[test]
    fn test_task_optional() {
        let lf = parse_loop_file(&minimal_valid("")).expect("parse failed");
        assert!(lf.task.is_none());
        assert!(check_loop_file(&lf).is_ok());
    }

    #[test]
    fn test_task_present() {
        let lf = parse_loop_file(&minimal_valid(
            r#"Task { "do a" "do b" }"#
        )).expect("parse failed");
        assert_eq!(lf.task.as_ref().unwrap().items, vec!["do a", "do b"]);
    }

    #[test]
    fn test_goal_wrong_bracket_errors() {
        // Goal must use [] not {}
        let content = r#"Goal { write some code }"#;
        let result = parse_loop_file(content);
        assert!(result.is_err(), "Goal with {{}} should fail");
    }

    #[test]
    fn test_task_wrong_bracket_errors() {
        // Task must use {} not []
        let content = r#"
            Goal [ fix ]
            Task [ "do a" "do b" ]
            Discovery { find: ["x"] }
            Planning { steps: ["x"] }
            Execution { tools: [ read_file(path: string) -> string ] strategy: "s" }
            Verification { checks: ["c"] on_fail: retry }
        "#;
        let result = parse_loop_file(content);
        assert!(result.is_err(), "Task with [] should fail");
    }

    #[test]
    fn test_duplicate_block_error() {
        let content = r#"
            Goal [ First ]
            Goal [ Second ]
            Discovery { find: ["x"] }
            Planning { steps: ["x"] }
            Execution { tools: [ read_file(path: string) -> string ] strategy: "s" }
            Verification { checks: ["c"] on_fail: retry }
        "#;
        assert!(parse_loop_file(content).is_err());
    }

    #[test]
    fn test_on_fail_complete() {
        let content = r#"
            Goal [ Ship it ]
            Discovery { find: ["check"] }
            Planning { steps: ["deploy"] max_iterations: 1 }
            Execution {
                tools: [ run_command(cmd: string) -> string ]
                strategy: "run deploy"
            }
            Verification { checks: ["up"] on_fail: complete max_retries: 0 }
        "#;
        let lf = parse_loop_file(content).expect("parse failed");
        assert_eq!(lf.verification.on_fail, OnFail::Complete);
    }
}
