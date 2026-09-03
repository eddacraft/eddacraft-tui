use ratatui::style::Color;

use super::traits::Theme;

/// The `eddacraft` Terminal Standard palette.
///
/// Colour names follow the brand design system:
/// - The Void (bg), Structure (border)
/// - Off-White (fg), Ghost Grey (muted)
/// - anvil Ember (accent), edda Growth (success)
/// - Brick Red (error), Dull Amber (warning)
///
/// **Contrast:** every colour used as *text* must clear the WCAG AA 4.5:1
/// floor against The Void, and `contrast_of_every_text_colour_clears_wcag_aa`
/// pins that. anvil Ember was `#CC5500` and Brick Red `#C94A4A`, which
/// measured 4.4998:1 and 4.22:1 — both *below* the floor, the accent only
/// fractionally so, across ~7,100 rendered cells in 55 of the repo's TUI
/// snapshots. Both were lightened along their own hue to land near 5.0:1.
/// Lightening the accent raises `highlighted()` too (The Void *on* Ember), so
/// one change fixes both directions.
pub struct EddaCraftTheme;

const VOID: Color = Color::Rgb(13, 13, 15);
const STRUCTURE: Color = Color::Rgb(42, 42, 46);
const OFF_WHITE: Color = Color::Rgb(235, 235, 235);
const GHOST_GREY: Color = Color::Rgb(133, 133, 138);
const ANVIL_EMBER: Color = Color::Rgb(217, 90, 0);
const EDDA_GROWTH: Color = Color::Rgb(46, 139, 87);
const BRICK_RED: Color = Color::Rgb(207, 94, 94);
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

    fn srgb_channel(channel: u8) -> f64 {
        let c = f64::from(channel) / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    fn relative_luminance(color: Color) -> f64 {
        let Color::Rgb(r, g, b) = color else {
            panic!("palette colours must be Rgb, got {color:?}");
        };
        0.2126 * srgb_channel(r) + 0.7152 * srgb_channel(g) + 0.0722 * srgb_channel(b)
    }

    /// WCAG 2.1 relative-contrast ratio between two opaque colours.
    fn contrast_ratio(fg: Color, bg: Color) -> f64 {
        let a = relative_luminance(fg);
        let b = relative_luminance(bg);
        let (hi, lo) = if a > b { (a, b) } else { (b, a) };
        (hi + 0.05) / (lo + 0.05)
    }

    /// Every palette colour that is rendered as **text** must clear the WCAG
    /// AA 4.5:1 floor against The Void.
    ///
    /// This is the guard for the measured regression: anvil Ember was
    /// `#CC5500` (4.4998:1 — below the floor, not on it) and Brick Red
    /// `#C94A4A` (4.22:1 — under it), together covering ~7,100 rendered cells
    /// across 55 of the repo's TUI snapshots.
    ///
    /// `border()` is deliberately excluded: it paints box-drawing rules, never
    /// text, and low-contrast chrome is the intent there.
    #[test]
    fn contrast_of_every_text_colour_clears_wcag_aa() {
        let theme = EddaCraftTheme;
        let bg = theme.bg();
        let text_roles = [
            ("fg", theme.fg()),
            ("accent", theme.accent()),
            ("success", theme.success()),
            ("error", theme.error()),
            ("warning", theme.warning()),
            ("muted", theme.muted()),
        ];

        let failures: Vec<String> = text_roles
            .iter()
            .filter_map(|(name, colour)| {
                let ratio = contrast_ratio(*colour, bg);
                (ratio < 4.5).then(|| format!("{name} {colour:?} is {ratio:.2}:1"))
            })
            .collect();

        assert!(
            failures.is_empty(),
            "text colours below the WCAG AA 4.5:1 floor on The Void: {failures:?}",
        );
    }

    /// `highlighted()` paints The Void *on* the accent, so the accent has to
    /// clear the floor from the other direction too. Lightening the accent
    /// raises both, which is why one palette change fixes both.
    #[test]
    fn highlighted_style_clears_wcag_aa_in_both_directions() {
        let theme = EddaCraftTheme;
        let style = theme.highlighted();
        let ratio = contrast_ratio(style.fg.unwrap(), style.bg.unwrap());
        assert!(
            ratio >= 4.5,
            "highlighted() is {ratio:.2}:1; WCAG AA needs 4.5:1",
        );
    }

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
