use std::borrow::Cow;

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::theme::ResolvedTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoiceKind {
    Checkbox,
    Radio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceVisualState {
    pub selected: bool,
    pub focused: bool,
    pub enabled: bool,
}

/// Renders one stateless checkbox or radio row.
///
/// The caller owns selection and focus. The function consumes no keys and
/// registers no mouse regions. It uses the theme's selection styles and choice
/// symbols. Its minimum useful size is one row and the marker width plus one
/// column; Ratatui clips the label when space is insufficient.
pub fn render_choice_line<'a>(
    label: &'a str,
    kind: ChoiceKind,
    state: ChoiceVisualState,
    theme: &ResolvedTheme,
) -> Line<'a> {
    let mut style = theme.selection.normal;
    if state.selected {
        style = style.patch(theme.selection.selected);
    }
    if state.focused {
        style = style.patch(theme.selection.focused);
    }
    if !state.enabled {
        style = style.patch(theme.selection.disabled);
    }
    let marker = match (kind, state.selected) {
        (ChoiceKind::Checkbox, false) => theme.symbols.checkbox_unchecked,
        (ChoiceKind::Checkbox, true) => theme.symbols.checkbox_checked,
        (ChoiceKind::Radio, false) => theme.symbols.radio_unchecked,
        (ChoiceKind::Radio, true) => theme.symbols.radio_checked,
    };

    let mut spans = Vec::with_capacity(5);
    let focus_marker: Cow<'_, str> = if state.focused {
        Cow::Borrowed(theme.symbols.focus_marker)
    } else {
        Cow::Owned(" ".repeat(UnicodeWidthStr::width(theme.symbols.focus_marker)))
    };
    spans.push(Span::styled(focus_marker, style));
    spans.push(Span::styled(" ", style));
    spans.push(Span::styled(marker, style));
    spans.push(Span::styled(" ", style));
    spans.push(Span::styled(label, style));
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use ratatui::style::{Modifier, Style};

    use super::*;
    use crate::{ResolvedTheme, SelectionTheme, Symbols, ThemeVariant};

    fn theme(symbols: Symbols) -> ResolvedTheme {
        let variant = ThemeVariant::default();
        ResolvedTheme {
            text: variant.text,
            borders: variant.borders,
            surfaces: variant.surfaces,
            buttons: variant.buttons,
            status: variant.status,
            input: variant.input,
            selection: SelectionTheme {
                focused: Style::default().add_modifier(Modifier::REVERSED),
                ..Default::default()
            },
            metrics: variant.metrics,
            symbols,
        }
    }

    #[test]
    fn focused_choice_uses_resolved_ascii_symbols() {
        let line = render_choice_line(
            "Terms",
            ChoiceKind::Checkbox,
            ChoiceVisualState {
                selected: true,
                focused: true,
                enabled: true,
            },
            &theme(Symbols::ascii()),
        );
        let rendered: String = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(rendered, "> [x] Terms");
        assert!(
            line.spans
                .iter()
                .all(|span| span.style.add_modifier.contains(Modifier::REVERSED))
        );
    }

    #[test]
    fn unicode_radio_uses_theme_symbol() {
        let line = render_choice_line(
            "Mode",
            ChoiceKind::Radio,
            ChoiceVisualState {
                selected: true,
                focused: false,
                enabled: true,
            },
            &theme(Symbols::unicode()),
        );
        assert_eq!(line.spans[2].content, "◉");
    }

    #[test]
    fn selected_focused_and_disabled_styles_compose() {
        let variant = ThemeVariant::default();
        let theme = ResolvedTheme {
            text: variant.text,
            borders: variant.borders,
            surfaces: variant.surfaces,
            buttons: variant.buttons,
            status: variant.status,
            input: variant.input,
            selection: SelectionTheme {
                normal: Style::default().fg(ratatui::style::Color::White),
                selected: Style::default().bg(ratatui::style::Color::Blue),
                focused: Style::default().add_modifier(Modifier::BOLD),
                disabled: Style::default().add_modifier(Modifier::DIM),
            },
            metrics: variant.metrics,
            symbols: Symbols::ascii(),
        };
        let line = render_choice_line(
            "Unavailable selection",
            ChoiceKind::Checkbox,
            ChoiceVisualState {
                selected: true,
                focused: true,
                enabled: false,
            },
            &theme,
        );
        let style = line.spans[0].style;

        assert_eq!(style.fg, Some(ratatui::style::Color::White));
        assert_eq!(style.bg, Some(ratatui::style::Color::Blue));
        assert!(style.add_modifier.contains(Modifier::BOLD | Modifier::DIM));
    }
}
