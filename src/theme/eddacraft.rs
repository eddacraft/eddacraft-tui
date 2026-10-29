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
/// Since 0.6.0 the values follow brand-and-design 2.x (ADR-193): edda Growth is
/// `#3B9762`, Fjord carries info, and the raised and overlay surfaces, Iron,
/// Ash and the categorical chart colours fill the slots ADR-195 added.
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
const EDDA_GROWTH: Color = Color::Rgb(59, 151, 98);
const BRICK_RED: Color = Color::Rgb(207, 94, 94);
const DULL_AMBER: Color = Color::Rgb(208, 140, 56);
const SURFACE_RAISED: Color = Color::Rgb(20, 20, 22);
const SURFACE_OVERLAY: Color = Color::Rgb(24, 24, 27);
const IRON: Color = Color::Rgb(103, 103, 108);
const ASH: Color = Color::Rgb(183, 183, 186);
const FJORD: Color = Color::Rgb(97, 163, 230);
const HEATHER: Color = Color::Rgb(176, 138, 218);
const LINGON: Color = Color::Rgb(214, 126, 169);
const GLACIER: Color = Color::Rgb(0, 179, 194);

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

    fn info(&self) -> Color {
        FJORD
    }

    fn text_secondary(&self) -> Color {
        ASH
    }

    fn text_disabled(&self) -> Color {
        IRON
    }

    fn border_interactive(&self) -> Color {
        IRON
    }

    fn surface(&self) -> Color {
        SURFACE_RAISED
    }

    fn overlay(&self) -> Color {
        SURFACE_OVERLAY
    }

    fn chart_series(&self, n: usize) -> Color {
        const SERIES: [Color; 6] = [ANVIL_EMBER, FJORD, EDDA_GROWTH, HEATHER, LINGON, GLACIER];
        SERIES[n % SERIES.len()]
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
    /// text, and low-contrast chrome is the intent there. So are
    /// `text_disabled()` and `border_interactive()` (Iron): WCAG exempts
    /// disabled text, and a control boundary needs 3:1, which
    /// `interactive_border_clears_wcag_non_text_contrast` pins.
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
            ("info", theme.info()),
            ("text_secondary", theme.text_secondary()),
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
    /// Text colours also sit on the raised and overlay panels, so they must
    /// clear 4.5:1 on all three surfaces (guide §8 item 1).
    #[test]
    fn text_colours_clear_wcag_aa_on_every_surface() {
        let theme = EddaCraftTheme;
        let mut failures = Vec::new();
        for (surface_name, surface) in [
            ("bg", theme.bg()),
            ("surface", theme.surface()),
            ("overlay", theme.overlay()),
        ] {
            for (name, colour) in [
                ("fg", theme.fg()),
                ("accent", theme.accent()),
                ("success", theme.success()),
                ("error", theme.error()),
                ("warning", theme.warning()),
                ("muted", theme.muted()),
                ("info", theme.info()),
                ("text_secondary", theme.text_secondary()),
            ] {
                let ratio = contrast_ratio(colour, surface);
                if ratio < 4.5 {
                    failures.push(format!("{name} on {surface_name} is {ratio:.2}:1"));
                }
            }
        }
        assert!(failures.is_empty(), "below 4.5:1: {failures:?}");
    }

    #[test]
    fn interactive_border_clears_wcag_non_text_contrast() {
        let theme = EddaCraftTheme;
        let ratio = contrast_ratio(theme.border_interactive(), theme.bg());
        assert!(
            ratio >= 3.0,
            "border_interactive is {ratio:.2}:1; needs 3:1"
        );
    }

    #[test]
    fn blocked_status_clears_wcag_aa() {
        let style = EddaCraftTheme.status_blocked();
        let ratio = contrast_ratio(style.fg.unwrap(), style.bg.unwrap());
        assert!(ratio >= 4.5, "status_blocked() is {ratio:.2}:1");
    }

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
        let theme = EddaCraftTheme;
        for role in crate::theme::traits::tests::ALL_ROLES {
            let style = theme.role_style(role);
            assert!(
                style.fg.is_some(),
                "role {role:?} should resolve to a style with fg set"
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
        assert_eq!(theme.role_style(Role::Secondary), theme.disabled());
        assert_eq!(theme.role_style(Role::Accent), theme.title());
        assert_eq!(theme.role_style(Role::Success), theme.status_ok());
        assert_eq!(theme.role_style(Role::Warning), theme.status_warning());
        assert_eq!(
            theme.role_style(Role::BorderEmphasis),
            theme.border_focused()
        );
        assert_eq!(theme.role_style(Role::Info), theme.status_info());
        assert_eq!(theme.role_style(Role::Blocked), theme.status_blocked());
        assert_eq!(theme.role_style(Role::Unknown), theme.status_unknown());
        assert_eq!(theme.role_style(Role::Muted), theme.disabled());
        assert_eq!(theme.role_style(Role::Disabled).fg, Some(IRON));
        assert_eq!(theme.role_style(Role::BorderInteractive).fg, Some(IRON));
        assert_eq!(theme.role_style(Role::Surface).bg, Some(SURFACE_RAISED));
        assert_eq!(theme.role_style(Role::Overlay).bg, Some(SURFACE_OVERLAY));
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
            ("status_info", t.status_info()),
            ("status_blocked", t.status_blocked()),
            ("status_unknown", t.status_unknown()),
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

    /// The brand-and-design 2.x values, written out rather than compared with
    /// the constants they come from. `tests/brand_tokens.rs` also checks the
    /// crate against the vendored terminal token file when it is present.
    #[test]
    fn palette_methods_match_documented_colours() {
        let t = EddaCraftTheme;
        assert_eq!(t.bg(), Color::Rgb(0x0D, 0x0D, 0x0F));
        assert_eq!(t.fg(), Color::Rgb(0xEB, 0xEB, 0xEB));
        assert_eq!(t.accent(), Color::Rgb(0xD9, 0x5A, 0x00));
        assert_eq!(t.success(), Color::Rgb(0x3B, 0x97, 0x62));
        assert_eq!(t.error(), Color::Rgb(0xCF, 0x5E, 0x5E));
        assert_eq!(t.warning(), Color::Rgb(0xD0, 0x8C, 0x38));
        assert_eq!(t.muted(), Color::Rgb(0x85, 0x85, 0x8A));
        assert_eq!(t.border(), Color::Rgb(0x2A, 0x2A, 0x2E));
        assert_eq!(t.info(), Color::Rgb(0x61, 0xA3, 0xE6));
        assert_eq!(t.text_secondary(), Color::Rgb(0xB7, 0xB7, 0xBA));
        assert_eq!(t.text_disabled(), Color::Rgb(0x67, 0x67, 0x6C));
        assert_eq!(t.border_interactive(), Color::Rgb(0x67, 0x67, 0x6C));
        assert_eq!(t.surface(), Color::Rgb(0x14, 0x14, 0x16));
        assert_eq!(t.overlay(), Color::Rgb(0x18, 0x18, 0x1B));
        let series: Vec<Color> = (0..6).map(|n| t.chart_series(n)).collect();
        assert_eq!(
            series,
            [
                Color::Rgb(0xD9, 0x5A, 0x00),
                Color::Rgb(0x61, 0xA3, 0xE6),
                Color::Rgb(0x3B, 0x97, 0x62),
                Color::Rgb(0xB0, 0x8A, 0xDA),
                Color::Rgb(0xD6, 0x7E, 0xA9),
                Color::Rgb(0x00, 0xB3, 0xC2),
            ]
        );
    }
}
