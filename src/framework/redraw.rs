use std::{collections::BTreeSet, time::Instant};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RedrawState {
    requested: bool,
    deadlines: BTreeSet<Instant>,
}

impl RedrawState {
    pub fn request_redraw(&mut self) {
        self.requested = true;
    }

    pub fn request_redraw_at(&mut self, deadline: Instant) {
        self.deadlines.insert(deadline);
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        self.deadlines.first().copied()
    }

    pub const fn is_requested(&self) -> bool {
        self.requested
    }

    pub fn take_requested(&mut self) -> bool {
        let requested = self.requested;
        self.requested = false;
        requested
    }

    /// Acknowledges every timer wakeup that is due at `now`.
    ///
    /// Timed redraw requests are deliberately not consumed by rendering. The
    /// event driver must deliver `UiEvent::Tick` first, so screens can update
    /// their time-based state before the following frame is drawn.
    pub fn take_due_deadlines(&mut self, now: Instant) -> bool {
        let due = self.next_deadline().is_some_and(|deadline| deadline <= now);
        if due {
            self.deadlines.retain(|deadline| *deadline > now);
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn acknowledging_due_deadlines_preserves_later_deadlines() {
        let now = Instant::now();
        let mut redraw = RedrawState::default();
        redraw.request_redraw_at(now + Duration::from_secs(2));
        redraw.request_redraw_at(now + Duration::from_secs(1));
        redraw.request_redraw_at(now + Duration::from_secs(3));
        assert_eq!(redraw.next_deadline(), Some(now + Duration::from_secs(1)));
        redraw.request_redraw();
        assert!(redraw.take_requested());
        assert!(!redraw.take_requested());
        assert!(redraw.take_due_deadlines(now + Duration::from_secs(1)));
        assert_eq!(redraw.next_deadline(), Some(now + Duration::from_secs(2)));
        assert!(redraw.take_due_deadlines(now + Duration::from_secs(2)));
        assert_eq!(redraw.next_deadline(), Some(now + Duration::from_secs(3)));
    }

    #[test]
    fn elapsed_deadlines_do_not_make_a_frame_due() {
        let now = Instant::now();
        let mut redraw = RedrawState::default();
        redraw.request_redraw_at(now);

        assert!(!redraw.is_requested());
        assert_eq!(redraw.next_deadline(), Some(now));
    }
}
