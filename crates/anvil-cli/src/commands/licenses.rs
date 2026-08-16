//! `anvil licenses` — prints anvil's acknowledgements and the third-party
//! licence attribution. Both live in the repo-root `ACKNOWLEDGEMENTS.md`,
//! embedded here at build time via `include_str!`; the auto-generated half
//! of that file is produced by cargo-about.

use std::fmt::Write as _;

use crate::GlobalArgs;

const ACKNOWLEDGEMENTS: &str = include_str!("../../../../ACKNOWLEDGEMENTS.md");

#[derive(Debug, Default, Clone, Copy, clap::ValueEnum)]
pub enum Format {
    /// anvil version banner followed by ACKNOWLEDGEMENTS.md as markdown
    /// (default). Pipe through a pager or markdown renderer for the best
    /// reading experience.
    #[default]
    Plain,
    /// Just the raw ACKNOWLEDGEMENTS.md contents, suitable for piping or
    /// format conversion.
    Markdown,
}

#[derive(Debug, clap::Args)]
pub struct LicensesArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Plain)]
    pub format: Format,
}

pub fn run(args: &LicensesArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    // Issue #3947: an accepted `--json` means the whole of stdout is one
    // document; the licence text travels inside it (the `config convert
    // --stdout --json` envelope precedent). Colour is disabled so the
    // embedded text carries no ANSI escapes.
    if global.json {
        let format = match args.format {
            Format::Plain => "plain",
            Format::Markdown => "markdown",
        };
        crate::output::json::print(&serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "format": format,
            "text": render(args.format, false),
        }))?;
    } else {
        let output = render(args.format, supports_colour(global.no_tui));
        print!("{output}");
    }
    Ok(())
}

/// Produce the full `anvil licenses` output for the given format and colour
/// policy. Kept pure so unit tests can exercise every branch without
/// touching stdout or the `NO_COLOR` env var.
pub(crate) fn render(format: Format, use_colour: bool) -> String {
    match format {
        Format::Markdown => ACKNOWLEDGEMENTS.to_string(),
        Format::Plain => render_plain(use_colour),
    }
}

fn render_plain(use_colour: bool) -> String {
    let version = env!("CARGO_PKG_VERSION");
    let mut out = String::new();
    if use_colour {
        let _ = writeln!(out, "\x1b[1manvil {version}\x1b[0m");
    } else {
        let _ = writeln!(out, "anvil {version}");
    }
    let _ = writeln!(
        out,
        "Copyright (C) 2026 eddacraft, Inc. All rights reserved."
    );
    let _ = writeln!(out, "Licensed under LicenseRef-Proprietary.");
    let _ = writeln!(out);
    // ACKNOWLEDGEMENTS.md already starts with its own `# Acknowledgements`
    // heading; emit the body directly rather than duplicating it with a
    // synthetic section header.
    out.push_str(ACKNOWLEDGEMENTS);
    out
}

/// Whether to emit bold ANSI sequences in the plain-format header.
///
/// Disabled when `--no-tui` is set, when `NO_COLOR` is set (per
/// <https://no-color.org>), or when the output stream is not a TTY. Callers
/// pass `use_colour` explicitly so the pure `render` function stays testable.
fn supports_colour(no_tui: bool) -> bool {
    use std::io::IsTerminal as _;
    if no_tui || std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stdout().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_without_colour_contains_proprietary_header() {
        let out = render(Format::Plain, false);
        assert!(out.contains("Copyright (C) 2026 eddacraft, Inc. All rights reserved."));
        assert!(out.contains("Licensed under LicenseRef-Proprietary."));
        assert!(out.contains("anvil "));
        // ACKNOWLEDGEMENTS.md's own top heading shows through in plain mode.
        assert!(out.contains("# Acknowledgements"));
    }

    #[test]
    fn markdown_output_matches_embedded_file() {
        let out = render(Format::Markdown, false);
        assert_eq!(out, ACKNOWLEDGEMENTS);
    }

    #[test]
    fn plain_without_colour_has_no_ansi_sequences() {
        let out = render(Format::Plain, false);
        assert!(
            !out.contains('\x1b'),
            "expected no ANSI escape sequences when colour disabled"
        );
    }

    #[test]
    fn plain_with_colour_uses_bold_header() {
        let out = render(Format::Plain, true);
        assert!(
            out.contains("\x1b[1manvil "),
            "expected bold anvil version banner when colour enabled"
        );
    }
}
