use std::fs;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use anvil_tui::surfaces::init::InitState;
use anyhow::Context;
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;
use crate::commands::defaults::{default_available_checks, default_check_names};
use crate::output::plain;
use crate::services::sample_analyser::{
    self, AnalysisOutcome, SampleSource, run_post_init_analysis,
};
use anvil_checks::antipattern::WarningSeverity;

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Overwrite existing configuration without prompting.
    #[arg(long)]
    pub force: bool,
}

/// Schema version for generated `.anvilrc` files.
const SCHEMA_VERSION: &str = "1.0.0";

/// The single config filename anvil writes, regardless of the chosen
/// serialisation format (the format is stored inside the file, not in its
/// name). Callers that report the written file to the user derive their copy
/// from this so the summary can never drift from what was actually created
/// (CIB-171).
pub(crate) const CONFIG_FILE_NAME: &str = ".anvilrc";

/// Outcome of writing a fresh anvil config. Exposes the path actually written
/// so callers (e.g. the welcome landing summary) can name the real file rather
/// than a hardcoded literal (CIB-171).
#[derive(Debug, Clone)]
pub(crate) struct GeneratedConfig {
    /// The config file that was written — always `CONFIG_FILE_NAME` under the
    /// init root.
    pub config_path: PathBuf,
    /// Whether `.gitignore` was updated with anvil's entries.
    pub gitignore_updated: bool,
}

// Canonical snake_case key space (ADR-120 pt 3 / UCFG-003): serde's default
// field-name serialisation is already snake_case, so no rename attribute.
// Legacy camelCase files stay readable via `InitConfigView`'s camel-or-snake
// tolerance; `anvil migrate schema` rewrites them to snake_case (via
// `anvil_config::normalize_legacy_keys`, which is migration-only, not a
// read-path transform).
#[derive(Debug, Serialize)]
pub(crate) struct AnvilConfig {
    pub(crate) schema_version: String,
    pub(crate) planning_dir: String,
    pub(crate) format: String,
    pub(crate) checks: Vec<String>,
}

impl Default for AnvilConfig {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            planning_dir: "plans".to_string(),
            format: "yaml".to_string(),
            checks: default_check_names(),
        }
    }
}

/// Who is running init, which decides whether init owns its closing next-step
/// line. Standalone `anvil init` names the single next step (`anvil start`);
/// init composed inside the activation orchestrator suppresses that line so the
/// activation ending owns the one next step instead of telling the user to run
/// the command they just ran (CIB-163, UJ-001 one-next-step intent).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InitInvocation {
    /// Direct `anvil init` — keeps the "Next: run `anvil start`" line.
    Standalone,
    /// Init run inline by `anvil start`'s orchestrator — drops the next-step
    /// line; the activation ending owns it.
    FromStart,
}

pub fn run(args: &InitArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    let root = PathBuf::from(".");
    run_in(args, global, &root, InitInvocation::Standalone)
}

/// Run the init flow against a specific root. Public to the crate so
/// `activation::orchestrator` (LAUNCH-006) can compose init without going
/// through the process-CWD-bound `run` entrypoint, which makes the
/// orchestration unit-testable against a temp dir.
pub(crate) fn run_in(
    args: &InitArgs,
    global: &GlobalArgs,
    root: &Path,
    invocation: InitInvocation,
) -> anyhow::Result<()> {
    // DISTRIB-006 (ADR-060): `anvil init` (and the onboarding flow that composes
    // it) seeds `.anvilrc` + `.anvil/` — durable per-project config the production
    // binary reads. Refuse under a non-default ANVIL_HOME without
    // `--touch-project-state`. The orchestrator's own init step is separately
    // short-circuited before reaching here, so this only refuses the direct path.
    crate::install_root::ensure_project_write_allowed("init")?;

    // Use the shared `config_exists_in` helper so `init` and the onboarding
    // flow agree on whether a config exists — they previously diverged on
    // zero-byte `.anvilrc` files, leaving onboarding calling `init` and
    // `init` immediately bailing on the empty file it could not see.
    if let Some(existing) = anvil_tui::surfaces::onboarding::existing_config_name_in(root)
        && !args.force
    {
        // Name the file we actually detected — a repo with only `.anvil.yaml`
        // must not claim `.anvilrc` already exists (Dave pack-04 CFG-1).
        anyhow::bail!("{existing} already exists. Use --force to overwrite.");
    }

    // A zero-byte `.anvilrc` is treated as "missing" by `config_exists_in`,
    // but `write_new` (O_CREAT | O_EXCL) would still fail because the inode
    // exists. Remove the empty stub so the upcoming create proceeds cleanly
    // — the only information it could possibly hold is "nothing".
    if !args.force {
        let config_path = root.join(".anvilrc");
        if let Ok(meta) = fs::metadata(&config_path)
            && meta.is_file()
            && meta.len() == 0
        {
            fs::remove_file(&config_path)
                .with_context(|| format!("failed to remove empty {}", config_path.display()))?;
        }
    }

    if global.json {
        let config = AnvilConfig::default();
        generate_config_with_force(&config, root, args.force)?;
        let json = serde_json::to_string_pretty(&config)?;
        println!("{json}");
    } else if global.no_tui || !std::io::stdout().is_terminal() {
        run_plain(root, args.force, invocation)?;
    } else {
        run_tui(root, args.force, invocation)?;
    }

    Ok(())
}

fn run_tui(root: &Path, force: bool, invocation: InitInvocation) -> anyhow::Result<()> {
    let available = default_available_checks();
    let state = InitState::new(available);
    let state = crate::tui::run_surface(state)?;

    if !state.confirmed {
        println!("Init cancelled.");
        return Ok(());
    }

    let checks: Vec<String> = if state.config.checks.is_empty() {
        default_check_names()
    } else {
        state.config.checks
    };

    // Use the selected directory as the init root (not just the planning dir).
    let init_root = if state.config.directory == "." {
        root.to_path_buf()
    } else {
        root.join(&state.config.directory)
    };
    fs::create_dir_all(&init_root)
        .with_context(|| format!("failed to create directory {}", init_root.display()))?;

    let config = AnvilConfig {
        schema_version: SCHEMA_VERSION.to_string(),
        planning_dir: "plans".to_string(),
        format: format_label(state.config.format),
        checks: checks.clone(),
    };

    let generated = generate_config_with_force(&config, &init_root, force)?;
    print_success(
        &config.planning_dir,
        &checks,
        invocation,
        generated.gitignore_updated,
    );
    print_capacity_recommendation(&init_root);
    print_post_init_analysis(&init_root);
    Ok(())
}

fn run_plain(root: &Path, force: bool, invocation: InitInvocation) -> anyhow::Result<()> {
    let config = AnvilConfig::default();
    let generated = generate_config_with_force(&config, root, force)?;
    print_success(
        &config.planning_dir,
        &config.checks,
        invocation,
        generated.gitignore_updated,
    );
    print_capacity_recommendation(root);
    print_post_init_analysis(root);
    Ok(())
}

/// Run the post-init sample analysis (LAUNCH-004) and render an inline
/// summary so the user lands on a real first signal of value instead of
/// a "now run `anvil doctor`" stub. A project with no files matching the
/// first scan's language coverage gets a discoverable next-step hint rather
/// than silence.
fn print_post_init_analysis(root: &Path) {
    let Some(outcome) = run_post_init_analysis(root) else {
        render_no_matching_files_hint();
        return;
    };
    render_analysis(&outcome);
}

/// First-touch hint when no files match the first scan's language coverage.
/// Without it, the analysis would print nothing under "First scan", which
/// reads as a failure.
fn render_no_matching_files_hint() {
    plain::blank();
    plain::section("First scan");
    plain::dim("No files matched this first scan's language coverage — nothing was scanned.");
    plain::blank();
    plain::dim("Try one of:");
    plain::dim("  • `anvil tutorial` for a guided walkthrough");
    plain::dim("  • `anvil watch` once you've added some code");
    plain::dim("  • `anvil check --all` to scan the whole project later");
    plain::dim("  • `anvil auth login` to authenticate (unlocks gate-evaluated checks)");
    plain::blank();
}

fn render_analysis(outcome: &AnalysisOutcome) {
    plain::blank();
    plain::section("First scan");

    let source_label = match outcome.source {
        SampleSource::GitHistory => "from recent git history",
        SampleSource::RepoWalk => "sampled from project tree",
        // `Empty` is filtered before reaching here — guard rather than panic.
        SampleSource::Empty => "no files matched",
    };
    plain::dim(&format!(
        "Scanned {} file(s) ({source_label}) in {}ms",
        outcome.files_scanned,
        outcome.elapsed.as_millis()
    ));

    // LAUNCH-016: name the skip honestly only if the post-init
    // sample contained any unsupported-language files. The
    // `select_sample` step pre-filters via the antipattern
    // extension allowlist, so the ledger is typically empty in this
    // path — broader repo composition is surfaced separately via
    // `anvil status --verify`'s language profile, not here. This
    // branch is preserved for the contract: when downstream PRs
    // adopt the partition helper at scan/watch sites without an
    // existing pre-filter, this surfaces the skip honestly.
    if !outcome.skipped_unsupported_languages.is_empty() {
        let parts: Vec<String> = outcome
            .skipped_unsupported_languages
            .by_language
            .iter()
            .map(|(lang, count)| format!("{count} {lang}"))
            .collect();
        plain::dim(&format!(
            "Skipped {} ({}) — language-specific checks not yet shipped for these.",
            parts.join(", "),
            outcome.skipped_unsupported_languages.reason.label(),
        ));
    }

    if outcome.exceeded_budget {
        plain::dim(&format!(
            "Note: scan took longer than the {}s soft budget — large samples may be trimmed in a future release.",
            sample_analyser::ANALYSIS_TIME_BUDGET.as_secs()
        ));
    }

    plain::blank();
    // Guard: if no files were actually scanned (extension mismatch, unreadable
    // content, non-UTF8, etc.), "No warnings found" would be misleading.
    // Mirrors the pattern in `commands/check.rs`.
    if outcome.files_scanned == 0 {
        plain::warn("No analysable files found (0 scanned) — skipping first scan.");
        plain::blank();
        return;
    }

    let s = &outcome.summary;
    if s.total == 0 {
        plain::dim("No anti-pattern warnings found in this sample.");
        plain::dim("Run `anvil check --all` to scan the whole project.");
        plain::dim("Run `anvil auth login` to enable additional checks.");
        plain::blank();
        return;
    }

    plain::label("Total", s.total);
    if s.errors > 0 {
        plain::label("Errors", s.errors);
    }
    if s.warnings > 0 {
        plain::label("Warnings", s.warnings);
    }
    if s.info > 0 {
        plain::label("Info", s.info);
    }
    if s.suppressed > 0 {
        plain::label("Suppressed", s.suppressed);
    }

    if !outcome.top_warnings.is_empty() {
        plain::blank();
        plain::dim("Top findings:");
        for w in &outcome.top_warnings {
            let icon = severity_icon(w.severity);
            plain::item(icon, &format!("[{}] {}", w.id, w.title));
            plain::dim(&format!("{}:{}", w.file, w.line));
        }
    }

    plain::blank();
    plain::dim("Run `anvil check --all` for the full report.");
    plain::dim("Run `anvil auth login` to unlock gate-evaluated checks.");
    plain::blank();
}

const fn severity_icon(severity: WarningSeverity) -> &'static str {
    match severity {
        WarningSeverity::Error => "\u{2717}",
        WarningSeverity::Warning => "\u{26a0}",
        WarningSeverity::Info => "\u{2139}",
    }
}

/// Surface a one-shot recommendation when the host's inotify headroom is
/// tight enough that `anvil watch` would risk missing file changes. Silent
/// on non-Linux hosts and on healthy Linux hosts.
fn print_capacity_recommendation(root: &Path) {
    let Some(status) = crate::capacity::collect(root) else {
        return;
    };
    let lines = crate::capacity::recommendation_lines(&status);
    for line in lines {
        println!("{line}");
    }
}

pub(crate) fn generate_config(
    config: &AnvilConfig,
    root: &Path,
) -> anyhow::Result<GeneratedConfig> {
    generate_config_with_force(config, root, false)
}

pub(crate) fn generate_config_with_force(
    config: &AnvilConfig,
    root: &Path,
    force: bool,
) -> anyhow::Result<GeneratedConfig> {
    let content = match config.format.as_str() {
        "toml" => toml_serialise(config),
        "yaml" => yaml_serialise(config),
        _ => serde_json::to_string_pretty(config).context("failed to serialise config")?,
    };
    // Ensure the root exists before any file writes — `write_new` opens with
    // O_CREAT | O_EXCL and will fail with NotFound rather than a useful
    // "directory missing" error if a caller passes a freshly-picked path.
    fs::create_dir_all(root)
        .with_context(|| format!("failed to create directory {}", root.display()))?;
    let path = root.join(CONFIG_FILE_NAME);
    if force {
        crate::util::atomic_write(&path, content.as_bytes()).context("failed to write .anvilrc")?;
    } else {
        crate::util::write_new(&path, content.as_bytes()).context("failed to write .anvilrc")?;
    }

    fs::create_dir_all(root.join(".anvil/cache")).context("failed to create .anvil/cache/")?;

    // Seed the example gate-summary dashboard so `anvil dashboard gate-summary`
    // works out of the box after a gate run (#2237).
    seed_example_dashboard(root, force)?;

    let gitignore_updated = append_gitignore_entry(root)?;

    let planning_dir = root.join(&config.planning_dir);
    if !planning_dir.exists() {
        fs::create_dir_all(&planning_dir)
            .with_context(|| format!("failed to create {}/", config.planning_dir))?;
    }

    Ok(GeneratedConfig {
        config_path: path,
        gitignore_updated,
    })
}

/// The example `gate-summary` dashboard spec seeded into `.anvil/dashboards/`.
/// Re-exported from anvil-tui's crate assets — the single source of truth the
/// engine tests bind against (moved out of `.anvil/` under ADR-073, CIB-053).
const GATE_SUMMARY_SPEC: &str = anvil_tui::dashboard_catalog::GATE_SUMMARY_SPEC;

/// Seed the example gate-summary dashboard so `anvil dashboard gate-summary`
/// works after a gate run. Skips an existing spec unless `force`, so a user's
/// customised dashboard is never clobbered.
fn seed_example_dashboard(root: &Path, force: bool) -> anyhow::Result<()> {
    let dir = root.join(".anvil/dashboards");
    fs::create_dir_all(&dir).context("failed to create .anvil/dashboards/")?;
    let path = dir.join("gate-summary.dashboard.json");
    if force || !path.exists() {
        crate::util::atomic_write(&path, GATE_SUMMARY_SPEC.as_bytes())
            .context("failed to write gate-summary dashboard")?;
    }
    Ok(())
}

fn append_gitignore_entry(root: &Path) -> anyhow::Result<bool> {
    // ADR-073: the whole `.anvil/` tree is local runtime state and is ignored
    // wholesale (no tracked sub-path is justified today; use a `!` re-include
    // if one ever is). `anvil/exceptions/.lock` (EXCEPT-007) and
    // `anvil/witness/.chain-initialised` (CIB-126) are the runtime artefacts inside
    // the otherwise-tracked anvil/ governance tree — the witness chain-init marker
    // is durable *local* state (its presence, not its history, is load-bearing; it
    // self-heals via backfill on the first append), so it is ignored rather than
    // committed. Repos initialised before ADR-073 may also carry the narrow
    // `.anvil/cache/` / `.anvil/gates.json` entries — harmless duplicates.
    const ENTRIES: [&str; 3] = [
        ".anvil/",
        "anvil/exceptions/.lock",
        "anvil/witness/.chain-initialised",
    ];

    let gitignore = root.join(".gitignore");

    // Refuse to modify a symlinked .gitignore — a hostile symlink could
    // redirect the append into a file outside the project root. We use
    // `symlink_metadata` so we see the link itself rather than following it.
    match fs::symlink_metadata(&gitignore) {
        Ok(meta) if meta.file_type().is_symlink() => {
            anyhow::bail!(
                ".gitignore is a symbolic link; refusing to modify for safety. Resolve manually and re-run."
            );
        }
        Ok(_) | Err(_) => {}
    }

    // Read the file once so we can both scan for the existing entry and
    // decide whether a leading newline is needed. A missing file is not an
    // error; any other read failure is surfaced rather than silently
    // swallowed with `unwrap_or_default`.
    let existing: Option<String> = match fs::read_to_string(&gitignore) {
        Ok(s) => Some(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e).context("failed to read .gitignore"),
    };

    // Trimmed-line equality. We do not strip trailing inline comments (e.g.
    // `".anvil/cache/ # keep"`), so a hand-authored entry with a comment would
    // trigger a duplicate append — anvil never writes that form, and a duplicate
    // is harmless to git. Append only the entries not already present.
    let present: std::collections::HashSet<&str> = existing
        .as_deref()
        .map(|c| c.lines().map(str::trim).collect())
        .unwrap_or_default();
    let missing: Vec<&str> = ENTRIES
        .iter()
        .copied()
        .filter(|e| !present.contains(e))
        .collect();
    if missing.is_empty() {
        return Ok(false);
    }

    let needs_newline = existing
        .as_deref()
        .is_some_and(|c| !c.is_empty() && !c.ends_with('\n'));

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&gitignore)
        .context("failed to open .gitignore for appending")?;

    if needs_newline {
        writeln!(file).context("failed to write newline to .gitignore")?;
    }

    for entry in missing {
        writeln!(file, "{entry}").context("failed to append to .gitignore")?;
    }
    Ok(true)
}

fn print_success(
    planning_dir: &str,
    checks: &[String],
    invocation: InitInvocation,
    gitignore_updated: bool,
) {
    print!(
        "{}",
        success_message(planning_dir, checks, invocation, gitignore_updated)
    );
}

/// The init closing block. Standalone `anvil init` ends with a single next-step
/// line (UJ-001) so the onboarding path never dead-ends; init composed inside
/// `anvil start` drops that line, because the activation ending owns the one
/// next step and telling the user to run `anvil start` — the command they just
/// ran — is misdirection (CIB-163).
///
/// CIB-263: when init mutated `.gitignore`, name that path in the summary so
/// the blast radius is honest (Config/Plans/Checks alone under-reported it).
fn success_message(
    planning_dir: &str,
    checks: &[String],
    invocation: InitInvocation,
    gitignore_updated: bool,
) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out);
    let _ = writeln!(out, "anvil initialised successfully.");
    let _ = writeln!(out, "  Config:    {CONFIG_FILE_NAME}");
    let _ = writeln!(out, "  Plans:     {planning_dir}/");
    let _ = writeln!(out, "  Checks:    {}", checks.join(", "));
    if gitignore_updated {
        let _ = writeln!(out, "  Gitignore: .gitignore");
    }
    if invocation == InitInvocation::Standalone {
        let _ = writeln!(out);
        let _ = writeln!(out, "  Next: run `anvil start` to activate protection.");
    }
    out
}

/// Simple YAML serialisation (no external crate needed for this shape).
fn yaml_serialise(config: &AnvilConfig) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "schema_version: \"{}\"", config.schema_version);
    let _ = writeln!(out, "planning_dir: \"{}\"", config.planning_dir);
    let _ = writeln!(out, "format: \"{}\"", config.format);
    out.push_str("checks:\n");
    for check in &config.checks {
        let _ = writeln!(out, "  - \"{check}\"");
    }
    out
}

/// Simple TOML serialisation.
fn toml_serialise(config: &AnvilConfig) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "schema_version = \"{}\"", config.schema_version);
    let _ = writeln!(out, "planning_dir = \"{}\"", config.planning_dir);
    let _ = writeln!(out, "format = \"{}\"", config.format);
    let checks: Vec<String> = config.checks.iter().map(|c| format!("\"{c}\"")).collect();
    let _ = writeln!(out, "checks = [{}]", checks.join(", "));
    out
}

pub(crate) fn format_label(fmt: anvil_tui::surfaces::init::ConfigFormat) -> String {
    match fmt {
        anvil_tui::surfaces::init::ConfigFormat::Yaml => "yaml".to_string(),
        anvil_tui::surfaces::init::ConfigFormat::Json => "json".to_string(),
        anvil_tui::surfaces::init::ConfigFormat::Toml => "toml".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_tui_global() -> GlobalArgs {
        GlobalArgs {
            json: false,
            no_tui: true,
            verbose: false,
            ..Default::default()
        }
    }

    // ---- UCFG-003: owned writers emit canonical snake_case only ----

    #[test]
    fn yaml_serialise_emits_snake_case_keys() {
        let out = yaml_serialise(&AnvilConfig::default());
        assert!(out.contains("schema_version:"), "got:\n{out}");
        assert!(out.contains("planning_dir:"), "got:\n{out}");
        assert!(!out.contains("schemaVersion"), "got:\n{out}");
        assert!(!out.contains("planningDir"), "got:\n{out}");
    }

    #[test]
    fn json_serialise_emits_snake_case_keys() {
        let out = serde_json::to_string_pretty(&AnvilConfig::default()).unwrap();
        assert!(out.contains("\"schema_version\""), "got:\n{out}");
        assert!(out.contains("\"planning_dir\""), "got:\n{out}");
        assert!(!out.contains("schemaVersion"), "got:\n{out}");
    }

    /// Cross-format equivalence (UCFG-003 validation): each serialiser's
    /// output parses back through the MLP-011 loader and reads identically
    /// through MLP2-041's `InitConfigView`.
    #[test]
    fn all_formats_round_trip_through_init_config_view() {
        let config = AnvilConfig::default();
        let cases = [
            (yaml_serialise(&config), anvil_config::ConfigFormat::Yaml),
            (toml_serialise(&config), anvil_config::ConfigFormat::Toml),
            (
                serde_json::to_string_pretty(&config).unwrap(),
                anvil_config::ConfigFormat::Json,
            ),
        ];
        for (content, format) in cases {
            let value = anvil_config::parse_str(&content, format, std::path::Path::new("fixture"))
                .unwrap_or_else(|e| panic!("{format:?} parse failed: {e}\n{content}"));
            let view = crate::config_view::InitConfigView::from_value(&value)
                .unwrap_or_else(|e| panic!("{format:?} view failed: {e}\n{content}"));
            assert_eq!(view.schema_version, config.schema_version, "{format:?}");
            assert_eq!(view.planning_dir, config.planning_dir, "{format:?}");
            assert_eq!(view.checks, config.checks, "{format:?}");
        }
    }

    // UJ-001: the standalone init ending must name the single next step.
    #[test]
    fn init_success_names_anvil_start_as_next_step() {
        let msg = success_message(
            "plans",
            &["secret-detection".to_string()],
            InitInvocation::Standalone,
            true,
        );
        let next = msg
            .lines()
            .find(|l| l.contains("Next:"))
            .unwrap_or_else(|| panic!("init success must print a next-step line:\n{msg}"));
        assert!(
            next.contains("anvil start"),
            "the init next step is `anvil start`, got: {next}",
        );
    }

    // CIB-163: init composed inside `anvil start` must not tell the user to run
    // `anvil start` — the command they just ran. The activation ending owns the
    // single next step (UJ-001), so init's closing next-step line is suppressed
    // on the `FromStart` path while the standalone path is unchanged.
    #[test]
    fn init_from_start_suppresses_anvil_start_next_step() {
        let msg = success_message(
            "plans",
            &["secret-detection".to_string()],
            InitInvocation::FromStart,
            true,
        );
        assert!(
            !msg.contains("Next:"),
            "init run inline by `anvil start` must not print a next-step line (CIB-163):\n{msg}"
        );
        assert!(
            !msg.contains("anvil start"),
            "init run inline by `anvil start` must not instruct the user to run \
             `anvil start` (CIB-163):\n{msg}"
        );
        // The rest of the success block (still-useful context) is unchanged.
        assert!(
            msg.contains("anvil initialised successfully."),
            "the success confirmation is still printed on the FromStart path:\n{msg}"
        );
    }

    // The standalone path stays byte-for-byte what it was: same content plus the
    // next-step line the FromStart path drops.
    #[test]
    fn standalone_and_from_start_differ_only_by_next_step_line() {
        let checks = ["secret-detection".to_string()];
        let standalone = success_message("plans", &checks, InitInvocation::Standalone, true);
        let from_start = success_message("plans", &checks, InitInvocation::FromStart, true);
        assert!(
            standalone.starts_with(&from_start),
            "standalone output is the FromStart output plus a trailing next-step block:\n\
             standalone:\n{standalone}\nfrom_start:\n{from_start}"
        );
        assert!(standalone.len() > from_start.len());
    }

    // CIB-263: when init appends ignore entries, the summary must name `.gitignore`
    // so the blast radius matches the files actually touched.
    #[test]
    fn success_message_lists_gitignore_when_updated() {
        let msg = success_message(
            "plans",
            &["secret-detection".to_string()],
            InitInvocation::Standalone,
            true,
        );
        assert!(
            msg.lines().any(|l| l.contains(".gitignore")),
            "success summary must name .gitignore when it was updated:\n{msg}"
        );
        assert!(
            msg.contains("Gitignore:"),
            "gitignore line should use the same label layout as Config/Plans/Checks:\n{msg}"
        );
    }

    // CIB-263: when every managed entry was already present, do not claim we
    // touched `.gitignore`.
    #[test]
    fn success_message_omits_gitignore_when_unchanged() {
        let msg = success_message(
            "plans",
            &["secret-detection".to_string()],
            InitInvocation::Standalone,
            false,
        );
        assert!(
            !msg.contains(".gitignore") && !msg.contains("Gitignore:"),
            "success summary must not claim a gitignore update when none happened:\n{msg}"
        );
    }

    // CIB-263: fresh plain init mutates `.gitignore` and the success summary
    // must surface that path (integration of generate + summary flag).
    #[test]
    fn generate_config_reports_gitignore_updated_on_fresh_repo() {
        let dir = tempfile::tempdir().unwrap();
        // Pre-existing tracked-style gitignore without anvil entries.
        fs::write(dir.path().join(".gitignore"), "node_modules/\n").unwrap();
        let generated = generate_config(&AnvilConfig::default(), dir.path()).expect("generate");
        assert!(
            generated.gitignore_updated,
            "fresh repo with partial .gitignore must report an update"
        );
        let msg = success_message(
            "plans",
            &AnvilConfig::default().checks,
            InitInvocation::Standalone,
            generated.gitignore_updated,
        );
        assert!(
            msg.contains(".gitignore"),
            "summary for a gitignore-mutating init must name the file:\n{msg}"
        );
    }

    #[test]
    fn embedded_gate_summary_spec_is_valid_json() {
        let v: serde_json::Value =
            serde_json::from_str(GATE_SUMMARY_SPEC).expect("embedded spec parses");
        assert_eq!(v["title"], "Gate Summary");
        assert_eq!(v["root"], "page");
    }

    #[test]
    fn init_gitignores_anvil_runtime_tree_wholesale() {
        let dir = tempfile::tempdir().unwrap();
        run_plain(dir.path(), false, InitInvocation::Standalone).expect("init");
        let gi = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(
            gi.lines().any(|l| l.trim() == ".anvil/"),
            "the whole .anvil/ runtime tree is ignored (ADR-073): {gi}"
        );
        assert!(
            gi.contains("anvil/exceptions/.lock"),
            "exception-store write lock ignored so writes don't dirty the tracked governance dir"
        );
        assert!(
            !gi.contains(".anvil/cache/") && !gi.contains(".anvil/gates.json"),
            "narrow legacy entries are not seeded into fresh repos — the wholesale \
             .anvil/ entry covers them: {gi}"
        );
    }

    #[test]
    fn init_gitignore_upgrades_legacy_narrow_entries() {
        // A repo initialised before ADR-073 has only the narrow entries; a
        // re-run must add the wholesale `.anvil/` line (duplicates of the
        // legacy lines are not re-appended).
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".gitignore"),
            ".anvil/cache/\n.anvil/gates.json\nanvil/exceptions/.lock\n",
        )
        .unwrap();
        let updated = append_gitignore_entry(dir.path()).expect("append");
        assert!(updated, "legacy gitignore gains the wholesale entry");
        let gi = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(gi.lines().any(|l| l.trim() == ".anvil/"));
        assert_eq!(
            gi.matches("anvil/exceptions/.lock").count(),
            1,
            "already-present entries are not duplicated"
        );
    }

    #[test]
    fn init_gitignore_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let first = append_gitignore_entry(dir.path()).expect("first append");
        let second = append_gitignore_entry(dir.path()).expect("second append");
        assert!(first, "fresh repo gets the entries");
        assert!(!second, "second run appends nothing");
        let gi = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert_eq!(
            gi.lines().filter(|l| l.trim() == ".anvil/").count(),
            1,
            "exactly one wholesale entry"
        );
    }

    #[test]
    fn seeds_gate_summary_dashboard() {
        let dir = tempfile::tempdir().unwrap();
        run_plain(dir.path(), false, InitInvocation::Standalone).expect("init");
        let spec = dir
            .path()
            .join(".anvil/dashboards/gate-summary.dashboard.json");
        assert!(spec.exists(), "gate-summary dashboard seeded by init");
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&spec).unwrap()).unwrap();
        assert_eq!(v["title"], "Gate Summary");
    }

    #[test]
    fn seeding_preserves_a_user_dashboard_unless_forced() {
        let dir = tempfile::tempdir().unwrap();
        let dash = dir.path().join(".anvil/dashboards");
        std::fs::create_dir_all(&dash).unwrap();
        let path = dash.join("gate-summary.dashboard.json");
        std::fs::write(&path, "CUSTOM").unwrap();

        seed_example_dashboard(dir.path(), false).expect("seed");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "CUSTOM",
            "existing user dashboard is not clobbered without --force"
        );

        seed_example_dashboard(dir.path(), true).expect("seed force");
        assert_ne!(
            std::fs::read_to_string(&path).unwrap(),
            "CUSTOM",
            "--force overwrites with the shipped spec"
        );
    }

    #[test]
    fn creates_anvilrc_and_anvil_dir() {
        let dir = tempfile::tempdir().unwrap();

        let result = run_plain(dir.path(), false, InitInvocation::Standalone);

        assert!(result.is_ok());
        assert!(dir.path().join(".anvilrc").exists());
        assert!(dir.path().join(".anvil").is_dir());
        assert!(dir.path().join(".anvil/cache").is_dir());
        assert!(dir.path().join("plans").is_dir());

        let content = fs::read_to_string(dir.path().join(".anvilrc")).unwrap();
        // Default format is YAML in the canonical snake_case key space
        // (ADR-120 / UCFG-003), so check as text.
        assert!(content.contains("schema_version: \"1.0.0\""));
        assert!(content.contains("planning_dir: \"plans\""));
        assert!(content.contains("format: \"yaml\""));
        assert!(content.contains("- \"secret-detection\""));
        assert!(content.contains("- \"import-boundaries\""));
        assert!(content.contains("- \"antipattern-scan\""));

        let gitignore = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(gitignore.lines().any(|l| l.trim() == ".anvil/"));
    }

    // CIB-171: generate_config exposes the path it actually wrote so the
    // welcome landing summary names the real file rather than a literal.
    #[test]
    fn generate_config_reports_the_written_path() {
        let dir = tempfile::tempdir().unwrap();
        let config = AnvilConfig {
            format: "yaml".to_string(),
            ..Default::default()
        };

        let generated = generate_config(&config, dir.path()).expect("generate");

        assert_eq!(generated.config_path, dir.path().join(CONFIG_FILE_NAME));
        assert_eq!(
            generated.config_path.file_name().and_then(|n| n.to_str()),
            Some(CONFIG_FILE_NAME),
        );
        assert!(generated.config_path.exists());
    }

    #[test]
    fn existing_anvilrc_blocks_without_force() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".anvilrc"), "{}").unwrap();

        let args = InitArgs { force: false };
        let global = no_tui_global();
        let result = run_in(&args, &global, dir.path(), InitInvocation::Standalone);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already exists"), "got: {err}");
        assert!(
            err.contains(".anvilrc"),
            "refusal should name the existing file, got: {err}"
        );
    }

    #[test]
    fn existing_anvil_yaml_blocks_and_names_yaml() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".anvil.yaml"),
            "schemaVersion: 1.0.0
planningDir: plans
",
        )
        .unwrap();

        let args = InitArgs { force: false };
        let global = no_tui_global();
        let result = run_in(&args, &global, dir.path(), InitInvocation::Standalone);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains(".anvil.yaml already exists"),
            "must name .anvil.yaml, not a phantom .anvilrc; got: {err}"
        );
        assert!(
            !err.contains(".anvilrc already exists"),
            "must not claim .anvilrc exists when only .anvil.yaml is present; got: {err}"
        );
    }

    #[test]
    fn force_overwrites_existing_anvilrc() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".anvilrc"), r#"{"old": true}"#).unwrap();

        let args = InitArgs { force: true };
        let global = no_tui_global();
        let result = run_in(&args, &global, dir.path(), InitInvocation::Standalone);
        assert!(result.is_ok());

        let content = fs::read_to_string(dir.path().join(".anvilrc")).unwrap();
        assert!(content.contains("schema_version"));
        assert!(content.contains("1.0.0"));
        assert!(!content.contains("old"));
    }

    #[test]
    fn gitignore_not_duplicated() {
        let dir = tempfile::tempdir().unwrap();
        // Seed every managed entry so there is nothing left to append.
        fs::write(
            dir.path().join(".gitignore"),
            ".anvil/\nanvil/exceptions/.lock\nanvil/witness/.chain-initialised\n",
        )
        .unwrap();

        let updated = append_gitignore_entry(dir.path()).unwrap();
        assert!(
            !updated,
            "should report no update when all entries already present"
        );

        let content = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert_eq!(
            content.lines().filter(|l| l.trim() == ".anvil/").count(),
            1,
            "wholesale runtime entry not duplicated"
        );
        assert_eq!(
            content.matches("anvil/exceptions/.lock").count(),
            1,
            "exception-lock entry not duplicated"
        );
    }

    #[test]
    fn gitignore_reports_update_when_appending() {
        let dir = tempfile::tempdir().unwrap();
        let updated = append_gitignore_entry(dir.path()).unwrap();
        assert!(updated, "should report update when gitignore is created");

        let updated_again = append_gitignore_entry(dir.path()).unwrap();
        assert!(
            !updated_again,
            "should report no update on second call with entry present",
        );
    }

    #[test]
    fn default_config_has_expected_shape() {
        let config = AnvilConfig::default();
        assert_eq!(config.schema_version, "1.0.0");
        assert_eq!(config.planning_dir, "plans");
        assert_eq!(config.format, "yaml");
        assert_eq!(config.checks.len(), 3);
        assert!(config.checks.contains(&"secret-detection".to_string()));
        assert!(config.checks.contains(&"import-boundaries".to_string()));
        assert!(config.checks.contains(&"antipattern-scan".to_string()));
    }

    #[test]
    fn default_available_checks_count() {
        let checks = default_available_checks();
        assert_eq!(checks.len(), 4);
        assert!(checks[0].enabled);
        assert!(checks[1].enabled);
        assert!(checks[2].enabled);
    }

    // Regression guard for #1016: every check name init writes to
    // `.anvilrc#checks` (either as a default or via the TUI selector) must
    // map to a dispatchable gate check via the catalog.
    #[test]
    fn init_checks_are_registered_gate_names() {
        use crate::commands::check_catalog::gate_internal_name;
        for name in default_check_names() {
            assert!(
                gate_internal_name(&name).is_some(),
                "init default check '{name}' does not map to a gate-supported catalog entry"
            );
        }
        for check in default_available_checks() {
            assert!(
                gate_internal_name(&check.name).is_some(),
                "init default_available_checks contains unregistered '{}'",
                check.name
            );
        }
    }
}
