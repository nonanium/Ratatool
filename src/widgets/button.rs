use std::borrow::Cow;

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthStr;

use crate::theme::ResolvedTheme;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonKind {
    #[default]
    Primary,
    Secondary,
    Cancel,
    Destructive,
}

/// A stateless action button.
///
/// The caller owns focus and activation state. This renderer consumes no keys
/// and registers no mouse regions. It uses `ResolvedTheme::buttons` and the
/// resolved focus marker. A one-row area is meaningful; narrow labels are
/// clipped by Ratatui.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Button<'a> {
    pub label: Cow<'a, str>,
    pub kind: ButtonKind,
    pub enabled: bool,
    pub focused: bool,
}

impl<'a> Button<'a> {
    pub fn new(label: impl Into<Cow<'a, str>>) -> Self {
        Self {
            label: label.into(),
            kind: ButtonKind::Primary,
            enabled: true,
            focused: false,
        }
    }

    pub fn kind(mut self, kind: ButtonKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

pub fn render_button(
    frame: &mut Frame<'_>,
    area: Rect,
    button: &Button<'_>,
    theme: &ResolvedTheme,
) {
    let style = if !button.enabled {
        theme.buttons.disabled
    } else {
        match (button.kind, button.focused) {
            (ButtonKind::Primary, false) => theme.buttons.primary,
            (ButtonKind::Primary, true) => theme.buttons.primary_focused,
            (ButtonKind::Secondary, false) => theme.buttons.secondary,
            (ButtonKind::Secondary, true) => theme.buttons.secondary_focused,
            (ButtonKind::Cancel, false) => theme.buttons.cancel,
            (ButtonKind::Cancel, true) => theme.buttons.cancel_focused,
            (ButtonKind::Destructive, false) => theme.buttons.destructive,
            (ButtonKind::Destructive, true) => theme.buttons.destructive_focused,
        }
    };

    let marker: Cow<'_, str> = if button.focused {
        Cow::Borrowed(theme.symbols.focus_marker)
    } else {
        Cow::Owned(" ".repeat(UnicodeWidthStr::width(theme.symbols.focus_marker)))
    };
    let line = Line::from(vec![
        Span::styled(marker, style),
        Span::styled(" ", style),
        Span::styled(button.label.as_ref(), style),
    ]);
    frame.render_widget(Paragraph::new(line).alignment(Alignment::Center), area);
}

pub fn button_minimum_width(label: &str, theme: &ResolvedTheme) -> u16 {
    UnicodeWidthStr::width(label)
        .saturating_add(UnicodeWidthStr::width(theme.symbols.focus_marker))
        .saturating_add(1)
        .min(usize::from(u16::MAX)) as u16
}

pub fn horizontal_button_widths(available: u16, minimums: &[u16]) -> Option<Vec<u16>> {
    let required = minimums
        .iter()
        .try_fold(0u16, |total, width| total.checked_add(*width))?;
    if required > available {
        return None;
    }
    if minimums.is_empty() {
        return Some(Vec::new());
    }

    let count = u16::try_from(minimums.len()).ok()?;
    let extra = available - required;
    Some(
        minimums
            .iter()
            .zip(0..count)
            .map(|(width, index)| width + extra / count + u16::from(index < extra % count))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, style::Style};

    use super::*;
    use crate::{ResolvedTheme, Symbols, ThemeVariant};

    fn theme() -> ResolvedTheme {
        let variant = ThemeVariant::default();
        ResolvedTheme {
            text: variant.text,
            borders: variant.borders,
            surfaces: variant.surfaces,
            buttons: crate::ButtonTheme {
                primary_focused: Style::default().reversed(),
                disabled: Style::default().dim(),
                ..Default::default()
            },
            status: variant.status,
            input: variant.input,
            selection: variant.selection,
            metrics: variant.metrics,
            symbols: Symbols::ascii(),
        }
    }

    #[test]
    fn focused_button_uses_resolved_marker_and_style() {
        let mut terminal = Terminal::new(TestBackend::new(12, 1)).unwrap();
        terminal
            .draw(|frame| {
                render_button(
                    frame,
                    frame.area(),
                    &Button::new("Save").focused(true),
                    &theme(),
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let rendered: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
        assert_eq!(rendered, "   > Save   ");
        assert!(
            buffer[(3, 0)]
                .modifier
                .contains(ratatui::style::Modifier::REVERSED)
        );
    }

    #[test]
    fn focus_does_not_move_centered_label() {
        let mut terminal = Terminal::new(TestBackend::new(12, 1)).unwrap();
        terminal
            .draw(|frame| render_button(frame, frame.area(), &Button::new("Save"), &theme()))
            .unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert_eq!(rendered, "     Save   ");
    }

    #[test]
    fn width_distribution_is_even_and_detects_overflow() {
        assert_eq!(horizontal_button_widths(10, &[2, 3]), Some(vec![5, 5]));
        assert_eq!(horizontal_button_widths(4, &[2, 3]), None);
        assert_eq!(horizontal_button_widths(10, &[]), Some(Vec::new()));
        assert_eq!(horizontal_button_widths(u16::MAX, &[u16::MAX, 1]), None);
        assert_eq!(button_minimum_width("界", &theme()), 4);
    }

    #[test]
    fn cancel_button_has_independent_semantic_style() {
        let mut theme = theme();
        theme.buttons.cancel = Style::default().fg(ratatui::style::Color::Red);
        let mut terminal = Terminal::new(TestBackend::new(12, 1)).unwrap();
        terminal
            .draw(|frame| {
                render_button(
                    frame,
                    frame.area(),
                    &Button::new("Cancel").kind(ButtonKind::Cancel),
                    &theme,
                );
            })
            .unwrap();

        assert_eq!(
            terminal.backend().buffer()[(3, 0)].fg,
            ratatui::style::Color::Red
        );
    }
}
