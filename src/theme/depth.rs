//! Downsampling rendered colour to what the terminal can show.
//!
//! [`ColourDepth`] says how much colour a terminal supports. [`adapt_color`]
//! maps one colour to that depth, and [`adapt_buffer`] applies it to every
//! cell of a rendered frame, so a surface can draw in brand truecolour and
//! still come out right on a 256-colour or 16-colour terminal, or with no
//! colour at all under `NO_COLOR` or `TERM=dumb`.
//!
//! The brand colours map through the table in the brand's terminal token file
//! (`tokens/dist/terminal/eddacraft-dark.json`, brand-and-design 2.x): each
//! ANSI slot's hex, its `xterm256` index and its ANSI index. Any other
//! colour maps to the nearest entry. Ember has no ANSI slot: it is emitted as
//! index 166 at 256 colours and as bold yellow at 16 colours.

use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

use crate::mode::ColourDepth;

/// One brand colour and its fallbacks. `ansi` is `None` for colours that
/// should take the terminal's own default (the canvas).
struct Entry {
    rgb: (u8, u8, u8),
    xterm256: u8,
    ansi: Option<Color>,
}

const fn entry(rgb: (u8, u8, u8), xterm256: u8, ansi: Option<Color>) -> Entry {
    Entry {
        rgb,
        xterm256,
        ansi,
    }
}

/// The sixteen ANSI slots of the brand terminal palette, the canvas and the
/// accent, from `eddacraft-dark.json`, plus Lingon. The canvas has no ANSI
/// slot of its own: at 16 colours it becomes the terminal default.
const BRAND: [Entry; 19] = [
    entry((0x2A, 0x2A, 0x2E), 235, Some(Color::Black)),
    entry((0xCF, 0x5E, 0x5E), 167, Some(Color::Red)),
    entry((0x3B, 0x97, 0x62), 65, Some(Color::Green)),
    entry((0xD0, 0x8C, 0x38), 172, Some(Color::Yellow)),
    entry((0x61, 0xA3, 0xE6), 75, Some(Color::Blue)),
    entry((0xB0, 0x8A, 0xDA), 140, Some(Color::Magenta)),
    entry((0x00, 0xB3, 0xC2), 37, Some(Color::Cyan)),
    entry((0xB7, 0xB7, 0xBA), 250, Some(Color::Gray)),
    entry((0x85, 0x85, 0x8A), 102, Some(Color::DarkGray)),
    entry((0xEB, 0x77, 0x75), 174, Some(Color::LightRed)),
    entry((0x56, 0xB0, 0x79), 72, Some(Color::LightGreen)),
    entry((0xEB, 0xA5, 0x53), 179, Some(Color::LightYellow)),
    entry((0x7C, 0xBD, 0xFF), 111, Some(Color::LightBlue)),
    entry((0xC9, 0xA3, 0xF5), 183, Some(Color::LightMagenta)),
    entry((0x3C, 0xCD, 0xDC), 44, Some(Color::LightCyan)),
    entry((0xEB, 0xEB, 0xEB), 255, Some(Color::White)),
    // The canvas, The Void.
    entry((0x0D, 0x0D, 0x0F), 232, None),
    // anvil Ember, the accent: no ANSI slot, so yellow (made bold by
    // `adapt_buffer`).
    entry(EMBER, 166, Some(Color::Yellow)),
    // Lingon, the fifth chart colour, has no slot in the token file. Pinned to
    // bright magenta (guide §5(f)) so series 4 does not read as a red at 16
    // colours; 175 is its nearest xterm index.
    entry((0xD6, 0x7E, 0xA9), 175, Some(Color::LightMagenta)),
];

const EMBER: (u8, u8, u8) = (0xD9, 0x5A, 0x00);

fn distance(a: (u8, u8, u8), b: (u8, u8, u8)) -> u32 {
    let d = |x: u8, y: u8| u32::from(x.abs_diff(y)).pow(2);
    d(a.0, b.0) + d(a.1, b.1) + d(a.2, b.2)
}

/// The brand entry for a colour: its own entry when it is a brand colour,
/// otherwise the nearest ANSI slot or the canvas. Ember is matched only
/// exactly: its yellow fallback is a rule for the accent, so a red that
/// happens to sit nearest Ember must not turn yellow.
fn nearest(rgb: (u8, u8, u8)) -> &'static Entry {
    if let Some(exact) = BRAND.iter().find(|candidate| candidate.rgb == rgb) {
        return exact;
    }
    BRAND
        .iter()
        .filter(|candidate| candidate.rgb != EMBER)
        .min_by_key(|candidate| distance(candidate.rgb, rgb))
        .expect("the brand table is not empty")
}

/// The RGB value of an xterm 256-colour index (16–255; the first sixteen are
/// left to the terminal theme and map to the nearest brand slot instead).
fn xterm_rgb(index: u8) -> (u8, u8, u8) {
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    match index {
        16..=231 => {
            let i = index - 16;
            (
                LEVELS[usize::from(i / 36)],
                LEVELS[usize::from(i / 6 % 6)],
                LEVELS[usize::from(i % 6)],
            )
        }
        232..=255 => {
            let level = 8 + (index - 232) * 10;
            (level, level, level)
        }
        _ => BRAND[usize::from(index)].rgb,
    }
}

/// The nearest xterm index to an arbitrary colour, from the 6×6×6 cube and the
/// greyscale ramp.
fn nearest_xterm(rgb: (u8, u8, u8)) -> u8 {
    (16..=255)
        .min_by_key(|&index| distance(xterm_rgb(index), rgb))
        .expect("the xterm range is not empty")
}

/// Map one colour to what a terminal of the given depth can show.
///
/// - [`ColourDepth::TrueColor`] keeps every colour.
/// - [`ColourDepth::Ansi256`] turns RGB into an xterm index: the brand's own
///   index for a brand colour, the nearest cube or greyscale index otherwise.
///   Named ANSI colours are kept.
/// - [`ColourDepth::Ansi16`] turns RGB and high indexes into the nearest
///   brand ANSI slot. The canvas becomes the terminal default.
/// - [`ColourDepth::None`] resets every colour.
///
/// # Stability
///
/// **experimental**, like [`ColourDepth`].
pub fn adapt_color(color: Color, depth: ColourDepth) -> Color {
    match depth {
        ColourDepth::TrueColor => color,
        ColourDepth::Ansi256 => match color {
            Color::Rgb(r, g, b) => {
                let rgb = (r, g, b);
                match BRAND.iter().find(|candidate| candidate.rgb == rgb) {
                    Some(brand) => Color::Indexed(brand.xterm256),
                    None => Color::Indexed(nearest_xterm(rgb)),
                }
            }
            other => other,
        },
        ColourDepth::Ansi16 => match color {
            Color::Rgb(r, g, b) => nearest((r, g, b)).ansi.unwrap_or(Color::Reset),
            Color::Indexed(index) if index >= 16 => {
                nearest(xterm_rgb(index)).ansi.unwrap_or(Color::Reset)
            }
            other => other,
        },
        ColourDepth::None => Color::Reset,
    }
}

/// Whether a background still reads as a filled area at 16 colours, as
/// opposed to the canvas, which the terminal default replaces.
fn is_filled_background(color: Color) -> bool {
    !matches!(adapt_color(color, ColourDepth::Ansi16), Color::Reset)
}

fn is_ember(color: Color) -> bool {
    color == Color::Rgb(EMBER.0, EMBER.1, EMBER.2)
}

/// Map every cell of a rendered buffer to the given depth.
///
/// Call it on the frame buffer after drawing. Beyond [`adapt_color`]:
///
/// - at 16 colours and with no colour, text in the accent is made bold, the
///   brand's fallback for Ember;
/// - with no colour, a cell drawn on a filled background (a highlighted row,
///   a selection) is shown in reverse video, so the highlight survives.
///
/// Status meaning never depends on colour alone: badges carry the bracket
/// labels (`[ OK ]`, `[ ERR ]`, `[ WARN ]`), which read the same here.
///
/// # Stability
///
/// **experimental**, like [`ColourDepth`].
pub fn adapt_buffer(buffer: &mut Buffer, depth: ColourDepth) {
    if depth == ColourDepth::TrueColor {
        return;
    }
    for cell in &mut buffer.content {
        let bold_accent = depth <= ColourDepth::Ansi16 && is_ember(cell.fg);
        let reverse = depth == ColourDepth::None && is_filled_background(cell.bg);
        cell.fg = adapt_color(cell.fg, depth);
        cell.bg = adapt_color(cell.bg, depth);
        if bold_accent {
            cell.modifier.insert(Modifier::BOLD);
        }
        if reverse {
            cell.modifier.insert(Modifier::REVERSED);
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;
    use ratatui::style::Style;

    use super::*;
    use crate::theme::{EddaCraftTheme, Theme};

    const DEPTHS: [ColourDepth; 4] = [
        ColourDepth::None,
        ColourDepth::Ansi16,
        ColourDepth::Ansi256,
        ColourDepth::TrueColor,
    ];

    #[test]
    fn truecolor_keeps_every_colour() {
        for color in [
            Color::Rgb(1, 2, 3),
            Color::Indexed(200),
            Color::Red,
            Color::Reset,
        ] {
            assert_eq!(adapt_color(color, ColourDepth::TrueColor), color);
        }
    }

    #[test]
    fn no_colour_resets_everything() {
        for color in [Color::Rgb(1, 2, 3), Color::Indexed(200), Color::Red] {
            assert_eq!(adapt_color(color, ColourDepth::None), Color::Reset);
        }
    }

    /// The brand roles map through the token file's own indexes (guide §5(f)).
    #[test]
    fn brand_roles_follow_the_terminal_token_table() {
        let t = EddaCraftTheme;
        let cases = [
            (t.fg(), 255, Color::White),
            (t.text_secondary(), 250, Color::Gray),
            (t.muted(), 102, Color::DarkGray),
            (t.border(), 235, Color::Black),
            (t.accent(), 166, Color::Yellow),
            (t.success(), 65, Color::Green),
            (t.warning(), 172, Color::Yellow),
            (t.error(), 167, Color::Red),
            (t.info(), 75, Color::Blue),
            (t.chart_series(3), 140, Color::Magenta),
            (t.chart_series(4), 175, Color::LightMagenta),
            (t.chart_series(5), 37, Color::Cyan),
        ];
        for (color, xterm, ansi) in cases {
            assert_eq!(
                adapt_color(color, ColourDepth::Ansi256),
                Color::Indexed(xterm),
                "{color:?} at 256 colours"
            );
            assert_eq!(
                adapt_color(color, ColourDepth::Ansi16),
                ansi,
                "{color:?} at 16 colours"
            );
        }
    }

    #[test]
    fn the_canvas_and_panels_take_the_terminal_default_at_16_colours() {
        let t = EddaCraftTheme;
        for color in [t.bg(), t.surface(), t.overlay()] {
            assert_eq!(adapt_color(color, ColourDepth::Ansi16), Color::Reset);
        }
        assert_eq!(
            adapt_color(t.bg(), ColourDepth::Ansi256),
            Color::Indexed(232)
        );
    }

    #[test]
    fn off_palette_colours_take_the_nearest_slot() {
        assert_eq!(
            adapt_color(Color::Rgb(255, 0, 0), ColourDepth::Ansi256),
            Color::Indexed(196)
        );
        assert_eq!(
            adapt_color(Color::Rgb(90, 150, 240), ColourDepth::Ansi16),
            Color::Blue
        );
        assert_eq!(
            adapt_color(Color::Indexed(196), ColourDepth::Ansi16),
            Color::Red
        );
        assert_eq!(
            adapt_color(Color::Indexed(9), ColourDepth::Ansi16),
            Color::Indexed(9)
        );
    }

    fn rendered(style: Style) -> Buffer {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
        buffer.set_string(0, 0, "ab", style);
        buffer
    }

    #[test]
    fn highlighted_rows_stay_visible_without_colour() {
        let t = EddaCraftTheme;
        let mut buffer = rendered(t.highlighted());
        adapt_buffer(&mut buffer, ColourDepth::None);
        let cell = &buffer.content[0];
        assert_eq!(cell.fg, Color::Reset);
        assert_eq!(cell.bg, Color::Reset);
        assert!(cell.modifier.contains(Modifier::REVERSED));

        let mut plain = rendered(t.base());
        adapt_buffer(&mut plain, ColourDepth::None);
        assert!(!plain.content[0].modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn accent_text_is_bold_at_16_colours_and_without_colour() {
        let t = EddaCraftTheme;
        for depth in DEPTHS {
            let mut buffer = rendered(Style::default().fg(t.accent()));
            adapt_buffer(&mut buffer, depth);
            let bold = buffer.content[0].modifier.contains(Modifier::BOLD);
            assert_eq!(bold, depth <= ColourDepth::Ansi16, "{depth:?}");
        }
    }

    #[test]
    fn truecolor_buffers_are_untouched() {
        let t = EddaCraftTheme;
        let original = rendered(t.highlighted());
        let mut buffer = original.clone();
        adapt_buffer(&mut buffer, ColourDepth::TrueColor);
        assert_eq!(buffer, original);
    }

    #[test]
    fn adapted_buffers_never_hold_rgb_below_truecolor() {
        let t = EddaCraftTheme;
        for depth in [ColourDepth::None, ColourDepth::Ansi16, ColourDepth::Ansi256] {
            for style in [t.base(), t.highlighted(), t.status_ok(), t.status_blocked()] {
                let mut buffer = rendered(style);
                adapt_buffer(&mut buffer, depth);
                for cell in &buffer.content {
                    assert!(!matches!(cell.fg, Color::Rgb(..)), "{depth:?} fg");
                    assert!(!matches!(cell.bg, Color::Rgb(..)), "{depth:?} bg");
                }
            }
        }
    }

    /// A frame with the shapes whose meaning matters: plain text, a
    /// highlighted row, every status badge and a blocked fill.
    fn sample_frame(depth: ColourDepth) -> Buffer {
        use ratatui::widgets::Widget;

        use crate::widgets::status_badge::{BadgeStatus, StatusBadge};

        let t = EddaCraftTheme;
        let area = Rect::new(0, 0, 24, 10);
        let mut buffer = Buffer::empty(area);
        buffer.set_style(area, t.base());
        buffer.set_string(0, 0, "plain text", t.base());
        buffer.set_string(0, 1, "selected row", t.highlighted());
        for (row, status) in [
            BadgeStatus::Success,
            BadgeStatus::Error,
            BadgeStatus::Warning,
            BadgeStatus::Info,
            BadgeStatus::Unknown,
            BadgeStatus::Blocked,
        ]
        .into_iter()
        .enumerate()
        {
            let y = 2 + u16::try_from(row).expect("row fits");
            StatusBadge::new(status, &t).render(Rect::new(0, y, 24, 1), &mut buffer);
        }
        buffer.set_string(0, 8, "blocked fill", t.status_blocked());
        adapt_buffer(&mut buffer, depth);
        buffer
    }

    /// Guide §8 item 4: how a frame reads at 16 colours and with none.
    #[test]
    fn snapshot_frame_at_16_colours() {
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(
            &sample_frame(ColourDepth::Ansi16)
        ));
    }

    #[test]
    fn snapshot_frame_without_colour() {
        insta::assert_snapshot!(crate::test_utils::snapshot::buffer_to_string(
            &sample_frame(ColourDepth::None)
        ));
    }

    /// The SGR parameters a terminal receives when the adapted frame is
    /// flushed through the crossterm backend.
    fn emitted_bytes(depth: ColourDepth) -> String {
        use ratatui::Terminal;
        use ratatui::backend::CrosstermBackend;

        let mut out = Vec::new();
        {
            let backend = CrosstermBackend::new(&mut out);
            let mut terminal = Terminal::with_options(
                backend,
                ratatui::TerminalOptions {
                    viewport: ratatui::Viewport::Fixed(Rect::new(0, 0, 24, 10)),
                },
            )
            .expect("terminal");
            terminal
                .draw(|frame| {
                    let sample = sample_frame(ColourDepth::TrueColor);
                    frame.buffer_mut().merge(&sample);
                    adapt_buffer(frame.buffer_mut(), depth);
                })
                .expect("draw");
        }
        String::from_utf8(out).expect("utf-8 output")
    }

    /// Colour-setting SGR parameters (guide §8 item 7(b)): 30–37, 40–47,
    /// 90–97, 100–107, and the 38;/48; extended forms. Defaults (39, 49),
    /// bold, underline, reverse and resets are allowed.
    fn colour_parameters(bytes: &str) -> Vec<String> {
        let mut found = Vec::new();
        for sequence in bytes.split("\x1b[").skip(1) {
            let Some(end) = sequence.find(|c: char| c.is_ascii_alphabetic()) else {
                continue;
            };
            if &sequence[end..=end] != "m" {
                continue;
            }
            let params: Vec<&str> = sequence[..end].split(';').collect();
            let mut i = 0;
            while i < params.len() {
                let n: u16 = params[i].parse().unwrap_or(0);
                if n == 38 || n == 48 {
                    found.push(params[i..].join(";"));
                    break;
                }
                if (30..=37).contains(&n)
                    || (40..=47).contains(&n)
                    || (90..=97).contains(&n)
                    || (100..=107).contains(&n)
                {
                    found.push(n.to_string());
                }
                i += 1;
            }
        }
        found
    }

    #[test]
    fn emitted_escapes_match_each_depth() {
        let none = emitted_bytes(ColourDepth::None);
        assert!(
            colour_parameters(&none).is_empty(),
            "no colour must emit no colour SGR: {:?}",
            colour_parameters(&none)
        );
        assert!(none.contains("\x1b[7m"), "highlights need reverse video");

        // crossterm writes even the sixteen named colours in the indexed form
        // (`38;5;1` for red), so at 16 colours every index must be one of the
        // ANSI slots 0–15 and nothing may be truecolour.
        let ansi16 = emitted_bytes(ColourDepth::Ansi16);
        let params16 = colour_parameters(&ansi16);
        assert!(!params16.is_empty(), "16 colours still colours the frame");
        assert!(!ansi16.contains("38;2;") && !ansi16.contains("48;2;"));
        for param in &params16 {
            let parts: Vec<&str> = param.split(';').collect();
            if let ["38" | "48", "5", index, ..] = parts.as_slice() {
                let index: u8 = index.parse().expect("an index");
                assert!(index < 16, "16 colours emitted index {index}: {param}");
            }
        }

        let ansi256 = emitted_bytes(ColourDepth::Ansi256);
        assert!(ansi256.contains("38;5;"), "256 colours use indexed colour");
        assert!(
            !ansi256.contains("38;2;"),
            "256 colours must not emit truecolour"
        );

        let truecolor = emitted_bytes(ColourDepth::TrueColor);
        assert!(truecolor.contains("38;2;"), "truecolour uses RGB colour");
    }
}
