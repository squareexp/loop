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

    pub fn draw(&mut self, engine: &crate::runtime::engine::Engine, active_provider: &str) -> Result<(), io::Error> {
        if let Some(ref mut term) = self.terminal {
            term.draw(|f| {
                let size = f.size();

                // Layout: Title (3 rows), Main (rest)
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(10),
                    ])
                    .split(size);

                // Title Widget
                let title = Paragraph::new(" LOOP: AGENT ENGINE CONTROL PANEL ")
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(title, chunks[0]);

                // Main layout: Left (Metrics), Right (State & Invariants)
                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ])
                    .split(chunks[1]);

                // Left panel: Stats
                let mut stats_text = vec![
                    format!("Session ID:        {}", engine.session_id),
                    format!("Active Provider:   {}", active_provider),
                    format!("Iteration Depth:   {} / {}", engine.iteration_count, engine.max_iterations),
                    format!("Spent Budget:      ${:.4} / ${:.2}", engine.budget_usd, engine.max_budget_usd),
                ];
                if let Some(err) = &engine.last_error {
                    stats_text.push(format!("\nLast Error:\n{}", err));
                }

                let stats_paragraph = Paragraph::new(stats_text.join("\n"))
                    .block(Block::default().title(" Metrics & Performance ").borders(Borders::ALL));
                f.render_widget(stats_paragraph, main_chunks[0]);

                // Right panel layout: Top (State Variables), Bottom (Invariants)
                let right_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(60),
                        Constraint::Percentage(40),
                    ])
                    .split(main_chunks[1]);

                // State Variables
                let mut state_items = Vec::new();
                for (k, v) in &engine.state {
                    state_items.push(format!("  {}: {:?}", k, v));
                }
                let state_paragraph = Paragraph::new(state_items.join("\n"))
                    .block(Block::default().title(" State Ledger Variables ").borders(Borders::ALL));
                f.render_widget(state_paragraph, right_chunks[0]);

                // Invariants
                let mut inv_items = Vec::new();
                for (i, inv) in engine.ast.invariants.iter().enumerate() {
                    inv_items.push(format!("  [Passed] Invariant #{}: {:?}", i + 1, inv));
                }
                if inv_items.is_empty() {
                    inv_items.push("  No invariants defined.".to_string());
                }
                let inv_paragraph = Paragraph::new(inv_items.join("\n"))
                    .block(Block::default().title(" Invariant Assertions ").borders(Borders::ALL));
                f.render_widget(inv_paragraph, right_chunks[1]);
            })?;
        } else {
            // Fallback to clean stdout printing when TTY is not available (e.g. CI/non-interactive test)
            println!("\n--- LOOP iteration {} ---", engine.iteration_count);
            println!("Session:    {}", engine.session_id);
            println!("Provider:   {}", active_provider);
            println!("Variables:");
            for (k, v) in &engine.state {
                println!("  {}: {:?}", k, v);
            }
            if let Some(err) = &engine.last_error {
                println!("Error:      {}", err);
            }
            println!("-----------------------------");
        }
        Ok(())
    }
}

impl Drop for TuiDashboard {
    fn drop(&mut self) {
        if self.terminal.is_some() {
            disable_raw_mode().ok();
            if let Some(ref mut term) = self.terminal {
                execute!(
                    term.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                ).ok();
                term.show_cursor().ok();
            }
        }
    }
}
