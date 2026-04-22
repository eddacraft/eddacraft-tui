use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationClass {
    Info,
    Progress,
    Finding,
    Nudge,
    Warning,
    Failure,
    Block,
    Interrupt,
    FenceState,
    Health,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NotificationContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    pub class: NotificationClass,
    pub priority: NotificationPriority,
    pub title: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<NotificationContext>,
}

impl Notification {
    #[must_use]
    pub fn new(
        class: NotificationClass,
        priority: NotificationPriority,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            class,
            priority,
            title: title.into(),
            message: message.into(),
            context: None,
        }
    }

    #[must_use]
    pub fn with_context(mut self, context: NotificationContext) -> Self {
        self.context = Some(context);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_round_trips_through_json() {
        let notification = Notification::new(
            NotificationClass::Finding,
            NotificationPriority::High,
            "Boundary violation",
            "Cross-layer import detected",
        )
        .with_context(NotificationContext {
            file: Some("src/api/user.ts".into()),
            source: Some("watch".into()),
        });

        let json = serde_json::to_string(&notification).unwrap();
        let back: Notification = serde_json::from_str(&json).unwrap();
        assert_eq!(back.class, NotificationClass::Finding);
        assert_eq!(back.priority, NotificationPriority::High);
        assert_eq!(back.title, "Boundary violation");
        assert_eq!(
            back.context.unwrap().file.as_deref(),
            Some("src/api/user.ts")
        );
    }
}
