use std::io;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use animate::is_animating;
use anvil_kernel_types::{EngineEvent, EventType};
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

/// Best-effort restore: leave the alternate screen and disable raw mode. Safe
/// to call when those modes are not active — crossterm treats both as no-ops.
fn restore_terminal() {
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();
}

/// Install a process-wide panic hook that restores the terminal before the
/// previous hook prints the panic message. Idempotent — only the first call
/// installs the hook; subsequent calls are no-ops. Without this, a panic
/// inside a `run_*` loop leaves the user at a shell prompt with raw mode and
/// the alternate screen still active.
fn install_panic_hook() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore_terminal();
            prev(info);
        }));
    });
}

/// RAII guard for the TUI terminal session. Enables raw mode + the alternate
/// screen on `enter`, restores both on `Drop` — including on unwinding panic.
/// Pair with [`install_panic_hook`] (invoked automatically) so the panic
/// backtrace is rendered against a normal terminal.
pub struct TerminalGuard {
    active: bool,
}

impl TerminalGuard {
    /// Enable raw mode and the alternate screen. Installs the panic-restore
    /// hook on first use.
    pub fn enter() -> anyhow::Result<Self> {
        install_panic_hook();
        terminal::enable_raw_mode()?;
        if let Err(e) = execute!(io::stdout(), EnterAlternateScreen) {
            // Roll back the raw-mode change so we don't leak it on
            // partial-setup failure.
            let _ = terminal::disable_raw_mode();
            return Err(e.into());
        }
        Ok(Self { active: true })
    }

    /// Explicit teardown. After this, `Drop` is a no-op. Returns any error
    /// from the underlying crossterm calls — prefer this over relying solely
    /// on `Drop` when callers want to surface restoration errors.
    pub fn leave(mut self) -> anyhow::Result<()> {
        self.disarm_and_restore()
    }

    fn disarm_and_restore(&mut self) -> anyhow::Result<()> {
        if !self.active {
            return Ok(());
        }
        // Disarm only AFTER both restore calls succeed. If either errors,
        // `active` stays true so `Drop` still runs a best-effort restore —
        // otherwise an early `?` would leak raw mode / alt-screen, which is
        // exactly the failure mode this guard exists to prevent.
        execute!(io::stdout(), LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        self.active = false;
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.active {
            restore_terminal();
            self.active = false;
        }
    }
}

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
pub fn run_surface<S: Surface>(state: S) -> anyhow::Result<S> {
    run_surface_with_exit(state).map(|(state, _)| state)
}

/// Like [`run_surface`], but also reports how the surface exited
/// ([`SurfaceExit::Quit`] vs [`SurfaceExit::Back`]). Callers that nest a
/// surface under a parent (e.g. the dashboard picker launching a subview) use
/// the exit reason to decide between returning to the parent and exiting.
pub fn run_surface_with_exit<S: Surface>(mut state: S) -> anyhow::Result<(S, SurfaceExit)> {
    let guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let theme = EddaCraftTheme;

    let result = surface_loop(&mut terminal, &mut state, &theme);

    guard.leave()?;

    result.map(|exit| (state, exit))
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
///
/// Installs the panic-restore hook so a panic between setup and teardown
/// still restores the terminal. For the RAII-style alternative, see
/// [`TerminalGuard`].
pub fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    install_panic_hook();
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
                    // #2881: text-entry steps (a path/name field) map printable
                    // keys literally so h/j/k/l/q aren't hijacked as vim
                    // navigation; list steps keep the vim-style mapping.
                    let action = if state.text_entry_active() {
                        KeyHandler::map_text_entry(key)
                    } else {
                        KeyHandler::map(key)
                    };
                    state.handle_key(action);
                    dirty = true;

                    // Check exit immediately after key — avoids waiting for
                    // the next poll timeout before responding to quit/back.
                    // Draw the post-key frame first so any acknowledgement
                    // the surface added (e.g. "Applying auto-fix...") is
                    // visible to the user before the TUI tears down.
                    if state.should_quit() || state.should_back() {
                        if dirty {
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
                        }
                        if state.should_quit() {
                            return Ok(SurfaceExit::Quit);
                        }
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
    let guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let theme = EddaCraftTheme;

    let result = tutorial_loop(&mut terminal, &mut state, file_rx, &theme);

    guard.leave()?;

    result.map(|()| state)
}

fn tutorial_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut anvil_tui::surfaces::tutorial::TutorialState,
    file_rx: Option<&Receiver<anvil_kernel::watcher::events::ChangeBatch>>,
    theme: &EddaCraftTheme,
) -> anyhow::Result<()> {
    // WOW-002: the typed-command reveal advances on a fixed wall-clock
    // cadence, independent of the terminal's event stream. Pacing must not
    // depend on how many key-release/repeat/resize events a terminal emits —
    // otherwise the reveal (and with it the command's execution) races ahead
    // and the cancel window shrinks. The state machine stays deterministic:
    // it only ever advances via `reveal_tick`, which we call on this schedule.
    const REVEAL_TICK: Duration = Duration::from_millis(100);
    let mut next_reveal_tick = Instant::now() + REVEAL_TICK;

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

        // Wake in time for the next reveal tick even when no input arrives.
        let timeout = next_reveal_tick.saturating_duration_since(Instant::now());
        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            if state.hand_back_autoplay() {
                continue;
            }
            // While the inline editor is open, letters must be typed as text
            // rather than consumed as navigation/quit commands. The default
            // KeyHandler maps j/k/h/l→arrows, q→quit and space→toggle, so it
            // cannot enter those characters — switch to a text-input map.
            let action = if state.is_editing() {
                map_key_text(key)
            } else {
                KeyHandler::map(key)
            };
            state.handle_key(action);
        }
        // Non-press key events (release/repeat) and other events (resize,
        // focus) are read and discarded above — they must never drive reveal
        // pacing. Advance the reveal only on the fixed schedule, resetting the
        // deadline from `now` so a burst of input that delayed us past a tick
        // boundary cannot fire a backlog of ticks at once and complete the
        // reveal instantly (WOW-002). No-op when no reveal is in flight.
        let now = Instant::now();
        if now >= next_reveal_tick {
            state.reveal_tick();
            next_reveal_tick = now + REVEAL_TICK;
        }

        if state.should_quit()
            || state.should_back()
            || state.wants_watch_demo
            || state.wants_autoplay_setup
            || state.autoplay_failure().is_some()
            || state.autoplay_teardown_requested()
        {
            return Ok(());
        }
    }
}

/// Text-input key map for the tutorial's inline editor. Unlike
/// [`KeyHandler::map`], every printable character (including `j`/`k`/`h`/`l`/
/// `q` and space) is forwarded as [`Action::Character`] so it is typed into the
/// editor. Enter inserts a newline; Ctrl-S saves (as `Character('\x13')`, the
/// same save signal the fix surface uses); Esc cancels. Only Ctrl-C quits.
fn map_key_text(event: crossterm::event::KeyEvent) -> eddacraft_tui::keyboard::Action {
    use crossterm::event::{KeyCode, KeyModifiers};
    use eddacraft_tui::keyboard::Action;

    if event.modifiers.contains(KeyModifiers::CONTROL) {
        return match event.code {
            // Accept the shifted forms too: some terminals deliver Ctrl+Shift+S
            // as Char('S'), which would otherwise fall through to None and make
            // save/quit unreachable in that configuration.
            KeyCode::Char('s' | 'S') => Action::Character('\x13'), // Ctrl-S → save
            KeyCode::Char('c' | 'C') => Action::Quit,
            _ => Action::None,
        };
    }

    match event.code {
        KeyCode::Enter => Action::Character('\n'),
        KeyCode::Esc => Action::Back,
        KeyCode::Backspace => Action::Backspace,
        KeyCode::Delete => Action::Delete,
        KeyCode::Up => Action::Up,
        KeyCode::Down => Action::Down,
        KeyCode::Left => Action::Left,
        KeyCode::Right => Action::Right,
        KeyCode::Home => Action::Home,
        KeyCode::End => Action::End,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::PageDown => Action::PageDown,
        KeyCode::Char(c) => Action::Character(c),
        _ => Action::None,
    }
}

/// Run the watch demo with guided overlay (WELCOME-014).
/// Receives engine events from the kernel watcher and renders the watch
/// dashboard with progressive overlay hints.
pub fn run_watch_demo(
    mut state: anvil_tui::surfaces::tutorial::watch_demo::WatchDemoState,
    event_rx: &Receiver<EngineEvent>,
) -> anyhow::Result<()> {
    let guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let theme = EddaCraftTheme;

    let result = watch_demo_loop(&mut terminal, &mut state, event_rx, &theme, None).map(|_| ());

    guard.leave()?;

    result
}

pub fn run_watch_demo_autoplay(
    mut state: anvil_tui::surfaces::tutorial::watch_demo::WatchDemoState,
    event_rx: &Receiver<EngineEvent>,
    tutorial: &mut anvil_tui::surfaces::tutorial::TutorialState,
    scripted_edit: &mut dyn FnMut() -> anyhow::Result<()>,
) -> anyhow::Result<anvil_tui::surfaces::tutorial::watch_demo::WatchDemoOutcome> {
    let guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let theme = EddaCraftTheme;
    let result = watch_demo_loop(
        &mut terminal,
        &mut state,
        event_rx,
        &theme,
        Some((tutorial, scripted_edit)),
    );
    guard.leave()?;
    result
}

fn watch_demo_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut anvil_tui::surfaces::tutorial::watch_demo::WatchDemoState,
    event_rx: &Receiver<EngineEvent>,
    theme: &EddaCraftTheme,
    mut autoplay: Option<(
        &mut anvil_tui::surfaces::tutorial::TutorialState,
        &mut dyn FnMut() -> anyhow::Result<()>,
    )>,
) -> anyhow::Result<anvil_tui::surfaces::tutorial::watch_demo::WatchDemoOutcome> {
    const AUTOPLAY_WATCH_TIMEOUT: Duration = Duration::from_secs(30);
    const MAX_EVENTS_PER_TICK: usize = 256;
    let mut last_tick = Instant::now();
    let mut scripted_edit_done = false;
    let autoplay_deadline = autoplay
        .as_ref()
        .map(|_| Instant::now() + AUTOPLAY_WATCH_TIMEOUT);

    loop {
        if autoplay_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            anyhow::bail!(
                "autoplay watch timed out after 30 seconds (snapshots={}, scripted_edit={scripted_edit_done})",
                state.snapshot_count
            );
        }
        // Drain engine events.
        for _ in 0..MAX_EVENTS_PER_TICK {
            match event_rx.try_recv() {
                Ok(engine_event) => {
                    if let Some((tutorial, edit)) = autoplay.as_mut() {
                        let outcome = state.autoplay_engine_event(&engine_event);
                        if state.snapshot_count == 1 && !scripted_edit_done {
                            edit()?;
                            scripted_edit_done = true;
                        }
                        tutorial.apply_watch_demo_outcome(outcome);
                        if outcome
                            == anvil_tui::surfaces::tutorial::watch_demo::WatchDemoOutcome::CycleComplete
                        {
                            return Ok(outcome);
                        }
                    } else {
                        state.handle_engine_event(&engine_event);
                    }
                }
                Err(TryRecvError::Disconnected) if autoplay.is_some() => {
                    anyhow::bail!(
                        "autoplay watch event channel disconnected (snapshots={}, scripted_edit={scripted_edit_done})",
                        state.snapshot_count
                    );
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        // Advance time-based overlay hints.
        state.tick();

        tick_animations(&mut last_tick, state.is_dirty(), || {
            state.mark_dirty();
        });

        // While animating, cap to ~60 fps to avoid a busy-spin between frames.
        let mut poll_timeout = if state.is_dirty() {
            if is_animating() {
                Duration::from_millis(16)
            } else {
                Duration::ZERO
            }
        } else {
            Duration::from_millis(50)
        };
        if let Some(deadline) = autoplay_deadline {
            poll_timeout = poll_timeout.min(deadline.saturating_duration_since(Instant::now()));
        }

        if event::poll(poll_timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if let Some((tutorial, _)) = autoplay.as_mut()
                        && tutorial.hand_back_autoplay()
                    {
                        return Ok(
                            anvil_tui::surfaces::tutorial::watch_demo::WatchDemoOutcome::HandBack,
                        );
                    }
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

    Ok(anvil_tui::surfaces::tutorial::watch_demo::WatchDemoOutcome::Continue)
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
    watch_demo_loop(terminal, &mut state, event_rx, theme, None).map(|_| ())
}

pub fn run_watch_demo_autoplay_in(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut state: anvil_tui::surfaces::tutorial::watch_demo::WatchDemoState,
    event_rx: &Receiver<EngineEvent>,
    theme: &EddaCraftTheme,
    tutorial: &mut anvil_tui::surfaces::tutorial::TutorialState,
    scripted_edit: &mut dyn FnMut() -> anyhow::Result<()>,
) -> anyhow::Result<anvil_tui::surfaces::tutorial::watch_demo::WatchDemoOutcome> {
    watch_demo_loop(
        terminal,
        &mut state,
        event_rx,
        theme,
        Some((tutorial, scripted_edit)),
    )
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
    action_link: Option<&crate::commands::watch::WatchActionLink<'_>>,
    shutdown: Option<&Arc<AtomicBool>>,
) -> anyhow::Result<()> {
    let guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let theme = EddaCraftTheme;

    let result = watch_loop(
        &mut terminal,
        &mut state,
        event_rx,
        action_link,
        shutdown,
        &theme,
    );

    guard.leave()?;

    result
}

/// Run the watch surface inside an already-initialised terminal session.
/// Used by the welcome hub to launch watch mode without teardown/setup.
/// The welcome-hub path doesn't dispatch `--action`, so it always passes
/// `None` for the action link.
pub fn run_watch_in(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut WatchState,
    event_rx: &Receiver<EngineEvent>,
) -> anyhow::Result<SurfaceExit> {
    let theme = EddaCraftTheme;
    watch_loop(terminal, state, event_rx, None, None, &theme)?;
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
    action_link: Option<&crate::commands::watch::WatchActionLink<'_>>,
    shutdown: Option<&Arc<AtomicBool>>,
    theme: &EddaCraftTheme,
) -> anyhow::Result<()> {
    let mut adapter = WatchEventAdapter::new();
    let mut last_tick = Instant::now();
    // LAUNCH-002: gate snapshot-driven action dispatch the same way
    // commands::watch's non-TUI branch does — skip the initial scan, then
    // fire on each subsequent Snapshot.
    let mut snapshot_count: u64 = 0;

    loop {
        // Drain all pending engine events.
        while let Ok(engine_event) = event_rx.try_recv() {
            adapter.handle_event(&engine_event, &mut state.data);
            state.mark_dirty();
            if let Some(link) = action_link
                && matches!(engine_event.event_type, EventType::Snapshot)
            {
                snapshot_count += 1;
                if snapshot_count > 1 {
                    link.dispatcher
                        .on_snapshot(crate::commands::watch::snapshot_changed_path(&engine_event));
                }
            }
        }

        // LAUNCH-002: drain pending action results and fold them into
        // last_action via the adapter (single-writer invariant).
        if let Some(link) = action_link {
            while let Ok(result) = link.action_rx.try_recv() {
                WatchEventAdapter::handle_action_result(&result, &mut state.data);
                state.mark_dirty();
            }
        }

        tick_animations(&mut last_tick, state.is_dirty(), || {
            state.mark_dirty();
        });
        state.sync_animations();

        // When dirty, render quickly. While animating, cap to ~60 fps so the
        // poll loop doesn't busy-spin between frames.
        let poll_timeout = if state.is_dirty() {
            if is_animating() {
                Duration::from_millis(16)
            } else {
                Duration::ZERO
            }
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

fn tick_animations<F>(last_tick: &mut Instant, already_dirty: bool, mut mark_dirty: F)
where
    F: FnMut(),
{
    let now = Instant::now();
    let elapsed = now.saturating_duration_since(*last_tick);

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let ms = elapsed.as_millis().min(usize::MAX as u128) as usize;
    if ms > 0 {
        // Advance last_tick by the whole-ms portion we consumed; the sub-ms
        // remainder accumulates into the next iteration so animations still
        // progress when individual loop iterations run faster than 1 ms.
        *last_tick += Duration::from_millis(ms as u64);
        animate::tick(ms);
    }

    if !already_dirty && is_animating() {
        mark_dirty();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    mod map_key_text {
        use super::super::map_key_text;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        use eddacraft_tui::keyboard::Action;

        fn key(code: KeyCode) -> KeyEvent {
            KeyEvent::new(code, KeyModifiers::empty())
        }
        fn ctrl(code: KeyCode) -> KeyEvent {
            KeyEvent::new(code, KeyModifiers::CONTROL)
        }

        #[test]
        fn navigation_letters_become_text_not_movement() {
            // The whole point: j/k/h/l/q and space are typed, not consumed as
            // navigation/quit/toggle (which is what KeyHandler::map does).
            for c in ['j', 'k', 'h', 'l', 'q', ' ', 'x'] {
                assert_eq!(map_key_text(key(KeyCode::Char(c))), Action::Character(c));
            }
        }

        #[test]
        fn enter_inserts_newline() {
            assert_eq!(map_key_text(key(KeyCode::Enter)), Action::Character('\n'));
        }

        #[test]
        fn ctrl_s_is_the_save_signal() {
            assert_eq!(
                map_key_text(ctrl(KeyCode::Char('s'))),
                Action::Character('\x13')
            );
        }

        #[test]
        fn ctrl_shift_forms_still_save_and_quit() {
            // Some terminals deliver Ctrl+Shift+S/C as the uppercase char.
            assert_eq!(
                map_key_text(ctrl(KeyCode::Char('S'))),
                Action::Character('\x13')
            );
            assert_eq!(map_key_text(ctrl(KeyCode::Char('C'))), Action::Quit);
        }

        #[test]
        fn esc_cancels_and_ctrl_c_quits() {
            assert_eq!(map_key_text(key(KeyCode::Esc)), Action::Back);
            assert_eq!(map_key_text(ctrl(KeyCode::Char('c'))), Action::Quit);
        }

        #[test]
        fn editing_keys_map_through() {
            assert_eq!(map_key_text(key(KeyCode::Backspace)), Action::Backspace);
            assert_eq!(map_key_text(key(KeyCode::Left)), Action::Left);
            assert_eq!(map_key_text(key(KeyCode::Up)), Action::Up);
        }
    }

    /// The panic hook MUST be idempotent — multiple TUI sessions (or repeated
    /// `setup_terminal` calls) must not re-wrap the previous hook. Without
    /// the `OnceLock` guard, three installs would invoke the captured
    /// previous hook three times per panic.
    ///
    /// We prove the contract by installing an atomic-counter as the previous
    /// hook *before* calling `install_panic_hook`, calling it three times,
    /// triggering a panic inside `catch_unwind`, and asserting the counter
    /// observed exactly one invocation.
    ///
    /// This is the only test in the binary that touches the panic-hook
    /// machinery; no other unit test in `crates/anvil-cli/src/` installs a
    /// hook, so the global state seen here is determined by this test alone.
    #[test]
    fn install_panic_hook_does_not_stack_on_repeat_installs() {
        static PREV_HOOK_CALLS: AtomicUsize = AtomicUsize::new(0);

        let saved = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {
            PREV_HOOK_CALLS.fetch_add(1, Ordering::SeqCst);
        }));

        install_panic_hook();
        install_panic_hook();
        install_panic_hook();

        PREV_HOOK_CALLS.store(0, Ordering::SeqCst);

        let result = std::panic::catch_unwind(|| panic!("panic-hook idempotence probe"));
        assert!(result.is_err(), "catch_unwind should report the panic");

        let calls = PREV_HOOK_CALLS.load(Ordering::SeqCst);
        assert_eq!(
            calls, 1,
            "previous hook ran {calls} times — `install_panic_hook` re-wrapped on repeat \
             install (expected exactly 1 via the OnceLock guard)"
        );

        std::panic::set_hook(saved);
    }
}
