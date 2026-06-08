//! `anvil capsule` command (GITGOV-004 CLI lane).
//!
//! Packages a commit range's governance evidence into an ADR-074
//! review capsule directory a reviewer, auditor, or supplier can
//! verify locally without trusting Anvil Cloud (ADR-072).
//!
//! ## v0 scope
//!
//! - **`anvil capsule create --range <base>..<head> --out <dir>`** —
//!   collect the range (GITGOV-005), the policy/baseline/rules digest
//!   documents (GITGOV-006), and write the capsule directory with a
//!   digest-complete `manifest.json`. Evidence whose collectors land
//!   later (witness chain — GITGOV-007, SARIF diagnostics —
//!   GITGOV-008, applied exceptions — EXCEPT-009) is written
//!   present-but-empty; `verification.json` starts as the degraded
//!   no-checks placeholder, so an unverified capsule never claims
//!   `pass`.
//! - `verify` / `explain` / `inspect` land with GITGOV-009/-010/-011.
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

use std::path::{Path, PathBuf};

use anvil_capsule::{
    CapsuleContent, Producer, ToolIdentity, collect_commits, collect_digests, write_capsule,
};
use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};

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
    /// (ADR-074: on-demand/external by default).
    #[arg(long)]
    out: PathBuf,
}

pub fn run(args: &CapsuleArgs, _global: &GlobalArgs) -> Result<()> {
    match &args.command {
        CapsuleCommand::Create(create) => {
            let repo_root = workspace_root()?;
            run_create(&repo_root, &create.range, &create.out)
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

    let content = CapsuleContent {
        commits,
        digests,
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
    use anvil_capsule::{CapsuleManifest, REQUIRED_FILES};
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
        git(
            dir,
            &[
                "-c",
                "user.name=capsule-test",
                "-c",
                "user.email=capsule@test.invalid",
                "commit",
                "-q",
                "-m",
                message,
            ],
        );
    }

    /// A scratch repo with two commits, an `.anvil.yml` config, and a
    /// policy file. Returns (dir, `base_sha`, `head_sha`).
    fn scratch_repo() -> (tempfile::TempDir, String, String) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        git(root, &["init", "-q"]);

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
}
