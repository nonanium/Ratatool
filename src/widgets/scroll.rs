use std::ops::Range;

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::theme::ResolvedTheme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollOptions {
    pub show_scrollbar: bool,
    pub focused: bool,
}

impl Default for ScrollOptions {
    fn default() -> Self {
        Self {
            show_scrollbar: true,
            focused: false,
        }
    }
}

/// Vertical viewport state for long content.
///
/// The default key handler consumes Up, Down, PageUp, PageDown, Home, and End.
/// Rendering uses the semantic border styles and resolved scroll symbols,
/// registers no mouse regions, and needs at least one row. Content and the
/// scrollbar are clipped to the supplied area.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScrollState {
    offset: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollResult {
    Unhandled,
    Changed,
    Unchanged,
}

impl ScrollState {
    pub fn offset(&self) -> u16 {
        self.offset
    }

    pub fn set_offset(&mut self, offset: u16, content_height: u16, viewport_height: u16) {
        self.offset = offset.min(max_offset(content_height, viewport_height));
    }

    pub fn update_geometry(&mut self, content_height: u16, viewport_height: u16) -> ScrollResult {
        self.update(self.offset.min(max_offset(content_height, viewport_height)))
    }

    pub fn visible_range(&self, content_height: u16, viewport_height: u16) -> Range<u16> {
        let start = self.offset.min(max_offset(content_height, viewport_height));
        start..start.saturating_add(viewport_height).min(content_height)
    }

    pub fn up(&mut self, amount: u16) -> ScrollResult {
        self.update(self.offset.saturating_sub(amount))
    }

    pub fn down(&mut self, amount: u16, content_height: u16, viewport_height: u16) -> ScrollResult {
        self.update(
            self.offset
                .saturating_add(amount)
                .min(max_offset(content_height, viewport_height)),
        )
    }

    pub fn handle_key(
        &mut self,
        key: KeyCode,
        content_height: u16,
        viewport_height: u16,
    ) -> ScrollResult {
        match key {
            KeyCode::Up => self.up(1),
            KeyCode::Down => self.down(1, content_height, viewport_height),
            KeyCode::PageUp => self.up(viewport_height.max(1)),
            KeyCode::PageDown => self.down(viewport_height.max(1), content_height, viewport_height),
            KeyCode::Home => self.update(0),
            KeyCode::End => self.update(max_offset(content_height, viewport_height)),
            _ => ScrollResult::Unhandled,
        }
    }

    fn update(&mut self, offset: u16) -> ScrollResult {
        if self.offset == offset {
            ScrollResult::Unchanged
        } else {
            self.offset = offset;
            ScrollResult::Changed
        }
    }
}

fn max_offset(content_height: u16, viewport_height: u16) -> u16 {
    content_height.saturating_sub(viewport_height)
}

pub fn render_scroll(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &ScrollState,
    options: ScrollOptions,
    content: Paragraph<'_>,
    content_height: u16,
    theme: &ResolvedTheme,
) {
    let offset = state.offset.min(max_offset(content_height, area.height));
    frame.render_widget(content.scroll((offset, 0)), area);
    if !options.show_scrollbar || content_height <= area.height || area.is_empty() {
        return;
    }

    render_scrollbar(frame, area, state, options.focused, content_height, theme);
}

pub fn render_scrollbar(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &ScrollState,
    focused: bool,
    content_height: u16,
    theme: &ResolvedTheme,
) {
    if area.is_empty() || content_height <= area.height {
        return;
    }
    let symbols = theme.symbols.scroll;
    let thumb_style = if focused {
        theme.borders.focused
    } else {
        theme.borders.normal
    };
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some(symbols.up))
        .end_symbol(Some(symbols.down))
        .track_symbol(Some(symbols.track))
        .thumb_symbol(symbols.thumb)
        .begin_style(theme.borders.normal)
        .end_style(theme.borders.normal)
        .track_style(theme.borders.normal)
        .thumb_style(thumb_style);
    let mut scrollbar_state = ScrollbarState::new(content_height as usize)
        .position(state.offset.min(max_offset(content_height, area.height)) as usize)
        .viewport_content_length(area.height as usize);
    frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, text::Line};

    use super::*;
    use crate::{ResolvedTheme, Symbols, ThemeVariant};

    fn theme() -> ResolvedTheme {
        let variant = ThemeVariant::default();
        ResolvedTheme {
            text: variant.text,
            borders: variant.borders,
            surfaces: variant.surfaces,
            buttons: variant.buttons,
            status: variant.status,
            input: variant.input,
            selection: variant.selection,
            metrics: variant.metrics,
            symbols: Symbols::ascii(),
        }
    }

    #[test]
    fn navigation_clamps_and_reports_changes() {
        let mut state = ScrollState::default();
        assert_eq!(state.down(10, 8, 3), ScrollResult::Changed);
        assert_eq!(state.offset(), 5);
        assert_eq!(state.down(1, 8, 3), ScrollResult::Unchanged);
        assert_eq!(state.handle_key(KeyCode::Home, 8, 3), ScrollResult::Changed);
        assert_eq!(state.offset(), 0);
        state.set_offset(5, 8, 3);
        assert_eq!(state.visible_range(8, 3), 5..8);
        assert_eq!(state.update_geometry(2, 3), ScrollResult::Changed);
        assert_eq!(state.offset(), 0);
    }

    #[test]
    fn rendering_uses_ascii_scroll_symbols() {
        let state = ScrollState::default();
        let mut terminal = Terminal::new(TestBackend::new(6, 4)).unwrap();
        terminal
            .draw(|frame| {
                render_scroll(
                    frame,
                    frame.area(),
                    &state,
                    ScrollOptions::default(),
                    Paragraph::new(vec![
                        Line::from("one"),
                        Line::from("two"),
                        Line::from("three"),
                        Line::from("four"),
                        Line::from("five"),
                    ]),
                    5,
                    &theme(),
                );
            })
            .unwrap();
        let right: String = (0..4)
            .map(|y| terminal.backend().buffer()[(5, y)].symbol())
            .collect();
        assert!(right.starts_with('^'));
        assert!(right.ends_with('v'));
    }
}
