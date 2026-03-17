use std::io;
use std::time::Duration;

use anvil_kernel_types::EngineEvent;
use anvil_tui::shell::render_shell;
use anvil_tui::surface::Surface;
use anvil_tui::surfaces::watch::WatchState;
use anvil_tui::surfaces::watch::event_adapter::WatchEventAdapter;
use crossterm::event::{self, Event};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute};
use eddacraft_tui::keyboard::KeyHandler;
use eddacraft_tui::theme::EddaCraftTheme;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

/// Run an interactive TUI surface inside the branded shell chrome.
pub fn run_surface<S: Surface>(mut state: S) -> anyhow::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let theme = EddaCraftTheme;

    let result = surface_loop(&mut terminal, &mut state, &theme);

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn surface_loop<S: Surface>(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut S,
    theme: &EddaCraftTheme,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let content = render_shell(
                frame,
                area,
                state.surface_name(),
                state.help_text(),
                theme,
            );
            state.render(frame, content, theme);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let action = KeyHandler::map(key);
                state.handle_key(action);
            }
        }

        if state.should_quit() {
            break;
        }
    }

    Ok(())
}

/// Run the watch dashboard, draining kernel events from the given channel.
pub fn run_watch(
    mut state: WatchState,
    event_rx: std::sync::mpsc::Receiver<EngineEvent>,
) -> anyhow::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let theme = EddaCraftTheme;

    let result = watch_loop(&mut terminal, &mut state, &event_rx, &theme);

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn watch_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut WatchState,
    event_rx: &std::sync::mpsc::Receiver<EngineEvent>,
    theme: &EddaCraftTheme,
) -> anyhow::Result<()> {
    let mut adapter = WatchEventAdapter::new();

    loop {
        // Drain all pending engine events.
        while let Ok(engine_event) = event_rx.try_recv() {
            adapter.handle_event(&engine_event, &mut state.data);
        }

        terminal.draw(|frame| {
            let area = frame.area();
            anvil_tui::surfaces::watch::render::render(frame, area, state, theme);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                let action = KeyHandler::map(key);
                state.handle_key(action);
            }
        }

        if state.should_quit {
            break;
        }
    }

    Ok(())
}
