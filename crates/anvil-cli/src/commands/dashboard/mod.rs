//! `anvil dashboard [name]` — native read-only TUI dashboards over persisted
//! `.anvil/` state.
//!
//! TDASH-001 ships the command plus the picker scaffold. Per-domain dashboards
//! (architecture, drift, suppressions) land in TDASH-002+ by flipping their
//! catalogue entry to `available` and adding a launch arm in [`launch`].

use std::fmt::Write as _;
use std::io::IsTerminal;

use clap::Args;
use serde::Serialize;

use anvil_tui::surfaces::dashboard::{DashboardEntry, DashboardPickerState};

use crate::{GlobalArgs, tui};

mod architecture;
mod drift;

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
            description: "Active suppressions with scope, justification, approver",
            available: false,
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

    // Every terminal branch handles `--json` itself so a global `--json` never
    // leaks human text: a launched dashboard emits its own data; the picker and
    // coming-soon paths emit the catalogue.
    match resolve(args.name.as_deref(), &catalog) {
        Resolution::Unknown(name) => {
            let names = catalog
                .iter()
                .map(|entry| entry.name)
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!("unknown dashboard '{name}'. Valid dashboards: {names}")
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
        Resolution::Picker => run_picker(&catalog, global),
    }
}

fn run_picker(catalog: &[CatalogEntry], global: &GlobalArgs) -> anyhow::Result<()> {
    if global.json {
        println!("{}", serde_json::to_string_pretty(catalog)?);
        return Ok(());
    }

    if global.no_tui || !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
        print_picker(catalog);
        return Ok(());
    }

    let entries = catalog.iter().map(to_entry).collect();
    let state = tui::run_surface(DashboardPickerState::new(entries))?;
    // `run_surface` collapses quit vs back into the returned state; we act only
    // on an explicit choice. Picking an `available` dashboard sets `chosen`,
    // which launches it; quitting leaves it `None`.
    match state.chosen {
        Some(name) => launch(&name, global),
        None => Ok(()),
    }
}

/// Launch a wired dashboard surface. The seam per-domain dashboards extend:
/// each `available` catalogue entry needs a matching arm here. An `available`
/// entry without an arm bails loudly rather than silently no-opping.
fn launch(name: &str, global: &GlobalArgs) -> anyhow::Result<()> {
    match name {
        "architecture" => architecture::run(global),
        "drift" => drift::run(global),
        other => anyhow::bail!("dashboard '{other}' has no surface wired yet"),
    }
}

fn to_entry(entry: &CatalogEntry) -> DashboardEntry {
    DashboardEntry::new(entry.name, entry.title, entry.description, entry.available)
}

fn print_picker(catalog: &[CatalogEntry]) {
    print!("{}", format_picker(catalog));
}

/// Render the plain-text picker. Split out from [`print_picker`] so the column
/// layout is unit-testable without capturing stdout. The name column is
/// self-sizing to the longest catalogue entry, so adding a longer dashboard
/// name never runs the name into its description.
fn format_picker(catalog: &[CatalogEntry]) -> String {
    let mut out = String::from("Anvil Dashboards\n\n");
    let width = catalog
        .iter()
        .map(|entry| entry.name.len())
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
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_lists_three_dashboards_with_architecture_and_drift_wired() {
        let catalog = catalog();
        let names: Vec<_> = catalog.iter().map(|entry| entry.name).collect();
        assert_eq!(names, ["architecture", "drift", "suppressions"]);
        // TDASH-002 wires architecture, TDASH-003 wires drift; suppressions
        // stays coming-soon.
        let by_name = |n: &str| catalog.iter().find(|e| e.name == n).unwrap().available;
        assert!(by_name("architecture"), "architecture should be available");
        assert!(by_name("drift"), "drift should be available");
        assert!(!by_name("suppressions"));
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
        assert_eq!(
            resolve(Some("suppressions"), &catalog()),
            Resolution::ComingSoon("suppressions".to_string())
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
        let text = format_picker(&catalog());
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
        assert!(text.contains("coming soon"), "got:\n{text}");
    }

    #[test]
    fn launch_bails_for_dashboard_without_an_arm() {
        // Defensive seam: a name with no launch arm (e.g. a coming-soon entry
        // flipped to `available` without wiring) must fail loudly, not no-op.
        let global = GlobalArgs::default();
        assert!(launch("suppressions", &global).is_err());
    }
}
