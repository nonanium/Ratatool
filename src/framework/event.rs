use crossterm::event::{Event, KeyEvent, KeyEventKind, MouseEvent};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiEvent<M> {
    Key(KeyEvent),
    /// A terminal mouse event before runtime hit testing.
    Mouse(MouseEvent),
    /// A mouse event enriched with the action under the pointer.
    Pointer {
        event: MouseEvent,
        hit: Option<M>,
    },
    Paste(String),
    Resize {
        width: u16,
        height: u16,
    },
    Message(M),
    Tick,
}

impl<M> UiEvent<M> {
    pub fn from_terminal(event: Event) -> Option<Self> {
        match event {
            Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
                Some(Self::Key(key))
            }
            Event::Key(_) => None,
            Event::Mouse(mouse) => Some(Self::Mouse(mouse)),
            Event::Paste(value) => Some(Self::Paste(value)),
            Event::Resize(width, height) => Some(Self::Resize { width, height }),
            Event::FocusGained | Event::FocusLost => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    use super::*;

    fn key_event(kind: KeyEventKind) -> Event {
        Event::Key(KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::NONE,
            kind,
            state: KeyEventState::NONE,
        })
    }

    #[test]
    fn delivers_key_presses_and_repeats_but_ignores_releases() {
        assert!(matches!(
            UiEvent::<()>::from_terminal(key_event(KeyEventKind::Press)),
            Some(UiEvent::Key(_))
        ));
        assert!(matches!(
            UiEvent::<()>::from_terminal(key_event(KeyEventKind::Repeat)),
            Some(UiEvent::Key(_))
        ));
        assert_eq!(
            UiEvent::<()>::from_terminal(key_event(KeyEventKind::Release)),
            None
        );
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Interaction<M> {
    Unhandled,
    Handled { changed: bool, message: Option<M> },
    Exit,
}

impl<M> Interaction<M> {
    pub const fn unchanged() -> Self {
        Self::Handled {
            changed: false,
            message: None,
        }
    }

    pub const fn changed() -> Self {
        Self::Handled {
            changed: true,
            message: None,
        }
    }

    pub fn message(message: M) -> Self {
        Self::Handled {
            changed: false,
            message: Some(message),
        }
    }
}
