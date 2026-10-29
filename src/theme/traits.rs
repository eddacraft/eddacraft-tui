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
    /// Informational status (brand `status.info`). Added in 0.6.0 (ADR-195).
    Info,
    /// A refused or held state, solid fill (brand `status.blocked`). Added in
    /// 0.6.0 (ADR-195).
    Blocked,
    /// A state the tool could not determine (brand `status.unknown`). Added in
    /// 0.6.0 (ADR-195).
    Unknown,
    /// Disabled text (brand `text.disabled`). Added in 0.6.0 (ADR-195).
    Disabled,
    /// The boundary of an interactive control (brand `border.interactive`).
    /// Added in 0.6.0 (ADR-195).
    BorderInteractive,
    /// A raised panel (brand `surface.raised`). Added in 0.6.0 (ADR-195).
    Surface,
    /// A floating layer such as a popup (brand `surface.overlay`). Added in
    /// 0.6.0 (ADR-195).
    Overlay,
    /// Muted text: the brand's name for what [`Role::Secondary`] has always
    /// resolved to. Added in 0.6.0 (ADR-195); `Secondary` keeps its meaning.
    Muted,
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
///
/// # Stability
///
/// **stable** (D-TUIN-005). The theme override hook is a committed extension
/// surface: a breaking change requires a breaking version bump (a minor bump while the crate is 0.x, per its `SemVer` policy) *and* an ADR.
pub trait Theme {
    fn bg(&self) -> Color;
    fn fg(&self) -> Color;
    fn accent(&self) -> Color;
    fn success(&self) -> Color;
    fn error(&self) -> Color;
    fn warning(&self) -> Color;
    fn muted(&self) -> Color;
    fn border(&self) -> Color;

    // Palette slots added in 0.6.0 (ADR-195). Each default falls back to one
    // of the eight required colours, so existing implementors keep compiling
    // and keep today's rendering until they override it.

    /// Informational status colour. Defaults to [`Theme::accent`], which is
    /// what info badges used before 0.6.0.
    fn info(&self) -> Color {
        self.accent()
    }

    /// Secondary text, one step below [`Theme::fg`] (brand `text.secondary`).
    /// Not the [`Role::Secondary`] colour, which stays muted text.
    fn text_secondary(&self) -> Color {
        self.fg()
    }

    /// Disabled text (brand `text.disabled`). Defaults to [`Theme::muted`].
    fn text_disabled(&self) -> Color {
        self.muted()
    }

    /// The boundary of an interactive control. Defaults to [`Theme::border`].
    fn border_interactive(&self) -> Color {
        self.border()
    }

    /// A raised panel background. Defaults to [`Theme::bg`].
    fn surface(&self) -> Color {
        self.bg()
    }

    /// A floating layer background. Defaults to [`Theme::bg`].
    fn overlay(&self) -> Color {
        self.bg()
    }

    /// The colour of chart series `n`, cycling through a fixed order. The
    /// default draws on the palette above; themes with a categorical palette
    /// override it.
    fn chart_series(&self, n: usize) -> Color {
        let series = [
            self.accent(),
            self.info(),
            self.success(),
            self.warning(),
            self.error(),
            self.muted(),
        ];
        series[n % series.len()]
    }

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

    /// Informational status: [`Theme::info`], bold.
    fn status_info(&self) -> Style {
        Style::default()
            .fg(self.info())
            .add_modifier(Modifier::BOLD)
    }

    /// Blocked status: the background colour on a solid [`Theme::error`]
    /// fill, bold. The only solid status by default.
    fn status_blocked(&self) -> Style {
        Style::default()
            .fg(self.bg())
            .bg(self.error())
            .add_modifier(Modifier::BOLD)
    }

    /// Unknown status: muted, so it never reads as a pass or a failure.
    fn status_unknown(&self) -> Style {
        Style::default().fg(self.muted())
    }

    fn role_style(&self, role: Role) -> Style {
        match role {
            Role::Primary => self.base(),
            Role::Secondary | Role::Muted => self.disabled(),
            Role::Accent => self.title(),
            Role::Highlight => self.highlighted(),
            Role::HighlightInactive => self.highlight_inactive(),
            Role::Success => self.status_ok(),
            Role::Warning => self.status_warning(),
            Role::Error => self.status_error(),
            Role::BorderSubtle => self.border_unfocused(),
            Role::BorderEmphasis => self.border_focused(),
            Role::Info => self.status_info(),
            Role::Blocked => self.status_blocked(),
            Role::Unknown => self.status_unknown(),
            Role::Disabled => Style::default().fg(self.text_disabled()),
            Role::BorderInteractive => Style::default().fg(self.border_interactive()),
            Role::Surface => Style::default().fg(self.fg()).bg(self.surface()),
            Role::Overlay => Style::default().fg(self.fg()).bg(self.overlay()),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
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

    /// Every [`Role`] variant, so a new one cannot be added without a test
    /// noticing.
    pub(crate) const ALL_ROLES: [Role; 18] = [
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
        Role::Info,
        Role::Blocked,
        Role::Unknown,
        Role::Disabled,
        Role::BorderInteractive,
        Role::Surface,
        Role::Overlay,
        Role::Muted,
    ];

    #[test]
    fn new_palette_slots_default_to_todays_colours() {
        let t = MinimalTheme;
        assert_eq!(t.info(), t.accent());
        assert_eq!(t.text_secondary(), t.fg());
        assert_eq!(t.text_disabled(), t.muted());
        assert_eq!(t.border_interactive(), t.border());
        assert_eq!(t.surface(), t.bg());
        assert_eq!(t.overlay(), t.bg());
        assert_eq!(t.chart_series(0), t.accent());
        assert_eq!(t.chart_series(6), t.chart_series(0));
    }

    #[test]
    fn secondary_and_muted_roles_resolve_alike() {
        let t = MinimalTheme;
        assert_eq!(t.role_style(Role::Secondary), t.role_style(Role::Muted));
        assert_eq!(t.role_style(Role::Secondary), t.disabled());
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
            ("status_info", t.status_info()),
            ("status_blocked", t.status_blocked()),
            ("status_unknown", t.status_unknown()),
        ];
        for (name, style) in styles {
            assert!(style.fg.is_some(), "default {name} must populate fg");
        }
    }

    #[test]
    fn role_style_dispatches_every_role_for_any_impl() {
        let t = MinimalTheme;
        for role in ALL_ROLES {
            let style = t.role_style(role);
            // The trait contract requires fg to be set; role_style is built from
            // the style methods, so every role must resolve to an fg-bearing style.
            assert!(
                style.fg.is_some(),
                "role {role:?} should resolve to a style with fg set",
            );
        }
    }
}
