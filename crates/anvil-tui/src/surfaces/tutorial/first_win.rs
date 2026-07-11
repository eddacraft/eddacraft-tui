//! WOW-005: first-win reroute after discovery.
//!
//! After the first-run discovery scan, this surface lands the user on their
//! repository's highest-value actionable real finding — explaining why it
//! matters and showing the proposed change — before the generic tutorial
//! path picker. Applying the fix requires explicit, named, per-action consent
//! through ACTTUI's shared consent chrome ([`ConsentState`], CIB-165 unticked
//! default, suppressed under `project_writes_gated`). Declining lands on the
//! path picker exactly as before.
//!
//! CIB-127 coordination: the only write this flow can perform is the single
//! consented line edit in the user's source file. It never touches the
//! activation finding-baseline (written by `anvil start`), so a tutorial-time
//! fix cannot confuse baseline state — activation simply records one fewer
//! finding if it runs afterwards.

use eddacraft_tui::keyboard::Action;
use eddacraft_tui::theme::EddaCraftTheme;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::surface::Surface;
use crate::surfaces::activation::consent::{ConsentItem, ConsentKind, ConsentState};
use crate::surfaces::fix_request::FixRequest;

use super::discovery::{Finding, ScanResults};
use super::first_win_render;

/// Consent row id for the single first-win fix action.
pub const FIRST_WIN_CONSENT_ID: &str = "first-win-fix";

/// Deterministically select the highest-value actionable real finding.
///
/// Rules (pinned by tests):
/// - Showcase results are never candidates — example findings must never be
///   presented as local truth (CIB-170).
/// - Actionable means the finding carries a deterministic auto-fix
///   ([`Finding::fix_request`] returns `Some`).
/// - Highest value: severity descending, tie-broken by file ascending, then
///   line ascending, then title ascending — the same total order the
///   discovery list renders in, so the selected finding is the first
///   actionable row the user just saw. The comparison is explicit (not
///   first-in-input-order) so selection is independent of input ordering.
#[must_use]
pub fn first_win_candidate(results: &ScanResults) -> Option<&Finding> {
    if results.is_showcase {
        return None;
    }
    results
        .findings
        .iter()
        .filter(|f| f.fix_request().is_some())
        .min_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.file.cmp(&b.file))
                .then_with(|| a.line.cmp(&b.line))
                .then_with(|| a.title.cmp(&b.title))
        })
}

/// The exact line change the consented fix would write, computed by the CLI
/// from the same deterministic transform that performs the write — so the
/// diff shown is byte-for-byte what apply produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixPreview {
    /// 1-based line number of the change.
    pub line: usize,
    /// The line as it exists on disk now.
    pub before: String,
    /// The line after the fix is applied.
    pub after: String,
}

/// The finding offered as the first win, with its diff and consent state.
pub struct FirstWinOffer {
    pub finding: Finding,
    pub preview: FixPreview,
    pub consent: ConsentState,
}

/// Current phase of the first-win surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirstWinPhase {
    /// Honest clean result: the real scan ran and found nothing.
    Clean { files_scanned: usize },
    /// Offering the highest-value actionable finding with diff + consent.
    Offer,
    /// Terminal acknowledgement after an apply attempt.
    Done { applied: bool, message: String },
}

/// TUI surface for the WOW-005 first-win reroute.
///
/// Pure state machine — the caller applies the consented fix (via the shared
/// interactive-fix service) when [`FirstWinState::take_pending_apply`] yields
/// a request, then calls [`FirstWinState::mark_outcome`].
pub struct FirstWinState {
    pub phase: FirstWinPhase,
    pub offer: Option<FirstWinOffer>,
    pub should_quit: bool,
    /// Set when the user declines — the caller lands on the path picker
    /// exactly as before the reroute existed.
    pub declined: bool,
    /// Set when the user acknowledges the Done screen (or clean result).
    pub wants_continue: bool,
    pending_apply: Option<FixRequest>,
}

impl FirstWinState {
    /// Offer `finding` as the first win. The consent row is unticked by
    /// default (CIB-165) and disabled under `project_writes_gated` via the
    /// shared ACTTUI gating, never a hand-rolled second consent surface.
    #[must_use]
    pub fn offer(finding: Finding, preview: FixPreview, project_writes_gated: bool) -> Self {
        let location = match finding.line {
            Some(line) => format!("{}:{line}", finding.file),
            None => finding.file.clone(),
        };
        let item = ConsentItem::new(
            FIRST_WIN_CONSENT_ID,
            format!("Apply this fix to {location}"),
            format!("rewrite one line of {}", finding.file),
            ConsentKind::Project,
        )
        .repo_scoped();
        let consent = ConsentState::new(vec![item], project_writes_gated);
        Self {
            phase: FirstWinPhase::Offer,
            offer: Some(FirstWinOffer {
                finding,
                preview,
                consent,
            }),
            should_quit: false,
            declined: false,
            wants_continue: false,
            pending_apply: None,
        }
    }

    /// Honest clean result: the real scan ran (`files_scanned` files) and
    /// produced no findings. Never constructed from showcase substitutes.
    #[must_use]
    pub fn clean(files_scanned: usize) -> Self {
        Self {
            phase: FirstWinPhase::Clean { files_scanned },
            offer: None,
            should_quit: false,
            declined: false,
            wants_continue: false,
            pending_apply: None,
        }
    }

    /// Take the consented fix request, if the user ticked the action and
    /// pressed apply. Cleared atomically so the caller applies it once.
    pub fn take_pending_apply(&mut self) -> Option<FixRequest> {
        self.pending_apply.take()
    }

    /// Record the outcome of the apply attempt; transitions to `Done`.
    pub fn mark_outcome(&mut self, applied: bool, message: impl Into<String>) {
        self.phase = FirstWinPhase::Done {
            applied,
            message: message.into(),
        };
    }

    // ---- key handling helpers ------------------------------------------

    fn handle_clean(&mut self, action: Action) {
        match action {
            Action::Select | Action::Back => self.wants_continue = true,
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_offer(&mut self, action: Action) {
        let Some(offer) = self.offer.as_mut() else {
            return;
        };
        match action {
            Action::Up => offer.consent.previous(),
            Action::Down => offer.consent.next(),
            // Enter and Space both toggle the consent row (the ACTTUI key
            // model) — Enter never applies, so Enter-without-tick can never
            // write anything.
            Action::Toggle => offer.consent.toggle_current(),
            Action::Select => offer.consent.select_current(),
            Action::Character('a' | 'A') => {
                offer.consent.submit();
                if offer.consent.is_selected(FIRST_WIN_CONSENT_ID) {
                    self.pending_apply = offer.finding.fix_request();
                } else {
                    // Applying an empty selection writes nothing and lands on
                    // the path picker — the same contract as declining.
                    self.declined = true;
                }
            }
            Action::Character('s') | Action::Back => self.declined = true,
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_done(&mut self, action: Action) {
        match action {
            Action::Select | Action::Back => self.wants_continue = true,
            Action::Quit => self.should_quit = true,
            _ => {}
        }
    }
}

impl Surface for FirstWinState {
    fn surface_name(&self) -> &'static str {
        "First Win"
    }

    fn help_text(&self) -> &'static str {
        match self.phase {
            FirstWinPhase::Clean { .. } | FirstWinPhase::Done { .. } => "enter continue  q quit",
            FirstWinPhase::Offer => "space tick  a apply  s skip to paths  q quit",
        }
    }

    fn handle_key(&mut self, action: Action) {
        match self.phase {
            FirstWinPhase::Clean { .. } => self.handle_clean(action),
            FirstWinPhase::Offer => self.handle_offer(action),
            FirstWinPhase::Done { .. } => self.handle_done(action),
        }
    }

    fn should_quit(&self) -> bool {
        self.should_quit || self.declined || self.wants_continue || self.pending_apply.is_some()
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &EddaCraftTheme) {
        first_win_render::render(frame, area, self, theme);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::discovery::{FindingSeverity, FindingSource};
    use super::*;

    fn actionable(
        severity: FindingSeverity,
        file: &str,
        line: usize,
        title: &str,
        warning_id: &str,
    ) -> Finding {
        Finding {
            file: file.to_string(),
            line: Some(line),
            severity,
            source: FindingSource::AntiPattern,
            title: title.to_string(),
            message: "message".to_string(),
            suggestion: "suggestion".to_string(),
            warning_id: Some(warning_id.to_string()),
        }
    }

    fn non_actionable(severity: FindingSeverity, file: &str) -> Finding {
        Finding {
            file: file.to_string(),
            line: Some(1),
            severity,
            source: FindingSource::Secret,
            title: "secret".to_string(),
            message: "message".to_string(),
            suggestion: "suggestion".to_string(),
            warning_id: None,
        }
    }

    fn results(findings: Vec<Finding>) -> ScanResults {
        ScanResults {
            findings,
            files_scanned: 10,
            duration_ms: 5,
            truncated: false,
            files_skipped_by_ignore: 0,
            is_showcase: false,
        }
    }

    fn preview() -> FixPreview {
        FixPreview {
            line: 3,
            before: "const value: any = source;".to_string(),
            after: "const value: unknown = source;".to_string(),
        }
    }

    fn offer_state() -> FirstWinState {
        FirstWinState::offer(
            actionable(
                FindingSeverity::Warning,
                "src/app.ts",
                3,
                "Avoid any",
                "AP-003",
            ),
            preview(),
            false,
        )
    }

    // ── first_win_candidate: selection + tie-breaking ────────────────────

    #[test]
    fn candidate_prefers_actionable_over_higher_severity_unfixable() {
        let r = results(vec![
            non_actionable(FindingSeverity::Error, "src/a.rs"),
            actionable(
                FindingSeverity::Warning,
                "src/b.ts",
                4,
                "Avoid any",
                "AP-003",
            ),
        ]);
        let candidate = first_win_candidate(&r).expect("candidate");
        assert_eq!(candidate.file, "src/b.ts");
    }

    #[test]
    fn candidate_picks_highest_severity_among_actionable() {
        let r = results(vec![
            actionable(FindingSeverity::Info, "src/a.ts", 1, "info", "AP-001"),
            actionable(FindingSeverity::Error, "src/z.ts", 9, "error", "AP-004"),
            actionable(FindingSeverity::Warning, "src/b.ts", 2, "warn", "AP-003"),
        ]);
        let candidate = first_win_candidate(&r).expect("candidate");
        assert_eq!(candidate.severity, FindingSeverity::Error);
        assert_eq!(candidate.file, "src/z.ts");
    }

    #[test]
    fn candidate_tie_breaks_by_file_then_line_then_title() {
        // Same severity: lexicographically smallest file wins.
        let r = results(vec![
            actionable(FindingSeverity::Warning, "src/b.ts", 1, "t", "AP-003"),
            actionable(FindingSeverity::Warning, "src/a.ts", 9, "t", "AP-003"),
        ]);
        assert_eq!(first_win_candidate(&r).expect("candidate").file, "src/a.ts");

        // Same severity + file: lowest line wins.
        let r = results(vec![
            actionable(FindingSeverity::Warning, "src/a.ts", 9, "t", "AP-003"),
            actionable(FindingSeverity::Warning, "src/a.ts", 2, "t", "AP-003"),
        ]);
        assert_eq!(first_win_candidate(&r).expect("candidate").line, Some(2));

        // Same severity + file + line: lexicographically smallest title wins.
        let r = results(vec![
            actionable(FindingSeverity::Warning, "src/a.ts", 2, "beta", "AP-003"),
            actionable(FindingSeverity::Warning, "src/a.ts", 2, "alpha", "AP-003"),
        ]);
        assert_eq!(first_win_candidate(&r).expect("candidate").title, "alpha");
    }

    #[test]
    fn candidate_is_independent_of_input_order() {
        let a = actionable(FindingSeverity::Warning, "src/a.ts", 2, "t", "AP-003");
        let b = actionable(FindingSeverity::Error, "src/z.ts", 9, "t", "AP-004");
        let forward = results(vec![a.clone(), b.clone()]);
        let reversed = results(vec![b, a]);
        assert_eq!(
            first_win_candidate(&forward).expect("candidate").file,
            first_win_candidate(&reversed).expect("candidate").file,
        );
    }

    #[test]
    fn candidate_none_for_showcase_results() {
        // CIB-170: example findings must never be offered as a local win.
        let mut r = results(vec![actionable(
            FindingSeverity::Error,
            "src/a.ts",
            1,
            "t",
            "AP-003",
        )]);
        r.is_showcase = true;
        assert!(first_win_candidate(&r).is_none());
    }

    #[test]
    fn candidate_none_when_nothing_actionable() {
        let r = results(vec![non_actionable(FindingSeverity::Error, "src/a.rs")]);
        assert!(first_win_candidate(&r).is_none());
    }

    #[test]
    fn candidate_none_for_empty_results() {
        assert!(first_win_candidate(&results(vec![])).is_none());
    }

    // ── Offer phase: consent boundary ────────────────────────────────────

    #[test]
    fn offer_consent_defaults_unticked() {
        let state = offer_state();
        let offer = state.offer.as_ref().expect("offer");
        assert!(offer.consent.selected_ids().is_empty());
    }

    #[test]
    fn enter_without_tick_writes_nothing() {
        let mut state = offer_state();
        // Enter toggles; a second Enter unticks. Neither produces an apply.
        state.handle_key(Action::Select);
        state.handle_key(Action::Select);
        assert!(state.take_pending_apply().is_none());
        // Apply with nothing ticked writes nothing and declines to the picker.
        state.handle_key(Action::Character('a'));
        assert!(state.take_pending_apply().is_none());
        assert!(state.declined);
    }

    #[test]
    fn apply_requires_ticked_consent() {
        let mut state = offer_state();
        state.handle_key(Action::Toggle);
        state.handle_key(Action::Character('a'));
        let request = state.take_pending_apply().expect("consented apply");
        assert_eq!(
            request,
            FixRequest::AntiPatternWarning {
                file: "src/app.ts".to_string(),
                line: 3,
                warning_id: "AP-003".to_string(),
            }
        );
        assert!(!state.declined);
    }

    #[test]
    fn gated_project_writes_suppress_the_fix() {
        let mut state = FirstWinState::offer(
            actionable(FindingSeverity::Warning, "src/app.ts", 3, "t", "AP-003"),
            preview(),
            true,
        );
        // The shared ACTTUI gating disables the repo-scoped row: ticking is a
        // no-op and apply cannot produce a write.
        state.handle_key(Action::Toggle);
        state.handle_key(Action::Character('a'));
        assert!(state.take_pending_apply().is_none());
        assert!(state.declined);
    }

    #[test]
    fn decline_keys_return_to_picker() {
        let mut state = offer_state();
        state.handle_key(Action::Character('s'));
        assert!(state.declined);
        assert!(Surface::should_quit(&state));

        let mut state = offer_state();
        state.handle_key(Action::Back);
        assert!(state.declined);
        assert!(state.take_pending_apply().is_none());
    }

    #[test]
    fn quit_sets_should_quit() {
        let mut state = offer_state();
        state.handle_key(Action::Quit);
        assert!(state.should_quit);
        assert!(!state.declined);
    }

    #[test]
    fn pending_apply_exits_the_surface_loop() {
        let mut state = offer_state();
        state.handle_key(Action::Toggle);
        state.handle_key(Action::Character('a'));
        assert!(Surface::should_quit(&state));
    }

    // ── Done phase ───────────────────────────────────────────────────────

    #[test]
    fn mark_outcome_transitions_to_done_and_enter_continues() {
        let mut state = offer_state();
        state.mark_outcome(true, "Applied fix in src/app.ts:3");
        assert!(matches!(
            state.phase,
            FirstWinPhase::Done { applied: true, .. }
        ));
        assert!(!Surface::should_quit(&state));
        state.handle_key(Action::Select);
        assert!(state.wants_continue);
        assert!(Surface::should_quit(&state));
    }

    #[test]
    fn failed_outcome_is_reported_honestly() {
        let mut state = offer_state();
        state.mark_outcome(false, "Line 3 is out of range for src/app.ts");
        match &state.phase {
            FirstWinPhase::Done { applied, message } => {
                assert!(!applied);
                assert!(message.contains("out of range"));
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    // ── Clean phase ──────────────────────────────────────────────────────

    #[test]
    fn clean_state_continues_on_enter() {
        let mut state = FirstWinState::clean(42);
        assert!(matches!(
            state.phase,
            FirstWinPhase::Clean { files_scanned: 42 }
        ));
        state.handle_key(Action::Select);
        assert!(state.wants_continue);
    }

    // ── Help text ────────────────────────────────────────────────────────

    #[test]
    fn help_text_changes_per_phase() {
        let mut state = offer_state();
        assert!(state.help_text().contains("a apply"));
        state.mark_outcome(true, "done");
        assert!(state.help_text().contains("continue"));
        let clean = FirstWinState::clean(1);
        assert!(clean.help_text().contains("continue"));
    }
}
