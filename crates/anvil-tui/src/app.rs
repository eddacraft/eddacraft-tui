use std::sync::mpsc;

use anvil_kernel_types::EngineEvent;

use crate::compat::{TerminalInfo, detect_terminal, validate_minimum_size};
use crate::migration::TuiBackend;
use crate::surfaces::watch::event_adapter::WatchEventAdapter;
use crate::surfaces::watch::{WatchData, WatchState, WatchStats, WatchStatus};

/// Error type for TUI application failures.
#[derive(Debug, thiserror::Error)]
pub enum TuiError {
    #[error("{0}")]
    TerminalTooSmall(String),
    #[error("backend not supported: {0}")]
    BackendNotSupported(String),
    #[error("event channel closed")]
    ChannelClosed,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Configuration for the TUI application.
pub struct TuiAppConfig {
    pub backend: TuiBackend,
    pub event_rx: mpsc::Receiver<EngineEvent>,
    pub skip_terminal_check: bool,
}

/// Top-level TUI application that wires the kernel watch mode to the
/// Ratatui event loop.
///
/// Entry point for `anvil watch --tui=ratatui`.
pub struct TuiApp {
    pub state: WatchState,
    pub adapter: WatchEventAdapter,
    pub event_rx: mpsc::Receiver<EngineEvent>,
    pub terminal_info: TerminalInfo,
    pub backend: TuiBackend,
}

impl TuiApp {
    /// Create a new TUI application with the given configuration.
    pub fn new(config: TuiAppConfig) -> Result<Self, TuiError> {
        let terminal_info = detect_terminal();

        if !config.skip_terminal_check {
            validate_minimum_size(&terminal_info).map_err(TuiError::TerminalTooSmall)?;
        }

        if config.backend == TuiBackend::Ink {
            return Err(TuiError::BackendNotSupported(
                "ink backend is handled by the Node.js process".to_string(),
            ));
        }

        let data = WatchData {
            status: WatchStatus::Idle,
            queue: std::collections::VecDeque::new(),
            history: Vec::new(),
            stats: WatchStats {
                total_runs: 0,
                pass_rate: 0.0,
                avg_duration_ms: 0,
                files_watched: 0,
            },
            warmup: None,
            last_action: None,
            update_hint: None,
            insights_hint: None,
        };

        Ok(Self {
            state: WatchState::new(data),
            adapter: WatchEventAdapter::new(),
            event_rx: config.event_rx,
            terminal_info,
            backend: config.backend,
        })
    }

    /// Process all pending events from the kernel.
    pub fn drain_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            self.adapter.handle_event(&event, &mut self.state.data);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_config(backend: TuiBackend) -> TuiAppConfig {
        let (_tx, rx) = mpsc::channel();
        TuiAppConfig {
            backend,
            event_rx: rx,
            skip_terminal_check: true,
        }
    }

    #[test]
    fn creates_with_ratatui_backend() {
        let app = TuiApp::new(mock_config(TuiBackend::Ratatui));
        assert!(app.is_ok());
    }

    #[test]
    fn ink_backend_rejected() {
        let app = TuiApp::new(mock_config(TuiBackend::Ink));
        assert!(app.is_err());
        assert!(matches!(app, Err(TuiError::BackendNotSupported(_))));
    }

    #[test]
    fn initial_state_is_idle() {
        let app = TuiApp::new(mock_config(TuiBackend::Ratatui)).unwrap();
        assert_eq!(app.state.data.status, WatchStatus::Idle);
        assert!(app.state.data.queue.is_empty());
        assert!(app.state.data.history.is_empty());
        assert_eq!(app.state.data.stats.total_runs, 0);
    }

    #[test]
    fn drain_events_processes_channel() {
        use anvil_kernel_types::{EngineId, EventPayload, EventType};

        let (tx, rx) = mpsc::channel();
        let config = TuiAppConfig {
            backend: TuiBackend::Ratatui,
            event_rx: rx,
            skip_terminal_check: true,
        };
        let mut app = TuiApp::new(config).unwrap();

        // Send a snapshot event
        tx.send(EngineEvent {
            event_type: EventType::Snapshot,
            seq: 1,
            timestamp: "10:00:00".to_string(),
            engine: EngineId::Rust,
            payload: EventPayload::Snapshot {
                node_count: 50,
                edge_count: 20,
                files_watched: 35,
                changed_path: None,
            },
        })
        .unwrap();

        app.drain_events();

        assert_eq!(app.state.data.stats.files_watched, 35);
    }

    #[test]
    fn backend_field_set_correctly() {
        let app = TuiApp::new(mock_config(TuiBackend::Ratatui)).unwrap();
        assert_eq!(app.backend, TuiBackend::Ratatui);
    }
}
