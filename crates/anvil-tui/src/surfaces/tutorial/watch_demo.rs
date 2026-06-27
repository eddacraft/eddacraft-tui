use std::time::{Duration, Instant};

use animate::{Animate, Lerp, Once};
use anvil_kernel_types::EngineEvent;
use eddacraft_tui::keyboard::Action;

use crate::surfaces::watch::WatchData;
use crate::surfaces::watch::event_adapter::WatchEventAdapter;

type AnimatedF64 = Once<f64, fn(f64) -> f64, fn(&f64, &f64, f64) -> f64>;

const OVERLAY_ANIM_DURATION_MS: f64 = 220.0;

fn animated_f64(initial: f64) -> AnimatedF64 {
    Once::new(
        initial,
        OVERLAY_ANIM_DURATION_MS,
        animate::easing::quad_out as fn(f64) -> f64,
        <f64 as Lerp>::lerp as fn(&f64, &f64, f64) -> f64,
    )
}

/// Guided overlay phase during the watch demo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayPhase {
    /// Initial introduction — explain what the watch dashboard shows.
    Intro,
    /// First progressive hint at ~10s.
    Hint1,
    /// Second progressive hint at ~20s.
    Hint2,
    /// Third progressive hint at ~30s — offer to continue.
    Hint3,
    /// User saw a full cycle (file change → check → result).
    CycleComplete,
    /// Overlay dismissed — user is exploring freely.
    Dismissed,
}

/// State for the watch mode demo with guided overlay (WELCOME-014).
///
/// Wraps a `WatchData` (the same data the real watch dashboard uses)
/// and adds overlay annotations, timing, and auto-continue logic.
#[allow(clippy::struct_excessive_bools)]
pub struct WatchDemoState {
    pub data: WatchData,
    pub adapter: WatchEventAdapter,
    pub overlay: OverlayPhase,
    pub started_at: Instant,
    pub should_quit: bool,
    pub wants_back: bool,
    /// Whether the user has witnessed a full file-change → check → result cycle.
    pub cycle_seen: bool,
    /// Number of engine events received (used to detect cycle completion).
    pub event_count: u64,
    /// Number of snapshot events (end-of-cycle markers).
    pub snapshot_count: u64,
    /// Whether the overlay hints should auto-advance with time.
    pub auto_hints: bool,
    /// Whether the state has changed since last render.
    dirty: bool,
    overlay_reveal: AnimatedF64,
    overlay_reveal_target: f64,
}

impl WatchDemoState {
    pub fn new(data: WatchData) -> Self {
        Self {
            data,
            adapter: WatchEventAdapter::new(),
            overlay: OverlayPhase::Intro,
            started_at: Instant::now(),
            should_quit: false,
            wants_back: false,
            cycle_seen: false,
            event_count: 0,
            snapshot_count: 0,
            auto_hints: true,
            dirty: true,
            // Start hidden so the first sync_overlay_animation animates the
            // intro overlay in from 0 → 1 instead of jumping straight to full
            // height on the first frame.
            overlay_reveal: animated_f64(0.0),
            overlay_reveal_target: 0.0,
        }
    }

    /// Feed an engine event into the watch data adapter.
    pub fn handle_engine_event(&mut self, event: &EngineEvent) {
        self.adapter.handle_event(event, &mut self.data);
        self.event_count += 1;
        self.dirty = true;

        if matches!(event.event_type, anvil_kernel_types::EventType::Snapshot) {
            self.snapshot_count += 1;
            // A cycle is complete when we've seen at least 2 snapshots
            // (initial scan + one triggered by file change).
            if self.snapshot_count >= 2 && !self.cycle_seen {
                self.cycle_seen = true;
                self.overlay = OverlayPhase::CycleComplete;
            }
        }

        self.sync_overlay_animation();
    }

    /// Advance overlay hints based on elapsed time.
    pub fn tick(&mut self) {
        if !self.auto_hints || self.cycle_seen || self.overlay == OverlayPhase::Dismissed {
            self.sync_overlay_animation();
            return;
        }
        let elapsed = self.started_at.elapsed();
        let new_phase = if elapsed >= Duration::from_secs(30) {
            OverlayPhase::Hint3
        } else if elapsed >= Duration::from_secs(20) {
            OverlayPhase::Hint2
        } else if elapsed >= Duration::from_secs(10) {
            OverlayPhase::Hint1
        } else {
            OverlayPhase::Intro
        };

        if new_phase != self.overlay {
            self.overlay = new_phase;
            self.dirty = true;
        }

        self.sync_overlay_animation();
    }

    pub fn handle_key(&mut self, action: Action) {
        match action {
            // 's' — skip the demo
            Action::Character('s') | Action::Back | Action::Quit => self.wants_back = true,
            // Enter — continue if cycle complete or hint3 reached
            Action::Select => {
                if self.overlay == OverlayPhase::CycleComplete
                    || self.overlay == OverlayPhase::Hint3
                {
                    self.wants_back = true;
                } else if self.overlay == OverlayPhase::Intro {
                    // Dismiss the intro overlay so user can see the full dashboard.
                    self.overlay = OverlayPhase::Dismissed;
                    self.dirty = true;
                    self.sync_overlay_animation();
                }
            }
            _ => {}
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn take_dirty(&mut self) -> bool {
        std::mem::replace(&mut self.dirty, false)
    }

    pub fn overlay_reveal(&self) -> f64 {
        *self.overlay_reveal
    }

    fn sync_overlay_animation(&mut self) {
        let target = if self.overlay == OverlayPhase::Dismissed {
            0.0
        } else {
            1.0
        };

        if (target - self.overlay_reveal_target).abs() > f64::EPSILON {
            self.overlay_reveal.set(target);
            self.overlay_reveal_target = target;
        }
        self.overlay_reveal.update();
    }

    pub fn overlay_text(&self) -> &'static str {
        match self.overlay {
            OverlayPhase::Intro => {
                "This is the anvil watch dashboard. It monitors your files \
                 and runs checks automatically when changes are detected."
            }
            OverlayPhase::Hint1 => {
                "Try editing a source file in your project. The dashboard \
                 will detect the change and re-run checks."
            }
            OverlayPhase::Hint2 => {
                "The file watcher status shows active monitoring. Check \
                 results appear in the history panel below."
            }
            OverlayPhase::Hint3 => {
                "Press enter to continue to the next tutorial step, \
                 or keep exploring the watch dashboard."
            }
            OverlayPhase::CycleComplete => {
                "You just saw a full cycle: file change \u{2192} check \u{2192} result. \
                 Press enter to continue."
            }
            OverlayPhase::Dismissed => "",
        }
    }

    pub fn help_text(&self) -> &'static str {
        match self.overlay {
            OverlayPhase::Intro => "enter dismiss overlay  s skip  esc back",
            OverlayPhase::Hint1 | OverlayPhase::Hint2 | OverlayPhase::Dismissed => {
                "s skip  esc back"
            }
            OverlayPhase::Hint3 | OverlayPhase::CycleComplete => "enter continue  s skip  esc back",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surfaces::watch::{WatchStats, WatchStatus};
    use std::collections::VecDeque;

    fn empty_data() -> WatchData {
        WatchData {
            status: WatchStatus::Idle,
            queue: VecDeque::new(),
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
            daemon_fallback_notice: None,
        }
    }

    #[test]
    fn starts_with_intro_overlay() {
        let state = WatchDemoState::new(empty_data());
        assert_eq!(state.overlay, OverlayPhase::Intro);
        assert!(!state.cycle_seen);
    }

    #[test]
    fn enter_on_intro_dismisses_overlay() {
        let mut state = WatchDemoState::new(empty_data());
        state.handle_key(Action::Select);
        assert_eq!(state.overlay, OverlayPhase::Dismissed);
        assert!(!state.wants_back);
    }

    #[test]
    fn skip_exits() {
        let mut state = WatchDemoState::new(empty_data());
        state.handle_key(Action::Character('s'));
        assert!(state.wants_back);
    }

    #[test]
    fn enter_on_cycle_complete_exits() {
        let mut state = WatchDemoState::new(empty_data());
        state.overlay = OverlayPhase::CycleComplete;
        state.handle_key(Action::Select);
        assert!(state.wants_back);
    }

    #[test]
    fn enter_on_hint3_exits() {
        let mut state = WatchDemoState::new(empty_data());
        state.overlay = OverlayPhase::Hint3;
        state.handle_key(Action::Select);
        assert!(state.wants_back);
    }

    #[test]
    fn overlay_text_non_empty_for_active_phases() {
        let state = WatchDemoState::new(empty_data());
        assert!(!state.overlay_text().is_empty());

        for phase in [
            OverlayPhase::Intro,
            OverlayPhase::Hint1,
            OverlayPhase::Hint2,
            OverlayPhase::Hint3,
            OverlayPhase::CycleComplete,
        ] {
            let mut s = WatchDemoState::new(empty_data());
            s.overlay = phase;
            assert!(
                !s.overlay_text().is_empty(),
                "overlay text should be non-empty for {phase:?}"
            );
        }
    }

    #[test]
    fn dismissed_overlay_has_empty_text() {
        let mut state = WatchDemoState::new(empty_data());
        state.overlay = OverlayPhase::Dismissed;
        assert!(state.overlay_text().is_empty());
    }

    #[test]
    fn dirty_flag_management() {
        let mut state = WatchDemoState::new(empty_data());
        assert!(state.is_dirty());
        assert!(state.take_dirty());
        assert!(!state.is_dirty());
        state.mark_dirty();
        assert!(state.is_dirty());
    }
}
