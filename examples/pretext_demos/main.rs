//! Interactive demos for the `pretext` layout engine.
//!
//! Three tabs showcase the public value of two-phase layout:
//! streaming styled AI output, text flowing around a moving exclusion,
//! and multi-panel reflow on resize.
//!
//! ```text
//! cargo run -p eddacraft-tui --example pretext_demos
//! ```
//!
//! Keys: `Tab` / `1`–`3` switch demos · `Space` pause · `r` reset streaming ·
//! `+`/`-` speed · arrows move exclusion · `q` quit.

// Showcase demos prioritise readability over pedantic clippy nits; the library
// itself stays under `-D warnings`.
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::collapsible_if,
    clippy::needless_pass_by_ref_mut,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnested_or_patterns
)]

mod app;
mod exclusion;
mod masonry;
mod streaming;

use app::{App, DemoTab};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use std::io::{self, stdout};
use std::time::Duration;

fn main() -> io::Result<()> {
    let res = (|| -> io::Result<()> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        let mut app = App::new();

        loop {
            terminal.draw(|frame| app.render(frame))?;

            app.tick();

            if event::poll(Duration::from_millis(16))?
                && let Event::Key(key) = event::read()?
            {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.should_quit = true;
                    }
                    KeyCode::Tab => app.next_tab(),
                    KeyCode::BackTab => app.prev_tab(),
                    KeyCode::Char('1') => app.active_tab = DemoTab::Streaming,
                    KeyCode::Char('2') => app.active_tab = DemoTab::Exclusion,
                    KeyCode::Char('3') => app.active_tab = DemoTab::Masonry,

                    KeyCode::Char(' ') => match app.active_tab {
                        DemoTab::Streaming => app.streaming.toggle_pause(),
                        DemoTab::Exclusion => app.exclusion.toggle_animate(),
                        DemoTab::Masonry => {}
                    },
                    KeyCode::Char('r') if app.active_tab == DemoTab::Streaming => {
                        app.streaming.reset();
                    }
                    KeyCode::Char('+') | KeyCode::Char('=')
                        if app.active_tab == DemoTab::Streaming =>
                    {
                        app.streaming.speed_up();
                    }
                    KeyCode::Char('-') if app.active_tab == DemoTab::Streaming => {
                        app.streaming.slow_down();
                    }
                    KeyCode::Left if app.active_tab == DemoTab::Exclusion => {
                        app.exclusion.move_shape(-2, 0);
                    }
                    KeyCode::Right if app.active_tab == DemoTab::Exclusion => {
                        app.exclusion.move_shape(2, 0);
                    }
                    KeyCode::Up if app.active_tab == DemoTab::Exclusion => {
                        app.exclusion.move_shape(0, -1);
                    }
                    KeyCode::Down if app.active_tab == DemoTab::Exclusion => {
                        app.exclusion.move_shape(0, 1);
                    }
                    _ => {}
                }
            }

            if app.should_quit {
                break;
            }
        }

        Ok(())
    })();

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    res
}
