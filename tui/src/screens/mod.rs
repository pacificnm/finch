use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use finch_core::{run_command, CliCommand};
use nest_error::NestResult;
use nest_tui::prelude::*;
use nest_tui::AppContext;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// The root TUI screen. It is a thin presentation layer: all domain logic is
/// delegated to `finch_core`.
pub struct MainScreen {
    message: String,
}

impl MainScreen {
    pub fn new() -> Self {
        Self {
            message: "Press 'g' to greet, 'v' for version, 'q' to quit.".to_string(),
        }
    }
}

impl TuiScreen for MainScreen {
    fn draw(&mut self, frame: &mut Frame, _ctx: &AppContext) -> NestResult<()> {
        let paragraph = Paragraph::new(self.message.as_str());
        frame.render_widget(paragraph, frame.area());
        Ok(())
    }

    fn on_event(&mut self, event: Event, _ctx: &AppContext) -> NestResult<TuiAction> {
        if let Event::Key(KeyEvent {
            kind: KeyEventKind::Press,
            code,
            modifiers: KeyModifiers::NONE,
            ..
        }) = event
        {
            match code {
                KeyCode::Char('q') => return Ok(TuiAction::Quit),
                KeyCode::Char('g') => {
                    self.message = run_command(CliCommand::Greet {
                        name: "World".into(),
                    })
                    .unwrap_or_else(|e| format!("Error: {e}"));
                }
                KeyCode::Char('v') => {
                    self.message = run_command(CliCommand::AboutVersion)
                        .unwrap_or_else(|e| format!("Error: {e}"));
                }
                _ => {}
            }
        }
        Ok(TuiAction::Continue)
    }
}
