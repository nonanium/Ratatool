use ratatui::{Frame, layout::Rect};

use crate::{
    terminal::TerminalProfile,
    theme::ResolvedTheme,
    widgets::{
        Button, Dialog, MenuRenderOptions, MenuState, NotificationOverlay, Panel, TextInputConfig,
        TextInputState, render_button, render_dialog, render_menu_with_options,
        render_notifications, render_panel, render_text_input,
    },
};

use super::HitMap;

pub struct UiFrame<'a, 'terminal, M> {
    frame: &'a mut Frame<'terminal>,
    theme: &'a ResolvedTheme,
    profile: TerminalProfile,
    hit_map: &'a mut HitMap<M>,
}

impl<'a, 'terminal, M> UiFrame<'a, 'terminal, M> {
    pub fn new(
        frame: &'a mut Frame<'terminal>,
        theme: &'a ResolvedTheme,
        profile: TerminalProfile,
        hit_map: &'a mut HitMap<M>,
    ) -> Self {
        Self {
            frame,
            theme,
            profile,
            hit_map,
        }
    }

    pub fn frame(&mut self) -> &mut Frame<'terminal> {
        self.frame
    }

    pub fn frame_and_theme(&mut self) -> (&mut Frame<'terminal>, &ResolvedTheme) {
        (self.frame, self.theme)
    }

    pub const fn theme(&self) -> &ResolvedTheme {
        self.theme
    }

    pub const fn profile(&self) -> TerminalProfile {
        self.profile
    }

    pub fn hit_map(&mut self) -> &mut HitMap<M> {
        self.hit_map
    }

    pub fn button(&mut self, area: Rect, button: &Button<'_>, action: Option<M>) {
        render_button(self.frame, area, button, self.theme);
        if button.enabled
            && let Some(action) = action
        {
            self.hit_map.register(area, action);
        }
    }

    pub fn panel(&mut self, area: Rect, panel: &Panel<'_>) -> Rect {
        render_panel(self.frame, area, panel, self.theme)
    }

    pub fn menu<T>(&mut self, area: Rect, state: &MenuState<T>, options: MenuRenderOptions) {
        render_menu_with_options(self.frame, area, state, options, self.theme);
    }

    pub fn text_input(&mut self, area: Rect, state: &TextInputState, config: &TextInputConfig<'_>) {
        render_text_input(self.frame, area, state, config, self.theme);
    }

    pub fn notifications(&mut self, area: Rect, overlay: &NotificationOverlay) {
        render_notifications(self.frame, area, overlay, self.theme);
    }
}

impl<M: Clone> UiFrame<'_, '_, M> {
    pub fn dialog(&mut self, area: Rect, dialog: &Dialog<'_, M>) -> Rect {
        render_dialog(self.frame, area, dialog, self.theme, self.hit_map)
    }
}
