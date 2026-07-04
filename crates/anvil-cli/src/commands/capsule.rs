//! `anvil capsule` command (GITGOV-004 CLI lane).
//!
//! Packages a commit range's governance evidence into an ADR-074
//! review capsule directory a reviewer, auditor, or supplier can
//! verify locally without trusting anvil Cloud (ADR-072).
//!
//! ## v0 scope
//!
//! - **`anvil capsule create --range <base>..<head> --out <dir>`** —
//!   collect the range (GITGOV-005), the policy/baseline/rules digest
//!   documents (GITGOV-006), the verbatim witness chain with the
//!   range's `seq` window (GITGOV-007), the SARIF diagnostics document
//!   (GITGOV-008, via the shared ADR-058 emitter), and write the capsule
//!   directory with a digest-complete `manifest.json`. Evidence whose
//!   collector lands later (applied exceptions — EXCEPT-009) is written
//!   present-but-empty; `verification.json` starts as the degraded
//!   no-checks placeholder, so an unverified capsule never claims
//!   `pass`.
//! - **`anvil capsule verify <dir>`** (GITGOV-009) — re-collect the
//!   repo-present digests, reuse `verify_chain_dag` (witness) and the
//!   EXCEPT-005 exception verifier, combine into closed-state verdicts,
//!   persist `verification.json`, and exit per the ADR-074 table
//!   (`0` pass/warn, `1` block, `2` degraded, `3` error).
//! - `explain` / `inspect` land with GITGOV-010/-011.
//!
//! ## Identity discipline (GITGOV-006 council follow-up)
//!
//! The manifest's `Producer.anvil_version` and the rules digest's
//! `ToolIdentity.anvil_version` are filled from the **same binding**
//! (this crate's `CARGO_PKG_VERSION`), and the OPA runtime version
//! comes from the shared `anvil_rules::OPA_RUNTIME_VERSION` constant
//! the witness-writing hook also uses — so the capsule's rule identity
//! matches witnessed lines by construction, enforced at the single
//! fill-site below rather than by convention.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anvil_capsule::{
    BaselineDigest, CapsuleContent, CapsuleManifest, CapsuleVerification, CommitsDocument,
    PolicyDigest, Producer, PrunePlan, RulesDigest, ToolIdentity, apply_prune,
    canonical_json_bytes, collect_commits, collect_diagnostics, collect_digests, collect_witness,
    plan_prune, verify_capsule, write_capsule,
};
use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use serde_json::{Value, json};

use crate::GlobalArgs;
use crate::util::workspace_root;

#[derive(Debug, Args)]
pub struct CapsuleArgs {
    #[command(subcommand)]
    command: CapsuleCommand,
}

#[derive(Debug, Subcommand)]
enum CapsuleCommand {
    /// Create a review capsule directory for a commit range.
    Create(CreateArgs),
    /// Verify a capsule directory and print closed-state verdicts
    /// (closed-state, offline-verifiable). Exits `0` pass/warn, `1`
    /// block, `2` degraded, `3` error. Repo-present: digests are
    /// re-collected from the current repository.
    Verify(VerifyArgs),
    /// Print a human-readable summary of a capsule directory
    /// — range, commits, policy/rules/baseline, witness
    /// coverage, diagnostics counts, exceptions, and the recorded
    /// verdict. Read-only and repo-independent — it reports the verdict
    /// the capsule carries, it does not re-verify (use `verify` for
    /// that). Always exits `0` on success regardless of the recorded
    /// verdict (non-zero only when the capsule cannot be read); gate on
    /// the verdict with `anvil capsule verify`, not this command.
    Explain(ExplainArgs),
    /// Explicitly dispose of staged capsules.
    /// Dry-run by default: prints what `--apply` would delete and
    /// touches nothing. Candidates are schema-gated (only parseable
    /// `anvil.capsule.v1` directories), ordered by head-commit
    /// committer date; capsules the repository cannot order are always
    /// kept. `--apply` deletes tracked capsules via the git index
    /// (staged deletion — committing remains your act) and never
    /// commits. Nothing in anvil prunes capsules automatically.
    Prune(PruneArgs),
}

#[derive(Debug, Args)]
struct VerifyArgs {
    /// The capsule directory to verify.
    capsule: PathBuf,
    /// Emit the `anvil.capsule-verification.v1` document as canonical
    /// JSON on stdout instead of the human per-check lines (for CI
    /// consumption). The exit code is unchanged, so a CI step can gate
    /// on the exit status and parse the verdict from the same run.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ExplainArgs {
    /// The capsule directory to summarise.
    capsule: PathBuf,
    /// Emit an `anvil.capsule-explain.v1` summary as JSON on stdout
    /// instead of the human report. Like the human form it
    /// is descriptive — it reports the capsule's recorded verdict, it
    /// does not adjudicate (gate with `anvil capsule verify --json`).
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct CreateArgs {
    /// Commit range to package, as `<base>..<head>`. Both sides may
    /// be any commit-ish (SHA, ref, tag).
    #[arg(long)]
    range: String,
    /// Directory to write the capsule into. Created if missing;
    /// refused if it already contains files. Keep it outside the
    /// repository — in-repo staging is a deliberate opt-in
    /// (on-demand/external by default).
    #[arg(long)]
    out: PathBuf,
}

#[derive(Debug, Args)]
struct PruneArgs {
    /// Staging root to prune. Defaults to `anvil/evidence/capsules/`
    /// by default. Must resolve inside the repository working tree and
    /// outside `.git`.
    #[arg(long)]
    root: Option<PathBuf>,
    /// Keep the newest N orderable capsules; the rest are selected for
    /// deletion. Must be at least 1 — deleting every capsule is a
    /// manual `git rm` decision, not a prune invocation.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    keep_last: u32,
    /// Perform the deletion. Without this flag prune is a dry run:
    /// it prints the would-delete list and touches nothing.
    #[arg(long)]
    apply: bool,
}

pub fn run(args: &CapsuleArgs, _global: &GlobalArgs) -> Result<()> {
    match &args.command {
        CapsuleCommand::Create(create) => {
            let repo_root = workspace_root()?;
            run_create(&repo_root, &create.range, &create.out)
        }
        CapsuleCommand::Prune(prune) => {
            let repo_root = workspace_root()?;
            run_prune(
                &repo_root,
                prune.root.as_deref(),
                prune.keep_last as usize,
                prune.apply,
            )
        }
        CapsuleCommand::Verify(verify) => {
            let repo_root = workspace_root()?;
            let verification = verify_and_record(&repo_root, &verify.capsule)?;
            if verify.json {
                // The machine-readable verdict is exactly the persisted
                // `verification.json` document — already schema-gated and
                // digest-stable, so CI parses the same bytes the capsule
                // carries. `?` surfaces an encode failure as the generic
                // error arm (exit 1) before we commit to a verdict code.
                print!("{}", verification_json_line(&verification)?);
            } else {
                print_verification(&verification);
            }
            // `process::exit` skips destructors, including the libstd
            // stdout flush — block-buffered when piped, so the verdict
            // could be lost down a pipe. Flush explicitly first.
            let _ = std::io::Write::flush(&mut std::io::stdout());
            // ADR-074 exit-code contract (0 pass/warn, 1 block, 2 degraded,
            // 3 error). `process::exit` because this is a terminal verb and
            // the generic `run() -> Result<()>` arm cannot carry a custom
            // code; the verdict logic itself is `verify_and_record`.
            std::process::exit(verification.verdict.exit_code());
        }
        CapsuleCommand::Explain(explain) => {
            // No `workspace_root()`: explain reads only the capsule, so a
            // reviewer can run it on a received directory outside any repo
            // (ADR-072 offline-verifiable posture).
            let report = if explain.json {
                render_explanation_json(&explain.capsule)
            } else {
                render_explanation(&explain.capsule)
            }
            .with_context(|| format!("explaining capsule {}", explain.capsule.display()))?;
            print!("{report}");
            Ok(())
        }
    }
}

/// The testable create flow: collect, assemble, write, report.
fn run_create(repo_root: &Path, range: &str, out: &Path) -> Result<()> {
    let (base, head) = parse_range(range)?;
    refuse_out_inside_git_dir(repo_root, out)?;

    // Single fill-site for both identity surfaces — see module docs.
    let anvil_version = env!("CARGO_PKG_VERSION");
    let producer = Producer {
        anvil_version: anvil_version.to_string(),
    };
    let tool_identity = ToolIdentity {
        anvil_version: anvil_version.to_string(),
        opa_runtime_version: anvil_rules::OPA_RUNTIME_VERSION.to_string(),
        // Empty for v1, mirroring the witness writer.
        rules: Vec::new(),
    };

    let commits = collect_commits(repo_root, base, head).context("collecting commit range")?;
    let digests =
        collect_digests(repo_root, &tool_identity).context("collecting evidence digests")?;

    // The witness window marks which seq attest commits in the range;
    // the full chain ships whole (the verifier is genesis-anchored).
    let range_commits: BTreeSet<String> = commits.commits.iter().map(|c| c.sha.clone()).collect();
    let witness = collect_witness(repo_root, &range_commits).context("collecting witness chain")?;

    // v0: no diagnostics source is wired into capsule create yet, so the
    // collector renders a complete empty SARIF document. A verify-time
    // check pass will feed real diagnostics here later (GITGOV-009+).
    let diagnostics = collect_diagnostics(&[]).context("rendering diagnostics")?;

    // EXCEPT-009: name the active tracked grants in the capsule so
    // verify can re-check scope/expiry/revocation/attribution.
    let exceptions =
        anvil_capsule::collect_exceptions(repo_root).context("collecting exception grants")?;

    let content = CapsuleContent {
        commits,
        digests,
        witness,
        diagnostics,
        exceptions,
        producer,
    };
    let manifest = write_capsule(out, &content).context("writing capsule directory")?;

    println!(
        "capsule written: {out} ({commits} commit{plural} {base}..{head}, {files} files)",
        out = out.display(),
        commits = content.commits.commits.len(),
        plural = if content.commits.commits.len() == 1 {
            ""
        } else {
            "s"
        },
        base = &manifest.range.base[..12.min(manifest.range.base.len())],
        head = &manifest.range.head[..12.min(manifest.range.head.len())],
        files = manifest.files.len() + 1, // + manifest.json itself
    );
    println!("verify with: anvil capsule verify {}", out.display());
    Ok(())
}

/// Verify `capsule_dir` against `repo_root`, persist the verdict back
/// into the capsule (`verification.json` + its re-recorded manifest
/// digest, ADR-074), and return the verification document.
///
/// The write-back is skipped when the manifest is unreadable (the engine
/// returns an `error` verdict in that case) — there is nothing to update.
fn verify_and_record(repo_root: &Path, capsule_dir: &Path) -> Result<CapsuleVerification> {
    let verification = verify_capsule(capsule_dir, repo_root);

    if let Ok(manifest_bytes) = std::fs::read(capsule_dir.join("manifest.json"))
        && let Ok(mut manifest) = CapsuleManifest::from_json_bytes(&manifest_bytes)
    {
        let bytes = verification
            .to_canonical_bytes()
            .context("encoding verification.json")?;
        std::fs::write(capsule_dir.join("verification.json"), &bytes)
            .context("writing verification.json")?;
        manifest.record_file("verification.json", &bytes);
        let manifest_bytes = manifest
            .to_canonical_bytes()
            .context("encoding manifest.json")?;
        std::fs::write(capsule_dir.join("manifest.json"), &manifest_bytes)
            .context("writing manifest.json")?;
    }

    Ok(verification)
}

/// Print the per-check results and the combined verdict + exit code.
/// The lowercase verdict token is [`Verdict::as_token`] — the single
/// source the JSON encoding and `explain` also use.
fn print_verification(verification: &CapsuleVerification) {
    for check in &verification.checks {
        let detail = check.detail.as_deref().unwrap_or("");
        println!("  [{}] {} — {detail}", check.verdict.as_token(), check.name);
    }
    println!(
        "verdict: {} (exit {})",
        verification.verdict.as_token(),
        verification.verdict.exit_code()
    );
}

/// Number of leading hex characters shown for a SHA-like identifier —
/// the short form `capsule create` also prints.
const SHORT_LEN: usize = 12;

/// Render a human-readable summary of the capsule at `capsule_dir`
/// (GITGOV-010).
///
/// Reads the capsule's own files in place — no repository access, no
/// re-verification. Only `manifest.json` (the digest root, carrying the
/// range and producer) is required; every other evidence document
/// degrades to an inline marker (`absent` for a present-but-empty
/// field, `missing` for a layout file that is gone, `(unreadable)` for
/// one that will not parse) so the summary still stands on a partial or
/// tampered capsule — seeing *which* evidence is gone is exactly what a
/// reviewer needs.
///
/// The verdict shown is the one recorded in `verification.json` (the
/// `degraded` placeholder until `capsule verify` overwrites it):
/// `explain` reports the capsule's claim, it does not adjudicate it.
fn render_explanation(capsule_dir: &Path) -> Result<String> {
    let manifest_bytes = std::fs::read(capsule_dir.join("manifest.json"))
        .context("cannot read manifest.json (the capsule's digest root)")?;
    let manifest =
        CapsuleManifest::from_json_bytes(&manifest_bytes).context("invalid manifest.json")?;

    let mut lines = Vec::new();
    lines.push(format!("anvil Review Capsule  ({})", manifest.schema));
    lines.push(String::new());
    field(
        &mut lines,
        "Producer",
        &format!("anvil {}", manifest.producer.anvil_version),
    );
    field(
        &mut lines,
        "Range",
        &format!(
            "{}..{}",
            short(&manifest.range.base),
            short(&manifest.range.head)
        ),
    );
    field(&mut lines, "Commits", &commits_field(capsule_dir));
    field(&mut lines, "Policy", &policy_field(capsule_dir));
    field(&mut lines, "Rules", &rules_field(capsule_dir));
    field(&mut lines, "Baseline", &baseline_field(capsule_dir));
    field(
        &mut lines,
        "Witness",
        &witness_field(capsule_dir, &manifest),
    );
    field(&mut lines, "Diagnostics", &diagnostics_field(capsule_dir));
    field(&mut lines, "Exceptions", &exceptions_field(capsule_dir));
    lines.push(String::new());
    verdict_section(&mut lines, capsule_dir);

    // A single trailing newline; the join supplies the inter-line ones.
    lines.push(String::new());
    Ok(lines.join("\n"))
}

/// First [`SHORT_LEN`] characters of a SHA-like string (whole string if
/// shorter). SHAs are ASCII hex, so the byte slice is a char boundary.
fn short(sha: &str) -> &str {
    sha.get(..SHORT_LEN).unwrap_or(sha)
}

/// Push one aligned `  Label   value` line. The label column is padded
/// to the width of the longest label (`Diagnostics`, 11) so values line
/// up; the padding is constant, so output stays byte-deterministic. The
/// value is flattened to a single line ([`one_line`]) — every field is
/// one-line by construction, but the value can originate in an untrusted
/// capsule file, and a planted newline or escape must not forge extra
/// output rows.
fn field(lines: &mut Vec<String>, label: &str, value: &str) {
    lines.push(format!("  {label:<11} {}", one_line(value)));
}

/// How a capsule file read resolved — present bytes, cleanly absent, or
/// a read error — so each field keeps "the layout file is gone" (a
/// tamper signal) distinct from "could not read it".
enum Slot {
    Bytes(Vec<u8>),
    Missing,
    Unreadable,
}

fn slot(capsule_dir: &Path, name: &str) -> Slot {
    match std::fs::read(capsule_dir.join(name)) {
        Ok(bytes) => Slot::Bytes(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Slot::Missing,
        Err(_) => Slot::Unreadable,
    }
}

/// `Commits` — the count of commits in the range (`commits.json`).
fn commits_field(capsule_dir: &Path) -> String {
    match slot(capsule_dir, "commits.json") {
        Slot::Bytes(bytes) => CommitsDocument::from_json_bytes(&bytes).map_or_else(
            |_| "(unreadable)".to_string(),
            |doc| doc.commits.len().to_string(),
        ),
        Slot::Missing => "missing".to_string(),
        Slot::Unreadable => "(unreadable)".to_string(),
    }
}

/// `Policy` — the effective policy file path, or `absent`.
fn policy_field(capsule_dir: &Path) -> String {
    match slot(capsule_dir, "policy.json") {
        Slot::Bytes(bytes) => match PolicyDigest::from_json_bytes(&bytes) {
            Ok(doc) => doc
                .policy_file
                .map_or_else(|| "absent".to_string(), |file| file.path),
            Err(_) => "(unreadable)".to_string(),
        },
        Slot::Missing => "missing".to_string(),
        Slot::Unreadable => "(unreadable)".to_string(),
    }
}

/// `Rules` — the short `rules_sha` identity, or `absent (no config)`.
fn rules_field(capsule_dir: &Path) -> String {
    match slot(capsule_dir, "rules.json") {
        Slot::Bytes(bytes) => match RulesDigest::from_json_bytes(&bytes) {
            Ok(doc) => doc.rules_sha.as_deref().map_or_else(
                || "absent (no config)".to_string(),
                |sha| short(sha).to_string(),
            ),
            Err(_) => "(unreadable)".to_string(),
        },
        Slot::Missing => "missing".to_string(),
        Slot::Unreadable => "(unreadable)".to_string(),
    }
}

/// `Baseline` — `cutoff <short>`, `present (no cutoff)`, or `absent`.
fn baseline_field(capsule_dir: &Path) -> String {
    match slot(capsule_dir, "baseline.json") {
        Slot::Bytes(bytes) => match BaselineDigest::from_json_bytes(&bytes) {
            Ok(doc) => match (doc.cutoff_commit, doc.digest) {
                (Some(cutoff), _) => format!("cutoff {}", short(&cutoff)),
                (None, Some(_)) => "present (no cutoff)".to_string(),
                (None, None) => "absent".to_string(),
            },
            Err(_) => "(unreadable)".to_string(),
        },
        Slot::Missing => "missing".to_string(),
        Slot::Unreadable => "(unreadable)".to_string(),
    }
}

/// `Witness` — the range `seq` window (from the manifest) plus the
/// embedded chain's line count. Names the three honest states the
/// capsule README's coverage line also distinguishes: an absent chain,
/// a chain present with no range coverage, and a `[start, end]` window.
fn witness_field(capsule_dir: &Path, manifest: &CapsuleManifest) -> String {
    // The chain is copied verbatim; explain only needs the line count,
    // so it counts NDJSON records without parsing them. `str::lines`
    // handles `\n`/`\r\n`; `trim().is_empty()` drops blank/whitespace-only
    // lines so a chain of spaces cannot inflate the count. An unreadable
    // chain is surfaced like every other field, never folded into a
    // misleading `0 lines`.
    let line_count = match slot(capsule_dir, "witness.ndjson") {
        Slot::Bytes(bytes) => count_ndjson_lines(&bytes),
        Slot::Missing => 0,
        Slot::Unreadable => return "(unreadable)".to_string(),
    };

    match (
        manifest.range.witness_seq_start,
        manifest.range.witness_seq_end,
    ) {
        // The write path always pairs the pointers; one without the other
        // is a tampered manifest, not a state a real capsule produces.
        (Some(_), None) | (None, Some(_)) => "malformed (asymmetric seq window)".to_string(),
        (Some(start), Some(end)) => {
            // Inclusive `[start, end]`; collapse the single-line case so
            // it does not read as a confusing `seq 1 to 1`.
            let window = if start == end {
                format!("seq {start}")
            } else {
                format!("seq {start} to {end} (inclusive)")
            };
            if line_count == 0 {
                // The manifest claims a window but the chain is empty or
                // gone — an unbacked claim a reviewer must see, not a
                // reassuring window with nothing behind it.
                format!("{window}, chain absent")
            } else {
                format!("{window}, {}", plural(line_count, "line"))
            }
        }
        (None, None) if line_count == 0 => "absent (no witness chain)".to_string(),
        (None, None) => format!("present, no range coverage, {}", plural(line_count, "line")),
    }
}

/// SARIF result tallies by level — every result lands in exactly one
/// bucket, so `errors + warnings + notes + suppressed` is the total.
struct DiagnosticCounts {
    errors: usize,
    warnings: usize,
    notes: usize,
    suppressed: usize,
}

/// Count SARIF 2.1.0 §3.27.10 result levels in a parsed `diagnostics.sarif`
/// document. An absent level defaults to `warning`; `none` is a
/// *suppressed* result, kept in its own bucket so it is never laundered
/// into a `note`. Shared by the text field and the JSON summary so the
/// two can never disagree on the tally.
fn diagnostic_counts(doc: &Value) -> DiagnosticCounts {
    let (mut errors, mut warnings, mut notes, mut suppressed) = (0usize, 0usize, 0usize, 0usize);
    if let Some(runs) = doc.get("runs").and_then(Value::as_array) {
        for run in runs {
            let Some(results) = run.get("results").and_then(Value::as_array) else {
                continue;
            };
            for result in results {
                match result.get("level").and_then(Value::as_str) {
                    Some("error") => errors += 1,
                    None | Some("warning") => warnings += 1,
                    Some("none") => suppressed += 1,
                    _ => notes += 1,
                }
            }
        }
    }
    DiagnosticCounts {
        errors,
        warnings,
        notes,
        suppressed,
    }
}

/// Count NDJSON records in a witness chain: non-blank lines, the same
/// rule the text `Witness` field uses (`str::lines` handles `\n`/`\r\n`;
/// whitespace-only lines are not records). Shared so the JSON summary
/// reports the identical line count.
fn count_ndjson_lines(bytes: &[u8]) -> usize {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count()
}

/// `Diagnostics` — total SARIF results plus a per-level breakdown
/// (`diagnostics.sarif`); `none` when there are no results.
fn diagnostics_field(capsule_dir: &Path) -> String {
    let bytes = match slot(capsule_dir, "diagnostics.sarif") {
        Slot::Bytes(bytes) => bytes,
        Slot::Missing => return "missing".to_string(),
        Slot::Unreadable => return "(unreadable)".to_string(),
    };
    let Ok(doc) = serde_json::from_slice::<Value>(&bytes) else {
        return "(unreadable)".to_string();
    };

    let DiagnosticCounts {
        errors,
        warnings,
        notes,
        suppressed,
    } = diagnostic_counts(&doc);

    let total = errors + warnings + notes + suppressed;
    if total == 0 {
        return "none".to_string();
    }
    // `suppressed` is an adjective, not a count noun, so it is not
    // pluralised the way `error`/`warning`/`note` are.
    let parts: Vec<String> = [
        (errors, plural(errors, "error")),
        (warnings, plural(warnings, "warning")),
        (notes, plural(notes, "note")),
        (suppressed, format!("{suppressed} suppressed")),
    ]
    .into_iter()
    .filter(|(n, _)| *n > 0)
    .map(|(_, label)| label)
    .collect();
    format!("{total} ({})", parts.join(", "))
}

/// `Exceptions` — the count of applied exceptions (`exceptions.json`, a
/// JSON array); `none` when empty.
fn exceptions_field(capsule_dir: &Path) -> String {
    let bytes = match slot(capsule_dir, "exceptions.json") {
        Slot::Bytes(bytes) => bytes,
        Slot::Missing => return "missing".to_string(),
        Slot::Unreadable => return "(unreadable)".to_string(),
    };
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(Value::Array(items)) if items.is_empty() => "none".to_string(),
        Ok(Value::Array(items)) => format!("{} applied", items.len()),
        _ => "(unreadable)".to_string(),
    }
}

/// The `Verdict:` section — the recorded verdict, its exit code, and the
/// per-check breakdown from `verification.json`.
fn verdict_section(lines: &mut Vec<String>, capsule_dir: &Path) {
    let bytes = match slot(capsule_dir, "verification.json") {
        Slot::Bytes(bytes) => bytes,
        Slot::Missing => {
            lines.push("Verdict: missing".to_string());
            return;
        }
        Slot::Unreadable => {
            lines.push("Verdict: (unreadable)".to_string());
            return;
        }
    };
    let verification = match CapsuleVerification::from_json_bytes(&bytes) {
        Ok(verification) => verification,
        Err(e) => {
            // The error text can echo untrusted document fields (e.g. a
            // `SchemaMismatch`'s `found` string), so flatten it too.
            lines.push(format!(
                "Verdict: (unreadable: {})",
                one_line(&e.to_string())
            ));
            return;
        }
    };

    lines.push(format!(
        "Verdict: {} (exit {})",
        verification.verdict.as_token().to_uppercase(),
        verification.verdict.exit_code()
    ));

    if verification.checks.is_empty() {
        // The create-time placeholder: a machine-readable `degraded`
        // with no checks. Say so plainly rather than showing a bare
        // verdict with nothing supporting it.
        lines.push("  (no checks recorded — run `anvil capsule verify`)".to_string());
        return;
    }
    for check in &verification.checks {
        // Pad the bracketed token to the width of the longest
        // (`[degraded]`, 10) so the check names line up. The name and
        // detail come from the capsule's own `verification.json`, which
        // explain does not digest-verify — flatten them so a planted
        // newline cannot forge extra check rows ([`one_line`]).
        let token = format!("[{}]", check.verdict.as_token());
        let name = one_line(&check.name);
        match &check.detail {
            Some(detail) => lines.push(format!("  {token:<10} {name} — {}", one_line(detail))),
            None => lines.push(format!("  {token:<10} {name}")),
        }
    }
}

/// `"1 line"` / `"3 lines"` — naive English pluralisation (append `s`
/// unless the count is exactly one), enough for the count nouns here.
fn plural(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// Flatten a string sourced from an untrusted capsule file to a single,
/// honestly-displayed terminal line: every control character (newline,
/// carriage return, ANSI escape) **and** every Unicode bidi-control /
/// zero-width format character (Trojan-Source-style reordering or hiding)
/// becomes a space. `explain` does not digest-verify the files it reads,
/// so a planted `\n`, `\x1b[`, or `U+202E` in a path, check name, or
/// detail must not forge extra rows, rewrite the terminal, or spoof the
/// reading order — the report of a tampered capsule has to stay honest
/// about its own shape. Ordinary non-ASCII text (e.g. accented paths) is
/// preserved.
fn one_line(value: &str) -> String {
    value
        .chars()
        .map(|c| if is_display_unsafe(c) { ' ' } else { c })
        .collect()
}

/// Whether `c` must not reach the terminal verbatim in `explain` output:
/// any control character, or a Unicode bidi-control / zero-width format
/// character that can reorder or hide adjacent text without showing a
/// glyph. Deliberately narrow — it does not touch ordinary printable
/// non-ASCII, which is legitimate in paths and identifiers.
fn is_display_unsafe(c: char) -> bool {
    c.is_control()
        || matches!(c,
            '\u{200B}'..='\u{200F}'   // zero-width space .. RTL mark
            | '\u{202A}'..='\u{202E}' // bidi embeddings / overrides
            | '\u{2060}'..='\u{2064}' // word joiner .. invisible plus
            | '\u{2066}'..='\u{2069}' // bidi isolates
            | '\u{FEFF}'              // zero-width no-break space (BOM)
        )
}

// ----- GITGOV-011: `--json` output -----

/// The schema identifier for the `explain --json` summary document.
const EXPLAIN_SUMMARY_SCHEMA: &str = "anvil.capsule-explain.v1";

/// The `verify --json` line: the verification document as canonical JSON
/// (sorted keys, minimal whitespace — exactly the persisted
/// `verification.json` bytes) plus a trailing newline. Routed through
/// [`CapsuleVerification::to_canonical_bytes`] so the emitted verdict is
/// byte-identical to the one the capsule carries, for any verdict
/// including `error`.
fn verification_json_line(verification: &CapsuleVerification) -> Result<String> {
    let bytes = verification
        .to_canonical_bytes()
        .context("encoding verification JSON")?;
    let mut line = String::from_utf8(bytes).context("verification JSON is not UTF-8")?;
    line.push('\n');
    Ok(line)
}

/// Render the `explain --json` summary (`anvil.capsule-explain.v1`) for
/// the capsule at `capsule_dir` as a single line of canonical JSON plus a
/// trailing newline.
///
/// The JSON analogue of [`render_explanation`]: same facts, machine
/// typed. Every degradable evidence field is a status-tagged object: a
/// `present` field carries its typed payload, and every degraded state
/// (`absent`, `missing`, `unreadable`, `malformed`) carries **only**
/// `{"status": …}` with no payload keys — so a consumer branches on
/// `status` once, then reads payload only on `present`. That keeps a
/// partial or tampered capsule honest about *which* evidence is gone, the
/// same distinctions the human report draws. Only `manifest.json` (the
/// digest root) is required; everything else degrades in place.
///
/// Output is [`canonical_json_bytes`] — recursively sorted keys, minimal
/// whitespace — the same canonical discipline as `verify --json`, so both
/// capsule JSON surfaces are byte-stable across runs and platforms.
///
/// Unlike the text form this does **not** sanitise string values, by
/// design. JSON encoding escapes the structural/control set (quotes,
/// backslash, `U+0000`–`U+001F` including newline and `ESC`), so
/// row-forging and ANSI injection are neutralised — but Unicode
/// bidi-control and zero-width characters (the Trojan-Source class the
/// text path's [`one_line`]/[`is_display_unsafe`] strips) pass through
/// verbatim, because this is a machine surface (parse the data, don't
/// render it). A tool that displays this JSON to a terminal owns its own
/// bidi safety; the sanitised, terminal-safe view is the human `explain`.
fn render_explanation_json(capsule_dir: &Path) -> Result<String> {
    let manifest_bytes = std::fs::read(capsule_dir.join("manifest.json"))
        .context("cannot read manifest.json (the capsule's digest root)")?;
    let manifest =
        CapsuleManifest::from_json_bytes(&manifest_bytes).context("invalid manifest.json")?;

    let summary = json!({
        "schema": EXPLAIN_SUMMARY_SCHEMA,
        "capsule_schema": manifest.schema,
        "producer": { "anvil_version": manifest.producer.anvil_version },
        "range": { "base": manifest.range.base, "head": manifest.range.head },
        "commits": commits_json(capsule_dir),
        "policy": policy_json(capsule_dir),
        "rules": rules_json(capsule_dir),
        "baseline": baseline_json(capsule_dir),
        "witness": witness_json(capsule_dir, &manifest),
        "diagnostics": diagnostics_json(capsule_dir),
        "exceptions": exceptions_json(capsule_dir),
        "verdict": verdict_json(capsule_dir),
    });

    let bytes = canonical_json_bytes(&summary).context("encoding explain summary JSON")?;
    let mut line = String::from_utf8(bytes).context("explain summary JSON is not UTF-8")?;
    line.push('\n');
    Ok(line)
}

/// A bare `{"status":"<token>"}` object for a degraded evidence field —
/// `absent` (the digest records no such evidence), `missing` (layout file
/// gone), `unreadable` (read/parse error), or `malformed` (present but
/// structurally wrong). Degraded states carry no payload keys; only a
/// `present` field does.
fn status(token: &str) -> Value {
    json!({ "status": token })
}

/// `commits` — `{status, count}` when readable.
fn commits_json(capsule_dir: &Path) -> Value {
    match slot(capsule_dir, "commits.json") {
        Slot::Bytes(bytes) => CommitsDocument::from_json_bytes(&bytes).map_or_else(
            |_| status("unreadable"),
            |doc| json!({ "status": "present", "count": doc.commits.len() }),
        ),
        Slot::Missing => status("missing"),
        Slot::Unreadable => status("unreadable"),
    }
}

/// `policy` — `{status, path}` for an effective policy file, else
/// `absent` (no policy in the digest).
fn policy_json(capsule_dir: &Path) -> Value {
    match slot(capsule_dir, "policy.json") {
        Slot::Bytes(bytes) => match PolicyDigest::from_json_bytes(&bytes) {
            Ok(doc) => doc.policy_file.map_or_else(
                || status("absent"),
                |file| json!({ "status": "present", "path": file.path }),
            ),
            Err(_) => status("unreadable"),
        },
        Slot::Missing => status("missing"),
        Slot::Unreadable => status("unreadable"),
    }
}

/// `rules` — `{status, rules_sha}` (the full identity, not the short
/// form) when config is present, else `absent`.
fn rules_json(capsule_dir: &Path) -> Value {
    match slot(capsule_dir, "rules.json") {
        Slot::Bytes(bytes) => match RulesDigest::from_json_bytes(&bytes) {
            Ok(doc) => doc.rules_sha.map_or_else(
                || status("absent"),
                |sha| json!({ "status": "present", "rules_sha": sha }),
            ),
            Err(_) => status("unreadable"),
        },
        Slot::Missing => status("missing"),
        Slot::Unreadable => status("unreadable"),
    }
}

/// `baseline` — `present` with `cutoff_commit` when the baseline pins a
/// cutoff; `present` with no `cutoff_commit` key when a baseline exists
/// without one; `absent` when there is no baseline. `cutoff_commit` is
/// only ever a string — its absence is carried by the key being omitted,
/// not by a `null`, so the field stays consistent with the rest of the
/// schema (absence is a state, never a `null` value).
fn baseline_json(capsule_dir: &Path) -> Value {
    match slot(capsule_dir, "baseline.json") {
        Slot::Bytes(bytes) => match BaselineDigest::from_json_bytes(&bytes) {
            Ok(doc) => match (doc.cutoff_commit, doc.digest) {
                (Some(cutoff), _) => json!({ "status": "present", "cutoff_commit": cutoff }),
                (None, Some(_)) => json!({ "status": "present" }),
                (None, None) => status("absent"),
            },
            Err(_) => status("unreadable"),
        },
        Slot::Missing => status("missing"),
        Slot::Unreadable => status("unreadable"),
    }
}

/// `witness` — the range `seq` window (from the manifest) and the
/// embedded chain's line count. A `present` field carries the payload
/// (`seq_start`/`seq_end` — `null` when the chain is present with no
/// range coverage — plus `chain_present` and `lines`); every degraded
/// state carries only its `status`: `missing` (the required
/// `witness.ndjson` is gone — a tamper signal, surfaced like every other
/// missing evidence file), `unreadable`, `malformed` (an asymmetric seq
/// window — a tampered manifest), or `absent` (the file is present but
/// empty and no window covers the range — a legitimately empty chain).
/// The `missing`-vs-`absent` split is sharper here than in the text
/// field, which renders a removed and an empty chain alike: on the
/// machine surface a removed required file is a distinct, actionable
/// tamper signal. `seq_start`/`seq_end` are the only payload keys that
/// may be `null` (no range window on a present chain).
fn witness_json(capsule_dir: &Path, manifest: &CapsuleManifest) -> Value {
    let lines = match slot(capsule_dir, "witness.ndjson") {
        Slot::Bytes(bytes) => count_ndjson_lines(&bytes),
        // A removed required file is a tamper signal, surfaced as
        // `missing` like every other evidence file — not folded into the
        // window logic below where it would read as `absent`/`present`.
        Slot::Missing => return status("missing"),
        Slot::Unreadable => return status("unreadable"),
    };

    match (
        manifest.range.witness_seq_start,
        manifest.range.witness_seq_end,
    ) {
        (Some(_), None) | (None, Some(_)) => status("malformed"),
        (Some(start), Some(end)) => json!({
            "status": "present",
            "seq_start": start,
            "seq_end": end,
            // The window is recorded but no chain backs it — the same
            // unbacked-claim signal the text field spells as "chain absent".
            "chain_present": lines > 0,
            "lines": lines,
        }),
        (None, None) if lines == 0 => status("absent"),
        (None, None) => json!({
            "status": "present",
            "seq_start": Value::Null,
            "seq_end": Value::Null,
            "chain_present": true,
            "lines": lines,
        }),
    }
}

/// `diagnostics` — `{status:"present"}` with the SARIF total and the
/// per-level breakdown; the total always equals the breakdown sum.
fn diagnostics_json(capsule_dir: &Path) -> Value {
    let bytes = match slot(capsule_dir, "diagnostics.sarif") {
        Slot::Bytes(bytes) => bytes,
        Slot::Missing => return status("missing"),
        Slot::Unreadable => return status("unreadable"),
    };
    let Ok(doc) = serde_json::from_slice::<Value>(&bytes) else {
        return status("unreadable");
    };
    let DiagnosticCounts {
        errors,
        warnings,
        notes,
        suppressed,
    } = diagnostic_counts(&doc);
    json!({
        "status": "present",
        "total": errors + warnings + notes + suppressed,
        "error": errors,
        "warning": warnings,
        "note": notes,
        "suppressed": suppressed,
    })
}

/// `exceptions` — `{status:"present", count}` (count of applied
/// exceptions). A document that is valid JSON but not an array is
/// `malformed` (wrong shape — a CI-actionable signal); bytes that are not
/// JSON at all are `unreadable`. The machine surface keeps these distinct
/// where the text field collapses both to `(unreadable)`.
fn exceptions_json(capsule_dir: &Path) -> Value {
    let bytes = match slot(capsule_dir, "exceptions.json") {
        Slot::Bytes(bytes) => bytes,
        Slot::Missing => return status("missing"),
        Slot::Unreadable => return status("unreadable"),
    };
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(Value::Array(items)) => json!({ "status": "present", "count": items.len() }),
        Ok(_) => status("malformed"),
        Err(_) => status("unreadable"),
    }
}

/// `verdict` — the recorded `verification.json` verdict, its exit code,
/// and the per-check breakdown. `present` carries the structured
/// verdict; the create-time placeholder (degraded, no checks) is
/// reported faithfully as `present` with an empty `checks` array.
fn verdict_json(capsule_dir: &Path) -> Value {
    let bytes = match slot(capsule_dir, "verification.json") {
        Slot::Bytes(bytes) => bytes,
        Slot::Missing => return status("missing"),
        Slot::Unreadable => return status("unreadable"),
    };
    match CapsuleVerification::from_json_bytes(&bytes) {
        Ok(verification) => json!({
            "status": "present",
            "verdict": verification.verdict.as_token(),
            "exit_code": verification.verdict.exit_code(),
            "checks": verification.checks,
        }),
        Err(_) => status("unreadable"),
    }
}

/// Split `<base>..<head>`, rejecting empty or whitespace-bearing
/// sides, extra `..` separators, and the three-dot
/// (symmetric-difference) form — capsule semantics are exactly
/// `git rev-list base..head`, and a malformed range should fail here
/// with a `--range` message, not downstream with an opaque git error.
fn parse_range(range: &str) -> Result<(&str, &str)> {
    let Some((base, head)) = range.split_once("..") else {
        bail!("--range must be <base>..<head>; got `{range}`");
    };
    if head.starts_with('.') {
        bail!("--range uses two-dot <base>..<head> semantics; got `{range}`");
    }
    if head.contains("..") {
        bail!("--range must contain exactly one `..` separator; got `{range}`");
    }
    if base.is_empty() || head.is_empty() {
        bail!("--range must name both sides of <base>..<head>; got `{range}`");
    }
    if base.chars().any(char::is_whitespace) || head.chars().any(char::is_whitespace) {
        bail!("--range sides must not contain whitespace; got `{range}`");
    }
    Ok((base, head))
}

/// Default in-repo staging root (ADR-073).
const DEFAULT_STAGING_ROOT: &str = "anvil/evidence/capsules";

/// The `prune` flow (ADR-078): plan over the staging root, report on
/// stdout (would-delete list, one path per line) with warnings on
/// stderr, and — only under `--apply` — delete via the git index.
fn run_prune(repo_root: &Path, root: Option<&Path>, keep_last: usize, apply: bool) -> Result<()> {
    let staging_root = match root {
        Some(root) => validate_prune_root(repo_root, root)?,
        None => repo_root.join(DEFAULT_STAGING_ROOT),
    };
    if !staging_root.exists() {
        eprintln!(
            "staging root {} does not exist — nothing to prune",
            staging_root.display()
        );
        return Ok(());
    }

    let plan = plan_prune(repo_root, &staging_root, keep_last)
        .with_context(|| format!("planning prune of {}", staging_root.display()))?;
    report_prune_warnings(&plan);
    if plan.keep.is_empty() && plan.delete.is_empty() && plan.unordered.is_empty() {
        eprintln!(
            "warning: no capsules found under {} — is this the staging root?",
            staging_root.display()
        );
        eprintln!("nothing to prune");
        return Ok(());
    }

    if plan.delete.is_empty() {
        eprintln!(
            "nothing to prune: {} orderable capsule(s) <= --keep-last {keep_last}",
            plan.keep.len()
        );
        return Ok(());
    }

    if !apply {
        // One sanitised path per line: the only machine-readable surface
        // until `--json` lands, so a planted newline in a directory name
        // must not forge extra rows.
        for capsule in &plan.delete {
            println!("{}", one_line(&capsule.dir.display().to_string()));
        }
        // Summary is chatter, not a path — stderr keeps stdout a pure
        // line-oriented would-delete list for scripts.
        eprintln!(
            "dry run: {} capsule(s) would be deleted, {} kept — re-run with --apply to delete",
            plan.delete.len(),
            plan.keep.len() + plan.unordered.len()
        );
        return Ok(());
    }

    let failures = apply_prune(repo_root, &plan);
    // List what actually went, not what was planned — on partial failure
    // the stdout set must reflect the resulting state (ADR-078).
    for capsule in &plan.delete {
        if !failures.iter().any(|f| f.dir == capsule.dir) {
            println!("{}", one_line(&capsule.dir.display().to_string()));
        }
    }
    eprintln!(
        "pruned {} capsule(s), kept {} — deletions are staged; commit to record the prune",
        plan.delete.len() - failures.len(),
        plan.keep.len() + plan.unordered.len()
    );
    if !failures.is_empty() {
        for failure in &failures {
            eprintln!(
                "error: failed to remove {}: {}",
                failure.dir.display(),
                failure.error
            );
        }
        bail!(
            "{} of {} deletion(s) failed",
            failures.len(),
            plan.delete.len()
        );
    }
    Ok(())
}

/// Stderr warnings for the parts of a prune plan that need operator
/// attention but never change the exit code (ADR-002 posture).
fn report_prune_warnings(plan: &PrunePlan) {
    for entry in &plan.skipped {
        eprintln!(
            "warning: skipped {} ({})",
            entry.path.display(),
            entry.reason
        );
    }
    if !plan.unordered.is_empty() {
        eprintln!(
            "warning: {} capsule(s) kept because the repository does not know their head \
             commit (cannot be ordered honestly):",
            plan.unordered.len()
        );
        for capsule in &plan.unordered {
            eprintln!("warning:   {}", capsule.dir.display());
        }
    }
}

/// `--root` must resolve to a directory inside the repository working
/// tree and never inside `.git` (ADR-078; mirrors
/// `refuse_out_inside_git_dir`). Prune stages deletions through the
/// index, so an out-of-repo root is meaningless — external capsule
/// directories are the operator's to manage directly.
fn validate_prune_root(repo_root: &Path, root: &Path) -> Result<PathBuf> {
    let absolute = if root.is_absolute() {
        root.to_path_buf()
    } else {
        repo_root.join(root)
    };
    let canonical_repo = repo_root
        .canonicalize()
        .with_context(|| format!("resolving repository root {}", repo_root.display()))?;
    let Ok(resolved) = absolute.canonicalize() else {
        // A missing root is handled (as "nothing to prune") by the
        // caller — but the containment check must still hold, and
        // `Path::starts_with` is component-wise (it does not collapse
        // `..`), so normalise lexically before comparing. Without this,
        // `--root ../missing` would skip validation entirely and a later
        // creation of that directory would put the scan outside the repo.
        let normalized = lexical_normalize(&absolute);
        if !normalized.starts_with(&canonical_repo) && !normalized.starts_with(repo_root) {
            bail!(
                "--root {} resolves outside the repository working tree; prune only \
                 manages in-repo staging (ADR-078)",
                root.display()
            );
        }
        return Ok(absolute);
    };
    if !resolved.starts_with(&canonical_repo) {
        bail!(
            "--root {} resolves outside the repository working tree; prune only manages \
             in-repo staging (ADR-078) — external capsule directories are yours to manage \
             directly",
            root.display()
        );
    }
    if let Ok(git_dir) = repo_root.join(".git").canonicalize()
        && resolved.starts_with(&git_dir)
    {
        bail!(
            "--root {} resolves inside the repository's .git directory; choose a staging \
             root in the working tree",
            root.display()
        );
    }
    Ok(resolved)
}

/// Collapse `.` and `..` components without touching the filesystem, so
/// containment checks hold for paths that do not exist yet. Leading `..`
/// segments (escaping past the root) are preserved, which correctly
/// fails a `starts_with` containment test.
fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// Refuse `--out` resolving inside the repository's `.git` directory —
/// capsule files there could corrupt repository state or be mistaken
/// for plumbing objects. (Elsewhere inside the repo is allowed:
/// ADR-074 acknowledges in-repo staging as a deliberate opt-in.)
fn refuse_out_inside_git_dir(repo_root: &Path, out: &Path) -> Result<()> {
    // Bare/worktree layouts where `.git` is a file (or absent) can't
    // contain the out dir as a subpath.
    let Ok(git_dir) = repo_root.join(".git").canonicalize() else {
        return Ok(());
    };
    // `out` may not exist yet; canonicalise its nearest existing
    // ancestor to resolve symlinks before the containment test.
    let mut probe = out.to_path_buf();
    let resolved = loop {
        match probe.canonicalize() {
            Ok(resolved) => break resolved,
            Err(_) => match probe.parent() {
                Some(parent) if parent != probe => probe = parent.to_path_buf(),
                _ => return Ok(()),
            },
        }
    };
    if resolved.starts_with(&git_dir) {
        bail!(
            "--out {} resolves inside the repository's .git directory; \
             choose a destination outside it",
            out.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_capsule::{
        BASELINE_DIGEST_SCHEMA, COMMITS_SCHEMA, CapsuleManifest, CheckResult, CollectedDigests,
        CollectedWitness, CommitEntry, FileDigest, POLICY_DIGEST_SCHEMA, REQUIRED_FILES,
        RULES_DIGEST_SCHEMA, Verdict,
    };
    use std::process::Command;

    fn git(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("spawn git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("utf-8 git output")
    }

    fn commit(dir: &Path, message: &str) {
        // Identity + signing are pinned at the repo level in
        // `scratch_repo`.
        git(dir, &["commit", "-q", "-m", message]);
    }

    /// A scratch repo with two commits, an `.anvil.yml` config, and a
    /// policy file. Returns (dir, `base_sha`, `head_sha`).
    fn scratch_repo() -> (tempfile::TempDir, String, String) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // `--template=` (empty) keeps host git template hooks out of
        // the fixture; identity/signing pins keep commits deterministic
        // on hosts with global `commit.gpgsign=true`.
        git(root, &["init", "-q", "--template="]);
        for (key, value) in [
            ("user.email", "capsule@test.invalid"),
            ("user.name", "capsule-test"),
            ("commit.gpgsign", "false"),
        ] {
            git(root, &["config", key, value]);
        }

        std::fs::write(root.join(".anvil.yml"), "checks:\n  enabled: true\n").unwrap();
        std::fs::create_dir_all(root.join("anvil")).unwrap();
        std::fs::write(
            root.join("anvil/policy.yml"),
            "branches:\n  - pattern: main\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n",
        )
        .unwrap();
        std::fs::write(root.join("a.txt"), "one").unwrap();
        git(root, &["add", "."]);
        commit(root, "base");
        let base = git(root, &["rev-parse", "HEAD"]).trim().to_string();

        std::fs::write(root.join("b.txt"), "two").unwrap();
        git(root, &["add", "."]);
        commit(root, "head");
        let head = git(root, &["rev-parse", "HEAD"]).trim().to_string();

        (dir, base, head)
    }

    #[test]
    fn capsule_create_writes_complete_verifiable_capsule() {
        let (dir, base, head) = scratch_repo();
        let out_dir = tempfile::tempdir().unwrap();
        let out = out_dir.path().join("capsule");

        run_create(dir.path(), &format!("{base}..{head}"), &out).unwrap();

        // Every ADR-074 file plus the manifest exists, and every
        // manifest digest matches the bytes on disk.
        let manifest_bytes = std::fs::read(out.join("manifest.json")).unwrap();
        let manifest = CapsuleManifest::from_json_bytes(&manifest_bytes).unwrap();
        assert!(manifest.missing_required().is_empty());
        for name in REQUIRED_FILES {
            assert!(out.join(name).exists(), "{name} missing");
            let bytes = std::fs::read(out.join(name)).unwrap();
            assert_eq!(
                anvil_capsule::sha256_hex(&bytes),
                manifest.files[name],
                "digest mismatch for {name}"
            );
        }
        assert_eq!(manifest.range.base, base);
        assert_eq!(manifest.range.head, head);
    }

    /// The single-fill-site identity discipline: the manifest's
    /// producer version and the rules digest's recorded version are
    /// the same string.
    #[test]
    fn capsule_create_unifies_producer_and_rules_identity() {
        let (dir, base, head) = scratch_repo();
        let out_dir = tempfile::tempdir().unwrap();
        let out = out_dir.path().join("capsule");

        run_create(dir.path(), &format!("{base}..{head}"), &out).unwrap();

        let manifest =
            CapsuleManifest::from_json_bytes(&std::fs::read(out.join("manifest.json")).unwrap())
                .unwrap();
        let rules = anvil_capsule::RulesDigest::from_json_bytes(
            &std::fs::read(out.join("rules.json")).unwrap(),
        )
        .unwrap();
        assert!(rules.rules_sha.is_some(), "config present in scratch repo");
        assert_eq!(rules.anvil_version, manifest.producer.anvil_version);
        assert_eq!(rules.opa_runtime_version, anvil_rules::OPA_RUNTIME_VERSION);
    }

    #[test]
    fn capsule_create_collects_policy_and_commits_evidence() {
        let (dir, base, head) = scratch_repo();
        let out_dir = tempfile::tempdir().unwrap();
        let out = out_dir.path().join("capsule");

        run_create(dir.path(), &format!("{base}..{head}"), &out).unwrap();

        let commits = anvil_capsule::CommitsDocument::from_json_bytes(
            &std::fs::read(out.join("commits.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(commits.commits.len(), 1);
        assert_eq!(commits.commits[0].changed_paths, vec!["b.txt".to_string()]);

        let policy = anvil_capsule::PolicyDigest::from_json_bytes(
            &std::fs::read(out.join("policy.json")).unwrap(),
        )
        .unwrap();
        let policy_file = policy.policy_file.expect("policy present in scratch repo");
        assert_eq!(policy_file.path, "anvil/policy.yml");
    }

    /// A witness tree in the repo is collected into `witness.ndjson`
    /// and the manifest's range pointers mark the head commit's line
    /// (GITGOV-007 wiring).
    #[test]
    fn capsule_create_collects_witness_chain_for_range() {
        use anvil_witness::{GenesisAnchor, WitnessLine};

        let (dir, base, head) = scratch_repo();

        // Seed a one-line witness chain attesting the head commit under
        // `anvil/witness/`, where `witness_paths` discovers it.
        let witness_dir = dir.path().join("anvil/witness");
        std::fs::create_dir_all(&witness_dir).unwrap();
        let mut wl = WitnessLine::genesis(
            &GenesisAnchor::Fresh,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            "active",
            "2026-06-08T00:00:00Z",
            "pre-commit",
            None,
        );
        wl.commit_sha = Some(head.clone());
        std::fs::write(
            witness_dir.join("active.ndjson"),
            wl.to_ndjson_line().unwrap(),
        )
        .unwrap();

        let out_dir = tempfile::tempdir().unwrap();
        let out = out_dir.path().join("capsule");
        run_create(dir.path(), &format!("{base}..{head}"), &out).unwrap();

        // The chain landed verbatim and the head line is inside it.
        let witness = std::fs::read_to_string(out.join("witness.ndjson")).unwrap();
        assert!(witness.contains(&head), "head commit witnessed in chain");

        // The manifest marks the head line's seq as the range window.
        let manifest =
            CapsuleManifest::from_json_bytes(&std::fs::read(out.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest.range.witness_seq_start, Some(1));
        assert_eq!(manifest.range.witness_seq_end, Some(1));
    }

    /// `verify` of a fresh, intact capsule passes and persists the verdict
    /// back into the capsule (GITGOV-009 CLI wiring + write-back).
    #[test]
    fn capsule_verify_passes_and_records_verdict() {
        use anvil_witness::{GenesisAnchor, WitnessLine};

        let (dir, base, head) = scratch_repo();
        let witness_dir = dir.path().join("anvil/witness");
        std::fs::create_dir_all(&witness_dir).unwrap();
        let mut wl = WitnessLine::genesis(
            &GenesisAnchor::Fresh,
            "01997e4a-1b2c-7345-8901-abcdef123456",
            "active",
            "2026-06-08T00:00:00Z",
            "pre-commit",
            None,
        );
        wl.commit_sha = Some(head.clone());
        std::fs::write(
            witness_dir.join("active.ndjson"),
            wl.to_ndjson_line().unwrap(),
        )
        .unwrap();

        let out_dir = tempfile::tempdir().unwrap();
        let out = out_dir.path().join("capsule");
        run_create(dir.path(), &format!("{base}..{head}"), &out).unwrap();

        let verification = verify_and_record(dir.path(), &out).unwrap();
        assert_eq!(
            verification.verdict,
            Verdict::Pass,
            "checks: {:?}",
            verification.checks
        );

        // The verdict was persisted, and the manifest still digests the
        // rewritten verification.json (re-verifies clean).
        let written = anvil_capsule::CapsuleVerification::from_json_bytes(
            &std::fs::read(out.join("verification.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(written.verdict, Verdict::Pass);
        let reverify = verify_and_record(dir.path(), &out).unwrap();
        assert_eq!(
            reverify.verdict,
            Verdict::Pass,
            "manifest digest re-recorded"
        );
    }

    #[test]
    fn capsule_create_rejects_malformed_ranges() {
        let (dir, _, head) = scratch_repo();
        let out_dir = tempfile::tempdir().unwrap();

        for bad in [
            "deadbeef",
            &format!("..{head}"),
            "base..",
            "a...b",
            "a..b..c",
            "abc .. def",
        ] {
            let err = run_create(dir.path(), bad, &out_dir.path().join("c")).unwrap_err();
            assert!(
                err.to_string().contains("--range"),
                "expected range error for `{bad}`: {err}"
            );
        }
    }

    #[test]
    fn capsule_create_refuses_non_empty_out_dir() {
        let (dir, base, head) = scratch_repo();
        let out_dir = tempfile::tempdir().unwrap();
        std::fs::write(out_dir.path().join("keep.txt"), "existing").unwrap();

        let err = run_create(dir.path(), &format!("{base}..{head}"), out_dir.path()).unwrap_err();

        assert!(format!("{err:#}").contains("not empty"), "{err:#}");
    }

    /// `--out` inside `.git/` is refused — capsule files there could
    /// corrupt repository state.
    #[test]
    fn capsule_create_refuses_out_inside_git_dir() {
        let (dir, base, head) = scratch_repo();
        let out = dir.path().join(".git").join("capsule-stash");

        let err = run_create(dir.path(), &format!("{base}..{head}"), &out).unwrap_err();

        assert!(format!("{err:#}").contains(".git"), "{err:#}");
        assert!(!out.exists(), "nothing written inside .git");
    }

    #[test]
    fn capsule_create_unresolvable_ref_fails_loudly() {
        let (dir, _, head) = scratch_repo();
        let out_dir = tempfile::tempdir().unwrap();

        let err = run_create(
            dir.path(),
            &format!("no-such-ref..{head}"),
            &out_dir.path().join("c"),
        )
        .unwrap_err();

        assert!(format!("{err:#}").contains("no-such-ref"), "{err:#}");
    }

    // ----- GITGOV-010: `capsule explain` -----

    /// Overwrite a capsule file *and* re-record its manifest digest, so
    /// the capsule stays internally consistent after a fixture tweak.
    fn rewrite_recorded(out: &Path, name: &str, bytes: &[u8]) {
        std::fs::write(out.join(name), bytes).unwrap();
        let mut manifest =
            CapsuleManifest::from_json_bytes(&std::fs::read(out.join("manifest.json")).unwrap())
                .unwrap();
        manifest.record_file(name, bytes);
        std::fs::write(
            out.join("manifest.json"),
            manifest.to_canonical_bytes().unwrap(),
        )
        .unwrap();
    }

    /// A deterministic, git-free capsule with rich-but-fixed evidence:
    /// two commits, a present policy/rules/baseline, a 3-line witness
    /// chain covering seq 2..4, one error + one warning diagnostic, and
    /// one applied exception. The verdict is left as the create
    /// placeholder; tests overwrite `verification.json` to drive states.
    fn rich_capsule(out: &Path) {
        let content = CapsuleContent {
            commits: CommitsDocument {
                schema: COMMITS_SCHEMA.to_string(),
                base: "1111111111111111111111111111111111111111".to_string(),
                head: "2222222222222222222222222222222222222222".to_string(),
                commits: vec![
                    CommitEntry {
                        sha: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                        tree: "cccccccccccccccccccccccccccccccccccccccc".to_string(),
                        parents: vec!["1111111111111111111111111111111111111111".to_string()],
                        changed_paths: vec!["src/lib.rs".to_string()],
                    },
                    CommitEntry {
                        sha: "2222222222222222222222222222222222222222".to_string(),
                        tree: "dddddddddddddddddddddddddddddddddddddddd".to_string(),
                        parents: vec!["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()],
                        changed_paths: vec!["src/main.rs".to_string()],
                    },
                ],
            },
            digests: CollectedDigests {
                policy: PolicyDigest {
                    schema: POLICY_DIGEST_SCHEMA.to_string(),
                    policy_file: Some(FileDigest {
                        path: "anvil/policy.yml".to_string(),
                        digest: "0".repeat(64),
                    }),
                    config_file: None,
                },
                rules: RulesDigest {
                    schema: RULES_DIGEST_SCHEMA.to_string(),
                    anvil_version: "0.8.0-beta".to_string(),
                    opa_runtime_version: "opa-runtime-0.0.0".to_string(),
                    rules: vec![],
                    config_sha: Some("e".repeat(64)),
                    rules_sha: Some(
                        "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
                            .to_string(),
                    ),
                },
                baseline: BaselineDigest {
                    schema: BASELINE_DIGEST_SCHEMA.to_string(),
                    cutoff_commit: Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string()),
                    digest: Some("f".repeat(64)),
                },
            },
            witness: CollectedWitness {
                ndjson: b"{\"seq\":2}\n{\"seq\":3}\n{\"seq\":4}\n".to_vec(),
                seq_start: Some(2),
                seq_end: Some(4),
            },
            diagnostics: collect_diagnostics(&[]).unwrap(),
            exceptions: anvil_capsule::CollectedExceptions {
                exceptions: Vec::new(),
            },
            producer: Producer {
                anvil_version: "0.8.0-beta".to_string(),
            },
        };
        write_capsule(out, &content).unwrap();

        // Replace the empty SARIF + empty exceptions placeholders with a
        // fixed populated set so the count fields are exercised. explain
        // never checks digests, but re-recording keeps the capsule sane.
        rewrite_recorded(
            out,
            "diagnostics.sarif",
            br#"{"runs":[{"results":[{"level":"error"},{"level":"warning"}]}]}"#,
        );
        rewrite_recorded(out, "exceptions.json", br#"[{"id":"exc_legacy_001"}]"#);
    }

    /// Overwrite `verification.json` with a constructed verdict and
    /// re-record its digest (the verify step's write-back, in miniature).
    fn set_verification(out: &Path, verification: &CapsuleVerification) {
        let bytes = verification.to_canonical_bytes().unwrap();
        rewrite_recorded(out, "verification.json", &bytes);
    }

    fn check(name: &str, verdict: Verdict, detail: &str) -> CheckResult {
        CheckResult {
            name: name.to_string(),
            verdict,
            detail: Some(detail.to_string()),
        }
    }

    /// The fixed header every golden shares — built with `concat!` (no
    /// `\` line continuations, which would eat the leading-space column
    /// alignment) so the indentation is byte-exact.
    const HEADER: &str = concat!(
        "anvil Review Capsule  (anvil.capsule.v1)\n",
        "\n",
        "  Producer    anvil 0.8.0-beta\n",
        "  Range       111111111111..222222222222\n",
        "  Commits     2\n",
        "  Policy      anvil/policy.yml\n",
        "  Rules       abcdef012345\n",
        "  Baseline    cutoff bbbbbbbbbbbb\n",
        "  Witness     seq 2 to 4 (inclusive), 3 lines\n",
        "  Diagnostics 2 (1 error, 1 warning)\n",
        "  Exceptions  1 applied\n",
    );

    #[test]
    fn capsule_explain_renders_header_from_capsule_files() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);

        let report = render_explanation(&out).unwrap();
        assert!(
            report.starts_with(HEADER),
            "header mismatch.\n--- got ---\n{report}\n--- want prefix ---\n{HEADER}"
        );
    }

    #[test]
    fn capsule_explain_golden_pass() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        set_verification(
            &out,
            &CapsuleVerification::from_checks(vec![
                check("manifest-digests", Verdict::Pass, "10 files verified"),
                check("witness-chain", Verdict::Pass, "3 lines, 0 merge(s)"),
                check(
                    "digests-vs-repo",
                    Verdict::Pass,
                    "commits + policy/rules/baseline match the repo",
                ),
                check("exceptions", Verdict::Pass, "1 applied exception(s) valid"),
            ]),
        );

        let report = render_explanation(&out).unwrap();
        assert_eq!(
            report,
            format!(
                "{HEADER}{}",
                concat!(
                    "\n",
                    "Verdict: PASS (exit 0)\n",
                    "  [pass]     manifest-digests — 10 files verified\n",
                    "  [pass]     witness-chain — 3 lines, 0 merge(s)\n",
                    "  [pass]     digests-vs-repo — commits + policy/rules/baseline match the repo\n",
                    "  [pass]     exceptions — 1 applied exception(s) valid\n",
                )
            )
        );
    }

    #[test]
    fn capsule_explain_golden_warn() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        set_verification(
            &out,
            &CapsuleVerification::from_checks(vec![
                check(
                    "manifest-digests",
                    Verdict::Warn,
                    "unexpected file not in manifest: stowaway.txt",
                ),
                check("witness-chain", Verdict::Pass, "3 lines, 0 merge(s)"),
            ]),
        );

        let report = render_explanation(&out).unwrap();
        assert_eq!(
            report,
            format!(
                "{HEADER}{}",
                concat!(
                    "\n",
                    "Verdict: WARN (exit 0)\n",
                    "  [warn]     manifest-digests — unexpected file not in manifest: stowaway.txt\n",
                    "  [pass]     witness-chain — 3 lines, 0 merge(s)\n",
                )
            )
        );
    }

    #[test]
    fn capsule_explain_golden_degraded() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        set_verification(
            &out,
            &CapsuleVerification::from_checks(vec![
                check("manifest-digests", Verdict::Pass, "10 files verified"),
                check("witness-chain", Verdict::Degraded, "witness.ndjson absent"),
            ]),
        );

        let report = render_explanation(&out).unwrap();
        assert_eq!(
            report,
            format!(
                "{HEADER}{}",
                concat!(
                    "\n",
                    "Verdict: DEGRADED (exit 2)\n",
                    "  [pass]     manifest-digests — 10 files verified\n",
                    "  [degraded] witness-chain — witness.ndjson absent\n",
                )
            )
        );
    }

    #[test]
    fn capsule_explain_golden_block() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        set_verification(
            &out,
            &CapsuleVerification::from_checks(vec![
                check("manifest-digests", Verdict::Pass, "10 files verified"),
                check(
                    "witness-chain",
                    Verdict::Block,
                    "witness chain broken: seq gap at 3",
                ),
            ]),
        );

        let report = render_explanation(&out).unwrap();
        assert_eq!(
            report,
            format!(
                "{HEADER}{}",
                concat!(
                    "\n",
                    "Verdict: BLOCK (exit 1)\n",
                    "  [pass]     manifest-digests — 10 files verified\n",
                    "  [block]    witness-chain — witness chain broken: seq gap at 3\n",
                )
            )
        );
    }

    /// The create-time placeholder (degraded, no checks) explains as an
    /// honest "not yet verified", never a bare verdict with no support.
    #[test]
    fn capsule_explain_unverified_placeholder_says_run_verify() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        // write_capsule already left the degraded placeholder in place.

        let report = render_explanation(&out).unwrap();
        assert!(
            report.contains(
                "Verdict: DEGRADED (exit 2)\n  (no checks recorded — run `anvil capsule verify`)\n"
            ),
            "got:\n{report}"
        );
    }

    /// A capsule whose repo carried no governance evidence: an empty
    /// commit list, no policy/rules/baseline, no witness chain, and the
    /// write-time empty SARIF + exceptions placeholders.
    fn absent_capsule(out: &Path) {
        let content = CapsuleContent {
            commits: CommitsDocument {
                schema: COMMITS_SCHEMA.to_string(),
                base: "1111111111111111111111111111111111111111".to_string(),
                head: "1111111111111111111111111111111111111111".to_string(),
                commits: vec![],
            },
            digests: CollectedDigests {
                policy: PolicyDigest {
                    schema: POLICY_DIGEST_SCHEMA.to_string(),
                    policy_file: None,
                    config_file: None,
                },
                rules: RulesDigest {
                    schema: RULES_DIGEST_SCHEMA.to_string(),
                    anvil_version: "0.8.0-beta".to_string(),
                    opa_runtime_version: "opa-runtime-0.0.0".to_string(),
                    rules: vec![],
                    config_sha: None,
                    rules_sha: None,
                },
                baseline: BaselineDigest {
                    schema: BASELINE_DIGEST_SCHEMA.to_string(),
                    cutoff_commit: None,
                    digest: None,
                },
            },
            witness: CollectedWitness::default(),
            diagnostics: collect_diagnostics(&[]).unwrap(),
            exceptions: anvil_capsule::CollectedExceptions {
                exceptions: Vec::new(),
            },
            producer: Producer {
                anvil_version: "0.8.0-beta".to_string(),
            },
        };
        write_capsule(out, &content).unwrap();
    }

    /// A repo with no governance evidence: every absent field reads as
    /// `absent`/`none`, never a crash or a misleading zero.
    #[test]
    fn capsule_explain_renders_absent_fields() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        absent_capsule(&out);

        let report = render_explanation(&out).unwrap();
        assert!(report.contains("Commits     0"), "{report}");
        assert!(report.contains("Policy      absent"), "{report}");
        assert!(
            report.contains("Rules       absent (no config)"),
            "{report}"
        );
        assert!(report.contains("Baseline    absent"), "{report}");
        assert!(
            report.contains("Witness     absent (no witness chain)"),
            "{report}"
        );
        assert!(report.contains("Diagnostics none"), "{report}");
        assert!(report.contains("Exceptions  none"), "{report}");
    }

    /// A missing required file is a tamper signal, not "nothing to
    /// report": explain marks it `missing` and still renders the rest.
    #[test]
    fn capsule_explain_marks_missing_evidence_file() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        std::fs::remove_file(out.join("commits.json")).unwrap();

        let report = render_explanation(&out).unwrap();
        assert!(report.contains("Commits     missing"), "{report}");
        // The manifest still carries the range, so the report stands.
        assert!(
            report.contains("Range       111111111111..222222222222"),
            "{report}"
        );
    }

    /// An unreadable manifest is the one hard failure — the digest root
    /// carries the range and producer, so there is nothing to explain.
    #[test]
    fn capsule_explain_errors_when_manifest_unreadable() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        std::fs::create_dir_all(&out).unwrap();
        // No manifest.json at all.
        assert!(render_explanation(&out).is_err());
    }

    /// Single-line witness coverage collapses to `seq N` (no `seq 1 to
    /// 1`), and a unique result level pluralises correctly.
    #[test]
    fn capsule_explain_single_seq_window_and_singular_counts() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        let content = CapsuleContent {
            commits: CommitsDocument {
                schema: COMMITS_SCHEMA.to_string(),
                base: "1111111111111111111111111111111111111111".to_string(),
                head: "2222222222222222222222222222222222222222".to_string(),
                commits: vec![CommitEntry {
                    sha: "2222222222222222222222222222222222222222".to_string(),
                    tree: "cccccccccccccccccccccccccccccccccccccccc".to_string(),
                    parents: vec![],
                    changed_paths: vec![],
                }],
            },
            digests: CollectedDigests {
                policy: PolicyDigest {
                    schema: POLICY_DIGEST_SCHEMA.to_string(),
                    policy_file: None,
                    config_file: None,
                },
                rules: RulesDigest {
                    schema: RULES_DIGEST_SCHEMA.to_string(),
                    anvil_version: "0.8.0-beta".to_string(),
                    opa_runtime_version: "opa-runtime-0.0.0".to_string(),
                    rules: vec![],
                    config_sha: None,
                    rules_sha: None,
                },
                baseline: BaselineDigest {
                    schema: BASELINE_DIGEST_SCHEMA.to_string(),
                    cutoff_commit: None,
                    digest: None,
                },
            },
            witness: CollectedWitness {
                ndjson: b"{\"seq\":5}\n".to_vec(),
                seq_start: Some(5),
                seq_end: Some(5),
            },
            diagnostics: collect_diagnostics(&[]).unwrap(),
            exceptions: anvil_capsule::CollectedExceptions {
                exceptions: Vec::new(),
            },
            producer: Producer {
                anvil_version: "0.8.0-beta".to_string(),
            },
        };
        write_capsule(&out, &content).unwrap();
        rewrite_recorded(
            &out,
            "diagnostics.sarif",
            br#"{"runs":[{"results":[{"level":"warning"}]}]}"#,
        );

        let report = render_explanation(&out).unwrap();
        assert!(report.contains("Witness     seq 5, 1 line\n"), "{report}");
        assert!(report.contains("Diagnostics 1 (1 warning)\n"), "{report}");
    }

    /// `digest`-without-cutoff keeps "baseline with no cutoff"
    /// distinguishable from "no baseline at all".
    #[test]
    fn capsule_explain_baseline_present_without_cutoff() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        let baseline = BaselineDigest {
            schema: BASELINE_DIGEST_SCHEMA.to_string(),
            cutoff_commit: None,
            digest: Some("a".repeat(64)),
        };
        rewrite_recorded(
            &out,
            "baseline.json",
            &baseline.to_canonical_bytes().unwrap(),
        );

        let report = render_explanation(&out).unwrap();
        assert!(
            report.contains("Baseline    present (no cutoff)"),
            "{report}"
        );
    }

    /// The whole report ends in exactly one newline — no trailing blank
    /// line, no missing terminator.
    #[test]
    fn capsule_explain_ends_with_single_newline() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);

        let report = render_explanation(&out).unwrap();
        assert!(report.ends_with('\n'));
        assert!(!report.ends_with("\n\n"));
    }

    /// explain reads only the capsule — it never checks the manifest
    /// digest. A capsule whose `verification.json` was rewritten without
    /// re-recording the digest still explains its stated verdict
    /// (explain reports, it does not adjudicate).
    #[test]
    fn capsule_explain_does_not_verify_digests() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        let pass = CapsuleVerification::from_checks(vec![check(
            "manifest-digests",
            Verdict::Pass,
            "10 files verified",
        )]);
        std::fs::write(
            out.join("verification.json"),
            pass.to_canonical_bytes().unwrap(),
        )
        .unwrap();

        let report = render_explanation(&out).unwrap();
        assert!(report.contains("Verdict: PASS (exit 0)"), "{report}");
    }

    #[test]
    fn capsule_explain_short_truncates_and_tolerates_short_input() {
        assert_eq!(short("1234567890abcdef"), "1234567890ab");
        assert_eq!(short("abc"), "abc");
    }

    /// The fifth verdict — `error` (e.g. an unparseable evidence file) —
    /// renders on the same path as the other four (council follow-up:
    /// the work item lists pass/warn/degraded/block, but `error` is a
    /// live `Verdict` variant and must not silently drift).
    #[test]
    fn capsule_explain_golden_error() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        set_verification(
            &out,
            &CapsuleVerification::from_checks(vec![
                check("manifest-digests", Verdict::Pass, "10 files verified"),
                check(
                    "exceptions",
                    Verdict::Error,
                    "unparseable exceptions.json: expected value",
                ),
            ]),
        );

        let report = render_explanation(&out).unwrap();
        assert_eq!(
            report,
            format!(
                "{HEADER}{}",
                concat!(
                    "\n",
                    "Verdict: ERROR (exit 3)\n",
                    "  [pass]     manifest-digests — 10 files verified\n",
                    "  [error]    exceptions — unparseable exceptions.json: expected value\n",
                )
            )
        );
    }

    /// A check with no `detail` renders as just `[token] name` — the
    /// `None` arm of `verdict_section` (the verify engine always sets a
    /// detail, so this guards the rendering path directly).
    #[test]
    fn capsule_explain_check_without_detail() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        set_verification(
            &out,
            &CapsuleVerification::from_checks(vec![CheckResult {
                name: "manifest-digests".to_string(),
                verdict: Verdict::Pass,
                detail: None,
            }]),
        );

        let report = render_explanation(&out).unwrap();
        assert!(
            report.ends_with("Verdict: PASS (exit 0)\n  [pass]     manifest-digests\n"),
            "got:\n{report}"
        );
    }

    /// A manifest that claims a witness window whose chain file is gone
    /// is an unbacked claim — explain says `chain absent`, never a
    /// reassuring window with nothing behind it.
    #[test]
    fn capsule_explain_witness_window_with_missing_chain_says_chain_absent() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out); // manifest records seq 2..4
        std::fs::remove_file(out.join("witness.ndjson")).unwrap();

        let report = render_explanation(&out).unwrap();
        assert!(
            report.contains("Witness     seq 2 to 4 (inclusive), chain absent\n"),
            "{report}"
        );
    }

    /// An unreadable witness chain surfaces as `(unreadable)` like every
    /// other field — never folded into a misleading `0 lines`.
    #[cfg(unix)]
    #[test]
    fn capsule_explain_witness_unreadable_is_marked() {
        use std::os::unix::fs::PermissionsExt;
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        let chain = out.join("witness.ndjson");
        std::fs::set_permissions(&chain, std::fs::Permissions::from_mode(0o000)).unwrap();
        let blocked = std::fs::read(&chain).is_err();
        let report = render_explanation(&out);
        std::fs::set_permissions(&chain, std::fs::Permissions::from_mode(0o644)).unwrap();
        if !blocked {
            return; // running as root (CAP_DAC_OVERRIDE); nothing to assert
        }
        assert!(
            report.unwrap().contains("Witness     (unreadable)\n"),
            "expected unreadable witness marker"
        );
    }

    /// Whitespace-only witness lines are not records: a chain of blank
    /// lines under a seq window counts as zero → `chain absent`.
    #[test]
    fn capsule_explain_witness_blank_lines_are_not_counted() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        rewrite_recorded(&out, "witness.ndjson", b"   \n\t\n  \r\n");

        let report = render_explanation(&out).unwrap();
        assert!(
            report.contains("Witness     seq 2 to 4 (inclusive), chain absent\n"),
            "{report}"
        );
    }

    /// SARIF `none` is a *suppressed* result, kept in its own bucket and
    /// never laundered into the `note` count; the total still equals the
    /// breakdown sum.
    #[test]
    fn capsule_explain_diagnostics_counts_suppressed_separately() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        rewrite_recorded(
            &out,
            "diagnostics.sarif",
            br#"{"runs":[{"results":[{"level":"note"},{"level":"none"},{"level":"none"}]}]}"#,
        );

        let report = render_explanation(&out).unwrap();
        assert!(
            report.contains("Diagnostics 3 (1 note, 2 suppressed)\n"),
            "{report}"
        );
    }

    /// A tampered string field with a planted newline cannot forge an
    /// extra output row — `one_line` flattens control characters, so the
    /// report of a tampered capsule stays one line per field.
    #[test]
    fn capsule_explain_sanitises_injected_newlines() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        // A hostile policy path carrying a newline + a forged field line.
        let policy = PolicyDigest {
            schema: POLICY_DIGEST_SCHEMA.to_string(),
            policy_file: Some(FileDigest {
                path: "anvil/policy.yml\n  Baseline    forged".to_string(),
                digest: "0".repeat(64),
            }),
            config_file: None,
        };
        rewrite_recorded(&out, "policy.json", &policy.to_canonical_bytes().unwrap());

        let report = render_explanation(&out).unwrap();
        // The newline is flattened to a space — one Policy line, and the
        // real Baseline line still reads `cutoff …`, not the forgery.
        assert!(
            report.contains("Policy      anvil/policy.yml   Baseline    forged\n"),
            "{report}"
        );
        assert!(
            report.contains("Baseline    cutoff bbbbbbbbbbbb\n"),
            "{report}"
        );
    }

    /// `one_line` neutralises ANSI escapes and Unicode bidi/zero-width
    /// format characters (Trojan-Source spoofing) too, while preserving
    /// ordinary non-ASCII text.
    #[test]
    fn capsule_explain_one_line_strips_control_and_bidi_chars() {
        assert_eq!(one_line("a\nb\r\tc"), "a b  c");
        assert_eq!(one_line("x\u{1b}[31my"), "x [31my");
        assert_eq!(one_line("a\u{202e}b\u{200b}c"), "a b c"); // RLO + ZWSP
        assert_eq!(one_line("ünïcode/path"), "ünïcode/path"); // legit non-ASCII kept
        assert_eq!(one_line("clean"), "clean");
    }

    /// A `verification.json` that fails to parse renders a flattened,
    /// single-line `(unreadable: …)` — the error text echoes untrusted
    /// document fields, so a planted newline in it cannot forge a row.
    #[test]
    fn capsule_explain_sanitises_verdict_parse_error() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        // A schema-mismatched verification.json: `from_json_bytes` echoes
        // the (here newline-bearing) `found` schema string in its error.
        rewrite_recorded(
            &out,
            "verification.json",
            b"{\"schema\":\"anvil.capsule-verification.vEVIL\\n  Verdict: PASS (exit 0)\",\"verdict\":\"degraded\",\"checks\":[]}",
        );

        let report = render_explanation(&out).unwrap();
        let verdict_lines: Vec<&str> = report
            .lines()
            .filter(|l| l.starts_with("Verdict:"))
            .collect();
        assert_eq!(verdict_lines.len(), 1, "exactly one Verdict line: {report}");
        assert!(
            verdict_lines[0].starts_with("Verdict: (unreadable:"),
            "{report}"
        );
        // The forged `Verdict: PASS` text must not appear as its own line.
        assert!(
            !report.lines().any(|l| l == "  Verdict: PASS (exit 0)"),
            "forged line leaked: {report}"
        );
    }

    // ----- GITGOV-011: `--json` output -----

    /// The single trailing-newline JSON line emitted by `verify --json`.
    /// Asserts it is exactly the verification document (round-trips
    /// through the schema-gated parser) and a single line — for any
    /// verdict, including the `error` one create never produces.
    #[test]
    fn capsule_json_verify_emits_verification_document() {
        let doc = CapsuleVerification::from_checks(vec![
            check("manifest-digests", Verdict::Pass, "10 files verified"),
            check("witness-chain", Verdict::Block, "seq gap at 3"),
        ]);
        assert_eq!(doc.verdict, Verdict::Block); // worst-of

        let line = verification_json_line(&doc).unwrap();
        assert!(line.ends_with('\n'), "trailing newline: {line:?}");
        assert_eq!(line.matches('\n').count(), 1, "single line: {line:?}");

        let parsed = CapsuleVerification::from_json_bytes(line.trim_end().as_bytes()).unwrap();
        assert_eq!(parsed, doc, "round-trips through the schema-gated parser");
    }

    /// An `error`-verdict document (e.g. an unreadable manifest) still
    /// encodes to a clean JSON line — the JSON surface never panics on a
    /// verdict the human path also renders.
    #[test]
    fn capsule_json_verify_encodes_error_verdict() {
        let doc = CapsuleVerification::from_checks(vec![check(
            "manifest-digests",
            Verdict::Error,
            "manifest.json unreadable",
        )]);
        let line = verification_json_line(&doc).unwrap();
        let value: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["verdict"], "error");
    }

    /// The full `explain --json` summary over a rich capsule: every field
    /// is present and typed, and the recorded verdict is the create-time
    /// placeholder (degraded, no checks).
    #[test]
    fn capsule_json_explain_summary_is_fully_typed() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);

        let line = render_explanation_json(&out).unwrap();
        assert!(line.ends_with('\n'));
        assert_eq!(line.matches('\n').count(), 1, "single JSON line: {line:?}");
        let v: Value = serde_json::from_str(&line).unwrap();

        assert_eq!(v["schema"], "anvil.capsule-explain.v1");
        assert_eq!(v["capsule_schema"], "anvil.capsule.v1");
        assert_eq!(v["producer"]["anvil_version"], "0.8.0-beta");
        assert_eq!(
            v["range"]["base"],
            "1111111111111111111111111111111111111111"
        );
        assert_eq!(
            v["range"]["head"],
            "2222222222222222222222222222222222222222"
        );
        assert_eq!(v["commits"], json!({"status": "present", "count": 2}));
        assert_eq!(
            v["policy"],
            json!({"status": "present", "path": "anvil/policy.yml"})
        );
        assert_eq!(
            v["rules"],
            json!({
                "status": "present",
                "rules_sha": "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
            })
        );
        assert_eq!(
            v["baseline"],
            json!({
                "status": "present",
                "cutoff_commit": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            })
        );
        assert_eq!(
            v["witness"],
            json!({
                "status": "present",
                "seq_start": 2,
                "seq_end": 4,
                "chain_present": true,
                "lines": 3
            })
        );
        assert_eq!(
            v["diagnostics"],
            json!({
                "status": "present",
                "total": 2,
                "error": 1,
                "warning": 1,
                "note": 0,
                "suppressed": 0
            })
        );
        assert_eq!(v["exceptions"], json!({"status": "present", "count": 1}));
        // rich_capsule leaves the create-time placeholder in place.
        assert_eq!(
            v["verdict"],
            json!({
                "status": "present",
                "verdict": "degraded",
                "exit_code": 2,
                "checks": []
            })
        );
    }

    /// A verified capsule's recorded verdict appears in `explain --json`
    /// with its checks — the structured analogue of the text verdict
    /// section.
    #[test]
    fn capsule_json_explain_reports_recorded_verdict_with_checks() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        set_verification(
            &out,
            &CapsuleVerification::from_checks(vec![
                check("manifest-digests", Verdict::Pass, "10 files verified"),
                check("witness-chain", Verdict::Warn, "1 merge line"),
            ]),
        );

        let v: Value = serde_json::from_str(&render_explanation_json(&out).unwrap()).unwrap();
        assert_eq!(v["verdict"]["status"], "present");
        assert_eq!(v["verdict"]["verdict"], "warn");
        assert_eq!(v["verdict"]["exit_code"], 0);
        let checks = v["verdict"]["checks"].as_array().unwrap();
        assert_eq!(checks.len(), 2);
        assert_eq!(checks[0]["name"], "manifest-digests");
        assert_eq!(checks[0]["verdict"], "pass");
        assert_eq!(checks[1]["verdict"], "warn");
        assert_eq!(checks[1]["detail"], "1 merge line");
    }

    /// A repo with no governance evidence: each degradable field carries
    /// its honest `absent`/`present`-with-zero state — never a crash or a
    /// misleading omission.
    #[test]
    fn capsule_json_explain_absent_fields_are_typed_states() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        absent_capsule(&out);

        let v: Value = serde_json::from_str(&render_explanation_json(&out).unwrap()).unwrap();
        assert_eq!(v["commits"], json!({"status": "present", "count": 0}));
        assert_eq!(v["policy"], json!({"status": "absent"}));
        assert_eq!(v["rules"], json!({"status": "absent"}));
        assert_eq!(v["baseline"], json!({"status": "absent"}));
        assert_eq!(v["witness"], json!({"status": "absent"}));
        assert_eq!(
            v["diagnostics"],
            json!({
                "status": "present",
                "total": 0,
                "error": 0,
                "warning": 0,
                "note": 0,
                "suppressed": 0
            })
        );
        assert_eq!(v["exceptions"], json!({"status": "present", "count": 0}));
    }

    /// A missing layout file is a tamper signal in JSON too: `status:
    /// missing`, and the manifest-backed range still reports.
    #[test]
    fn capsule_json_explain_marks_missing_file() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        std::fs::remove_file(out.join("commits.json")).unwrap();

        let v: Value = serde_json::from_str(&render_explanation_json(&out).unwrap()).unwrap();
        assert_eq!(v["commits"], json!({"status": "missing"}));
        assert_eq!(
            v["range"]["head"],
            "2222222222222222222222222222222222222222"
        );
    }

    /// A removed required `witness.ndjson` is a tamper signal — `missing`,
    /// surfaced like every other gone evidence file, not folded into the
    /// window logic where it would read as a backed-looking `present`.
    #[test]
    fn capsule_json_explain_witness_file_removed_is_missing() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out); // manifest records seq 2..4
        std::fs::remove_file(out.join("witness.ndjson")).unwrap();

        let v: Value = serde_json::from_str(&render_explanation_json(&out).unwrap()).unwrap();
        assert_eq!(v["witness"], json!({"status": "missing"}));
    }

    /// A present-but-empty chain under a recorded window is an unbacked
    /// claim — `present` with `chain_present: false` (distinct from the
    /// removed-file `missing` case above).
    #[test]
    fn capsule_json_explain_witness_empty_under_window() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out); // manifest records seq 2..4
        // The file is present but holds no records (blank lines only).
        rewrite_recorded(&out, "witness.ndjson", b"\n");

        let v: Value = serde_json::from_str(&render_explanation_json(&out).unwrap()).unwrap();
        assert_eq!(
            v["witness"],
            json!({
                "status": "present",
                "seq_start": 2,
                "seq_end": 4,
                "chain_present": false,
                "lines": 0
            })
        );
    }

    /// An asymmetric witness seq window (one pointer without the other)
    /// is a tampered manifest — reported as `status: malformed`.
    #[test]
    fn capsule_json_explain_malformed_witness_window() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        // Drop one side of the seq window directly in the manifest JSON
        // (explain does not digest-verify the manifest it reads).
        let mut manifest: Value =
            serde_json::from_slice(&std::fs::read(out.join("manifest.json")).unwrap()).unwrap();
        manifest["range"]["witness_seq_end"] = Value::Null;
        std::fs::write(
            out.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        let v: Value = serde_json::from_str(&render_explanation_json(&out).unwrap()).unwrap();
        assert_eq!(v["witness"], json!({"status": "malformed"}));
    }

    /// JSON encoding neutralises a planted control character structurally:
    /// a newline in an untrusted policy path is escaped, so the output
    /// stays a single JSON line and the value round-trips verbatim — no
    /// forged row, no lossy `one_line` mangling of legitimate data.
    #[test]
    fn capsule_json_explain_escapes_injected_control_chars() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        let hostile = "anvil/policy.yml\n  forged: true";
        let policy = PolicyDigest {
            schema: POLICY_DIGEST_SCHEMA.to_string(),
            policy_file: Some(FileDigest {
                path: hostile.to_string(),
                digest: "0".repeat(64),
            }),
            config_file: None,
        };
        rewrite_recorded(&out, "policy.json", &policy.to_canonical_bytes().unwrap());

        let line = render_explanation_json(&out).unwrap();
        assert_eq!(line.matches('\n').count(), 1, "single JSON line: {line:?}");
        let v: Value = serde_json::from_str(&line).unwrap();
        // The raw path is preserved exactly (JSON data, not terminal display).
        assert_eq!(v["policy"]["path"], hostile);
    }

    /// An unreadable manifest is the one hard failure for `explain
    /// --json` too — the digest root carries the range, so there is
    /// nothing to summarise.
    #[test]
    fn capsule_json_explain_errors_when_manifest_unreadable() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        std::fs::create_dir_all(&out).unwrap();
        assert!(render_explanation_json(&out).is_err());
    }

    /// A baseline present without a cutoff is `present` with no
    /// `cutoff_commit` key — absence is a state (key omitted), never a
    /// `null` value, so the schema carries no nulls in optional payload.
    #[test]
    fn capsule_json_explain_baseline_present_without_cutoff() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        let baseline = BaselineDigest {
            schema: BASELINE_DIGEST_SCHEMA.to_string(),
            cutoff_commit: None,
            digest: Some("a".repeat(64)),
        };
        rewrite_recorded(
            &out,
            "baseline.json",
            &baseline.to_canonical_bytes().unwrap(),
        );

        let v: Value = serde_json::from_str(&render_explanation_json(&out).unwrap()).unwrap();
        assert_eq!(v["baseline"], json!({"status": "present"}));
        assert!(
            v["baseline"].get("cutoff_commit").is_none(),
            "no null cutoff_commit key: {}",
            v["baseline"]
        );
    }

    /// A missing required evidence file is a `missing` status in JSON, the
    /// machine-readable tamper signal — covered here for `diagnostics`
    /// (the `commits` missing path is tested separately).
    #[test]
    fn capsule_json_explain_marks_missing_diagnostics() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        std::fs::remove_file(out.join("diagnostics.sarif")).unwrap();

        let v: Value = serde_json::from_str(&render_explanation_json(&out).unwrap()).unwrap();
        assert_eq!(v["diagnostics"], json!({"status": "missing"}));
    }

    /// An `exceptions.json` that parses as JSON but is not an array is
    /// `malformed` (wrong shape), kept distinct from `unreadable`
    /// (bytes-not-JSON) on the machine surface.
    #[test]
    fn capsule_json_explain_malformed_exceptions() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        // Valid JSON, wrong shape (object, not the expected array).
        rewrite_recorded(&out, "exceptions.json", br#"{"oops": true}"#);

        let v: Value = serde_json::from_str(&render_explanation_json(&out).unwrap()).unwrap();
        assert_eq!(v["exceptions"], json!({"status": "malformed"}));
    }

    /// An existing-but-unreadable witness chain surfaces as `unreadable`
    /// on the JSON path too — never folded into a misleading `lines: 0`.
    #[cfg(unix)]
    #[test]
    fn capsule_json_explain_witness_unreadable_is_marked() {
        use std::os::unix::fs::PermissionsExt;
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        let chain = out.join("witness.ndjson");
        std::fs::set_permissions(&chain, std::fs::Permissions::from_mode(0o000)).unwrap();
        let blocked = std::fs::read(&chain).is_err();
        let report = render_explanation_json(&out);
        std::fs::set_permissions(&chain, std::fs::Permissions::from_mode(0o644)).unwrap();
        if !blocked {
            return; // running as root (CAP_DAC_OVERRIDE); nothing to assert
        }
        let v: Value = serde_json::from_str(&report.unwrap()).unwrap();
        assert_eq!(v["witness"], json!({"status": "unreadable"}));
    }

    /// The full `verify --json` path on a corrupt-manifest capsule: the
    /// engine yields an `error` verdict (not an `Err`), and the emitted
    /// JSON line carries it — exercising `verify_and_record` +
    /// `verification_json_line` together, not just the encoder.
    #[test]
    fn capsule_json_verify_emits_error_verdict_on_corrupt_manifest() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);
        // Garbage manifest: the digest root no longer parses.
        std::fs::write(out.join("manifest.json"), b"not json at all").unwrap();

        // repo_root is irrelevant — the engine errors on the manifest
        // before reaching any repository checks.
        let verification = verify_and_record(stage.path(), &out).unwrap();
        assert_eq!(verification.verdict, Verdict::Error);

        let line = verification_json_line(&verification).unwrap();
        let v: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["verdict"], "error");
    }

    /// Both capsule JSON surfaces are canonical (recursively sorted keys):
    /// re-encoding the parsed `explain --json` output through the same
    /// canonical encoder is a fixed point, so the bytes are stable.
    #[test]
    fn capsule_json_explain_output_is_canonical() {
        let stage = tempfile::tempdir().unwrap();
        let out = stage.path().join("capsule");
        rich_capsule(&out);

        let line = render_explanation_json(&out).unwrap();
        let v: Value = serde_json::from_str(&line).unwrap();
        let recanonicalised = canonical_json_bytes(&v).unwrap();
        assert_eq!(
            line.trim_end().as_bytes(),
            recanonicalised.as_slice(),
            "explain --json output is already canonical"
        );
    }

    // --- prune (ADR-078, GITGOV-013) ---

    /// A minimal capsule directory (manifest only) pointing at `head` —
    /// enough for the schema gate and ordering; prune never reads the
    /// evidence files.
    fn write_min_capsule(root: &Path, name: &str, head: &str) -> std::path::PathBuf {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = CapsuleManifest::new(
            anvil_capsule::CapsuleRange {
                base: "0".repeat(40),
                head: head.to_string(),
                witness_seq_start: None,
                witness_seq_end: None,
            },
            Producer {
                anvil_version: "0.0.0-test".to_string(),
            },
        );
        std::fs::write(
            dir.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        dir
    }

    #[test]
    fn prune_root_outside_repo_is_refused() {
        let (repo, _base, _head) = scratch_repo();
        let outside = tempfile::tempdir().unwrap();
        let err = validate_prune_root(repo.path(), outside.path()).unwrap_err();
        assert!(err.to_string().contains("outside the repository"), "{err}");
    }

    #[test]
    fn prune_root_inside_git_dir_is_refused() {
        let (repo, _base, _head) = scratch_repo();
        let inside = repo.path().join(".git/capsules");
        std::fs::create_dir_all(&inside).unwrap();
        let err = validate_prune_root(repo.path(), &inside).unwrap_err();
        assert!(err.to_string().contains(".git"), "{err}");
    }

    #[test]
    fn prune_nonexistent_traversal_root_is_refused() {
        // `..` segments in a NOT-yet-existing --root must still fail the
        // containment check (canonicalize can't resolve them; the lexical
        // normalisation must).
        let (repo, _base, _head) = scratch_repo();
        let err = validate_prune_root(repo.path(), Path::new("../missing-outside")).unwrap_err();
        assert!(err.to_string().contains("outside the repository"), "{err}");
    }

    #[test]
    fn prune_zero_candidates_is_a_warned_noop() {
        let (repo, _base, _head) = scratch_repo();
        let root = repo.path().join("anvil/evidence/capsules");
        std::fs::create_dir_all(root.join("not-a-capsule")).unwrap();
        std::fs::write(root.join("not-a-capsule/manifest.json"), b"{}").unwrap();
        run_prune(repo.path(), None, 1, true).expect("noop with zero candidates");
        assert!(
            root.join("not-a-capsule").exists(),
            "non-capsule entries are never deleted"
        );
    }

    #[test]
    fn prune_missing_default_root_is_a_noop() {
        let (repo, _base, _head) = scratch_repo();
        run_prune(repo.path(), None, 1, false).expect("noop prune");
        run_prune(repo.path(), None, 1, true).expect("noop apply");
    }

    #[test]
    fn prune_dry_run_deletes_nothing_apply_deletes_oldest() {
        let (repo, _base, _head) = scratch_repo();
        // scratch_repo's two commits share a committer second; pin two
        // commits with distinct dates so "oldest" is unambiguous.
        let commit_at = |epoch: &str, msg: &str| -> String {
            let date = format!("{epoch} +0000");
            let out = Command::new("git")
                .args(["commit", "-q", "--allow-empty", "-m", msg])
                .env("GIT_COMMITTER_DATE", &date)
                .env("GIT_AUTHOR_DATE", &date)
                .current_dir(repo.path())
                .output()
                .expect("spawn git commit");
            assert!(out.status.success(), "{out:?}");
            git(repo.path(), &["rev-parse", "HEAD"]).trim().to_string()
        };
        let old = commit_at("1700000000", "old");
        let new = commit_at("1700000100", "new");
        let root = repo.path().join("anvil/evidence/capsules");
        let old_dir = write_min_capsule(&root, "cap-old", &old);
        let new_dir = write_min_capsule(&root, "cap-new", &new);

        run_prune(repo.path(), None, 1, false).expect("dry run");
        assert!(
            old_dir.exists() && new_dir.exists(),
            "dry run deletes nothing"
        );

        run_prune(repo.path(), None, 1, true).expect("apply");
        assert!(!old_dir.exists(), "oldest capsule deleted on --apply");
        assert!(new_dir.exists(), "newest capsule kept");
    }
}
