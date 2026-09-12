use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Clear, Paragraph, Wrap},
};

use crate::{
    framework::{HitMap, Interaction},
    layout::{RequiredSize, centered_rect, hard_wrapped_line_count},
    theme::ResolvedTheme,
};

use super::{
    Button, Panel, PanelKind, button_minimum_width, horizontal_button_widths, render_button,
    render_panel,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialogButton<'a, M> {
    pub button: Button<'a>,
    pub action: M,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dialog<'a, M> {
    pub title: Cow<'a, str>,
    pub body: Cow<'a, str>,
    pub buttons: Vec<DialogButton<'a, M>>,
    pub focused_button: Option<usize>,
    pub cancel_action: Option<M>,
    pub maximum_size: RequiredSize,
}

impl<'a, M> Dialog<'a, M> {
    pub fn new(title: impl Into<Cow<'a, str>>, body: impl Into<Cow<'a, str>>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            buttons: Vec::new(),
            focused_button: None,
            cancel_action: None,
            maximum_size: RequiredSize {
                width: 72,
                height: 24,
            },
        }
    }

    pub fn push_button(&mut self, button: Button<'a>, action: M) {
        let index = self.buttons.len();
        if self.focused_button.is_none() && button.enabled {
            self.focused_button = Some(index);
        }
        self.buttons.push(DialogButton { button, action });
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Interaction<M>
    where
        M: Clone,
    {
        match key.code {
            KeyCode::Esc => self
                .cancel_action
                .clone()
                .map_or_else(Interaction::unchanged, Interaction::message),
            KeyCode::Enter => self
                .focused_button
                .and_then(|index| self.buttons.get(index))
                .filter(|button| button.button.enabled)
                .map(|button| Interaction::message(button.action.clone()))
                .unwrap_or_else(Interaction::unchanged),
            KeyCode::Tab | KeyCode::Right => {
                let changed = self.move_focus(true);
                Interaction::Handled {
                    changed,
                    message: None,
                }
            }
            KeyCode::BackTab | KeyCode::Left => {
                let changed = self.move_focus(false);
                Interaction::Handled {
                    changed,
                    message: None,
                }
            }
            _ => Interaction::Unhandled,
        }
    }

    fn move_focus(&mut self, forward: bool) -> bool {
        let enabled: Vec<_> = self
            .buttons
            .iter()
            .enumerate()
            .filter_map(|(index, button)| button.button.enabled.then_some(index))
            .collect();
        if enabled.is_empty() {
            return false;
        }
        let position = self
            .focused_button
            .and_then(|focused| enabled.iter().position(|index| *index == focused))
            .unwrap_or(0);
        let next = if forward {
            (position + 1) % enabled.len()
        } else if position == 0 {
            enabled.len() - 1
        } else {
            position - 1
        };
        let target = Some(enabled[next]);
        let changed = self.focused_button != target;
        self.focused_button = target;
        changed
    }
}

pub fn render_dialog<M: Clone>(
    frame: &mut Frame<'_>,
    available: Rect,
    dialog: &Dialog<'_, M>,
    theme: &ResolvedTheme,
    hit_map: &mut HitMap<M>,
) -> Rect {
    let width = available.width.min(dialog.maximum_size.width).max(1);
    let body_width = width.saturating_sub(4);
    let body_height = hard_wrapped_line_count(&dialog.body, body_width);
    let minimums: Vec<_> = dialog
        .buttons
        .iter()
        .map(|entry| button_minimum_width(&entry.button.label, theme))
        .collect();
    let button_rows = if dialog.buttons.is_empty() {
        0
    } else if horizontal_button_widths(body_width, &minimums).is_some() {
        1
    } else {
        u16::try_from(dialog.buttons.len()).unwrap_or(u16::MAX)
    };
    let desired_height = body_height
        .saturating_add(button_rows)
        .saturating_add(4)
        .min(dialog.maximum_size.height)
        .max(3);
    let area = centered_rect(
        available,
        RequiredSize {
            width,
            height: desired_height,
        },
    );
    frame.render_widget(Clear, area);
    let inner = render_panel(
        frame,
        area,
        &Panel::new()
            .title(dialog.title.as_ref())
            .padding(1)
            .kind(PanelKind::Overlay),
        theme,
    );
    let button_height = button_rows.min(inner.height);
    let body_area = Rect::new(
        inner.x,
        inner.y,
        inner.width,
        inner.height.saturating_sub(button_height),
    );
    frame.render_widget(
        Paragraph::new(Line::from(dialog.body.as_ref()))
            .style(theme.text.normal)
            .wrap(Wrap { trim: true }),
        body_area,
    );

    if button_height == 0 {
        return area;
    }
    if let Some(widths) = horizontal_button_widths(inner.width, &minimums) {
        let mut x = inner.x;
        for (index, (entry, width)) in dialog.buttons.iter().zip(widths).enumerate() {
            let button_area = Rect::new(x, inner.bottom().saturating_sub(1), width, 1);
            render_dialog_button(frame, button_area, dialog, index, entry, theme, hit_map);
            x = x.saturating_add(width);
        }
    } else {
        let first = dialog
            .buttons
            .len()
            .saturating_sub(usize::from(button_height));
        for (row, (index, entry)) in dialog.buttons.iter().enumerate().skip(first).enumerate() {
            let button_area = Rect::new(
                inner.x,
                inner
                    .bottom()
                    .saturating_sub(button_height)
                    .saturating_add(u16::try_from(row).unwrap_or(u16::MAX)),
                inner.width,
                1,
            );
            render_dialog_button(frame, button_area, dialog, index, entry, theme, hit_map);
        }
    }
    area
}

fn render_dialog_button<M: Clone>(
    frame: &mut Frame<'_>,
    button_area: Rect,
    dialog: &Dialog<'_, M>,
    index: usize,
    entry: &DialogButton<'_, M>,
    theme: &ResolvedTheme,
    hit_map: &mut HitMap<M>,
) {
    let mut button = entry.button.clone();
    button.focused = dialog.focused_button == Some(index);
    render_button(frame, button_area, &button, theme);
    if button.enabled {
        hit_map.register(button_area, entry.action.clone());
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;
    use ratatui::{Terminal, backend::TestBackend};

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
    fn keyboard_and_mouse_actions_share_dialog_buttons() {
        let mut dialog = Dialog::new("Confirm", "Continue?");
        dialog.push_button(Button::new("Yes"), 1);
        dialog.push_button(Button::new("No"), 2);
        dialog.cancel_action = Some(2);
        assert_eq!(
            dialog.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Interaction::message(1)
        );
        dialog.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(
            dialog.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Interaction::message(2)
        );

        let mut hits = HitMap::new();
        let mut terminal = Terminal::new(TestBackend::new(30, 8)).unwrap();
        terminal
            .draw(|frame| {
                render_dialog(frame, frame.area(), &dialog, &theme(), &mut hits);
            })
            .unwrap();
        assert_eq!(hits.regions().len(), 2);
    }

    #[test]
    fn narrow_dialog_stacks_buttons_vertically() {
        let mut dialog = Dialog::new("Choose", "Pick one");
        dialog.push_button(Button::new("Cancel"), 1);
        dialog.push_button(Button::new("Confirm"), 2);
        let mut hits = HitMap::new();
        let mut terminal = Terminal::new(TestBackend::new(12, 8)).unwrap();

        terminal
            .draw(|frame| {
                render_dialog(frame, frame.area(), &dialog, &theme(), &mut hits);
            })
            .unwrap();

        assert_eq!(hits.regions().len(), 2);
        assert_ne!(hits.regions()[0].area.y, hits.regions()[1].area.y);
    }
}
