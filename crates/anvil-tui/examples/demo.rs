use std::io;
use std::time::Duration;

use anvil_kernel_types::{Notification, NotificationClass, NotificationPriority};
use anvil_tui::shell::render_shell;
use anvil_tui::surface::Surface;
use anvil_tui::surfaces::audit::{
    AuditData, AuditIssue, AuditState, HistoricalScore, IssueSeverity,
};
use anvil_tui::surfaces::browser::{
    BrowserState, TemplateCategory, TemplateEntry, TemplateVariable,
};
use anvil_tui::surfaces::doctor::{CheckStatus, DiagnosticCheck, DoctorState};
use anvil_tui::surfaces::gate::{GateCheck, GateCheckStatus, GateResult, GateState};
use anvil_tui::surfaces::init::{AvailableCheck, InitState};
use anvil_tui::surfaces::status::{
    GateRunResult, HookStatus, ProfileInfo, StatusData, StatusState,
};
use anvil_tui::surfaces::tutorial::TutorialState;
use anvil_tui::surfaces::watch::{
    QueuedNotification, RunHistory, WatchData, WatchState, WatchStats, WatchStatus,
};
use anvil_tui::surfaces::welcome::WelcomeState;
use anvil_tui::surfaces::wizard::{Template, WizardState};
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use eddacraft_tui::keyboard::{Action, KeyHandler};
use eddacraft_tui::theme::{EddaCraftTheme, Theme};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

// ---------------------------------------------------------------------------
// Surface descriptor for the picker
// ---------------------------------------------------------------------------

struct SurfaceInfo {
    name: &'static str,
    description: &'static str,
}

const SURFACES: [SurfaceInfo; 10] = [
    SurfaceInfo {
        name: "Welcome",
        description: "Quick-start landing screen",
    },
    SurfaceInfo {
        name: "Tutorial",
        description: "Guided walkthrough for learning Anvil",
    },
    SurfaceInfo {
        name: "Doctor",
        description: "Environment diagnostics and health checks",
    },
    SurfaceInfo {
        name: "Status",
        description: "Hook, profile, and gate run overview",
    },
    SurfaceInfo {
        name: "Gate",
        description: "Gate check explorer with filtering and search",
    },
    SurfaceInfo {
        name: "Watch",
        description: "Live file-watch dashboard with run history",
    },
    SurfaceInfo {
        name: "Init",
        description: "Project initialisation wizard",
    },
    SurfaceInfo {
        name: "Wizard",
        description: "Template-based project scaffolding",
    },
    SurfaceInfo {
        name: "Audit",
        description: "Audit results with historical scores",
    },
    SurfaceInfo {
        name: "Browser",
        description: "Template catalogue browser",
    },
];

// ---------------------------------------------------------------------------
// Demo surface enum
// ---------------------------------------------------------------------------

enum DemoSurface {
    Picker(PickerState),
    Welcome(WelcomeState),
    Tutorial(TutorialState),
    Doctor(DoctorState),
    Status(StatusState),
    Gate(GateState),
    Watch(WatchState),
    Init(InitState),
    Wizard(WizardState),
    Audit(AuditState),
    Browser(BrowserState),
}

struct PickerState {
    selected: usize,
}

impl PickerState {
    fn new() -> Self {
        Self { selected: 0 }
    }
}

struct DemoApp {
    surface: DemoSurface,
}

// ---------------------------------------------------------------------------
// Mock data factories
// ---------------------------------------------------------------------------

fn mock_doctor_checks() -> Vec<DiagnosticCheck> {
    use anvil_tui::surfaces::doctor::Remediation;

    vec![
        DiagnosticCheck {
            name: "Node.js runtime".into(),
            category: "Environment".into(),
            status: CheckStatus::Pass,
            message: "v22.4.0 detected".into(),
            details: Some("Path: /usr/local/bin/node".into()),
            auto_fixable: false,
            remediation: Remediation::default(),
        },
        DiagnosticCheck {
            name: "Rust toolchain".into(),
            category: "Environment".into(),
            status: CheckStatus::Pass,
            message: "rustc 1.85.0 (stable)".into(),
            details: Some("Installed via rustup".into()),
            auto_fixable: false,
            remediation: Remediation::default(),
        },
        DiagnosticCheck {
            name: "Anvil config file".into(),
            category: "Configuration".into(),
            status: CheckStatus::Fail,
            message: "No .anvil.yaml found in project root".into(),
            details: None,
            auto_fixable: true,
            remediation: Remediation {
                summary: "Create .anvil.yaml in the project root.".into(),
                command: Some("anvil init".into()),
                doc_url: None,
            },
        },
        DiagnosticCheck {
            name: "ESLint configuration".into(),
            category: "Configuration".into(),
            status: CheckStatus::Warn,
            message: "Config found but uses deprecated format".into(),
            details: None,
            auto_fixable: false,
            remediation: Remediation {
                summary: "Migrate from .eslintrc to eslint.config.js".into(),
                command: None,
                doc_url: Some(
                    "https://eslint.org/docs/latest/use/configure/migration-guide".into(),
                ),
            },
        },
        DiagnosticCheck {
            name: "Git hooks".into(),
            category: "Hooks".into(),
            status: CheckStatus::Skipped,
            message: "Hook installation skipped (no .husky dir)".into(),
            details: None,
            auto_fixable: true,
            remediation: Remediation::default(),
        },
        DiagnosticCheck {
            name: "Secret scanner".into(),
            category: "Environment".into(),
            status: CheckStatus::Pass,
            message: "gitleaks v8.21 available".into(),
            details: None,
            auto_fixable: false,
            remediation: Remediation::default(),
        },
    ]
}

fn mock_status_data() -> StatusData {
    StatusData {
        hooks: vec![
            HookStatus {
                name: "pre-commit".into(),
                active: true,
                path: ".husky/pre-commit".into(),
            },
            HookStatus {
                name: "pre-push".into(),
                active: true,
                path: ".husky/pre-push".into(),
            },
            HookStatus {
                name: "post-merge".into(),
                active: false,
                path: ".husky/post-merge".into(),
            },
        ],
        profile: ProfileInfo {
            name: "default".into(),
            checks: vec![
                "secret-detection".into(),
                "import-boundaries".into(),
                "naming-conventions".into(),
                "dependency-audit".into(),
            ],
            path: ".anvil/profiles/default.yaml".into(),
        },
        recent_runs: vec![
            GateRunResult {
                timestamp: "2026-03-17T09:15:00Z".into(),
                passed: true,
                score: 0.95,
                checks_run: 8,
                checks_passed: 8,
                duration_ms: 1850,
            },
            GateRunResult {
                timestamp: "2026-03-17T08:42:00Z".into(),
                passed: false,
                score: 0.75,
                checks_run: 8,
                checks_passed: 6,
                duration_ms: 2100,
            },
            GateRunResult {
                timestamp: "2026-03-16T17:30:00Z".into(),
                passed: true,
                score: 0.88,
                checks_run: 8,
                checks_passed: 7,
                duration_ms: 1920,
            },
        ],
        update_hint: None,
        insights_hint: None,
        whats_new_hint: None,
    }
}

fn mock_gate_result() -> GateResult {
    GateResult {
        plan_id: "demo-plan".into(),
        overall_passed: false,
        score: 0.62,
        checks: vec![
            GateCheck {
                id: "no-console-log".into(),
                name: "No console.log".into(),
                status: GateCheckStatus::Passed,
                score: 1.0,
                message: "No console statements found".into(),
                details: Some("Scanned 42 files".into()),
                file: None,
                line: None,
            },
            GateCheck {
                id: "import-boundaries".into(),
                name: "Import boundaries".into(),
                status: GateCheckStatus::Failed,
                score: 0.0,
                message: "3 cross-boundary imports detected".into(),
                details: Some(
                    "src/api/handler.ts imports from src/ui/components\n\
                     src/core/utils.ts imports from src/infra/db\n\
                     src/shared/helpers.ts imports from src/api/routes"
                        .into(),
                ),
                file: Some("src/api/handler.ts".into()),
                line: Some(5),
            },
            GateCheck {
                id: "secret-detection".into(),
                name: "Secret detection".into(),
                status: GateCheckStatus::Failed,
                score: 0.0,
                message: "Potential API key in config".into(),
                details: Some("Line 23: STRIPE_KEY=sk_live_...".into()),
                file: Some("src/config.ts".into()),
                line: Some(23),
            },
            GateCheck {
                id: "naming-conventions".into(),
                name: "Naming conventions".into(),
                status: GateCheckStatus::Passed,
                score: 1.0,
                message: "All files follow kebab-case".into(),
                details: None,
                file: None,
                line: None,
            },
            GateCheck {
                id: "dependency-audit".into(),
                name: "Dependency audit".into(),
                status: GateCheckStatus::Warning,
                score: 0.5,
                message: "2 outdated dependencies".into(),
                details: Some("lodash@4.17.20 -> 4.17.21\naxios@1.6.0 -> 1.7.2".into()),
                file: None,
                line: None,
            },
            GateCheck {
                id: "type-check".into(),
                name: "Type check".into(),
                status: GateCheckStatus::Passed,
                score: 1.0,
                message: "No type errors".into(),
                details: None,
                file: None,
                line: None,
            },
            GateCheck {
                id: "test-coverage".into(),
                name: "Test coverage".into(),
                status: GateCheckStatus::Skipped,
                score: 0.0,
                message: "Coverage reporter not configured".into(),
                details: None,
                file: None,
                line: None,
            },
            GateCheck {
                id: "architecture-drift".into(),
                name: "Architecture drift".into(),
                status: GateCheckStatus::Warning,
                score: 0.6,
                message: "1 module boundary mismatch".into(),
                details: Some(
                    "packages/core moved to packages/kernel without updating rules".into(),
                ),
                file: Some(".anvil/architecture.yaml".into()),
                line: None,
            },
        ],
        duration_ms: 3450,
        timestamp: "2026-03-17T09:15:00Z".into(),
    }
}

fn mock_watch_data() -> WatchData {
    WatchData {
        status: WatchStatus::Passing,
        queue: std::collections::VecDeque::from([
            QueuedNotification {
                notification: Notification::new(
                    NotificationClass::Finding,
                    NotificationPriority::High,
                    "src/lib.rs",
                    "modified",
                ),
                timestamp: "09:14:32".into(),
            },
            QueuedNotification {
                notification: Notification::new(
                    NotificationClass::Finding,
                    NotificationPriority::High,
                    "src/config.rs",
                    "modified",
                ),
                timestamp: "09:14:35".into(),
            },
            QueuedNotification {
                notification: Notification::new(
                    NotificationClass::Finding,
                    NotificationPriority::High,
                    "tests/integration.rs",
                    "created",
                ),
                timestamp: "09:14:38".into(),
            },
        ]),
        history: vec![
            RunHistory {
                passed: true,
                checks_run: 6,
                checks_passed: 6,
                duration_ms: 420,
                timestamp: "09:14:10".into(),
            },
            RunHistory {
                passed: true,
                checks_run: 6,
                checks_passed: 6,
                duration_ms: 380,
                timestamp: "09:13:45".into(),
            },
            RunHistory {
                passed: false,
                checks_run: 6,
                checks_passed: 4,
                duration_ms: 510,
                timestamp: "09:12:20".into(),
            },
            RunHistory {
                passed: true,
                checks_run: 5,
                checks_passed: 5,
                duration_ms: 490,
                timestamp: "09:11:00".into(),
            },
        ],
        stats: WatchStats {
            total_runs: 12,
            pass_rate: 0.83,
            avg_duration_ms: 450,
            files_watched: 87,
        },
        warmup: None,
        last_action: None,
        update_hint: None,
        insights_hint: None,
    }
}

fn mock_available_checks() -> Vec<AvailableCheck> {
    vec![
        AvailableCheck {
            name: "dependency-audit".into(),
            description: "Scan dependencies for known vulnerabilities".into(),
            enabled: true,
        },
        AvailableCheck {
            name: "secret-detection".into(),
            description: "Detect leaked secrets and API keys".into(),
            enabled: true,
        },
        AvailableCheck {
            name: "import-boundaries".into(),
            description: "Enforce module boundary rules".into(),
            enabled: false,
        },
        AvailableCheck {
            name: "naming-conventions".into(),
            description: "Check file and symbol naming patterns".into(),
            enabled: false,
        },
    ]
}

fn mock_templates() -> Vec<Template> {
    vec![
        Template {
            id: "typescript-monorepo".into(),
            name: "TypeScript Monorepo".into(),
            description: "Nx-based TypeScript monorepo with full Anvil gates".into(),
            tags: vec!["typescript".into(), "monorepo".into(), "nx".into()],
        },
        Template {
            id: "rust-workspace".into(),
            name: "Rust Workspace".into(),
            description: "Cargo workspace with architecture enforcement".into(),
            tags: vec!["rust".into(), "workspace".into()],
        },
        Template {
            id: "python-package".into(),
            name: "Python Package".into(),
            description: "Python package with linting and secret scanning".into(),
            tags: vec!["python".into(), "pypi".into()],
        },
    ]
}

fn mock_audit_data() -> AuditData {
    AuditData {
        project_name: "eddacraft".into(),
        total_files: 142,
        issues: vec![
            AuditIssue {
                severity: IssueSeverity::Critical,
                category: "Security".into(),
                message: "Hardcoded database credentials in config".into(),
                file: "src/db/connection.ts".into(),
                line: 12,
                fixable: false,
            },
            AuditIssue {
                severity: IssueSeverity::High,
                category: "Architecture".into(),
                message: "Circular dependency between core and infra modules".into(),
                file: "packages/core/src/index.ts".into(),
                line: 3,
                fixable: false,
            },
            AuditIssue {
                severity: IssueSeverity::Medium,
                category: "Quality".into(),
                message: "Function exceeds complexity threshold (cyclomatic: 15)".into(),
                file: "src/handlers/process.ts".into(),
                line: 42,
                fixable: false,
            },
            AuditIssue {
                severity: IssueSeverity::Low,
                category: "Style".into(),
                message: "Unused import detected".into(),
                file: "src/utils/helpers.ts".into(),
                line: 1,
                fixable: true,
            },
            AuditIssue {
                severity: IssueSeverity::Info,
                category: "Documentation".into(),
                message: "Public function missing JSDoc comment".into(),
                file: "src/api/routes.ts".into(),
                line: 28,
                fixable: false,
            },
        ],
        historical_scores: vec![
            HistoricalScore {
                timestamp: "2026-03-14".into(),
                score: 0.72,
                issue_count: 9,
            },
            HistoricalScore {
                timestamp: "2026-03-15".into(),
                score: 0.78,
                issue_count: 7,
            },
            HistoricalScore {
                timestamp: "2026-03-16".into(),
                score: 0.82,
                issue_count: 6,
            },
            HistoricalScore {
                timestamp: "2026-03-17".into(),
                score: 0.85,
                issue_count: 5,
            },
        ],
        next_steps: vec![
            "Remove hardcoded credentials from src/db/connection.ts".into(),
            "Break circular dependency between core and infra".into(),
            "Refactor process handler to reduce complexity".into(),
        ],
    }
}

fn mock_browser_data() -> (Vec<TemplateCategory>, Vec<TemplateEntry>) {
    let categories = vec![
        TemplateCategory {
            name: "Governance".into(),
            description: "Policy and compliance templates".into(),
            template_count: 2,
        },
        TemplateCategory {
            name: "Quality".into(),
            description: "Code quality and testing templates".into(),
            template_count: 2,
        },
    ];

    let templates = vec![
        TemplateEntry {
            id: "soc2-compliance".into(),
            name: "SOC 2 Compliance".into(),
            description: "Pre-built gates for SOC 2 Type II requirements".into(),
            category: "Governance".into(),
            tags: vec!["compliance".into(), "soc2".into(), "audit".into()],
            variables: vec![
                TemplateVariable {
                    name: "org_name".into(),
                    description: "Organisation name for reports".into(),
                    default_value: None,
                    required: true,
                },
                TemplateVariable {
                    name: "audit_frequency".into(),
                    description: "How often to run compliance checks".into(),
                    default_value: Some("weekly".into()),
                    required: false,
                },
            ],
        },
        TemplateEntry {
            id: "gdpr-data-handling".into(),
            name: "GDPR Data Handling".into(),
            description: "Gates for personal data protection compliance".into(),
            category: "Governance".into(),
            tags: vec!["compliance".into(), "gdpr".into(), "privacy".into()],
            variables: vec![TemplateVariable {
                name: "dpo_email".into(),
                description: "Data Protection Officer contact".into(),
                default_value: None,
                required: true,
            }],
        },
        TemplateEntry {
            id: "test-coverage-gates".into(),
            name: "Test Coverage Gates".into(),
            description: "Enforce minimum test coverage thresholds".into(),
            category: "Quality".into(),
            tags: vec!["testing".into(), "coverage".into()],
            variables: vec![TemplateVariable {
                name: "min_coverage".into(),
                description: "Minimum coverage percentage".into(),
                default_value: Some("80".into()),
                required: false,
            }],
        },
        TemplateEntry {
            id: "lint-standard".into(),
            name: "Lint Standard".into(),
            description: "Standardised linting rules across the organisation".into(),
            category: "Quality".into(),
            tags: vec!["linting".into(), "eslint".into(), "clippy".into()],
            variables: vec![],
        },
    ];

    (categories, templates)
}

// ---------------------------------------------------------------------------
// Surface construction
// ---------------------------------------------------------------------------

fn create_surface(index: usize) -> DemoSurface {
    match index {
        0 => DemoSurface::Welcome(WelcomeState::new()),
        1 => DemoSurface::Tutorial(TutorialState::new()),
        2 => DemoSurface::Doctor(DoctorState::new(mock_doctor_checks())),
        3 => DemoSurface::Status(StatusState::new(mock_status_data())),
        4 => DemoSurface::Gate(GateState::new(mock_gate_result())),
        5 => DemoSurface::Watch(WatchState::new(mock_watch_data())),
        6 => DemoSurface::Init(InitState::new(mock_available_checks())),
        7 => DemoSurface::Wizard(WizardState::new(mock_templates())),
        8 => DemoSurface::Audit(AuditState::new(mock_audit_data())),
        9 => {
            let (cats, tmpls) = mock_browser_data();
            DemoSurface::Browser(BrowserState::new(cats, tmpls))
        }
        _ => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// Picker rendering
// ---------------------------------------------------------------------------

fn render_picker(
    frame: &mut ratatui::Frame,
    area: Rect,
    state: &PickerState,
    theme: &EddaCraftTheme,
) {
    let mut lines = vec![Line::raw("")];

    for (i, info) in SURFACES.iter().enumerate() {
        let is_selected = i == state.selected;
        let indicator = if is_selected { ">> " } else { "   " };
        let name_style = if is_selected {
            Style::default().fg(theme.accent())
        } else {
            Style::default().fg(theme.fg())
        };
        let desc_style = Style::default().fg(theme.muted());

        lines.push(Line::from(vec![
            Span::styled(indicator, name_style),
            Span::styled(info.name, name_style),
            Span::styled("  ", Style::default()),
            Span::styled(info.description, desc_style),
        ]));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Shell integration helpers
// ---------------------------------------------------------------------------

fn surface_name(app: &DemoApp) -> &str {
    match &app.surface {
        DemoSurface::Picker(_) => "p i c k e r",
        DemoSurface::Welcome(s) => s.surface_name(),
        DemoSurface::Tutorial(s) => s.surface_name(),
        DemoSurface::Doctor(s) => s.surface_name(),
        DemoSurface::Status(s) => s.surface_name(),
        DemoSurface::Gate(s) => s.surface_name(),
        DemoSurface::Watch(s) => s.surface_name(),
        DemoSurface::Init(s) => s.surface_name(),
        DemoSurface::Wizard(s) => s.surface_name(),
        DemoSurface::Audit(s) => s.surface_name(),
        DemoSurface::Browser(s) => s.surface_name(),
    }
}

fn help_text(app: &DemoApp) -> &str {
    match &app.surface {
        DemoSurface::Picker(_) => "j/k navigate  enter select  q quit",
        DemoSurface::Welcome(s) => s.help_text(),
        DemoSurface::Tutorial(s) => s.help_text(),
        DemoSurface::Doctor(s) => s.help_text(),
        DemoSurface::Status(s) => s.help_text(),
        DemoSurface::Gate(s) => s.help_text(),
        DemoSurface::Watch(s) => s.help_text(),
        DemoSurface::Init(s) => s.help_text(),
        DemoSurface::Wizard(s) => s.help_text(),
        DemoSurface::Audit(s) => s.help_text(),
        DemoSurface::Browser(s) => s.help_text(),
    }
}

// ---------------------------------------------------------------------------
// Event loop
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let theme = EddaCraftTheme;
    let mut app = DemoApp {
        surface: DemoSurface::Tutorial(TutorialState::new()),
    };

    let result = run_loop(&mut terminal, &mut app, &theme);

    crossterm::terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn draw_surface(frame: &mut ratatui::Frame, app: &DemoApp, theme: &EddaCraftTheme) {
    let area = frame.area();
    let name = surface_name(app);
    let help = help_text(app);
    let core = render_shell(frame, area, name, help, theme);

    match &app.surface {
        DemoSurface::Picker(state) => render_picker(frame, core, state, theme),
        DemoSurface::Welcome(state) => {
            anvil_tui::surfaces::welcome::render::render(frame, core, state, theme);
        }
        DemoSurface::Tutorial(state) => {
            anvil_tui::surfaces::tutorial::render::render(frame, core, state, theme);
        }
        DemoSurface::Doctor(state) => {
            anvil_tui::surfaces::doctor::render::render(frame, core, state, theme);
        }
        DemoSurface::Status(state) => {
            anvil_tui::surfaces::status::render::render(frame, core, state, theme);
        }
        DemoSurface::Gate(state) => {
            anvil_tui::surfaces::gate::render::render(frame, core, state, theme);
        }
        DemoSurface::Watch(state) => {
            anvil_tui::surfaces::watch::render::render(frame, core, state, theme);
        }
        DemoSurface::Init(state) => {
            anvil_tui::surfaces::init::render::render(frame, core, state, theme);
        }
        DemoSurface::Wizard(state) => {
            anvil_tui::surfaces::wizard::render::render(frame, core, state, theme);
        }
        DemoSurface::Audit(state) => {
            anvil_tui::surfaces::audit::render::render(frame, core, state, theme);
        }
        DemoSurface::Browser(state) => {
            anvil_tui::surfaces::browser::render::render(frame, core, state, theme);
        }
    }
}

/// Returns `true` if the action should escape from a surface back to the picker.
fn should_escape_to_picker(app: &DemoApp, action: Action) -> bool {
    if action != Action::Back {
        return false;
    }
    match &app.surface {
        DemoSurface::Picker(_) => false,
        DemoSurface::Tutorial(s) => {
            s.phase == anvil_tui::surfaces::tutorial::TutorialPhase::PathSelect
        }
        DemoSurface::Init(s) => s.step == anvil_tui::surfaces::init::InitStep::Mode,
        DemoSurface::Wizard(s) => s.step == anvil_tui::surfaces::wizard::WizardStep::TemplateSelect,
        DemoSurface::Gate(s) => !s.search_mode,
        DemoSurface::Browser(s) => {
            s.view == anvil_tui::surfaces::browser::BrowserView::Categories && !s.search_mode
        }
        _ => true,
    }
}

/// Returns `true` if the current surface's `should_quit` flag is set.
fn surface_wants_quit(app: &DemoApp) -> bool {
    match &app.surface {
        DemoSurface::Picker(_) => false,
        DemoSurface::Welcome(s) => s.should_quit,
        DemoSurface::Tutorial(s) => s.should_quit,
        DemoSurface::Doctor(s) => s.should_quit,
        DemoSurface::Status(s) => s.should_quit,
        DemoSurface::Gate(s) => s.should_quit,
        DemoSurface::Watch(s) => s.should_quit,
        DemoSurface::Init(s) => s.should_quit,
        DemoSurface::Wizard(s) => s.should_quit,
        DemoSurface::Audit(s) => s.should_quit,
        DemoSurface::Browser(s) => s.should_quit,
    }
}

fn handle_surface_action(app: &mut DemoApp, action: Action) {
    match &mut app.surface {
        DemoSurface::Picker(_) => {}
        DemoSurface::Welcome(s) => s.handle_key(action),
        DemoSurface::Tutorial(s) => s.handle_key(action),
        DemoSurface::Doctor(s) => s.handle_key(action),
        DemoSurface::Status(s) => s.handle_key(action),
        DemoSurface::Gate(s) => s.handle_key(action),
        DemoSurface::Watch(s) => s.handle_key(action),
        DemoSurface::Init(s) => s.handle_key(action),
        DemoSurface::Wizard(s) => s.handle_key(action),
        DemoSurface::Audit(s) => s.handle_key(action),
        DemoSurface::Browser(s) => s.handle_key(action),
    }
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut DemoApp,
    theme: &EddaCraftTheme,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| draw_surface(frame, app, theme))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key_event) = event::read()?
            && key_event.kind == crossterm::event::KeyEventKind::Press
        {
            let action = KeyHandler::map(key_event);

            if let DemoSurface::Picker(state) = &mut app.surface {
                match action {
                    Action::Up if state.selected > 0 => {
                        state.selected -= 1;
                    }
                    Action::Down if state.selected < SURFACES.len() - 1 => {
                        state.selected += 1;
                    }
                    Action::Select => {
                        app.surface = create_surface(state.selected);
                    }
                    Action::Quit | Action::Back => return Ok(()),
                    _ => {}
                }
                continue;
            }

            if should_escape_to_picker(app, action) {
                app.surface = DemoSurface::Picker(PickerState::new());
                continue;
            }

            handle_surface_action(app, action);

            if surface_wants_quit(app) {
                app.surface = DemoSurface::Picker(PickerState::new());
            }
        }
    }
}
