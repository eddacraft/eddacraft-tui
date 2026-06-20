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

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal implementor that defines only the eight required palette
    /// colours and relies entirely on the trait's default style-method bodies.
    /// Proves the default bodies satisfy the documented contract for *any*
    /// implementor, not just `EddaCraftTheme`.
    struct MinimalTheme;

    impl Theme for MinimalTheme {
        fn bg(&self) -> Color {
            Color::Black
        }
        fn fg(&self) -> Color {
            Color::White
        }
        fn accent(&self) -> Color {
            Color::Cyan
        }
        fn success(&self) -> Color {
            Color::Green
        }
        fn error(&self) -> Color {
            Color::Red
        }
        fn warning(&self) -> Color {
            Color::Yellow
        }
        fn muted(&self) -> Color {
            Color::Gray
        }
        fn border(&self) -> Color {
            Color::DarkGray
        }
    }

    #[test]
    fn default_style_methods_populate_fg_for_any_impl() {
        let t = MinimalTheme;
        let styles = [
            ("base", t.base()),
            ("highlighted", t.highlighted()),
            ("highlight_inactive", t.highlight_inactive()),
            ("title", t.title()),
            ("border_focused", t.border_focused()),
            ("border_unfocused", t.border_unfocused()),
            ("status_ok", t.status_ok()),
            ("status_error", t.status_error()),
            ("status_warning", t.status_warning()),
            ("disabled", t.disabled()),
        ];
        for (name, style) in styles {
            assert!(style.fg.is_some(), "default {name} must populate fg");
        }
    }

    #[test]
    fn role_style_dispatches_every_role_for_any_impl() {
        let t = MinimalTheme;
        for role in [
            Role::Primary,
            Role::Secondary,
            Role::Accent,
            Role::Highlight,
            Role::HighlightInactive,
            Role::Success,
            Role::Warning,
            Role::Error,
            Role::BorderSubtle,
            Role::BorderEmphasis,
        ] {
            let style = t.role_style(role);
            assert!(
                style.fg.is_some() || style.bg.is_some(),
                "role {role:?} should resolve to a non-empty style",
            );
        }
    }
}
