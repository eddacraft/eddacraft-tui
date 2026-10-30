use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::theme::{DASHED_BORDER, Theme};

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum BadgeStatus {
    Success,
    Error,
    Warning,
    Info,
    Running,
    Skipped,
    /// A refused or held state. Added in 0.6.0 (ADR-195).
    Blocked,
    /// A state the tool could not determine. Added in 0.6.0 (ADR-195).
    Unknown,
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
            Self::Info | Self::Running => theme.status_info(),
            Self::Skipped => theme.disabled(),
            Self::Blocked => theme.status_blocked(),
            Self::Unknown => theme.status_unknown(),
        }
    }

    /// The border for a box showing this status: dashed for [`Self::Unknown`],
    /// the brand's mark for an undetermined state, and rounded otherwise.
    /// Shared by `Modal` and `Toast`.
    pub(crate) fn box_border(self) -> border::Set<'static> {
        match self {
            Self::Unknown => DASHED_BORDER,
            _ => border::ROUNDED,
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

    /// The bracket marker and default label for each status. The markers are
    /// the grammar the web surfaces and the plain CLI share (ADR-195), so a
    /// status reads the same with or without colour.
    fn status_config(&self) -> (&'static str, Style, &'a str) {
        let (icon, default_label) = match self.status {
            BadgeStatus::Success => ("[ OK ]", "Passed"),
            BadgeStatus::Error => ("[ ERR ]", "Failed"),
            BadgeStatus::Warning => ("[ WARN ]", "Warning"),
            BadgeStatus::Info => ("[ i ]", "Info"),
            BadgeStatus::Running => ("[ .. ]", "Running"),
            BadgeStatus::Skipped => ("[ N/A ]", "Skipped"),
            BadgeStatus::Blocked => ("[ BLOCKED ]", "Blocked"),
            BadgeStatus::Unknown => ("[ ?? ]", "Unknown"),
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
            Span::styled(icon, style),
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

    fn rendered(status: BadgeStatus) -> String {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 24, 1);
        let mut buf = Buffer::empty(area);
        StatusBadge::new(status, &theme).render(area, &mut buf);
        (0..area.width)
            .map(|x| buf[(x, 0)].symbol().to_string())
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn info_badge_uses_shared_severity_style() {
        let theme = EddaCraftTheme;
        let area = Rect::new(0, 0, 12, 1);
        let mut actual = Buffer::empty(area);
        let mut expected = Buffer::empty(area);

        StatusBadge::new(BadgeStatus::Info, &theme).render(area, &mut actual);
        let style = BadgeStatus::Info.severity_style(&theme);
        Line::from(vec![
            Span::styled("[ i ]", style),
            Span::raw(" "),
            Span::styled("Info", style),
        ])
        .render(area, &mut expected);

        assert_eq!(actual[(0, 0)].style(), expected[(0, 0)].style());
        assert_eq!(actual[(1, 0)].style(), expected[(1, 0)].style());
        assert_eq!(actual[(2, 0)].style(), expected[(2, 0)].style());
    }

    #[test]
    fn renders_the_bracket_grammar_for_positive_statuses() {
        assert_eq!(rendered(BadgeStatus::Success), "[ OK ] Passed");
        assert_eq!(rendered(BadgeStatus::Info), "[ i ] Info");
        assert_eq!(rendered(BadgeStatus::Running), "[ .. ] Running");
    }

    #[test]
    fn renders_the_bracket_grammar_for_other_statuses() {
        assert_eq!(rendered(BadgeStatus::Error), "[ ERR ] Failed");
        assert_eq!(rendered(BadgeStatus::Warning), "[ WARN ] Warning");
        assert_eq!(rendered(BadgeStatus::Skipped), "[ N/A ] Skipped");
        assert_eq!(rendered(BadgeStatus::Blocked), "[ BLOCKED ] Blocked");
        assert_eq!(rendered(BadgeStatus::Unknown), "[ ?? ] Unknown");
    }

    #[test]
    fn info_and_running_use_the_info_colour_not_the_accent() {
        let theme = EddaCraftTheme;
        for status in [BadgeStatus::Info, BadgeStatus::Running] {
            let style = status.severity_style(&theme);
            assert_eq!(style.fg, Some(theme.info()));
            assert_ne!(style.fg, Some(theme.accent()));
        }
        assert_eq!(
            BadgeStatus::Blocked.severity_style(&theme),
            theme.status_blocked()
        );
        assert_eq!(
            BadgeStatus::Unknown.severity_style(&theme),
            theme.status_unknown()
        );
    }
}
