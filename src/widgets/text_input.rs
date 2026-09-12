use std::{borrow::Cow, ops::Range};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Position, Rect},
    text::{Line, Span},
    widgets::Paragraph,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::theme::ResolvedTheme;

/// Editable, single-line UTF-8 text and selection state.
///
/// The default key handler consumes printable characters, Backspace, Delete,
/// Left, Right, Home, End, and Ctrl+A. Shift extends a selection. Rendering
/// uses the semantic input styles and password symbol, registers no mouse
/// regions, and needs one row. Long values scroll horizontally to keep the
/// cursor visible; Ratatui clips the remaining content.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextInputState {
    value: String,
    cursor: usize,
    selection_anchor: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextInputConfig<'a> {
    pub placeholder: Option<Cow<'a, str>>,
    pub secret: bool,
    pub focused: bool,
}

impl<'a> TextInputConfig<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn placeholder(mut self, placeholder: impl Into<Cow<'a, str>>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn secret(mut self, secret: bool) -> Self {
        self.secret = secret;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputResult {
    Unhandled,
    Changed,
    Moved,
    Unchanged,
}

impl TextInputState {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.len();
        Self {
            value,
            cursor,
            selection_anchor: None,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the cursor as a UTF-8 byte index into `value`.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn selection(&self) -> Option<Range<usize>> {
        let anchor = self.selection_anchor?;
        (anchor != self.cursor).then(|| anchor.min(self.cursor)..anchor.max(self.cursor))
    }

    pub fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.cursor = self.value.len();
        self.selection_anchor = None;
    }

    pub fn select_all(&mut self) {
        self.selection_anchor = (!self.value.is_empty()).then_some(0);
        self.cursor = self.value.len();
    }

    pub fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    /// Inserts text at the cursor, replacing the current selection.
    pub fn insert_text(&mut self, text: &str) -> TextInputResult {
        if text.is_empty() {
            return TextInputResult::Unchanged;
        }
        self.delete_selection();
        self.value.insert_str(self.cursor, text);
        self.cursor += text.len();
        TextInputResult::Changed
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> TextInputResult {
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        match key.code {
            KeyCode::Char(character)
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && character.eq_ignore_ascii_case(&'a') =>
            {
                let before = self.selection();
                self.select_all();
                if self.selection() == before {
                    TextInputResult::Unchanged
                } else {
                    TextInputResult::Moved
                }
            }
            KeyCode::Char(character)
                if !key.modifiers.intersects(
                    KeyModifiers::CONTROL
                        | KeyModifiers::ALT
                        | KeyModifiers::SUPER
                        | KeyModifiers::HYPER
                        | KeyModifiers::META,
                ) =>
            {
                let mut encoded = [0; 4];
                self.insert_text(character.encode_utf8(&mut encoded))
            }
            KeyCode::Backspace => {
                if self.delete_selection() {
                    return TextInputResult::Changed;
                }
                let previous = previous_boundary(&self.value, self.cursor);
                if previous == self.cursor {
                    TextInputResult::Unchanged
                } else {
                    self.value.drain(previous..self.cursor);
                    self.cursor = previous;
                    TextInputResult::Changed
                }
            }
            KeyCode::Delete => {
                if self.delete_selection() {
                    return TextInputResult::Changed;
                }
                let next = next_boundary(&self.value, self.cursor);
                if next == self.cursor {
                    TextInputResult::Unchanged
                } else {
                    self.value.drain(self.cursor..next);
                    TextInputResult::Changed
                }
            }
            KeyCode::Left => {
                let target = if !shift {
                    self.selection().map_or_else(
                        || previous_boundary(&self.value, self.cursor),
                        |range| range.start,
                    )
                } else {
                    previous_boundary(&self.value, self.cursor)
                };
                self.move_result(target, shift)
            }
            KeyCode::Right => {
                let target = if !shift {
                    self.selection().map_or_else(
                        || next_boundary(&self.value, self.cursor),
                        |range| range.end,
                    )
                } else {
                    next_boundary(&self.value, self.cursor)
                };
                self.move_result(target, shift)
            }
            KeyCode::Home => self.move_result(0, shift),
            KeyCode::End => self.move_result(self.value.len(), shift),
            _ => TextInputResult::Unhandled,
        }
    }

    fn move_cursor(&mut self, target: usize, extend_selection: bool) {
        if extend_selection {
            self.selection_anchor.get_or_insert(self.cursor);
        } else {
            self.selection_anchor = None;
        }
        self.cursor = target;
    }

    fn move_result(&mut self, target: usize, extend_selection: bool) -> TextInputResult {
        let before_cursor = self.cursor;
        let before_selection = self.selection();
        self.move_cursor(target, extend_selection);
        if self.cursor == before_cursor && self.selection() == before_selection {
            TextInputResult::Unchanged
        } else {
            TextInputResult::Moved
        }
    }

    fn delete_selection(&mut self) -> bool {
        let Some(selection) = self.selection() else {
            return false;
        };
        self.cursor = selection.start;
        self.value.drain(selection);
        self.selection_anchor = None;
        true
    }
}

fn previous_boundary(value: &str, cursor: usize) -> usize {
    value[..cursor]
        .grapheme_indices(true)
        .next_back()
        .map_or(cursor, |(index, _)| index)
}

fn next_boundary(value: &str, cursor: usize) -> usize {
    value[cursor..]
        .grapheme_indices(true)
        .nth(1)
        .map_or(value.len(), |(index, _)| cursor + index)
}

pub fn render_text_input(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &TextInputState,
    config: &TextInputConfig<'_>,
    theme: &ResolvedTheme,
) {
    if area.is_empty() {
        return;
    }

    if state.value.is_empty() {
        let placeholder = config.placeholder.as_deref().unwrap_or_default();
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                placeholder,
                theme.input.placeholder,
            ))),
            area,
        );
        if config.focused {
            frame.set_cursor_position(Position::new(area.x, area.y));
        }
        return;
    }

    let base_style = if config.focused {
        theme.input.focused
    } else {
        theme.input.normal
    };
    let selection = state.selection();
    let mut spans = Vec::with_capacity(state.value.graphemes(true).count());
    for (start, grapheme) in state.value.grapheme_indices(true) {
        let end = start + grapheme.len();
        let selected = selection
            .as_ref()
            .is_some_and(|range| range.start <= start && end <= range.end);
        let content: Cow<'_, str> = if config.secret {
            Cow::Borrowed(theme.symbols.password_mask)
        } else {
            Cow::Borrowed(grapheme)
        };
        spans.push(Span::styled(
            content,
            if selected {
                base_style.patch(theme.input.selection)
            } else {
                base_style
            },
        ));
    }

    let cursor_width = display_width_before_cursor(state, config, theme);
    let horizontal_offset = cursor_width.saturating_sub(usize::from(area.width).saturating_sub(1));
    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .scroll((0, horizontal_offset.min(usize::from(u16::MAX)) as u16)),
        area,
    );
    if config.focused {
        let cursor_x = cursor_width.saturating_sub(horizontal_offset);
        frame.set_cursor_position(Position::new(
            area.x
                .saturating_add(cursor_x.min(usize::from(area.width - 1)) as u16),
            area.y,
        ));
    }
}

fn display_width_before_cursor(
    state: &TextInputState,
    config: &TextInputConfig<'_>,
    theme: &ResolvedTheme,
) -> usize {
    if config.secret {
        state.value[..state.cursor].graphemes(true).count()
            * UnicodeWidthStr::width(theme.symbols.password_mask)
    } else {
        UnicodeWidthStr::width(&state.value[..state.cursor])
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{
        Terminal,
        backend::{Backend, TestBackend},
        style::Style,
    };

    use super::*;
    use crate::{InputTheme, ResolvedTheme, Symbols, ThemeVariant};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn theme() -> ResolvedTheme {
        let variant = ThemeVariant::default();
        ResolvedTheme {
            text: variant.text,
            borders: variant.borders,
            surfaces: variant.surfaces,
            buttons: variant.buttons,
            status: variant.status,
            input: InputTheme {
                selection: Style::default().reversed(),
                ..Default::default()
            },
            selection: variant.selection,
            metrics: variant.metrics,
            symbols: Symbols::ascii(),
        }
    }

    #[test]
    fn editing_respects_utf8_boundaries() {
        let mut state = TextInputState::new("a界");
        assert_eq!(state.handle_key(key(KeyCode::Left)), TextInputResult::Moved);
        assert_eq!(state.cursor(), 1);
        assert_eq!(
            state.handle_key(key(KeyCode::Delete)),
            TextInputResult::Changed
        );
        assert_eq!(state.value(), "a");
        assert_eq!(
            state.handle_key(key(KeyCode::Backspace)),
            TextInputResult::Changed
        );
        assert_eq!(state.value(), "");
    }

    #[test]
    fn editing_uses_grapheme_boundaries_and_reports_noops() {
        let mut state = TextInputState::new("e\u{301}👨‍👩‍👧‍👦");
        assert_eq!(state.handle_key(key(KeyCode::Left)), TextInputResult::Moved);
        assert_eq!(state.cursor(), "e\u{301}".len());
        assert_eq!(
            state.handle_key(key(KeyCode::Backspace)),
            TextInputResult::Changed
        );
        assert_eq!(state.value(), "👨‍👩‍👧‍👦");
        assert_eq!(
            state.handle_key(key(KeyCode::Home)),
            TextInputResult::Unchanged
        );
        assert_eq!(
            state.handle_key(key(KeyCode::Backspace)),
            TextInputResult::Unchanged
        );
    }

    #[test]
    fn arrows_collapse_selection_to_the_nearest_edge() {
        let mut state = TextInputState::new("alpha");
        state.select_all();
        assert_eq!(state.handle_key(key(KeyCode::Left)), TextInputResult::Moved);
        assert_eq!(state.cursor(), 0);

        state.select_all();
        assert_eq!(
            state.handle_key(key(KeyCode::Right)),
            TextInputResult::Moved
        );
        assert_eq!(state.cursor(), state.value().len());
    }

    #[test]
    fn selection_is_replaced_by_typed_text() {
        let mut state = TextInputState::new("alpha");
        state.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
        assert_eq!(state.selection(), Some(0..5));
        state.handle_key(key(KeyCode::Char('x')));
        assert_eq!(state.value(), "x");
        assert_eq!(state.selection(), None);
    }

    #[test]
    fn paste_insertion_replaces_selection_and_moves_cursor() {
        let mut state = TextInputState::new("alpha");
        state.select_all();

        assert_eq!(state.insert_text("界 beta"), TextInputResult::Changed);
        assert_eq!(state.value(), "界 beta");
        assert_eq!(state.cursor(), "界 beta".len());
        assert_eq!(state.selection(), None);
        assert_eq!(state.insert_text(""), TextInputResult::Unchanged);
    }

    #[test]
    fn secret_rendering_uses_resolved_mask_and_keeps_cursor_visible() {
        let state = TextInputState::new("secret");
        let mut terminal = Terminal::new(TestBackend::new(4, 1)).unwrap();
        terminal
            .draw(|frame| {
                render_text_input(
                    frame,
                    frame.area(),
                    &state,
                    &TextInputConfig::new().secret(true).focused(true),
                    &theme(),
                );
            })
            .unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert_eq!(rendered, "*** ");
        assert_eq!(
            terminal.backend_mut().get_cursor_position().unwrap(),
            Position::new(3, 0)
        );
    }
}
