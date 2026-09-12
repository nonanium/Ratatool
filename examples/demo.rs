use std::io;

use crossterm::event::KeyCode;
use ratatool::{
    Interaction, Panel, Runtime, Screen, SessionOptions, TerminalOptions, TerminalSession,
    ThemeFamily, ThemeId, ThemeRegistry, ThemeState, ThemeVariant, UiEvent, UiFrame,
    detect_current,
};
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Paragraph, Wrap},
};

struct Demo;

impl Screen<()> for Demo {
    fn render(&mut self, ui: &mut UiFrame<'_, '_, ()>, area: Rect) {
        let inner = ui.panel(area, &Panel::new().title(" Ratatool ").padding(1));
        ui.frame().render_widget(
            Paragraph::new(vec![
                Line::from("Adaptive terminal UI building blocks"),
                Line::from(""),
                Line::from("Press q or Escape to exit."),
            ])
            .wrap(Wrap { trim: true }),
            inner,
        );
    }

    fn handle_event(&mut self, event: &UiEvent<()>) -> Interaction<()> {
        match event {
            UiEvent::Key(key) if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) => {
                Interaction::Exit
            }
            _ => Interaction::Unhandled,
        }
    }
}

fn main() -> io::Result<()> {
    let profile = detect_current(&TerminalOptions::default());
    let theme_id = ThemeId::new("default");
    let family = ThemeFamily::new(
        theme_id.clone(),
        ThemeVariant::default(),
        ThemeVariant::default(),
        ThemeVariant::default(),
        ThemeVariant::default(),
    );
    let mut registry = ThemeRegistry::new();
    registry.register(family);
    let mut theme = ThemeState::new(theme_id, profile, &registry)
        .map_err(|error| io::Error::other(format!("invalid demo theme: {error}")))?;
    let mut session = TerminalSession::enter(SessionOptions::default())?;
    Runtime::new(Demo).run(session.terminal_mut(), &mut theme, |(), _, _| {})
}
