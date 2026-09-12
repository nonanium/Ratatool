use ratatui::layout::Rect;

use super::{Interaction, UiEvent, UiFrame};

pub trait Screen<M>: Send {
    fn render(&mut self, ui: &mut UiFrame<'_, '_, M>, area: Rect);
    fn handle_event(&mut self, event: &UiEvent<M>) -> Interaction<M>;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScreenPresentation {
    /// Hides every lower screen and disables its hit regions.
    #[default]
    Replace,
    /// Renders lower screens and retains their hit regions.
    Overlay,
    /// Renders lower screens and disables their hit regions.
    ModalOverlay,
}

struct ScreenEntry<M> {
    screen: Box<dyn Screen<M>>,
    presentation: ScreenPresentation,
}

#[derive(Default)]
pub struct ScreenStack<M> {
    screens: Vec<ScreenEntry<M>>,
}

impl<M> ScreenStack<M> {
    pub fn new(root: impl Screen<M> + 'static) -> Self {
        Self {
            screens: vec![ScreenEntry {
                screen: Box::new(root),
                presentation: ScreenPresentation::Replace,
            }],
        }
    }

    pub fn push(&mut self, screen: impl Screen<M> + 'static) {
        self.push_with_presentation(screen, ScreenPresentation::Replace);
    }

    pub fn push_boxed(&mut self, screen: Box<dyn Screen<M>>) {
        self.push_boxed_with_presentation(screen, ScreenPresentation::Replace);
    }

    pub fn push_with_presentation(
        &mut self,
        screen: impl Screen<M> + 'static,
        presentation: ScreenPresentation,
    ) {
        self.push_boxed_with_presentation(Box::new(screen), presentation);
    }

    pub fn push_boxed_with_presentation(
        &mut self,
        screen: Box<dyn Screen<M>>,
        presentation: ScreenPresentation,
    ) {
        self.screens.push(ScreenEntry {
            screen,
            presentation,
        });
    }

    /// Replaces the complete navigation history with a new root screen.
    pub fn replace(&mut self, screen: impl Screen<M> + 'static) {
        self.clear();
        self.push(screen);
    }

    /// Removes every screen above the root screen.
    pub fn pop_to_root(&mut self) {
        self.screens.truncate(1);
    }

    pub fn clear(&mut self) {
        self.screens.clear();
    }

    pub fn pop(&mut self) -> Option<Box<dyn Screen<M>>> {
        self.screens.pop().map(|entry| entry.screen)
    }

    pub const fn len(&self) -> usize {
        self.screens.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.screens.is_empty()
    }

    pub fn render(&mut self, ui: &mut UiFrame<'_, '_, M>, area: Rect) {
        let first_visible = self
            .screens
            .iter()
            .rposition(|entry| entry.presentation == ScreenPresentation::Replace)
            .unwrap_or(0);
        for entry in &mut self.screens[first_visible..] {
            if entry.presentation == ScreenPresentation::ModalOverlay {
                ui.hit_map().clear();
            }
            entry.screen.render(ui, area);
        }
    }

    pub fn handle_event(&mut self, event: &UiEvent<M>) -> Interaction<M> {
        self.screens
            .last_mut()
            .map_or(Interaction::Unhandled, |entry| {
                entry.screen.handle_event(event)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EmptyScreen;

    impl Screen<()> for EmptyScreen {
        fn render(&mut self, _ui: &mut UiFrame<'_, '_, ()>, _area: Rect) {}

        fn handle_event(&mut self, _event: &UiEvent<()>) -> Interaction<()> {
            Interaction::Unhandled
        }
    }

    #[test]
    fn boxed_push_root_navigation_and_replacement_manage_history() {
        fn assert_send<T: Send>() {}
        assert_send::<ScreenStack<()>>();
        assert_send::<crate::Runtime<()>>();

        let mut stack = ScreenStack::new(EmptyScreen);
        stack.push_boxed(Box::new(EmptyScreen));
        stack.push_boxed_with_presentation(Box::new(EmptyScreen), ScreenPresentation::ModalOverlay);
        assert_eq!(stack.len(), 3);

        stack.pop_to_root();
        assert_eq!(stack.len(), 1);

        stack.push(EmptyScreen);
        stack.replace(EmptyScreen);
        assert_eq!(stack.len(), 1);

        stack.clear();
        assert!(stack.is_empty());
    }
}
