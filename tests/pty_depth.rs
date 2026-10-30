//! A TUI frame under a real pseudo-terminal emits the escapes for the colour
//! depth `ColourDepth::detect()` sees (design-system upgrade guide §8 item
//! 7(b), DSYS-010).
//!
//! The unit tests prove detection (`ColourDepth::resolve`) and the bytes per
//! depth (`emitted_escapes_match_each_depth`) separately; this joins them. The
//! test binary re-runs itself under `script(1)`, which gives the child a
//! terminal on standard output, so detection reads a real TTY. Without
//! `script` the test reports that it skipped and passes.
#![cfg(unix)]

use std::io::Write;
use std::process::Command;

use crossterm::style::Print;
use eddacraft_tui::mode::ColourDepth;
use eddacraft_tui::theme::{EddaCraftTheme, adapt_buffer};
use eddacraft_tui::widgets::container::{Container, ContainerVariant};
use eddacraft_tui::widgets::status_badge::{BadgeStatus, StatusBadge};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

const CHILD: &str = "EDDACRAFT_TUI_PTY_CHILD";

/// The child half: renders when the parent launched it under `script`.
#[test]
fn pty_child_frame() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    let depth = ColourDepth::detect();
    let theme = EddaCraftTheme;
    let area = Rect::new(0, 0, 24, 3);
    let mut buffer = Buffer::empty(area);
    Container::new(&theme)
        .variant(ContainerVariant::Unknown)
        .render(area, &mut buffer);
    StatusBadge::new(BadgeStatus::Success, &theme).render(Rect::new(2, 1, 12, 1), &mut buffer);
    adapt_buffer(&mut buffer, depth);

    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, Print(format!("DEPTH={depth:?}\r\n"))).expect("marker");
    let mut backend = CrosstermBackend::new(stdout);
    let cells = buffer.content.iter().enumerate().map(|(i, cell)| {
        let (x, y) = buffer.pos_of(i);
        (x, y, cell)
    });
    backend.draw(cells).expect("draw");
    Backend::flush(&mut backend).expect("flush");
    std::io::stdout().write_all(b"\r\n").expect("newline");
}

/// Runs the child under `script` with the given environment, returning its
/// output, or `None` when `script` is not installed.
fn run_under_pty(env: &[(&str, &str)]) -> Option<String> {
    let exe = std::env::current_exe().expect("test binary");
    let child = format!(
        "{} --exact pty_child_frame --nocapture --test-threads=1",
        exe.display()
    );
    let mut command = Command::new("script");
    if cfg!(target_os = "macos") {
        command.args(["-q", "/dev/null", "/bin/sh", "-c", &child]);
    } else {
        command.args(["-q", "-e", "-c", &child, "/dev/null"]);
    }
    command
        .env(CHILD, "1")
        .env_remove("NO_COLOR")
        .env_remove("COLORTERM");
    for (key, value) in env {
        command.env(key, value);
    }
    let output = match command.output() {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => panic!("spawn script: {error}"),
    };
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(output.status.success(), "child failed under script: {text}");
    Some(text)
}

/// A foreground or background colour set by an SGR sequence.
#[derive(Debug)]
enum SgrColour {
    Indexed(u8),
    Rgb,
}

/// Every extended foreground and background colour in the output, read by
/// walking each `ESC[ … m` parameter list (`38;5;n`, `38;2;r;g;b` and the `48`
/// forms), so one sequence can carry several.
fn sgr_colours(output: &str) -> Vec<SgrColour> {
    let mut colours = Vec::new();
    for chunk in output.split("\x1b[").skip(1) {
        let Some((params, _)) = chunk.split_once('m') else {
            continue;
        };
        let mut parts = params.split(';');
        while let Some(part) = parts.next() {
            if part != "38" && part != "48" {
                continue;
            }
            match parts.next() {
                Some("5") => {
                    if let Some(index) = parts.next().and_then(|n| n.parse().ok()) {
                        colours.push(SgrColour::Indexed(index));
                    }
                }
                Some("2") => {
                    parts.nth(2);
                    colours.push(SgrColour::Rgb);
                }
                _ => {}
            }
        }
    }
    colours
}

#[test]
fn a_real_terminal_gets_the_escapes_for_its_depth() {
    let cases: [(&str, &[(&str, &str)]); 4] = [
        (
            "TrueColor",
            &[("TERM", "xterm-256color"), ("COLORTERM", "truecolor")],
        ),
        ("Ansi256", &[("TERM", "xterm-256color")]),
        ("Ansi16", &[("TERM", "xterm")]),
        ("None", &[("TERM", "xterm-256color"), ("NO_COLOR", "1")]),
    ];
    for (expected, env) in cases {
        let Some(output) = run_under_pty(env) else {
            eprintln!("script(1) not found; skipped the pseudo-terminal run");
            return;
        };
        assert!(
            output.contains(&format!("DEPTH={expected}")),
            "{expected}: detect() saw another depth\n{output:?}"
        );
        assert!(
            output.contains('╌'),
            "{expected}: dashed border missing\n{output:?}"
        );
        let colours = sgr_colours(&output);
        let truecolour = colours.iter().any(|c| matches!(c, SgrColour::Rgb));
        let indexes: Vec<u8> = colours
            .iter()
            .filter_map(|c| match c {
                SgrColour::Indexed(index) => Some(*index),
                SgrColour::Rgb => None,
            })
            .collect();
        match expected {
            "TrueColor" => assert!(truecolour, "truecolour escapes expected: {colours:?}"),
            "Ansi256" => {
                assert!(!truecolour, "no truecolour at 256 colours: {colours:?}");
                assert!(
                    indexes.iter().any(|&i| i >= 16),
                    "a 256-colour index: {colours:?}"
                );
            }
            "Ansi16" => {
                assert!(!truecolour, "no truecolour at 16 colours: {colours:?}");
                assert!(
                    !indexes.is_empty(),
                    "some colour at 16 colours: {colours:?}"
                );
                assert!(
                    indexes.iter().all(|&i| i < 16),
                    "only the 16 slots: {colours:?}"
                );
            }
            _ => assert!(colours.is_empty(), "no colour at all: {colours:?}"),
        }
    }
}
