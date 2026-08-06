//! Parameterised compliance tests for every registered `anvil-tui` surface.
//!
//! TCOV-025: verifies the runtime contract of [`crate::surface::Surface`] for
//! every concrete implementation under `surfaces/`. The compiler already
//! enforces that all required methods are present; these tests enforce the
//! **runtime** contract:
//!
//! * `surface_name()` returns a non-empty string
//! * `help_text()` returns a non-empty string
//! * `render()` into a `TestBackend` does not panic
//! * `handle_key()` over **all** `Action` variants does not panic
//! * `reset()` does not panic
//! * re-rendering after interaction does not panic
//!
//! # Adding a new surface
//!
//! When you add a new `impl Surface for FooState`, you must:
//! 1. Add a `Box::new(FooState::new(...))` entry to `all_surfaces()` below.
//! 2. Increment `EXPECTED_SURFACE_COUNT` by 1.
//!
//! The guard test `surface_registry_covers_all_impls` will fail if you forget.

#[cfg(test)]
mod tests {
    use eddacraft_tui::json_render::parse;
    use eddacraft_tui::keyboard::Action;
    use eddacraft_tui::theme::EddaCraftTheme;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::collections::VecDeque;
    use tempfile::{TempDir, tempdir};

    use crate::surface::Surface;
    use crate::surfaces::audit::{
        AuditData, AuditIssue, AuditState, HistoricalScore, IssueSeverity,
    };
    use crate::surfaces::browser::BrowserState;
    use crate::surfaces::dashboard::architecture::ArchitectureDashboardState;
    use crate::surfaces::dashboard::drift::DriftDashboardState;
    use crate::surfaces::dashboard::list::{DashboardListState, ListEntry};
    use crate::surfaces::dashboard::spec::SpecDashboardState;
    use crate::surfaces::dashboard::suppressions::{SuppressionsDashboardState, SuppressionsView};
    use crate::surfaces::doctor::DoctorState;
    use crate::surfaces::gate::{GateCheck, GateCheckStatus, GateResult, GateState};
    use crate::surfaces::init::InitState;
    use crate::surfaces::onboarding::{
        CompletionState, HooksState, InitCompleteState, InitCompleteSummary, OnboardingSummary,
        OnboardingWelcomeState,
    };
    use crate::surfaces::plan_dashboard::{PlanDashboardSnapshot, PlanDashboardState};
    use crate::surfaces::status::{
        GateRunResult, HookStatus, ProfileInfo, StatusData, StatusState,
    };
    use crate::surfaces::tutorial::TutorialState;
    use crate::surfaces::tutorial::discovery::{Finding, FindingSeverity, FindingSource};
    use crate::surfaces::tutorial::fix::FixState;
    use crate::surfaces::watch::{RunHistory, WatchData, WatchState, WatchStats, WatchStatus};
    use crate::surfaces::welcome::WelcomeState;
    use crate::surfaces::wizard::WizardState;

    // ---------------------------------------------------------------------------
    // Number of `impl Surface` blocks under crates/anvil-tui/src/surfaces/.
    // Run: grep -rnE '^impl .*Surface for' crates/anvil-tui/src/surfaces/ | wc -l
    // (anchored to the line start so backticked mentions of "impl Surface for"
    // in comments — including this one — are not counted).
    // Update this constant whenever a new surface is added.
    // ---------------------------------------------------------------------------
    const EXPECTED_SURFACE_COUNT: usize = 22;

    /// Temp-dir guards (kept alive for the test) plus the labelled surfaces.
    type SurfaceRegistry = (Vec<TempDir>, Vec<(&'static str, Box<dyn Surface>)>);

    // ---------------------------------------------------------------------------
    // Minimal fixture builders
    // ---------------------------------------------------------------------------

    fn sample_audit_data() -> AuditData {
        AuditData {
            project_name: "compliance-project".to_string(),
            total_files: 1,
            security_scope: "test scope".to_string(),
            issues: vec![AuditIssue {
                severity: IssueSeverity::Low,
                category: "Quality".to_string(),
                message: "console statement".to_string(),
                file: "src/lib.rs".to_string(),
                line: 1,
                fixable: true,
            }],
            historical_scores: vec![HistoricalScore {
                timestamp: "2026-01-01".to_string(),
                score: 0.9,
                issue_count: 1,
            }],
            next_steps: vec!["Fix the lint".to_string()],
        }
    }

    fn sample_status_data() -> StatusData {
        StatusData {
            hooks: vec![HookStatus {
                name: "pre-commit".to_string(),
                active: true,
                path: ".git/hooks/pre-commit".to_string(),
            }],
            profile: ProfileInfo {
                name: "default".to_string(),
                checks: vec!["eslint".to_string()],
                path: ".anvilrc".to_string(),
            },
            recent_runs: vec![GateRunResult {
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                passed: true,
                score: 1.0,
                checks_run: 1,
                checks_passed: 1,
                duration_ms: 100,
            }],
            update_hint: None,
            insights_hint: None,
            whats_new_hint: None,
        }
    }

    fn sample_watch_data() -> WatchData {
        WatchData {
            status: WatchStatus::Idle,
            queue: VecDeque::new(),
            history: vec![RunHistory {
                passed: true,
                checks_run: 1,
                checks_passed: 1,
                duration_ms: 50,
                timestamp: "00:00:00".to_string(),
            }],
            stats: WatchStats {
                total_runs: 1,
                pass_rate: 1.0,
                avg_duration_ms: 50,
                files_watched: 1,
            },
            warmup: None,
            last_action: None,
            update_hint: None,
            insights_hint: None,
            daemon_fallback_notice: None,
        }
    }

    fn sample_gate_result() -> GateResult {
        GateResult {
            plan_id: "default".to_string(),
            overall_passed: true,
            score: 1.0,
            checks: vec![GateCheck {
                id: "lint".to_string(),
                name: "Lint".to_string(),
                status: GateCheckStatus::Passed,
                score: 1.0,
                message: "Clean".to_string(),
                details: None,
                file: None,
                line: None,
            }],
            duration_ms: 100,
            timestamp: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn sample_plan_snapshot() -> PlanDashboardSnapshot {
        PlanDashboardSnapshot {
            modules: vec![],
            work_items: vec![],
            warnings: vec![],
            branch: None,
            sha: None,
        }
    }

    fn sample_finding() -> Finding {
        Finding {
            file: "src/lib.rs".to_string(),
            line: None,
            severity: FindingSeverity::Info,
            source: FindingSource::AntiPattern,
            title: "Example".to_string(),
            message: "No issue".to_string(),
            suggestion: "Nothing to do".to_string(),
            warning_id: None,
        }
    }

    // Minimal JSON spec for SpecDashboardState.
    const MINIMAL_SPEC: &str = r#"{
        "title": "Compliance", "version": "1.0", "root": "pg",
        "elements": {
            "pg": { "type": "Stack", "props": {}, "children": [] }
        }
    }"#;

    // ---------------------------------------------------------------------------
    // Registry
    // ---------------------------------------------------------------------------

    /// Build one instance of every surface that implements `Surface`.
    ///
    /// Returns the temp-dir guards alongside the surfaces: some surfaces capture
    /// a filesystem path at construction, so the `TempDir`s must outlive the
    /// surfaces. The caller binds the returned guards for the test's lifetime;
    /// they drop normally (cleaning `/tmp`) when the test ends.
    ///
    /// Surfaces that genuinely cannot be constructed without production changes
    /// are documented inline with a `// SKIPPED:` comment.
    fn all_surfaces() -> SurfaceRegistry {
        // SpecDashboardState needs a real (or temp) root directory for binding.
        let tmp = tempdir().expect("tempdir");
        let spec_root = tmp.path().to_path_buf();

        // HooksState inspects the filesystem; we give it a temp dir so it
        // constructs cleanly without touching the real project tree.
        let hooks_tmp = tempdir().expect("hooks-tempdir");
        let hooks_path = hooks_tmp.path().to_path_buf();

        // Held by the caller so the temp dirs outlive the surfaces, then drop.
        let guards = vec![tmp, hooks_tmp];

        let surfaces: Vec<(&'static str, Box<dyn Surface>)> = vec![
            ("WelcomeState", Box::new(WelcomeState::new())),
            ("TutorialState", Box::new(TutorialState::new())),
            (
                "DiscoveryState",
                Box::new(crate::surfaces::tutorial::discovery::DiscoveryState::new()),
            ),
            ("FixState", Box::new(FixState::new(sample_finding()))),
            (
                "OnboardingWelcomeState",
                Box::new(OnboardingWelcomeState::new()),
            ),
            (
                "CompletionState",
                Box::new(CompletionState::new(OnboardingSummary::default())),
            ),
            (
                "InitCompleteState",
                Box::new(InitCompleteState::new(InitCompleteSummary::default())),
            ),
            ("HooksState", Box::new(HooksState::new(&hooks_path))),
            ("BrowserState", Box::new(BrowserState::new(vec![], vec![]))),
            (
                "StatusState",
                Box::new(StatusState::new(sample_status_data())),
            ),
            ("InitState", Box::new(InitState::new(vec![]))),
            ("WatchState", Box::new(WatchState::new(sample_watch_data()))),
            ("GateState", Box::new(GateState::new(sample_gate_result()))),
            ("AuditState", Box::new(AuditState::new(sample_audit_data()))),
            ("DoctorState", Box::new(DoctorState::new(vec![]))),
            (
                "PlanDashboardState",
                Box::new(PlanDashboardState::new(sample_plan_snapshot())),
            ),
            (
                "SuppressionsDashboardState",
                Box::new(SuppressionsDashboardState::new(SuppressionsView::default())),
            ),
            (
                "ArchitectureDashboardState",
                Box::new(ArchitectureDashboardState::new(None)),
            ),
            (
                "DriftDashboardState",
                Box::new(DriftDashboardState::new(None)),
            ),
            (
                "DashboardListState",
                Box::new(DashboardListState::new(vec![ListEntry {
                    name: "test".to_string(),
                    title: "Test".to_string(),
                    description: "A test dashboard".to_string(),
                    available: true,
                    preview: None,
                }])),
            ),
            (
                "SpecDashboardState",
                Box::new(SpecDashboardState::new(
                    parse(MINIMAL_SPEC).expect("spec parse"),
                    spec_root,
                )),
            ),
            ("WizardState", Box::new(WizardState::new(vec![]))),
        ];

        (guards, surfaces)
    }

    // ---------------------------------------------------------------------------
    // Contract helper
    // ---------------------------------------------------------------------------

    const ALL_ACTIONS: &[Action] = &[
        Action::Up,
        Action::Down,
        Action::Left,
        Action::Right,
        Action::Select,
        Action::Toggle,
        Action::Back,
        Action::Quit,
        Action::Character('a'),
        Action::Character('z'),
        Action::Character(' '),
        Action::Backspace,
        Action::Delete,
        Action::Home,
        Action::End,
        Action::PageUp,
        Action::PageDown,
        Action::None,
    ];

    /// Assert the full runtime contract for one surface.
    fn assert_surface_contract(label: &str, surface: &mut dyn Surface) {
        // 1. Non-empty name and help text.
        let name = surface.surface_name();
        assert!(
            !name.is_empty(),
            "{label}: surface_name() must not be empty"
        );

        let help = surface.help_text();
        assert!(!help.is_empty(), "{label}: help_text() must not be empty");

        let theme = EddaCraftTheme;

        // 2. render() must not error.
        let mut terminal =
            Terminal::new(TestBackend::new(80, 24)).expect("Terminal::new(TestBackend) failed");
        terminal
            .draw(|frame| surface.render(frame, frame.area(), &theme))
            .unwrap_or_else(|e| panic!("{label}: initial draw returned Err: {e}"));

        // 3. handle_key() over all Action variants must not panic.
        for &action in ALL_ACTIONS {
            surface.handle_key(action);
        }

        // 4. reset() must not panic.
        surface.reset();

        // 5. Re-render after interaction must not error.
        terminal
            .draw(|frame| surface.render(frame, frame.area(), &theme))
            .unwrap_or_else(|e| panic!("{label}: post-interaction draw returned Err: {e}"));
    }

    // ---------------------------------------------------------------------------
    // Tests
    // ---------------------------------------------------------------------------

    /// Run the `Surface` runtime contract against every registered surface.
    ///
    /// Test name contains "surface" so `cargo test -p eddacraft-anvil-tui surface`
    /// selects it.
    #[test]
    fn surface_compliance_all_registered() {
        // `_guards` keeps the temp dirs alive for the duration of the test.
        let (_guards, mut surfaces) = all_surfaces();
        assert!(!surfaces.is_empty(), "surface registry must not be empty");
        for (label, surface) in &mut surfaces {
            assert_surface_contract(label, surface.as_mut());
        }
    }

    /// Guard: the registry must cover every `impl Surface` block.
    ///
    /// If this test fails you added a surface without registering it here.
    /// Update `EXPECTED_SURFACE_COUNT` and add your surface to `all_surfaces()`.
    #[test]
    fn surface_registry_covers_all_impls() {
        let (_guards, surfaces) = all_surfaces();
        assert_eq!(
            surfaces.len(),
            EXPECTED_SURFACE_COUNT,
            "registry has {} entries but expected {} — \
             add the new surface to all_surfaces() in compliance.rs \
             and increment EXPECTED_SURFACE_COUNT",
            surfaces.len(),
            EXPECTED_SURFACE_COUNT,
        );
    }
}
