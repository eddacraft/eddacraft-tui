//! `EddaCraftTheme` against the brand's terminal token file (design-system
//! upgrade guide §8 item 8).
//!
//! The file is vendored in the anvil monorepo at
//! `packages/libs/design-tokens/vendor/tokens/dist/terminal/eddacraft-dark.json`
//! and pinned by hash there. The published crate does not carry it, so outside
//! the monorepo this test reports that it skipped and passes.

use std::path::PathBuf;

use eddacraft_tui::mode::ColourDepth;
use eddacraft_tui::theme::{EddaCraftTheme, Theme, adapt_color};
use ratatui::style::Color;

fn token_file() -> Option<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/libs/design-tokens/vendor/tokens/dist/terminal/eddacraft-dark.json");
    std::fs::read_to_string(path).ok()
}

/// The text after `"key":` inside the object that follows `"scope": {`, or at
/// the top level when `scope` is `None`. The token file is generated with a
/// fixed shape, so a small scanner is enough and keeps the crate free of a
/// JSON dependency.
fn field<'a>(json: &'a str, scope: Option<&str>, key: &str) -> &'a str {
    let body = match scope {
        Some(scope) => {
            let start = json
                .find(&format!("\"{scope}\": {{"))
                .unwrap_or_else(|| panic!("no \"{scope}\" object in the token file"));
            let rest = &json[start..];
            &rest[..rest.find('}').expect("an object closes")]
        }
        None => json,
    };
    let needle = format!("\"{key}\": ");
    let at = body
        .find(&needle)
        .unwrap_or_else(|| panic!("no \"{key}\" in {scope:?}"));
    let value = &body[at + needle.len()..];
    let end = value.find([',', '\n', '}']).unwrap_or(value.len());
    value[..end].trim()
}

fn hex(json: &str, scope: Option<&str>, key: &str) -> Color {
    let raw = field(json, scope, key).trim_matches('"');
    let digits = raw.strip_prefix('#').expect("a #RRGGBB value");
    let channel = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).expect("hex digits");
    Color::Rgb(channel(0), channel(2), channel(4))
}

fn xterm(json: &str, scope: &str) -> u8 {
    field(json, Some(scope), "xterm256")
        .parse()
        .expect("an xterm index")
}

#[test]
fn theme_matches_the_brand_terminal_tokens() {
    let Some(json) = token_file() else {
        eprintln!("brand terminal token file not found; skipped outside the anvil monorepo");
        return;
    };
    let t = EddaCraftTheme;
    let slot = |name: &str| hex(&json, Some(name), "hex");
    let cases = [
        ("fg", t.fg(), hex(&json, None, "foreground")),
        ("bg", t.bg(), hex(&json, None, "background")),
        ("accent", t.accent(), hex(&json, Some("accent"), "ember")),
        ("success", t.success(), slot("green")),
        ("warning", t.warning(), slot("yellow")),
        ("error", t.error(), slot("red")),
        ("info", t.info(), slot("blue")),
        ("muted", t.muted(), slot("bright-black")),
        ("text_secondary", t.text_secondary(), slot("white")),
        ("border", t.border(), slot("black")),
        ("chart_series(3)", t.chart_series(3), slot("magenta")),
        ("chart_series(5)", t.chart_series(5), slot("cyan")),
    ];
    let drift: Vec<String> = cases
        .iter()
        .filter(|(_, theme, token)| theme != token)
        .map(|(name, theme, token)| format!("{name}: theme {theme:?}, token {token:?}"))
        .collect();
    assert!(
        drift.is_empty(),
        "EddaCraftTheme drifted from the brand tokens: {drift:?}"
    );
}

/// `adapt_color` emits the token file's own xterm index for each brand slot.
#[test]
fn depth_table_matches_the_brand_xterm_indexes() {
    let Some(json) = token_file() else {
        eprintln!("brand terminal token file not found; skipped outside the anvil monorepo");
        return;
    };
    let mut slots = vec![
        "black",
        "red",
        "green",
        "yellow",
        "blue",
        "magenta",
        "cyan",
        "white",
        "bright-black",
        "bright-red",
        "bright-green",
        "bright-yellow",
        "bright-blue",
        "bright-magenta",
        "bright-cyan",
        "bright-white",
    ]
    .into_iter()
    .map(|name| (name, hex(&json, Some(name), "hex"), xterm(&json, name)))
    .collect::<Vec<_>>();
    slots.push((
        "accent",
        hex(&json, Some("accent"), "ember"),
        xterm(&json, "accent"),
    ));
    for (name, color, index) in slots {
        assert_eq!(
            adapt_color(color, ColourDepth::Ansi256),
            Color::Indexed(index),
            "{name}"
        );
    }
}
