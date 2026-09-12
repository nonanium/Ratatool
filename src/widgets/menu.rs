use std::borrow::Cow;

use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthStr;

use crate::theme::ResolvedTheme;

use super::navigation::adjacent_index;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem<T> {
    pub label: String,
    pub description: Option<String>,
    pub value: T,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

/// Selection state for a vertically rendered menu.
///
/// The default key handler consumes Up, Down, Home, End, and Enter. Navigation
/// skips disabled items and wraps at the ends by default. Rendering uses the
/// semantic selection styles and focus marker, registers no mouse regions, and
/// needs at least one row. Extra items are clipped when the area is too short.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuState<T> {
    items: Vec<MenuItem<T>>,
    selected: Option<usize>,
    viewport: usize,
    wrap_navigation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuRenderOptions {
    pub focused: bool,
}

impl Default for MenuRenderOptions {
    fn default() -> Self {
        Self { focused: true }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuError {
    UnknownIndex(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuResult<T> {
    Unhandled,
    Moved,
    Unchanged,
    Activated(T),
    NoSelection,
}

impl<T> MenuState<T> {
    pub fn new(items: Vec<MenuItem<T>>) -> Self {
        let selected = items.iter().position(|item| item.enabled);
        Self {
            items,
            selected,
            viewport: 0,
            wrap_navigation: true,
        }
    }

    pub fn items(&self) -> &[MenuItem<T>] {
        &self.items
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected_item(&self) -> Option<&MenuItem<T>> {
        self.selected.and_then(|index| self.items.get(index))
    }

    pub const fn viewport(&self) -> usize {
        self.viewport
    }

    pub fn set_viewport(&mut self, viewport: usize) {
        self.viewport = viewport.min(self.items.len().saturating_sub(1));
    }

    pub fn ensure_selected_visible(&mut self, viewport_height: u16) {
        let height = usize::from(viewport_height);
        let Some(selected) = self.selected else {
            self.viewport = 0;
            return;
        };
        if height == 0 {
            return;
        }
        if selected < self.viewport {
            self.viewport = selected;
        } else if selected >= self.viewport.saturating_add(height) {
            self.viewport = selected + 1 - height;
        }
    }

    pub fn set_wrap_navigation(&mut self, wrap: bool) {
        self.wrap_navigation = wrap;
    }

    pub fn move_next(&mut self) -> bool {
        self.move_focus(true)
    }

    pub fn move_previous(&mut self) -> bool {
        self.move_focus(false)
    }

    pub fn first(&mut self) -> bool {
        self.set_selected_index(self.items.iter().position(|item| item.enabled))
    }

    pub fn last(&mut self) -> bool {
        self.set_selected_index(self.items.iter().rposition(|item| item.enabled))
    }

    pub fn set_enabled(&mut self, index: usize, enabled: bool) -> Result<(), MenuError> {
        let Some(item) = self.items.get_mut(index) else {
            return Err(MenuError::UnknownIndex(index));
        };
        item.enabled = enabled;
        if self.selected == Some(index) && !enabled {
            self.selected = self.items.iter().position(|item| item.enabled);
        } else if self.selected.is_none() && enabled {
            self.selected = Some(index);
        }
        Ok(())
    }

    fn move_focus(&mut self, forward: bool) -> bool {
        let Some(current) = self.selected else {
            return self.set_selected_index(self.items.iter().position(|item| item.enabled));
        };
        let target = adjacent_index(
            current,
            self.items.len(),
            forward,
            self.wrap_navigation,
            |index| self.items[index].enabled,
        );
        self.set_selected_index(Some(target))
    }

    fn set_selected_index(&mut self, selected: Option<usize>) -> bool {
        if self.selected == selected {
            false
        } else {
            self.selected = selected;
            true
        }
    }
}

impl<T: Clone> MenuState<T> {
    pub fn handle_key(&mut self, key: KeyCode) -> MenuResult<T> {
        match key {
            KeyCode::Up => {
                if self.move_previous() {
                    MenuResult::Moved
                } else {
                    MenuResult::Unchanged
                }
            }
            KeyCode::Down => {
                if self.move_next() {
                    MenuResult::Moved
                } else {
                    MenuResult::Unchanged
                }
            }
            KeyCode::Home => {
                if self.first() {
                    MenuResult::Moved
                } else {
                    MenuResult::Unchanged
                }
            }
            KeyCode::End => {
                if self.last() {
                    MenuResult::Moved
                } else {
                    MenuResult::Unchanged
                }
            }
            KeyCode::Enter => self
                .selected_item()
                .map(|item| MenuResult::Activated(item.value.clone()))
                .unwrap_or(MenuResult::NoSelection),
            _ => MenuResult::Unhandled,
        }
    }

    /// Handles navigation and keeps the selected item inside the given viewport.
    pub fn handle_key_in_viewport(&mut self, key: KeyCode, viewport_height: u16) -> MenuResult<T> {
        let result = self.handle_key(key);
        self.ensure_selected_visible(viewport_height);
        result
    }
}

pub fn render_menu<T>(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &MenuState<T>,
    theme: &ResolvedTheme,
) {
    render_menu_with_options(frame, area, state, MenuRenderOptions::default(), theme);
}

pub fn render_menu_with_options<T>(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &MenuState<T>,
    options: MenuRenderOptions,
    theme: &ResolvedTheme,
) {
    let lines = state
        .items
        .iter()
        .enumerate()
        .skip(state.viewport)
        .take(usize::from(area.height))
        .map(|(index, item)| {
            let selected = state.selected == Some(index);
            let style = if !item.enabled {
                theme.selection.disabled
            } else if selected && options.focused {
                theme.selection.focused
            } else if selected {
                theme.selection.selected
            } else {
                theme.selection.normal
            };
            let marker: Cow<'_, str> = if selected && options.focused {
                Cow::Borrowed(theme.symbols.focus_marker)
            } else {
                Cow::Owned(" ".repeat(UnicodeWidthStr::width(theme.symbols.focus_marker)))
            };
            let mut spans = vec![
                Span::styled(marker, style),
                Span::styled(" ", style),
                Span::styled(item.label.as_str(), style),
            ];
            if let Some(description) = item.description.as_deref() {
                spans.push(Span::styled("  ", style));
                spans.push(Span::styled(description, style));
            }
            Line::from(spans)
        });
    frame.render_widget(Paragraph::new(lines.collect::<Vec<_>>()), area);
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::{ResolvedTheme, Symbols, ThemeVariant};

    fn item(value: u8, enabled: bool) -> MenuItem<u8> {
        MenuItem {
            label: format!("Item {value}"),
            description: None,
            value,
            enabled,
            disabled_reason: None,
        }
    }

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
    fn navigation_skips_disabled_items_and_activation_returns_value() {
        let mut menu = MenuState::new(vec![item(1, true), item(2, false), item(3, true)]);
        assert_eq!(menu.handle_key(KeyCode::Down), MenuResult::Moved);
        assert_eq!(menu.selected_index(), Some(2));
        assert_eq!(menu.handle_key(KeyCode::Enter), MenuResult::Activated(3));
    }

    #[test]
    fn enabling_first_available_item_restores_selection() {
        let mut menu = MenuState::new(vec![item(1, false)]);
        assert_eq!(menu.handle_key(KeyCode::Enter), MenuResult::NoSelection);
        menu.set_enabled(0, true).unwrap();
        assert_eq!(menu.selected_index(), Some(0));
    }

    #[test]
    fn rendering_uses_the_resolved_focus_marker() {
        let menu = MenuState::new(vec![item(1, true), item(2, true)]);
        let mut terminal = Terminal::new(TestBackend::new(10, 2)).unwrap();
        terminal
            .draw(|frame| render_menu(frame, frame.area(), &menu, &theme()))
            .unwrap();
        let rows: Vec<String> = terminal
            .backend()
            .buffer()
            .content
            .chunks(10)
            .map(|row| row.iter().map(|cell| cell.symbol()).collect())
            .collect();
        assert_eq!(rows, ["> Item 1  ", "  Item 2  "]);
    }

    #[test]
    fn edge_navigation_and_viewport_report_accurately() {
        let mut menu = MenuState::new(vec![item(1, true), item(2, true), item(3, true)]);
        menu.set_wrap_navigation(false);
        assert_eq!(menu.handle_key(KeyCode::Up), MenuResult::Unchanged);
        menu.handle_key(KeyCode::End);
        menu.ensure_selected_visible(2);
        assert_eq!(menu.viewport(), 1);
        assert_eq!(menu.set_enabled(9, true), Err(MenuError::UnknownIndex(9)));
    }

    #[test]
    fn viewport_aware_key_handling_keeps_selection_visible() {
        let mut menu = MenuState::new(vec![item(1, true), item(2, true), item(3, true)]);

        assert_eq!(
            menu.handle_key_in_viewport(KeyCode::End, 2),
            MenuResult::Moved
        );
        assert_eq!(menu.selected_index(), Some(2));
        assert_eq!(menu.viewport(), 1);
    }
}
