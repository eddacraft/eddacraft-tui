//! Shared notification surface trait for TUI surfaces.
//!
//! Surfaces that carry user-facing notices (static-mode fallbacks, resume
//! hints, install errors) implement [`NotificationSource`] so renderers,
//! telemetry, and future daemon subscribers can consume them through the
//! canonical [`Notification`] envelope instead of surface-specific wording.

use anvil_kernel_types::{
    Notification, NotificationClass, NotificationContext, NotificationPriority,
};

/// Surfaces implement this to publish their current notifications through
/// the canonical notification envelope.
pub trait NotificationSource {
    /// Return the notifications currently held by this surface.
    fn notifications(&self) -> Vec<Notification>;
}

/// Build a notification tagged with `source` and no specific file.
pub fn surface_notification(
    source: &'static str,
    class: NotificationClass,
    priority: NotificationPriority,
    title: impl Into<String>,
    message: impl Into<String>,
) -> Notification {
    Notification::new(class, priority, title, message).with_context(NotificationContext {
        file: None,
        source: Some(source.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_notification_sets_source_and_fields() {
        let notification = surface_notification(
            "example",
            NotificationClass::Info,
            NotificationPriority::Low,
            "title",
            "message",
        );
        assert_eq!(notification.class, NotificationClass::Info);
        assert_eq!(notification.priority, NotificationPriority::Low);
        assert_eq!(notification.title, "title");
        assert_eq!(notification.message, "message");
        let ctx = notification.context.as_ref().unwrap();
        assert_eq!(ctx.source.as_deref(), Some("example"));
        assert!(ctx.file.is_none());
    }
}
