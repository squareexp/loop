use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::parser::ast::Value;

pub fn resolve_and_verify_path(sandbox_root: &Path, user_path: &str) -> Result<PathBuf, String> {
    let clean_path = user_path.strip_prefix("file://").unwrap_or(user_path);
    let path = Path::new(clean_path);

    let resolved = if path.is_absolute() {
        let relative = path.strip_prefix("/").unwrap_or(path);
        sandbox_root.join(relative)
    } else {
        sandbox_root.join(path)
    };

    let mut components = Vec::new();
    for component in resolved.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::Normal(c) => {
                components.push(c);
            }
            std::path::Component::RootDir => {}
            std::path::Component::CurDir => {}
            _ => {}
        }
    }

    let normalized: PathBuf = components.iter().collect();
    let root_norm: PathBuf = sandbox_root.components()
        .filter(|c| matches!(c, std::path::Component::Normal(_)))
        .collect();

    if normalized.starts_with(&root_norm) {
        let relative_parts: PathBuf = components.iter().skip(root_norm.components().count()).collect();
        Ok(sandbox_root.join(relative_parts))
    } else {
        Err(format!("Access denied: path {:?} escapes sandbox root {:?}", user_path, sandbox_root))
    }
}

pub struct ToolSandbox {
    pub root_dir: PathBuf,
}

impl ToolSandbox {
    pub fn new(root_dir: PathBuf) -> Self {
        fs::create_dir_all(&root_dir).ok();
        Self { root_dir }
    }

    pub fn execute_tool(&self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        match name {
            "read_file" => {
                if args.len() != 1 {
                    return Err(format!("read_file expects 1 argument, got {}", args.len()));
                }
                let path_str = match &args[0] {
                    Value::String(s) => s,
                    _ => return Err("read_file argument must be a string".to_string()),
                };
                let resolved = resolve_and_verify_path(&self.root_dir, path_str)?;
                let content = fs::read_to_string(&resolved)
                    .map_err(|e| format!("Failed to read file: {}", e))?;
                Ok(Value::String(content))
            }
            "write_file" => {
                if args.len() != 2 {
                    return Err(format!("write_file expects 2 arguments, got {}", args.len()));
                }
                let path_str = match &args[0] {
                    Value::String(s) => s,
                    _ => return Err("write_file argument 1 must be a string".to_string()),
                };
                let content = match &args[1] {
                    Value::String(s) => s,
                    _ => return Err("write_file argument 2 must be a string".to_string()),
                };
                let resolved = resolve_and_verify_path(&self.root_dir, path_str)?;
                if let Some(parent) = resolved.parent() {
                    fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent directories: {}", e))?;
                }
                fs::write(&resolved, content)
                    .map_err(|e| format!("Failed to write file: {}", e))?;
                Ok(Value::Boolean(true))
            }
            "execute" => {
                if args.len() != 1 {
                    return Err(format!("execute expects 1 argument, got {}", args.len()));
                }
                let cmd_str = match &args[0] {
                    Value::String(s) => s,
                    _ => return Err("execute argument must be a string".to_string()),
                };

                let output = Command::new("sh")
                    .arg("-c")
                    .arg(cmd_str)
                    .current_dir(&self.root_dir)
                    .output()
                    .map_err(|e| format!("Command execution failed: {}", e))?;

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                let result = if output.status.success() {
                    stdout
                } else {
                    format!("ERROR: {}\n{}", stderr, stdout)
                };
                Ok(Value::String(result))
            }
            _ => Err(format!("Tool {} is not supported by sandbox", name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sandbox_read_write() {
        let temp = TempDir::new().unwrap();
        let sandbox = ToolSandbox::new(temp.path().to_path_buf());

        let write_res = sandbox.execute_tool("write_file", vec![
            Value::String("test.txt".to_string()),
            Value::String("hello world".to_string()),
        ]).unwrap();
        assert_eq!(write_res, Value::Boolean(true));

        let read_res = sandbox.execute_tool("read_file", vec![
            Value::String("test.txt".to_string()),
        ]).unwrap();
        assert_eq!(read_res, Value::String("hello world".to_string()));
    }

    #[test]
    fn test_sandbox_traversal_prevention() {
        let temp = TempDir::new().unwrap();
        let sandbox = ToolSandbox::new(temp.path().to_path_buf());

        let res = sandbox.execute_tool("read_file", vec![
            Value::String("../outside.txt".to_string()),
        ]);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Access denied"));
    }
}
