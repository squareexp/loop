use std::io;
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders, Paragraph},
    layout::{Layout, Constraint, Direction},
    Terminal,
};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::DisableMouseCapture,
};
use crate::scaffold::LoopStatus;

pub struct TuiDashboard {
    terminal: Option<Terminal<CrosstermBackend<io::Stdout>>>,
}

impl TuiDashboard {
    pub fn new() -> Result<Self, io::Error> {
        match enable_raw_mode() {
            Ok(_) => {
                let mut stdout = io::stdout();
                if execute!(stdout, EnterAlternateScreen).is_ok() {
                    let backend = CrosstermBackend::new(stdout);
                    if let Ok(terminal) = Terminal::new(backend) {
                        return Ok(Self { terminal: Some(terminal) });
                    }
                }
                disable_raw_mode().ok();
            }
            Err(_) => {}
        }
        Ok(Self { terminal: None })
    }

    pub fn draw(
        &mut self,
        engine: &crate::runtime::engine::Engine,
        active_provider: &str,
    ) -> Result<(), io::Error> {
        let lf = &engine.loop_file;
        let ls = &engine.loop_state;

        if let Some(ref mut term) = self.terminal {
            term.draw(|f| {
                let size = f.size();

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(10)])
                    .split(size);

                let title = Paragraph::new(format!(
                    " LOOP AGENT  ·  {}  ·  {}",
                    active_provider,
                    lf.goal.text.trim().chars().take(60).collect::<String>()
                ))
                .block(Block::default().borders(Borders::ALL));
                f.render_widget(title, chunks[0]);

                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(chunks[1]);

                // Left: metrics + current step
                let status_str = match ls.status {
                    LoopStatus::Pending   => "pending",
                    LoopStatus::Running   => "running",
                    LoopStatus::Complete  => "complete ✓",
                    LoopStatus::Exhausted => "exhausted ✗",
                };
                let mut left_lines = vec![
                    format!("Session:     {}", engine.session_id),
                    format!("Provider:    {}", active_provider),
                    format!("Status:      {}", status_str),
                    format!("Iteration:   {} / {}", ls.iteration, ls.max_iterations),
                    format!("Budget:      ${:.4} / ${:.2}", engine.budget_usd, engine.max_budget_usd),
                    String::new(),
                    "Planning Steps:".to_string(),
                ];
                for (i, step) in lf.planning.steps.iter().enumerate() {
                    let marker = if ls.completed_steps.contains(&i) { "✓" }
                        else if i == ls.current_step { "→" }
                        else { "○" };
                    left_lines.push(format!("  {} {}. {}", marker, i + 1,
                        step.chars().take(40).collect::<String>()));
                }
                if let Some(err) = &engine.last_error {
                    left_lines.push(String::new());
                    left_lines.push(format!("Last Error:\n{}", err));
                }
                let left = Paragraph::new(left_lines.join("\n"))
                    .block(Block::default().title(" Status & Planning ").borders(Borders::ALL));
                f.render_widget(left, main_chunks[0]);

                // Right: tool history + verification
                let right_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                    .split(main_chunks[1]);

                let history: Vec<String> = ls.tool_history.iter().rev().take(10)
                    .map(|t| format!("  {}", &t[..t.len().min(60)]))
                    .collect::<Vec<_>>()
                    .into_iter().rev().collect();
                let failed_tools: Vec<String> = ls.failed_tools.iter()
                    .map(|ft| format!("  ✗ {}: {}", ft.tool, &ft.error[..ft.error.len().min(50)]))
                    .collect();

                let mut tool_lines = history;
                if !failed_tools.is_empty() {
                    tool_lines.push(String::new());
                    tool_lines.push("Failed:".to_string());
                    tool_lines.extend(failed_tools);
                }
                let tool_panel = Paragraph::new(tool_lines.join("\n"))
                    .block(Block::default().title(" Tool History ").borders(Borders::ALL));
                f.render_widget(tool_panel, right_chunks[0]);

                let verif_lines: Vec<String> = lf.verification.checks.iter()
                    .map(|c| {
                        let marker = if ls.failed_checks.contains(c) { "✗" } else { "○" };
                        format!("  {} {}", marker, c)
                    })
                    .collect();
                let verif_panel = Paragraph::new(verif_lines.join("\n"))
                    .block(Block::default().title(" Verification Checks ").borders(Borders::ALL));
                f.render_widget(verif_panel, right_chunks[1]);
            })?;
        } else {
            // Fallback for non-TTY (CI)
            println!("\n─── LOOP iteration {} ─────────────────────", ls.iteration);
            println!("Goal:      {}", lf.goal.text.trim().chars().take(80).collect::<String>());
            println!("Step:      {}/{}", ls.current_step + 1, lf.planning.steps.len());
            println!("Provider:  {}", active_provider);
            if let Some(err) = &engine.last_error {
                println!("Error:     {}", err);
            }
            println!("──────────────────────────────────────────────");
        }
        Ok(())
    }
}

impl Drop for TuiDashboard {
    fn drop(&mut self) {
        if self.terminal.is_some() {
            disable_raw_mode().ok();
            if let Some(ref mut term) = self.terminal {
                execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture).ok();
                term.show_cursor().ok();
            }
        }
    }
}
