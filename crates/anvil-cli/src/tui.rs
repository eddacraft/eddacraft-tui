use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::Duration;

use anvil_kernel_types::EngineEvent;
use anvil_tui::shell::render_shell;
use anvil_tui::surface::Surface;
use anvil_tui::surfaces::watch::WatchState;
use anvil_tui::surfaces::watch::event_adapter::WatchEventAdapter;
use crossterm::event::{self, Event, KeyEventKind};
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
    // Track whether state has changed since the last draw.
    // Starts true so the first frame always renders.
    let mut dirty = true;

    loop {
        if dirty {
            terminal.draw(|frame| {
                let area = frame.area();
                let content =
                    render_shell(frame, area, state.surface_name(), state.help_text(), theme);
                state.render(frame, content, theme);
            })?;
            dirty = false;
        }

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let action = KeyHandler::map(key);
                    state.handle_key(action);
                    dirty = true;

                    // Check exit immediately after key — avoids waiting for
                    // the next poll timeout before responding to quit/back.
                    if state.should_quit() {
                        return Ok(SurfaceExit::Quit);
                    }
                    if state.should_back() {
                        return Ok(SurfaceExit::Back);
                    }
                }
                Event::Resize(_, _) => {
                    dirty = true;
                }
                _ => {}
            }
        }
    }
}

/// Run the tutorial surface with optional file-watcher integration.
/// When `file_rx` is `Some`, file-change events trigger automatic
/// re-verification on watched steps (WELCOME-013).
pub fn run_tutorial(
    mut state: anvil_tui::surfaces::tutorial::TutorialState,
    file_rx: Option<&Receiver<anvil_kernel::watcher::events::ChangeBatch>>,
) -> anyhow::Result<anvil_tui::surfaces::tutorial::TutorialState> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let theme = EddaCraftTheme;

    let result = tutorial_loop(&mut terminal, &mut state, file_rx, &theme);

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result.map(|()| state)
}

fn tutorial_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut anvil_tui::surfaces::tutorial::TutorialState,
    file_rx: Option<&Receiver<anvil_kernel::watcher::events::ChangeBatch>>,
    theme: &EddaCraftTheme,
) -> anyhow::Result<()> {
    loop {
        // Drain file-change events before drawing so changes appear immediately.
        if let Some(rx) = file_rx {
            while let Ok(batch) = rx.try_recv() {
                let paths: Vec<std::path::PathBuf> =
                    batch.changes.iter().map(|c| c.path.clone()).collect();
                state.handle_file_change(&paths);
            }
        }

        terminal.draw(|frame| {
            let area = frame.area();
            let content = render_shell(frame, area, state.surface_name(), state.help_text(), theme);
            state.render(frame, content, theme);
        })?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            let action = KeyHandler::map(key);
            state.handle_key(action);
        }

        if state.should_quit() || state.should_back() || state.wants_watch_demo {
            return Ok(());
        }
    }
}

/// Run the watch demo with guided overlay (WELCOME-014).
/// Receives engine events from the kernel watcher and renders the watch
/// dashboard with progressive overlay hints.
pub fn run_watch_demo(
    mut state: anvil_tui::surfaces::tutorial::watch_demo::WatchDemoState,
    event_rx: &Receiver<EngineEvent>,
) -> anyhow::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let theme = EddaCraftTheme;

    let result = watch_demo_loop(&mut terminal, &mut state, event_rx, &theme);

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn watch_demo_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut anvil_tui::surfaces::tutorial::watch_demo::WatchDemoState,
    event_rx: &Receiver<EngineEvent>,
    theme: &EddaCraftTheme,
) -> anyhow::Result<()> {
    loop {
        // Drain engine events.
        while let Ok(engine_event) = event_rx.try_recv() {
            state.handle_engine_event(&engine_event);
        }

        // Advance time-based overlay hints.
        state.tick();

        let poll_timeout = if state.is_dirty() {
            Duration::ZERO
        } else {
            Duration::from_millis(50)
        };

        if event::poll(poll_timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let action = KeyHandler::map(key);
                    state.handle_key(action);
                }
                Event::Resize(_, _) => {
                    state.mark_dirty();
                }
                _ => {}
            }
        }

        if state.take_dirty() {
            terminal.draw(|frame| {
                let area = frame.area();
                let content = render_shell(frame, area, "Watch Demo", state.help_text(), theme);
                anvil_tui::surfaces::tutorial::watch_demo_render::render(
                    frame, content, state, theme,
                );
            })?;
        }

        if state.should_quit || state.wants_back {
            break;
        }
    }

    Ok(())
}

/// Run the tutorial surface inside an already-initialised terminal session,
/// with optional file-watcher integration (WELCOME-013). When `file_rx` is
/// `Some`, file-change events trigger automatic re-verification on watched
/// steps. The loop also exits on `wants_watch_demo` (WELCOME-014).
pub fn run_tutorial_in(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut anvil_tui::surfaces::tutorial::TutorialState,
    file_rx: Option<&Receiver<anvil_kernel::watcher::events::ChangeBatch>>,
    theme: &EddaCraftTheme,
) -> anyhow::Result<()> {
    tutorial_loop(terminal, state, file_rx, theme)
}

/// Run the watch demo inside an already-initialised terminal session.
pub fn run_watch_demo_in(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut state: anvil_tui::surfaces::tutorial::watch_demo::WatchDemoState,
    event_rx: &Receiver<EngineEvent>,
    theme: &EddaCraftTheme,
) -> anyhow::Result<()> {
    watch_demo_loop(terminal, &mut state, event_rx, theme)
}

/// Run the watch dashboard, draining kernel events from the given channel.
///
/// When `shutdown` is provided the loop also exits if the flag becomes `true`
/// — used by the CLI `watch` command to bridge a SIGINT handler into the TUI
/// (raw mode normally swallows Ctrl-C as a key event, but some terminal
/// multiplexers forward the signal anyway).
pub fn run_watch(
    mut state: WatchState,
    event_rx: &Receiver<EngineEvent>,
    shutdown: Option<&Arc<AtomicBool>>,
) -> anyhow::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let theme = EddaCraftTheme;

    let result = watch_loop(&mut terminal, &mut state, event_rx, shutdown, &theme);

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

/// Run the watch surface inside an already-initialised terminal session.
/// Used by the welcome hub to launch watch mode without teardown/setup.
pub fn run_watch_in(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut WatchState,
    event_rx: &Receiver<EngineEvent>,
) -> anyhow::Result<SurfaceExit> {
    let theme = EddaCraftTheme;
    watch_loop(terminal, state, event_rx, None, &theme)?;
    if state.should_quit() {
        Ok(SurfaceExit::Quit)
    } else {
        Ok(SurfaceExit::Back)
    }
}

fn watch_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut WatchState,
    event_rx: &Receiver<EngineEvent>,
    shutdown: Option<&Arc<AtomicBool>>,
    theme: &EddaCraftTheme,
) -> anyhow::Result<()> {
    let mut adapter = WatchEventAdapter::new();

    loop {
        // Drain all pending engine events.
        while let Ok(engine_event) = event_rx.try_recv() {
            adapter.handle_event(&engine_event, &mut state.data);
            state.mark_dirty();
        }

        // Skip the poll wait when already dirty — render immediately.
        let poll_timeout = if state.is_dirty() {
            Duration::ZERO
        } else {
            Duration::from_millis(50)
        };

        // Drain all pending terminal events so resize is never deferred
        // behind a burst of key events.
        if event::poll(poll_timeout)? {
            loop {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        let action = KeyHandler::map(key);
                        state.handle_key(action);
                    }
                    Event::Resize(_, _) => {
                        state.mark_dirty();
                    }
                    _ => {}
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }

        // Only redraw when state has actually changed.
        if state.take_dirty() {
            terminal.draw(|frame| {
                let area = frame.area();
                let content =
                    render_shell(frame, area, state.surface_name(), state.help_text(), theme);
                anvil_tui::surfaces::watch::render::render(frame, content, state, theme);
            })?;
        }

        if state.should_quit() || state.should_back() {
            break;
        }
        if let Some(flag) = shutdown
            && flag.load(Ordering::SeqCst)
        {
            break;
        }
    }

    Ok(())
}
