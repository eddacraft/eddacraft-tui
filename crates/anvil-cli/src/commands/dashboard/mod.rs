//! `anvil dashboard [name]` — native read-only TUI dashboards over persisted
//! `.anvil/` state.
//!
//! TDASH-001 ships the command plus the picker scaffold. Per-domain dashboards
//! (architecture, drift, suppressions) land in TDASH-002+ by flipping their
//! catalogue entry to `available` and adding a launch arm in [`launch`].

use std::fmt::Write as _;
use std::io::IsTerminal;
use std::path::Path;

use clap::Args;
use serde::Serialize;

use anvil_tui::sanitize;
use anvil_tui::surfaces::dashboard::list::{DashboardListState, ListEntry};
use anvil_tui::surfaces::dashboard::spec::{self, SavedDashboard};

use crate::{GlobalArgs, tui, util};

mod architecture;
mod drift;
mod suppressions;

#[derive(Debug, Args)]
pub struct DashboardArgs {
    /// Dashboard to open (`architecture`, `drift`, `suppressions`). Omit to
    /// open the interactive picker.
    pub name: Option<String>,
}

/// A native dashboard known to the CLI. `available` stays `false` until the
/// dashboard's surface lands (TDASH-002+).
#[derive(Debug, Clone, Serialize)]
pub struct CatalogEntry {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub available: bool,
}

/// One-line description shown for a saved spec dashboard in the picker.
const SAVED_SPEC_DESCRIPTION: &str = "Saved dashboard spec (.anvil/dashboards)";

/// Name of the gate-summary dashboard, whether saved or embedded.
const GATE_SUMMARY_NAME: &str = "gate-summary";

/// One-line description shown for the embedded gate-summary fallback.
const EMBEDDED_GATE_SUMMARY_DESCRIPTION: &str = "Latest gate runs by check (built-in)";

/// UJ-009: existing projects (initialised before gate-summary seeding) get the
/// embedded spec as a built-in fallback. A saved spec with the same name —
/// init-seeded or user-customised — always wins, so the fallback never
/// clobbers or shadows user state. The comparison is on the SANITISED stem:
/// a hostile stem whose display form collides with `gate-summary` must
/// shadow the built-in too, or every listing surface would show the name
/// twice.
fn embedded_gate_summary_available(specs: &[SavedDashboard]) -> bool {
    !specs.iter().any(|s| sanitize(&s.name) == GATE_SUMMARY_NAME)
}

/// The catalogue of native dashboards, in display order.
fn catalog() -> Vec<CatalogEntry> {
    vec![
        CatalogEntry {
            name: "architecture",
            title: "Architecture Health",
            description: "Layer boundaries, violations, and rule compliance",
            available: true,
        },
        CatalogEntry {
            name: "drift",
            title: "Drift Snapshots",
            description: "Snapshot history and new-edge deltas vs baseline",
            available: true,
        },
        CatalogEntry {
            name: "suppressions",
            title: "Suppressions",
            description: "Active suppressions with scope, file, reason, and expiry",
            available: true,
        },
    ]
}

/// What the command should do for a given (optional) dashboard name.
#[derive(Debug, PartialEq, Eq)]
enum Resolution {
    /// Open the picker (no name given).
    Picker,
    /// Launch a wired dashboard.
    Launch(String),
    /// Known dashboard whose surface has not landed yet.
    ComingSoon(String),
    /// Name not in the catalogue.
    Unknown(String),
}

fn resolve(name: Option<&str>, catalog: &[CatalogEntry]) -> Resolution {
    let Some(name) = name else {
        return Resolution::Picker;
    };
    match catalog.iter().find(|entry| entry.name == name) {
        None => Resolution::Unknown(name.to_string()),
        Some(entry) if entry.available => Resolution::Launch(entry.name.to_string()),
        Some(entry) => Resolution::ComingSoon(entry.name.to_string()),
    }
}

pub fn run(args: &DashboardArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let catalog = catalog();
    // Saved spec dashboards live under `.anvil/dashboards/`. Discovery is
    // best-effort: outside a workspace (no root) there are simply none.
    let root = util::workspace_root().ok();
    let specs = root.as_deref().map(spec::discover).unwrap_or_default();

    // Every terminal branch handles `--json` itself so a global `--json` never
    // leaks human text: a launched dashboard emits its own data; the picker and
    // coming-soon paths emit the catalogue.
    match resolve(args.name.as_deref(), &catalog) {
        Resolution::Unknown(name) => {
            // A name absent from the native catalogue may be a saved spec.
            if let (Some(saved), Some(root)) =
                (specs.iter().find(|s| s.name == name), root.as_deref())
            {
                return launch_spec(saved, root, global);
            }
            // UJ-009: no saved gate-summary spec — serve the embedded one.
            if name == GATE_SUMMARY_NAME
                && let Some(root) = root.as_deref()
            {
                return launch_embedded_gate_summary(root, global);
            }
            let names = catalog
                .iter()
                .map(|entry| entry.name.to_string())
                // Saved names are untrusted file stems — sanitise for display.
                .chain(specs.iter().map(|s| sanitize(&s.name)))
                .chain(
                    embedded_gate_summary_available(&specs).then(|| GATE_SUMMARY_NAME.to_string()),
                )
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!(
                "unknown dashboard '{}'. Valid dashboards: {names}",
                sanitize(&name)
            )
        }
        Resolution::ComingSoon(name) => {
            if global.json {
                println!("{}", serde_json::to_string_pretty(&catalog)?);
            } else {
                println!("Dashboard '{name}' is not available yet (coming soon).");
            }
            Ok(())
        }
        Resolution::Launch(name) => launch(&name, global),
        Resolution::Picker => run_picker(&catalog, &specs, root.as_deref(), global),
    }
}

fn run_picker(
    catalog: &[CatalogEntry],
    specs: &[SavedDashboard],
    root: Option<&Path>,
    global: &GlobalArgs,
) -> anyhow::Result<()> {
    if global.json {
        // Emit native dashboards AND saved specs (the picker lists both). Keeps
        // the array-of-objects shape; adds a `kind` discriminator. Saved names
        // are untrusted file stems, so they are sanitised here too.
        let mut listing: Vec<serde_json::Value> = catalog
            .iter()
            .map(|e| {
                serde_json::json!({
                    "name": e.name, "title": e.title, "description": e.description,
                    "available": e.available, "kind": "native",
                })
            })
            .collect();
        listing.extend(specs.iter().map(|s| {
            serde_json::json!({
                "name": sanitize(&s.name), "title": s.title,
                "description": SAVED_SPEC_DESCRIPTION, "available": true, "kind": "spec",
            })
        }));
        if embedded_gate_summary_available(specs) {
            listing.push(serde_json::json!({
                "name": GATE_SUMMARY_NAME, "title": "Gate Summary",
                "description": EMBEDDED_GATE_SUMMARY_DESCRIPTION,
                "available": true, "kind": "builtin",
            }));
        }
        println!("{}", serde_json::to_string_pretty(&listing)?);
        return Ok(());
    }

    if global.no_tui || !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        print_picker(catalog, specs, embedded_gate_summary_available(specs));
        return Ok(());
    }

    // Two-pane list with live previews (TUIDASH-012): native dashboards show a
    // description card; saved specs render a mini-preview through the engine.
    let mut items: Vec<ListEntry> = catalog
        .iter()
        .map(|e| ListEntry::native(e.name, e.title, e.description, e.available))
        .collect();
    if let Some(root) = root {
        for saved in specs {
            // Discovery already proved each spec parses, so a load failure here
            // is unexpected; skip it rather than aborting the whole picker.
            if let Ok(surface) = spec::load(&saved.path, root.to_path_buf()) {
                items.push(ListEntry::spec(
                    saved.name.clone(),
                    saved.title.clone(),
                    surface,
                ));
            }
        }
        // UJ-009: built-in gate-summary entry when no saved spec shadows it.
        if embedded_gate_summary_available(specs) {
            match spec::load_str(
                anvil_tui::dashboard_catalog::GATE_SUMMARY_SPEC,
                root.to_path_buf(),
            ) {
                Ok(surface) => items.push(ListEntry::spec(
                    GATE_SUMMARY_NAME.to_string(),
                    surface.title().to_string(),
                    surface,
                )),
                // The embedded spec is compiled in and pinned by tests; a
                // parse failure here is a release defect. Fail loudly in
                // debug builds and surface it via tracing in release rather
                // than silently hiding the entry.
                Err(err) => {
                    debug_assert!(false, "embedded gate-summary spec invalid: {err}");
                    tracing::error!(
                        error = %err,
                        "embedded gate-summary spec failed to parse; picker entry omitted",
                    );
                }
            }
        }
    }

    let state = tui::run_surface(DashboardListState::new(items))?;
    // `run_surface` collapses quit vs back into the returned state; we act only
    // on an explicit choice. Picking a dashboard sets `chosen`, which launches
    // it; quitting leaves it `None`.
    match state.chosen {
        Some(name) => {
            if let (Some(saved), Some(root)) = (specs.iter().find(|s| s.name == name), root) {
                return launch_spec(saved, root, global);
            }
            if name == GATE_SUMMARY_NAME
                && let Some(root) = root
            {
                return launch_embedded_gate_summary(root, global);
            }
            launch(&name, global)
        }
        None => Ok(()),
    }
}

/// Launch the embedded gate-summary dashboard (UJ-009). Mirrors
/// [`launch_spec`]'s surface contract: `--json` emits the spec verbatim,
/// non-interactive prints a one-line note, a TTY runs the spec surface.
fn launch_embedded_gate_summary(root: &Path, global: &GlobalArgs) -> anyhow::Result<()> {
    let text = anvil_tui::dashboard_catalog::GATE_SUMMARY_SPEC;
    if global.json {
        println!("{text}");
        return Ok(());
    }

    let state = spec::load_str(text, root.to_path_buf())?;

    if global.no_tui || !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        println!(
            "Dashboard '{GATE_SUMMARY_NAME}' ({}) — run in an interactive terminal to view.",
            state.title()
        );
        return Ok(());
    }

    tui::run_surface(state)?;
    Ok(())
}

/// Launch a saved spec dashboard: parse it and render it through the json-render
/// engine. `--json` emits the raw spec; non-interactive prints a one-line note;
/// a TTY runs the spec surface.
fn launch_spec(saved: &SavedDashboard, root: &Path, global: &GlobalArgs) -> anyhow::Result<()> {
    if global.json {
        // Emit the spec verbatim so `--json` stays machine-readable. `read_raw`
        // applies the same no-follow + bounded-read guards as `spec::load`, so
        // the verbatim path can't follow a symlink to a device/out-of-tree file.
        let text = spec::read_raw(&saved.path)?;
        println!("{text}");
        return Ok(());
    }

    let state = spec::load(&saved.path, root.to_path_buf())?;

    // The renderer does not evaluate `visible` conditions yet (every element
    // renders); warn the operator so a hidden-looking section isn't a surprise.
    if state.has_unevaluated_visibility() {
        eprintln!(
            "note: dashboard '{}' uses `visible` conditions, which are not yet \
             evaluated — all elements are shown.",
            sanitize(&saved.name)
        );
    }

    if global.no_tui || !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        println!(
            "Dashboard '{}' ({}) — run in an interactive terminal to view.",
            sanitize(&saved.name),
            state.title()
        );
        return Ok(());
    }

    tui::run_surface(state)?;
    Ok(())
}

/// Launch a wired dashboard surface. The seam per-domain dashboards extend:
/// each `available` catalogue entry needs a matching arm here. An `available`
/// entry without an arm bails loudly rather than silently no-opping.
fn launch(name: &str, global: &GlobalArgs) -> anyhow::Result<()> {
    match name {
        "architecture" => architecture::run(global),
        "drift" => drift::run(global),
        "suppressions" => suppressions::run(global),
        other => anyhow::bail!("dashboard '{other}' has no surface wired yet"),
    }
}

fn print_picker(catalog: &[CatalogEntry], specs: &[SavedDashboard], embedded_gate_summary: bool) {
    print!("{}", format_picker(catalog, specs, embedded_gate_summary));
}

/// Render the plain-text picker. Split out from [`print_picker`] so the column
/// layout is unit-testable without capturing stdout. The name column is
/// self-sizing to the longest entry (native, saved spec, or built-in), so
/// adding a longer dashboard name never runs the name into its description.
fn format_picker(
    catalog: &[CatalogEntry],
    specs: &[SavedDashboard],
    embedded_gate_summary: bool,
) -> String {
    let mut out = String::from("Anvil Dashboards\n\n");
    // Saved-spec names are file stems from a possibly-hostile repo; sanitise
    // them before they reach stdout. Native names are static and trusted.
    let saved_names: Vec<String> = specs.iter().map(|s| sanitize(&s.name)).collect();
    let width = catalog
        .iter()
        .map(|entry| entry.name.len())
        .chain(saved_names.iter().map(String::len))
        .chain(embedded_gate_summary.then_some(GATE_SUMMARY_NAME.len()))
        .max()
        .unwrap_or(0);
    for entry in catalog {
        let suffix = if entry.available {
            ""
        } else {
            "  (coming soon)"
        };
        // Writing to a String is infallible; the result is intentionally ignored.
        let _ = writeln!(
            out,
            "  {:<width$}  {}{suffix}",
            entry.name, entry.description
        );
    }
    for name in &saved_names {
        let _ = writeln!(out, "  {name:<width$}  {SAVED_SPEC_DESCRIPTION}");
    }
    if embedded_gate_summary {
        let _ = writeln!(
            out,
            "  {GATE_SUMMARY_NAME:<width$}  {EMBEDDED_GATE_SUMMARY_DESCRIPTION}"
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_wires_all_three_dashboards() {
        let catalog = catalog();
        let names: Vec<_> = catalog.iter().map(|entry| entry.name).collect();
        assert_eq!(names, ["architecture", "drift", "suppressions"]);
        // TDASH-002/003/004 wired architecture, drift, and suppressions.
        let by_name = |n: &str| catalog.iter().find(|e| e.name == n).unwrap().available;
        assert!(by_name("architecture"), "architecture should be available");
        assert!(by_name("drift"), "drift should be available");
        assert!(by_name("suppressions"), "suppressions should be available");
    }

    #[test]
    fn resolve_no_name_opens_picker() {
        assert_eq!(resolve(None, &catalog()), Resolution::Picker);
    }

    #[test]
    fn resolve_architecture_launches() {
        assert_eq!(
            resolve(Some("architecture"), &catalog()),
            Resolution::Launch("architecture".to_string())
        );
    }

    #[test]
    fn resolve_drift_launches() {
        assert_eq!(
            resolve(Some("drift"), &catalog()),
            Resolution::Launch("drift".to_string())
        );
    }

    #[test]
    fn resolve_known_unavailable_is_coming_soon() {
        // All shipped dashboards are now available; use a synthetic unavailable
        // entry to keep covering the coming-soon resolution path.
        let catalog = vec![CatalogEntry {
            name: "future",
            title: "Future",
            description: "not wired yet",
            available: false,
        }];
        assert_eq!(
            resolve(Some("future"), &catalog),
            Resolution::ComingSoon("future".to_string())
        );
    }

    #[test]
    fn resolve_unknown_name() {
        assert_eq!(
            resolve(Some("bogus"), &catalog()),
            Resolution::Unknown("bogus".to_string())
        );
    }

    #[test]
    fn resolve_available_dashboard_launches() {
        let catalog = vec![CatalogEntry {
            name: "architecture",
            title: "t",
            description: "d",
            available: true,
        }];
        assert_eq!(
            resolve(Some("architecture"), &catalog),
            Resolution::Launch("architecture".to_string())
        );
    }

    #[test]
    fn json_catalog_contains_dashboard_names_and_availability() {
        let json = serde_json::to_string(&catalog()).unwrap();
        for name in ["architecture", "drift", "suppressions", "available"] {
            assert!(json.contains(name), "json missing {name}: {json}");
        }
    }

    #[test]
    fn plain_picker_separates_name_and_description_columns() {
        let text = format_picker(&catalog(), &[], true);
        for entry in catalog() {
            let line = text
                .lines()
                .find(|line| line.contains(entry.description))
                .unwrap_or_else(|| panic!("no line for {}", entry.name));
            // Self-sizing column guarantees whitespace between name and
            // description even for the longest name.
            assert!(line.contains(entry.name), "missing name in: {line:?}");
            assert!(
                line.contains(&format!("{} ", entry.name)),
                "name runs into description: {line:?}"
            );
        }
        // All shipped dashboards are available — none should carry the
        // coming-soon marker.
        assert!(!text.contains("coming soon"), "got:\n{text}");
    }

    #[test]
    fn picker_lists_saved_specs_after_native_dashboards() {
        let specs = vec![SavedDashboard {
            name: "my-gate".to_string(),
            title: "My Gate".to_string(),
            path: std::path::PathBuf::from(".anvil/dashboards/my-gate.json"),
        }];
        let text = format_picker(&catalog(), &specs, true);
        let line = text
            .lines()
            .find(|l| l.contains("my-gate"))
            .expect("saved spec listed");
        assert!(line.contains(SAVED_SPEC_DESCRIPTION), "got: {line:?}");
        // Saved specs come after the native dashboards.
        let arch = text.find("architecture").expect("native listed");
        let saved = text.find("my-gate").expect("saved listed");
        assert!(arch < saved, "native dashboards precede saved specs");
    }

    #[test]
    fn plain_picker_sanitises_saved_spec_stem() {
        // A hostile repo can name a spec file with control bytes in the stem;
        // the plain-text picker must not emit them to the terminal.
        let specs = vec![SavedDashboard {
            name: "evil\u{1b}]0;pwned\u{07}name".to_string(),
            title: "T".to_string(),
            path: std::path::PathBuf::from(".anvil/dashboards/x.json"),
        }];
        let text = format_picker(&catalog(), &specs, true);
        assert!(!text.contains('\u{1b}'), "ESC stripped from stem");
        assert!(!text.contains('\u{07}'), "BEL stripped from stem");
        assert!(
            text.contains("evil]0;pwnedname"),
            "sanitised stem shown: {text:?}"
        );
    }

    // --- UJ-009: embedded gate-summary reaches projects without a saved spec ---

    #[test]
    fn embedded_gate_summary_spec_loads() {
        let state = spec::load_str(
            anvil_tui::dashboard_catalog::GATE_SUMMARY_SPEC,
            std::path::PathBuf::from("."),
        )
        .expect("embedded gate-summary spec must parse");
        assert!(
            !state.title().is_empty(),
            "embedded spec carries a human title",
        );
    }

    #[test]
    fn embedded_gate_summary_yields_to_a_saved_spec() {
        assert!(
            embedded_gate_summary_available(&[]),
            "no saved specs: the embedded gate-summary serves upgraders",
        );
        let saved = SavedDashboard {
            name: GATE_SUMMARY_NAME.to_string(),
            title: "Customised".to_string(),
            path: std::path::PathBuf::from(".anvil/dashboards/gate-summary.dashboard.json"),
        };
        assert!(
            !embedded_gate_summary_available(std::slice::from_ref(&saved)),
            "a saved gate-summary spec (init-seeded or user-customised) must win",
        );
    }

    #[test]
    fn picker_lists_embedded_gate_summary_without_saved_specs() {
        let text = format_picker(&catalog(), &[], true);
        let line = text
            .lines()
            .find(|l| l.contains(GATE_SUMMARY_NAME))
            .expect("embedded gate-summary listed for projects without a saved spec");
        assert!(
            line.contains(EMBEDDED_GATE_SUMMARY_DESCRIPTION),
            "got: {line:?}",
        );
    }

    #[test]
    fn picker_lists_gate_summary_exactly_once_when_saved_spec_exists() {
        let saved = SavedDashboard {
            name: GATE_SUMMARY_NAME.to_string(),
            title: "Gate Summary".to_string(),
            path: std::path::PathBuf::from(".anvil/dashboards/gate-summary.dashboard.json"),
        };
        let specs = vec![saved];
        let text = format_picker(&catalog(), &specs, embedded_gate_summary_available(&specs));
        assert_eq!(
            text.matches(GATE_SUMMARY_NAME).count(),
            1,
            "saved spec shadows the embedded entry, no double listing:\n{text}",
        );
    }

    #[test]
    fn launch_bails_for_dashboard_without_an_arm() {
        // Defensive seam: a name with no launch arm must fail loudly, not
        // no-op. All catalogue entries are wired, so use a non-catalogue name.
        let global = GlobalArgs::default();
        assert!(launch("nonexistent", &global).is_err());
    }
}
