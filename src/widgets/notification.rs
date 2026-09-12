use std::time::Instant;

use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::theme::ResolvedTheme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    pub id: u64,
    pub level: NotificationLevel,
    pub message: String,
    pub expires_at: Option<Instant>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NotificationOverlay {
    notifications: Vec<Notification>,
}

impl NotificationOverlay {
    pub fn push(&mut self, notification: Notification) {
        if let Some(existing) = self
            .notifications
            .iter_mut()
            .find(|existing| existing.id == notification.id)
        {
            *existing = notification;
        } else {
            self.notifications.push(notification);
        }
    }

    pub fn dismiss(&mut self, id: u64) -> NotificationResult {
        let before = self.notifications.len();
        self.notifications
            .retain(|notification| notification.id != id);
        if before == self.notifications.len() {
            NotificationResult::Unchanged
        } else {
            NotificationResult::Changed
        }
    }

    pub fn expire(&mut self, now: Instant) -> bool {
        let before = self.notifications.len();
        self.notifications
            .retain(|notification| notification.expires_at.is_none_or(|expiry| expiry > now));
        before != self.notifications.len()
    }

    pub fn next_expiration(&self) -> Option<Instant> {
        self.notifications
            .iter()
            .filter_map(|notification| notification.expires_at)
            .min()
    }

    pub fn notifications(&self) -> &[Notification] {
        &self.notifications
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationResult {
    Changed,
    Unchanged,
}

pub fn render_notifications(
    frame: &mut Frame<'_>,
    area: Rect,
    overlay: &NotificationOverlay,
    theme: &ResolvedTheme,
) {
    let mut bottom = area.bottom();
    for notification in overlay.notifications.iter().rev() {
        if bottom <= area.y {
            break;
        }
        let (symbol, status_style) = match notification.level {
            NotificationLevel::Info => (theme.symbols.status_info, theme.status.info),
            NotificationLevel::Success => (theme.symbols.status_success, theme.status.success),
            NotificationLevel::Warning => (theme.symbols.status_warning, theme.status.warning),
            NotificationLevel::Error => (theme.symbols.status_error, theme.status.error),
        };
        let content_width = UnicodeWidthStr::width(symbol)
            .saturating_add(1)
            .saturating_add(UnicodeWidthStr::width(notification.message.as_str()));
        let width = u16::try_from(content_width)
            .unwrap_or(u16::MAX)
            .min(area.width);
        bottom = bottom.saturating_sub(1);
        let notification_area = Rect::new(area.right().saturating_sub(width), bottom, width, 1);
        frame.render_widget(Clear, notification_area);
        let style = theme.surfaces.raised.patch(status_style);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(symbol, style),
                Span::styled(" ", style),
                Span::styled(notification.message.as_str(), style),
            ])),
            notification_area,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn expiration_and_replacement_are_deterministic() {
        let now = Instant::now();
        let mut overlay = NotificationOverlay::default();
        overlay.push(Notification {
            id: 1,
            level: NotificationLevel::Info,
            message: "first".into(),
            expires_at: Some(now + Duration::from_secs(1)),
        });
        overlay.push(Notification {
            id: 1,
            level: NotificationLevel::Success,
            message: "updated".into(),
            expires_at: Some(now + Duration::from_secs(2)),
        });
        assert_eq!(overlay.notifications().len(), 1);
        assert!(!overlay.expire(now + Duration::from_secs(1)));
        assert!(overlay.expire(now + Duration::from_secs(2)));
    }
}
