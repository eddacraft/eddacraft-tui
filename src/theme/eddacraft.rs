use ratatui::style::Color;

use super::traits::Theme;

/// The `eddacraft` Terminal Standard palette.
///
/// Colour names follow the brand design system:
/// - The Void (bg), Structure (border)
/// - Off-White (fg), Ghost Grey (muted)
/// - anvil Ember (accent), edda Growth (success)
/// - Brick Red (error), Dull Amber (warning)
pub struct EddaCraftTheme;

const VOID: Color = Color::Rgb(13, 13, 15);
const STRUCTURE: Color = Color::Rgb(42, 42, 46);
const OFF_WHITE: Color = Color::Rgb(235, 235, 235);
const GHOST_GREY: Color = Color::Rgb(133, 133, 138);
const ANVIL_EMBER: Color = Color::Rgb(204, 85, 0);
const EDDA_GROWTH: Color = Color::Rgb(46, 139, 87);
const BRICK_RED: Color = Color::Rgb(201, 74, 74);
const DULL_AMBER: Color = Color::Rgb(208, 140, 56);

impl Theme for EddaCraftTheme {
    fn bg(&self) -> Color {
        VOID
    }

    fn fg(&self) -> Color {
        OFF_WHITE
    }

    fn accent(&self) -> Color {
        ANVIL_EMBER
    }

    fn success(&self) -> Color {
        EDDA_GROWTH
    }

    fn error(&self) -> Color {
        BRICK_RED
    }

    fn warning(&self) -> Color {
        DULL_AMBER
    }

    fn muted(&self) -> Color {
        GHOST_GREY
    }

    fn border(&self) -> Color {
        STRUCTURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_colours_are_distinct() {
        let theme = EddaCraftTheme;
        let colours = [
            theme.bg(),
            theme.fg(),
            theme.accent(),
            theme.success(),
            theme.error(),
            theme.warning(),
            theme.muted(),
            theme.border(),
        ];

        for (i, a) in colours.iter().enumerate() {
            for (j, b) in colours.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "colour {i} and {j} should be distinct");
                }
            }
        }
    }

    #[test]
    fn base_style_uses_fg_and_bg() {
        let theme = EddaCraftTheme;
        let style = theme.base();
        assert_eq!(style.fg, Some(OFF_WHITE));
        assert_eq!(style.bg, Some(VOID));
    }

    #[test]
    fn role_style_resolves_each_variant() {
        use crate::theme::Role;
        let theme = EddaCraftTheme;
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
            let style = theme.role_style(role);
            assert!(
                style.fg.is_some() || style.bg.is_some(),
                "role {role:?} should resolve to a non-empty style"
            );
        }
    }

    #[test]
    fn role_style_matches_individual_methods() {
        use crate::theme::Role;
        let theme = EddaCraftTheme;
        assert_eq!(theme.role_style(Role::Primary), theme.base());
        assert_eq!(theme.role_style(Role::Highlight), theme.highlighted());
        assert_eq!(
            theme.role_style(Role::HighlightInactive),
            theme.highlight_inactive(),
        );
        assert_eq!(theme.role_style(Role::Error), theme.status_error());
        assert_eq!(
            theme.role_style(Role::BorderSubtle),
            theme.border_unfocused()
        );
    }

    #[test]
    fn all_style_methods_populate_fg() {
        // The trait contract: every style method returns a Style with fg set.
        let t = EddaCraftTheme;
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
            assert!(style.fg.is_some(), "{name} must set fg per Theme contract");
        }
    }

    #[test]
    fn highlight_styles_set_bg_and_bold() {
        use ratatui::style::Modifier;
        let t = EddaCraftTheme;
        for (name, style) in [
            ("highlighted", t.highlighted()),
            ("highlight_inactive", t.highlight_inactive()),
        ] {
            assert!(style.bg.is_some(), "{name} must set bg");
            assert!(
                style.add_modifier.contains(Modifier::BOLD),
                "{name} must be bold",
            );
        }
    }

    #[test]
    fn palette_methods_match_documented_colours() {
        let t = EddaCraftTheme;
        assert_eq!(t.bg(), VOID);
        assert_eq!(t.fg(), OFF_WHITE);
        assert_eq!(t.accent(), ANVIL_EMBER);
        assert_eq!(t.success(), EDDA_GROWTH);
        assert_eq!(t.error(), BRICK_RED);
        assert_eq!(t.warning(), DULL_AMBER);
        assert_eq!(t.muted(), GHOST_GREY);
        assert_eq!(t.border(), STRUCTURE);
    }
}
