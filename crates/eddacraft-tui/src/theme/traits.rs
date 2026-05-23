use ratatui::style::{Color, Modifier, Style};

/// Semantic role tokens that widgets can resolve to a [`Style`] via
/// [`Theme::role_style`]. Lets downstream widgets reference *what a colour
/// means* rather than which palette slot it occupies.
///
/// Currently exposed as a forward extensibility hook — the built-in
/// widgets resolve styles directly via [`Theme::title`] / [`Theme::base`] /
/// etc. New widgets are encouraged to use [`Theme::role_style`] instead so
/// downstream theme implementations can override roles centrally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Role {
    Primary,
    Secondary,
    Accent,
    Highlight,
    HighlightInactive,
    Success,
    Warning,
    Error,
    BorderSubtle,
    BorderEmphasis,
}

/// Visual theme for eddacraft-tui widgets.
///
/// **Implementor contract.** Every style method in this trait is expected to
/// return a [`Style`] with both `fg` and (where semantically meaningful) `bg`
/// explicitly set. Internal widget tests rely on
/// e.g. `theme.status_error().fg.unwrap()`; an implementation that returns
/// [`Style::default()`] for a given role will cause those tests to panic if
/// run against the custom theme.
///
/// The default method bodies on this trait satisfy the contract automatically
/// when the eight palette colours below are implemented; only override a
/// default if you also keep `fg`/`bg` populated.
pub trait Theme {
    fn bg(&self) -> Color;
    fn fg(&self) -> Color;
    fn accent(&self) -> Color;
    fn success(&self) -> Color;
    fn error(&self) -> Color;
    fn warning(&self) -> Color;
    fn muted(&self) -> Color;
    fn border(&self) -> Color;

    fn base(&self) -> Style {
        Style::default().fg(self.fg()).bg(self.bg())
    }

    fn highlighted(&self) -> Style {
        Style::default()
            .fg(self.bg())
            .bg(self.accent())
            .add_modifier(Modifier::BOLD)
    }

    fn highlight_inactive(&self) -> Style {
        Style::default()
            .fg(self.fg())
            .bg(self.border())
            .add_modifier(Modifier::BOLD)
    }

    fn title(&self) -> Style {
        Style::default()
            .fg(self.accent())
            .add_modifier(Modifier::BOLD)
    }

    fn border_focused(&self) -> Style {
        Style::default().fg(self.accent())
    }

    fn border_unfocused(&self) -> Style {
        Style::default().fg(self.border())
    }

    fn status_ok(&self) -> Style {
        Style::default()
            .fg(self.success())
            .add_modifier(Modifier::BOLD)
    }

    fn status_error(&self) -> Style {
        Style::default()
            .fg(self.error())
            .add_modifier(Modifier::BOLD)
    }

    fn status_warning(&self) -> Style {
        Style::default()
            .fg(self.warning())
            .add_modifier(Modifier::BOLD)
    }

    fn disabled(&self) -> Style {
        Style::default().fg(self.muted())
    }

    fn role_style(&self, role: Role) -> Style {
        match role {
            Role::Primary => self.base(),
            Role::Secondary => self.disabled(),
            Role::Accent => self.title(),
            Role::Highlight => self.highlighted(),
            Role::HighlightInactive => self.highlight_inactive(),
            Role::Success => self.status_ok(),
            Role::Warning => self.status_warning(),
            Role::Error => self.status_error(),
            Role::BorderSubtle => self.border_unfocused(),
            Role::BorderEmphasis => self.border_focused(),
        }
    }
}
