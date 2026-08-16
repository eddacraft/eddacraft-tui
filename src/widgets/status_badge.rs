use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::theme::Theme;

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum BadgeStatus {
    Success,
    Error,
    Warning,
    Info,
    Running,
    Skipped,
}

impl BadgeStatus {
    /// Resolve the severity to a [`Style`] using the supplied theme. Shared
    /// between widgets that surface badge status (currently `Modal` and
    /// `Toast`) so the mapping stays in one place.
    #[must_use]
    pub fn severity_style<T: Theme>(self, theme: &T) -> Style {
        match self {
            Self::Success => theme.status_ok(),
            Self::Error => theme.status_error(),
            Self::Warning => theme.status_warning(),
            Self::Info | Self::Running => theme.title(),
            Self::Skipped => theme.disabled(),
        }
    }
}

pub struct StatusBadge<'a, T: Theme> {
    theme: &'a T,
    status: BadgeStatus,
    label: Option<&'a str>,
}

impl<'a, T: Theme> StatusBadge<'a, T> {
    pub fn new(status: BadgeStatus, theme: &'a T) -> Self {
        Self {
            theme,
            status,
            label: None,
        }
    }

    #[must_use]
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = label.into();
        self
    }

    fn status_config(&self) -> (char, Style, &'a str) {
        let (icon, default_label) = match self.status {
            BadgeStatus::Success => ('◆', "Passed"),
            BadgeStatus::Error => ('✖', "Failed"),
            BadgeStatus::Warning => ('◈', "Warning"),
            BadgeStatus::Info => ('◇', "Info"),
            BadgeStatus::Running => ('●', "Running"),
            BadgeStatus::Skipped => ('○', "Skipped"),
        };

        (icon, self.status.severity_style(self.theme), default_label)
    }
}

impl<T: Theme> Widget for StatusBadge<'_, T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let (icon, style, default_label) = self.status_config();
        let label = self.label.unwrap_or(default_label);

        let line = Line::from(vec![
            Span::styled(icon.to_string(), style),
            Span::raw(" "),
            Span::styled(label, style),
        ]);
        line.render(Rect::new(area.x, area.y, area.width, 1), buf);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::widgets::Widget;

    use super::*;
    use crate::theme::EddaCraftTheme;

    fn icon_for(status: BadgeStatus) -> String {
        let theme = EddaCraftTheme;
        let mut buf = Buffer::empty(Rect::new(0, 0, 12, 1));
        StatusBadge::new(status, &theme).render(Rect::new(0, 0, 12, 1), &mut buf);
        buf[(0, 0)].symbol().to_string()
    }

    #[test]
    fn info_badge_uses_shared_severity_style() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 12, 1);
        let mut actual = Buffer::empty(area);
        let mut expected = Buffer::empty(area);

        StatusBadge::new(BadgeStatus::Info, &theme).render(area, &mut actual);
        Line::styled("◇ Info", BadgeStatus::Info.severity_style(&theme))
            .render(area, &mut expected);

        assert_eq!(actual[(0, 0)].style(), expected[(0, 0)].style());
        assert_eq!(actual[(2, 0)].style(), expected[(2, 0)].style());
    }

    #[test]
    fn renders_expected_icons_for_positive_statuses() {
        assert_eq!(icon_for(BadgeStatus::Success), "◆");
        assert_eq!(icon_for(BadgeStatus::Info), "◇");
        assert_eq!(icon_for(BadgeStatus::Running), "●");
    }

    #[test]
    fn renders_expected_icons_for_other_statuses() {
        assert_eq!(icon_for(BadgeStatus::Error), "✖");
        assert_eq!(icon_for(BadgeStatus::Warning), "◈");
        assert_eq!(icon_for(BadgeStatus::Skipped), "○");
    }
}
