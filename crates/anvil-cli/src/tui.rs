use std::io;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use anvil_kernel_types::EngineEvent;
use anvil_tui::shell::render_shell;
use anvil_tui::surface::Surface;
use anvil_tui::surfaces::watch::WatchState;
use anvil_tui::surfaces::watch::event_adapter::WatchEventAdapter;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use eddacraft_tui::keyboard::KeyHandler;
use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

/// How a surface exited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceExit {
    /// User wants to quit the program.
    Quit,
    /// User wants to go back to the previous screen.
    Back,
}

/// Run an interactive TUI surface inside the branded shell chrome.
/// Returns the state after the surface exits, so callers can inspect
/// final state (e.g. which menu option was chosen).
pub fn run_surface<S: Surface>(mut state: S) -> anyhow::Result<S> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let theme = EddaCraftTheme;

    let result = surface_loop(&mut terminal, &mut state, &theme);

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result.map(|_| state)
}

/// Run a surface within an already-initialised terminal session.
/// Used by the welcome hub to launch sub-surfaces without teardown.
pub fn run_surface_in<S: Surface>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut S,
    theme: &EddaCraftTheme,
) -> anyhow::Result<SurfaceExit> {
    surface_loop(terminal, state, theme)
}

/// Set up a TUI terminal session and return the terminal for caller-managed
/// surface switching. Caller must call `teardown_terminal` when done.
pub fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

/// Draw a loading frame with a message inside the shell chrome.
pub fn draw_loading(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    surface_name: &str,
    message: &str,
    theme: &EddaCraftTheme,
) -> anyhow::Result<()> {
    terminal.draw(|frame| {
        let area = frame.area();
        let content = render_shell(frame, area, surface_name, "", theme);
        let loading = ratatui::widgets::Paragraph::new(ratatui::text::Line::styled(
            format!("  {message}"),
            ratatui::style::Style::default().fg(theme.muted()),
        ));
        frame.render_widget(loading, content);
    })?;
    Ok(())
}

/// Tear down a TUI terminal session.
pub fn teardown_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> anyhow::Result<()> {
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn surface_loop<S: Surface>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut S,
    theme: &EddaCraftTheme,
) -> anyhow::Result<SurfaceExit> {
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let content = render_shell(frame, area, state.surface_name(), state.help_text(), theme);
            state.render(frame, content, theme);
        })?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            let action = KeyHandler::map(key);
            state.handle_key(action);
        }

        if state.should_quit() {
            return Ok(SurfaceExit::Quit);
        }
        if state.should_back() {
            return Ok(SurfaceExit::Back);
        }
    }
}

/// Run the watch dashboard, draining kernel events from the given channel.
#[allow(dead_code)]
pub fn run_watch(mut state: WatchState, event_rx: &Receiver<EngineEvent>) -> anyhow::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let theme = EddaCraftTheme;

    let result = watch_loop(&mut terminal, &mut state, event_rx, &theme);

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn watch_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut WatchState,
    event_rx: &Receiver<EngineEvent>,
    theme: &EddaCraftTheme,
) -> anyhow::Result<()> {
    let mut adapter = WatchEventAdapter::new();

    loop {
        // Drain all pending engine events.
        while let Ok(engine_event) = event_rx.try_recv() {
            adapter.handle_event(&engine_event, &mut state.data);
            state.mark_dirty();
        }

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    let action = KeyHandler::map(key);
                    state.handle_key(action);
                }
                Event::Resize(_, _) => {
                    state.mark_dirty();
                }
                _ => {}
            }
        }

        // Only redraw when state has actually changed.
        if state.take_dirty() {
            terminal.draw(|frame| {
                let area = frame.area();
                anvil_tui::surfaces::watch::render::render(frame, area, state, theme);
            })?;
        }

        if state.should_quit {
            break;
        }
    }

    Ok(())
}
