use crate::parser::ast::LoopFile;

#[derive(Debug, thiserror::Error)]
pub enum CheckError {
    #[error("{0}")]
    Violation(String),
}

pub struct CheckReport {
    pub errors: Vec<String>,
}

impl CheckReport {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Full validation pass — returns every error found, not just the first.
pub fn check_loop_file(lf: &LoopFile) -> Result<(), CheckError> {
    let report = audit(lf);
    if report.is_ok() {
        Ok(())
    } else {
        Err(CheckError::Violation(report.errors.join("\n")))
    }
}

/// Returns a CheckReport with all errors so the CLI can show a nice summary.
pub fn audit(lf: &LoopFile) -> CheckReport {
    let mut errors = Vec::new();

    // ── Goal ────────────────────────────────────────────────────────────────
    if lf.goal.text.trim().is_empty() {
        errors.push("Goal [] is empty — describe what you want to achieve".into());
    }

    // ── Discovery ───────────────────────────────────────────────────────────
    if lf.discovery.scan.is_empty() && lf.discovery.find.is_empty() {
        errors.push(
            "Discovery {} has no scan patterns or find questions — add at least one".into(),
        );
    }

    // ── Planning ────────────────────────────────────────────────────────────
    if lf.planning.steps.is_empty() {
        errors.push("Planning {} has no steps — add at least one step".into());
    }
    if lf.planning.max_iterations == 0 {
        errors.push("Planning.max_iterations must be >= 1".into());
    }

    // ── Execution ───────────────────────────────────────────────────────────
    if lf.execution.tools.is_empty() {
        errors.push("Execution {} has no tools — declare at least one tool the agent can use".into());
    }
    if lf.execution.strategy.trim().is_empty() {
        errors.push("Execution.strategy is empty — tell the agent how to execute the steps".into());
    }

    // ── Verification ────────────────────────────────────────────────────────
    if lf.verification.checks.is_empty() {
        errors.push(
            "Verification {} has no checks — add at least one condition that must pass".into(),
        );
    }

    CheckReport { errors }
}

/// Pretty-prints a full block-by-block status for `loop check`.
pub fn format_check_output(lf: &LoopFile) -> String {
    let report = audit(lf);
    let mut out = String::new();

    let block_status = |label: &str, present: bool, err: Option<&str>| -> String {
        if let Some(msg) = err {
            format!("  ✗ {} — {}\n", label, msg)
        } else if present {
            format!("  ✓ {}\n", label)
        } else {
            format!("  ○ {} (optional — not found)\n", label)
        }
    };

    // Goal
    let goal_err = report.errors.iter().find(|e| e.contains("Goal"));
    out.push_str(&block_status("Goal", !lf.goal.text.trim().is_empty(), goal_err.map(|s| s.as_str())));

    // Memory (optional)
    out.push_str(&block_status("Memory", lf.memory.is_some(), None));

    // Task (optional)
    out.push_str(&block_status("Task", lf.task.is_some(), None));

    // Discovery
    let disc_err = report.errors.iter().find(|e| e.contains("Discovery"));
    let disc_present = !lf.discovery.scan.is_empty() || !lf.discovery.find.is_empty();
    out.push_str(&block_status("Discovery", disc_present, disc_err.map(|s| s.as_str())));

    // Planning
    let plan_err = report.errors.iter().find(|e| e.contains("Planning"));
    out.push_str(&block_status("Planning", !lf.planning.steps.is_empty(), plan_err.map(|s| s.as_str())));

    // Execution
    let exec_err = report.errors.iter().find(|e| e.contains("Execution"));
    out.push_str(&block_status("Execution", !lf.execution.tools.is_empty(), exec_err.map(|s| s.as_str())));

    // Verification
    let verif_err = report.errors.iter().find(|e| e.contains("Verification"));
    out.push_str(&block_status("Verification", !lf.verification.checks.is_empty(), verif_err.map(|s| s.as_str())));

    if report.is_ok() {
        out.push_str("\n  All checks passed — loop file is valid.\n");
    } else {
        out.push_str(&format!(
            "\n  {} error{} found. Fix them before running.\n",
            report.errors.len(),
            if report.errors.len() == 1 { "" } else { "s" }
        ));
    }

    out
}
