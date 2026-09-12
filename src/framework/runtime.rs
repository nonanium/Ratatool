use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::event::{self, Event, MouseButton, MouseEventKind};
use ratatui::{Terminal, backend::Backend};

use crate::{
    terminal::TerminalProfile,
    theme::{ResolvedTheme, ThemeState},
};

use super::{HitMap, Interaction, RedrawState, Screen, ScreenStack, UiEvent, UiFrame};

pub struct Runtime<M> {
    screens: ScreenStack<M>,
    hit_map: HitMap<M>,
    redraw: RedrawState,
    running: bool,
}

impl<M> Runtime<M> {
    pub fn new(root: impl Screen<M> + 'static) -> Self {
        let mut redraw = RedrawState::default();
        redraw.request_redraw();
        Self {
            screens: ScreenStack::new(root),
            hit_map: HitMap::new(),
            redraw,
            running: true,
        }
    }

    pub fn screens(&self) -> &ScreenStack<M> {
        &self.screens
    }

    pub fn screens_mut(&mut self) -> &mut ScreenStack<M> {
        &mut self.screens
    }

    pub fn redraw(&self) -> &RedrawState {
        &self.redraw
    }

    pub fn redraw_mut(&mut self) -> &mut RedrawState {
        &mut self.redraw
    }

    pub const fn is_running(&self) -> bool {
        self.running && !self.screens.is_empty()
    }

    pub fn request_redraw(&mut self) {
        self.redraw.request_redraw();
    }

    pub fn request_redraw_at(&mut self, deadline: Instant) {
        self.redraw.request_redraw_at(deadline);
    }

    pub fn needs_redraw(&self) -> bool {
        self.redraw.is_requested()
    }

    pub fn next_redraw(&self) -> Option<Instant> {
        self.redraw.next_deadline()
    }

    pub fn action_at(&self, column: u16, row: u16) -> Option<&M> {
        self.hit_map.hit(column, row)
    }

    pub fn request_exit(&mut self) {
        self.running = false;
    }

    pub fn draw<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        theme: &ThemeState,
    ) -> Result<(), B::Error> {
        self.draw_with_theme(terminal, theme.resolved(), theme.profile())
    }

    pub fn draw_with_theme<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        theme: &ResolvedTheme,
        profile: TerminalProfile,
    ) -> Result<(), B::Error> {
        self.hit_map.clear();
        terminal.draw(|frame| {
            let area = frame.area();
            let mut ui = UiFrame::new(frame, theme, profile, &mut self.hit_map);
            self.screens.render(&mut ui, area);
        })?;
        self.redraw.take_requested();
        Ok(())
    }

    pub fn draw_if_due<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        theme: &ThemeState,
    ) -> Result<bool, B::Error> {
        self.draw_if_due_with_theme(terminal, theme.resolved(), theme.profile())
    }

    pub fn draw_if_due_with_theme<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        theme: &ResolvedTheme,
        profile: TerminalProfile,
    ) -> Result<bool, B::Error> {
        if !self.redraw.take_requested() {
            return Ok(false);
        }
        if let Err(error) = self.draw_with_theme(terminal, theme, profile) {
            self.redraw.request_redraw();
            return Err(error);
        }
        Ok(true)
    }
}

impl<M: Clone> Runtime<M> {
    pub fn dispatch(&mut self, event: UiEvent<M>) -> Option<M> {
        if matches!(&event, UiEvent::Tick) && self.redraw.take_due_deadlines(Instant::now()) {
            self.redraw.request_redraw();
        }
        if matches!(&event, UiEvent::Resize { .. }) {
            self.redraw.request_redraw();
        }
        let interaction = match event {
            UiEvent::Mouse(mouse) => self.dispatch_pointer(mouse, None),
            UiEvent::Pointer { event, hit } => self.dispatch_pointer(event, hit),
            event => self.screens.handle_event(&event),
        };
        match interaction {
            Interaction::Unhandled => None,
            Interaction::Handled { changed, message } => {
                if changed {
                    self.redraw.request_redraw();
                }
                message
            }
            Interaction::Exit => {
                self.running = false;
                None
            }
        }
    }

    fn dispatch_pointer(
        &mut self,
        event: crossterm::event::MouseEvent,
        supplied_hit: Option<M>,
    ) -> Interaction<M> {
        let hit = supplied_hit.or_else(|| self.hit_map.hit(event.column, event.row).cloned());
        if matches!(event.kind, MouseEventKind::Down(MouseButton::Left))
            && let Some(action) = hit
        {
            return Interaction::message(action);
        }
        self.screens.handle_event(&UiEvent::Pointer { event, hit })
    }

    pub fn run<B: Backend<Error = io::Error>, F>(
        &mut self,
        terminal: &mut Terminal<B>,
        theme: &mut ThemeState,
        mut on_message: F,
    ) -> io::Result<()>
    where
        F: FnMut(M, &mut Self, &mut ThemeState),
    {
        while self.is_running() {
            self.draw_if_due(terminal, theme)?;
            if let Some(event) = self.next_sync_event(event::poll, event::read)?
                && let Some(message) = self.dispatch(event)
            {
                on_message(message, self, theme);
            }
        }
        Ok(())
    }

    fn next_sync_event(
        &self,
        mut poll_terminal: impl FnMut(Duration) -> io::Result<bool>,
        mut read_terminal: impl FnMut() -> io::Result<Event>,
    ) -> io::Result<Option<UiEvent<M>>> {
        if self
            .redraw
            .next_deadline()
            .is_some_and(|deadline| deadline <= Instant::now())
        {
            return Ok(Some(UiEvent::Tick));
        }

        let timeout = self
            .redraw
            .next_deadline()
            .map_or(Duration::from_secs(60), |deadline| {
                deadline.saturating_duration_since(Instant::now())
            });
        if poll_terminal(timeout)? {
            return Ok(UiEvent::from_terminal(read_terminal()?));
        }
        Ok(self.redraw.next_deadline().map(|_| UiEvent::Tick))
    }

    pub fn dispatch_terminal(&mut self, event: Event) -> Option<M> {
        UiEvent::from_terminal(event).and_then(|event| self.dispatch(event))
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::*;

    struct EmptyScreen;

    impl Screen<()> for EmptyScreen {
        fn render(&mut self, _ui: &mut UiFrame<'_, '_, ()>, _area: Rect) {}

        fn handle_event(&mut self, _event: &UiEvent<()>) -> Interaction<()> {
            Interaction::Unhandled
        }
    }

    #[test]
    fn due_deadline_preempts_continuous_terminal_input() {
        let mut runtime = Runtime::new(EmptyScreen);
        runtime.request_redraw_at(Instant::now() - Duration::from_millis(1));

        let event = runtime
            .next_sync_event(
                |_| panic!("terminal polling must not precede an overdue tick"),
                || panic!("terminal reads must not precede an overdue tick"),
            )
            .unwrap();

        assert!(matches!(event, Some(UiEvent::Tick)));
    }
}
