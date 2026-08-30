use anvil_checks::conformance::git::{
    GitCommitExtraction, GitEvaluationIdentity, GitExtractionLimits, GitExtractionOutcome,
    GitExtractor, GitFootprintCommitExtraction, GitFootprintExtractionOutcome, GitNonEvaluation,
    GitSelection,
};
use anvil_kernel_types::{
    ClaimKind, EvidenceGrade, GitChangeStatus, GitObjectType, IntentSourceKind, IntentTier,
    ScopeAuthority,
};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
#[cfg(unix)]
use std::time::Instant;
use tempfile::TempDir;

fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf-8 git output")
        .trim()
        .to_owned()
}

#[cfg(unix)]
fn git_with_stdin(repo: &Path, args: &[&str], stdin: &[u8]) -> Vec<u8> {
    use std::io::Write as _;
    use std::process::Stdio;

    let mut child = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run git");
    child
        .stdin
        .take()
        .expect("git stdin")
        .write_all(stdin)
        .expect("write git stdin");
    let output = child.wait_with_output().expect("wait for git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn repository() -> TempDir {
    let repo = tempfile::tempdir().expect("temporary repository");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.name", "CONF test"]);
    git(
        repo.path(),
        &["config", "user.email", "conf-test@example.invalid"],
    );
    repo
}

fn commit_file(repo: &Path, path: &str, contents: &str, message: &str) -> String {
    let full_path = repo.join(path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(full_path, contents).expect("write fixture");
    git(repo, &["add", "--", path]);
    git(repo, &["commit", "-q", "-m", message]);
    git(repo, &["rev-parse", "HEAD"])
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::from("sha256:"), |mut output, byte| {
            use std::fmt::Write as _;
            write!(output, "{byte:02x}").expect("write digest");
            output
        })
}

fn identity(repository: &Path) -> GitEvaluationIdentity {
    GitExtractor::default()
        .identity_for_repository(repository, "run-conf-003")
        .expect("derive fixture identity")
}

fn unbound_identity() -> GitEvaluationIdentity {
    GitEvaluationIdentity {
        run_id: "run-conf-003".to_owned(),
        repository_id: "repo-fixture".to_owned(),
        canonical_worktree_id: "worktree-fixture".to_owned(),
        run_started: std::time::Instant::now(),
    }
}

#[test]
fn claim_agnostic_footprint_accepts_non_conventional_headers_and_preserves_binding() {
    let repo = repository();
    let head = commit_file(
        repo.path(),
        "docs/guide.md",
        "hello\n",
        "Add a guide without a Conventional Commit claim",
    );
    let identity = identity(repo.path());

    let outcome = GitExtractor::default().extract_footprint(
        repo.path(),
        GitSelection::Commit("HEAD".to_owned()),
        &identity,
    );

    let GitFootprintExtractionOutcome::Evaluated(extraction) = outcome else {
        panic!("expected evaluated footprint: {outcome:?}");
    };
    let GitFootprintCommitExtraction::Evaluated(commit) = &extraction.commits[0] else {
        panic!("non-Conventional commit must retain its footprint");
    };
    assert_eq!(commit.commit_revision, head);
    assert_eq!(commit.binding.run_id, identity.run_id);
    assert_eq!(commit.binding.repository_id, identity.repository_id);
    assert_eq!(
        commit.binding.canonical_worktree_id,
        identity.canonical_worktree_id
    );
    assert_eq!(commit.binding.base_revision, extraction.base_revision);
    assert_eq!(commit.binding.head_revision, extraction.head_revision);
    assert_eq!(commit.coverage.len(), 1);
    assert_eq!(commit.coverage[0].new_path, "docs/guide.md");
}

#[test]
fn claim_agnostic_footprint_preserves_identity_and_budget_failures() {
    let repo = repository();
    commit_file(
        repo.path(),
        "docs/guide.md",
        "hello\n",
        "Add a guide without a Conventional Commit claim",
    );
    let identity = identity(repo.path());
    let mut substituted_identity = identity.clone();
    substituted_identity.repository_id = "substituted-repository".to_owned();

    let mismatch = GitExtractor::default().extract_footprint(
        repo.path(),
        GitSelection::Commit("HEAD".to_owned()),
        &substituted_identity,
    );
    let GitFootprintExtractionOutcome::NotEvaluated(mismatch) = mismatch else {
        panic!("substituted identity must not evaluate");
    };
    assert_eq!(mismatch.reason, "identity.repository-mismatch");

    let limits = GitExtractionLimits {
        max_records_per_commit: 0,
        ..GitExtractionLimits::default()
    };
    let budget = GitExtractor::with_limits(limits).extract_footprint(
        repo.path(),
        GitSelection::Commit("HEAD".to_owned()),
        &identity,
    );
    let GitFootprintExtractionOutcome::Evaluated(extraction) = budget else {
        panic!("per-commit footprint failure must retain selected range");
    };
    let GitFootprintCommitExtraction::NotEvaluated(failure) = &extraction.commits[0] else {
        panic!("record budget must fail the retained commit");
    };
    assert_eq!(failure.reason, "budget.records");
    assert_eq!(
        failure.commit_revision.as_deref(),
        Some(extraction.head_revision.as_str())
    );
    assert_eq!(failure.limit, Some(0));
}

#[test]
fn extracts_a_root_docs_claim_into_the_canonical_contract_shape() {
    let repo = repository();
    let head = commit_file(
        repo.path(),
        "docs/guide.md",
        "hello\n",
        "docs(path:docs): add guide",
    );

    let outcome = GitExtractor::default().extract(
        repo.path(),
        GitSelection::Commit("HEAD".to_owned()),
        &identity(repo.path()),
    );

    let GitExtractionOutcome::Evaluated(extraction) = outcome else {
        panic!("expected evaluated extraction: {outcome:?}");
    };
    assert_eq!(extraction.head_revision, head);
    assert_eq!(extraction.commits.len(), 1);

    let GitCommitExtraction::Evaluated(commit) = &extraction.commits[0] else {
        panic!("root commit must be evaluated");
    };
    assert_eq!(commit.commit_revision, head);
    assert_eq!(commit.binding.run_id, "run-conf-003");
    assert!(commit.binding.repository_id.starts_with("sha256:"));
    assert!(commit.binding.canonical_worktree_id.starts_with("sha256:"));
    assert_eq!(commit.source.tier, IntentTier::Tier0);
    assert_eq!(commit.source.kind, IntentSourceKind::ConventionalCommit);
    assert_eq!(commit.source.evidence_grade, EvidenceGrade::Weak);
    assert_eq!(commit.source.reference, head);
    assert!(commit.source.digest.starts_with("sha256:"));
    assert_eq!(commit.claims.len(), 2);
    assert_eq!(commit.claims[0].kind, ClaimKind::FileClass);
    assert_eq!(commit.claims[0].value, "documentation-only");
    assert_eq!(commit.claims[1].kind, ClaimKind::PathPrefix);
    assert_eq!(commit.claims[1].value, "docs");
    assert_eq!(commit.coverage.len(), 1);
    let added = &commit.coverage[0];
    assert_eq!(added.status, GitChangeStatus::Added);
    assert_eq!(added.raw_status, "A");
    assert_eq!(added.rename_score, None);
    assert_eq!(added.old_path, None);
    assert_eq!(added.new_path, "docs/guide.md");
    assert_eq!(added.old_object_type, GitObjectType::Absent);
    assert_eq!(added.new_object_type, GitObjectType::Blob);
}

fn non_evaluation_reason(outcome: GitExtractionOutcome) -> &'static str {
    match outcome {
        GitExtractionOutcome::NotEvaluated(failure) => failure.reason,
        GitExtractionOutcome::Evaluated(extraction) => {
            panic!("expected not-evaluated, got {extraction:?}")
        }
    }
}

fn commit_failure(outcome: GitExtractionOutcome) -> GitNonEvaluation {
    let GitExtractionOutcome::Evaluated(extraction) = outcome else {
        panic!("expected selected commit result: {outcome:?}");
    };
    let mut commits = extraction.commits.into_iter();
    match commits.next().expect("one selected commit") {
        GitCommitExtraction::NotEvaluated(failure) => failure,
        GitCommitExtraction::Evaluated(commit) => {
            panic!("expected commit failure: {commit:?}")
        }
    }
}

fn extract_head(repo: &Path, limits: GitExtractionLimits) -> GitExtractionOutcome {
    GitExtractor::with_limits(limits).extract(
        repo,
        GitSelection::Commit("HEAD".to_owned()),
        &identity(repo),
    )
}

#[test]
fn option_shaped_revision_is_data_not_a_git_option() {
    let repo = repository();
    commit_file(repo.path(), "docs/guide.md", "hello\n", "docs: add guide");

    let outcome = GitExtractor::default().extract(
        repo.path(),
        GitSelection::Commit("--help".to_owned()),
        &identity(repo.path()),
    );

    assert_eq!(non_evaluation_reason(outcome), "revision.invalid");
}

#[test]
fn repository_identity_is_derived_verified_and_opaque() {
    let first = repository();
    commit_file(first.path(), "docs/first.md", "first\n", "docs: add first");
    let second = repository();
    commit_file(
        second.path(),
        "docs/second.md",
        "second\n",
        "docs: add second",
    );
    let extractor = GitExtractor::default();
    let identity = extractor
        .identity_for_repository(first.path(), "run-conf-003")
        .expect("derive repository-bound identity");

    assert!(identity.repository_id.starts_with("sha256:"));
    assert!(identity.canonical_worktree_id.starts_with("sha256:"));
    assert!(
        !identity
            .repository_id
            .contains(&first.path().display().to_string())
    );
    assert!(
        !identity
            .canonical_worktree_id
            .contains(&first.path().display().to_string())
    );

    let outcome = extractor.extract(
        first.path(),
        GitSelection::Commit("HEAD".to_owned()),
        &identity,
    );
    let GitExtractionOutcome::Evaluated(extraction) = outcome else {
        panic!("matching identity must evaluate: {outcome:?}");
    };
    let GitCommitExtraction::Evaluated(commit) = &extraction.commits[0] else {
        panic!("matching identity commit must evaluate");
    };
    assert_eq!(commit.binding.repository_id, identity.repository_id);
    assert_eq!(
        commit.binding.canonical_worktree_id,
        identity.canonical_worktree_id
    );

    let mismatch = extractor.extract(
        second.path(),
        GitSelection::Commit("HEAD".to_owned()),
        &identity,
    );
    let GitExtractionOutcome::NotEvaluated(failure) = mismatch else {
        panic!("cross-repository identity must not evaluate: {mismatch:?}");
    };
    assert_eq!(failure.reason, "identity.repository-mismatch");
    assert_eq!(failure.stage, "identity");

    let nested = first.path().join("docs/nested");
    std::fs::create_dir_all(&nested).expect("create nested repository path");
    let nested_identity = extractor
        .identity_for_repository(&nested, "run-conf-003")
        .expect("derive identity from repository subdirectory");
    assert_eq!(nested_identity.repository_id, identity.repository_id);
    assert_eq!(
        nested_identity.canonical_worktree_id,
        identity.canonical_worktree_id
    );

    let linked_parent = tempfile::tempdir().expect("linked worktree parent");
    let linked = linked_parent.path().join("linked");
    git(
        first.path(),
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "identity-linked",
            linked.to_str().expect("UTF-8 linked worktree path"),
        ],
    );
    let mismatch = extractor.extract(&linked, GitSelection::Commit("HEAD".to_owned()), &identity);
    let GitExtractionOutcome::NotEvaluated(failure) = mismatch else {
        panic!("cross-worktree identity must not evaluate: {mismatch:?}");
    };
    assert_eq!(failure.reason, "identity.worktree-mismatch");
    assert_eq!(failure.stage, "identity");

    let bare = tempfile::tempdir().expect("bare repository parent");
    git(bare.path(), &["init", "-q", "--bare"]);
    let failure = extractor
        .identity_for_repository(bare.path(), "run-conf-003")
        .expect_err("bare repositories have no canonical worktree identity");
    assert_eq!(failure.reason, "repository.bare");
    assert_eq!(failure.stage, "identity");
}

#[cfg(unix)]
#[test]
fn git_executable_resolution_rejects_relative_and_empty_path_entries() {
    let repo = repository();
    commit_file(repo.path(), "docs/guide.md", "guide\n", "docs: add guide");

    for path in [".", ""] {
        let status = Command::new(std::env::current_exe().expect("current test executable"))
            .args(["--exact", "git_executable_resolution_child", "--nocapture"])
            .current_dir(repo.path())
            .env("ANVIL_CONF_GIT_RESOLUTION_REPO", repo.path())
            .env("PATH", path)
            .status()
            .expect("run isolated Git resolution child");
        assert!(status.success(), "PATH={path:?} was not rejected");
    }
}

#[cfg(unix)]
#[test]
fn git_executable_resolution_child() {
    let Some(repository) = std::env::var_os("ANVIL_CONF_GIT_RESOLUTION_REPO") else {
        return;
    };
    let failure = GitExtractor::default()
        .identity_for_repository(Path::new(&repository), "run-conf-003")
        .expect_err("unresolved Git executable must not be invoked by name");
    assert_eq!(failure.reason, "git.executable-not-found");
    assert_eq!(failure.stage, "git");
}

#[test]
fn rejects_replacement_refs_and_legacy_grafts() {
    let replacement_repo = repository();
    let first = commit_file(
        replacement_repo.path(),
        "docs/one.md",
        "one\n",
        "docs: add one",
    );
    let second = commit_file(
        replacement_repo.path(),
        "docs/two.md",
        "two\n",
        "docs: add two",
    );
    git(replacement_repo.path(), &["replace", &first, &second]);
    let replaced = GitExtractor::default().extract(
        replacement_repo.path(),
        GitSelection::Commit("HEAD".to_owned()),
        &unbound_identity(),
    );
    assert_eq!(non_evaluation_reason(replaced), "repository.replace-ref");

    let graft_repo = repository();
    let head = commit_file(
        graft_repo.path(),
        "docs/guide.md",
        "hello\n",
        "docs: add guide",
    );
    let info = graft_repo.path().join(".git/info");
    std::fs::create_dir_all(&info).expect("create Git info directory");
    std::fs::write(info.join("grafts"), format!("{head}\n")).expect("write graft");
    let grafted = GitExtractor::default().extract(
        graft_repo.path(),
        GitSelection::Commit("HEAD".to_owned()),
        &unbound_identity(),
    );
    assert_eq!(non_evaluation_reason(grafted), "repository.graft");
}

#[test]
fn ignores_ambient_git_repository_overrides() {
    let repo = repository();
    commit_file(
        repo.path(),
        "tests/example.rs",
        "test\n",
        "test: add coverage",
    );

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "ambient_git_repository_override_child",
            "--nocapture",
        ])
        .env("ANVIL_CONF_AMBIENT_REPO", repo.path())
        .env("GIT_DIR", "/not/the/repository")
        .env("GIT_OBJECT_DIRECTORY", "/not/the/object-store")
        .env("GIT_ALTERNATE_OBJECT_DIRECTORIES", "/not/an/alternate")
        .env("GIT_REPLACE_REF_BASE", "refs/not-replace/")
        .env("GIT_CONFIG_COUNT", "99")
        .status()
        .expect("run isolated ambient-environment child");
    assert!(status.success(), "ambient-environment child failed");
}

#[test]
fn ambient_git_repository_override_child() {
    let Some(repository) = std::env::var_os("ANVIL_CONF_AMBIENT_REPO") else {
        return;
    };
    let outcome = GitExtractor::default().extract(
        Path::new(&repository),
        GitSelection::Commit("HEAD".to_owned()),
        &identity(Path::new(&repository)),
    );

    let GitExtractionOutcome::Evaluated(extraction) = outcome else {
        panic!("ambient GIT_DIR must be ignored: {outcome:?}");
    };
    let GitCommitExtraction::Evaluated(commit) = &extraction.commits[0] else {
        panic!("ambient extraction commit must be evaluated");
    };
    assert_eq!(commit.claims[0].value, "test-only");
}

#[test]
fn malformed_and_unknown_headers_are_reason_coded() {
    for (message, expected) in [
        ("not conventional", "claim.malformed"),
        ("feat(auth): free form scope", "claim.unknown"),
    ] {
        let repo = repository();
        commit_file(repo.path(), "src/lib.rs", "code\n", message);
        let outcome = GitExtractor::default().extract(
            repo.path(),
            GitSelection::Commit("HEAD".to_owned()),
            &identity(repo.path()),
        );
        let GitExtractionOutcome::Evaluated(extraction) = outcome else {
            panic!("selection itself must be evaluated: {outcome:?}");
        };
        let GitCommitExtraction::NotEvaluated(failure) = &extraction.commits[0] else {
            panic!("commit must be not-evaluated");
        };
        assert_eq!(failure.reason, expected);
    }
}

#[test]
fn base_tree_scope_mapping_authorises_sorted_path_claims() {
    let repo = repository();
    let config = "intent_conformance:\n  scope_mappings:\n    mappings:\n      core:\n        - crates/core\n        - src/core\n    schema_version: 1\n";
    commit_file(
        repo.path(),
        ".anvil.yaml",
        config,
        "docs: establish mapping authority",
    );
    commit_file(
        repo.path(),
        "crates/core/src/lib.rs",
        "pub fn core() {}\n",
        "feat(core): change core",
    );

    let outcome = extract_head(repo.path(), GitExtractionLimits::default());
    let GitExtractionOutcome::Evaluated(extraction) = outcome else {
        panic!("mapping extraction must be selected: {outcome:?}");
    };
    let GitCommitExtraction::Evaluated(commit) = &extraction.commits[0] else {
        panic!("base mapping must authorise the commit");
    };
    assert_eq!(commit.claims.len(), 2);
    assert!(
        commit
            .claims
            .iter()
            .all(|claim| claim.kind == ClaimKind::PathPrefix)
    );
    assert_eq!(commit.claims[0].value, "crates/core");
    assert_eq!(commit.claims[1].value, "src/core");
    let scope = commit.declared_scope.as_ref().expect("declared scope");
    let ScopeAuthority::BaseMapping {
        mapping_key,
        prefixes,
        schema_version,
        source_path,
        source_digest,
    } = &scope.authority
    else {
        panic!(
            "expected base-tree mapping authority: {:?}",
            scope.authority
        );
    };
    assert_eq!(mapping_key, "core");
    assert_eq!(prefixes, &["crates/core", "src/core"]);
    assert_eq!(*schema_version, 1);
    assert_eq!(source_path, ".anvil.yaml");
    assert_eq!(
        source_digest,
        &sha256(
            br#"{"intent_conformance":{"scope_mappings":{"mappings":{"core":["crates/core","src/core"]},"schema_version":1}}}"#
        )
    );
    assert_eq!(commit.contributing_base_config_paths, [".anvil.yaml"]);
}

#[test]
fn contributing_base_config_paths_do_not_depend_on_using_the_mapping() {
    for (message, expected_claim) in [
        ("docs(path:docs): explicit path", "docs"),
        (
            "docs(other): file class with unmapped scope",
            "documentation-only",
        ),
    ] {
        let repo = repository();
        commit_file(
            repo.path(),
            ".anvil.toml",
            "[intent_conformance.scope_mappings]\nschema_version = 1\n[intent_conformance.scope_mappings.mappings]\ncore = [\"crates/core\"]\n",
            "docs: establish mapping authority",
        );
        commit_file(repo.path(), "docs/guide.md", "guide\n", message);

        let outcome = extract_head(repo.path(), GitExtractionLimits::default());
        let GitExtractionOutcome::Evaluated(extraction) = outcome else {
            panic!("configured extraction must be selected: {outcome:?}");
        };
        let GitCommitExtraction::Evaluated(commit) = &extraction.commits[0] else {
            panic!("claim must evaluate independently of mapping use");
        };
        assert!(
            commit
                .claims
                .iter()
                .any(|claim| claim.value == expected_claim),
            "missing expected claim for {message}"
        );
        assert_eq!(commit.contributing_base_config_paths, [".anvil.toml"]);
    }
}

#[test]
fn mapping_added_only_in_head_cannot_authorise_that_head() {
    let repo = repository();
    commit_file(
        repo.path(),
        "docs/base.md",
        "base\n",
        "docs: establish unconfigured base",
    );
    let config = "intent_conformance:\n  scope_mappings:\n    schema_version: 1\n    mappings:\n      core: [crates/core]\n";
    let config_path = repo.path().join(".anvil.yaml");
    std::fs::write(&config_path, config).expect("write head-only config");
    let core_path = repo.path().join("crates/core/src/lib.rs");
    std::fs::create_dir_all(core_path.parent().expect("core parent")).expect("create core parent");
    std::fs::write(core_path, "pub fn core() {}\n").expect("write core fixture");
    git(
        repo.path(),
        &["add", "--", ".anvil.yaml", "crates/core/src/lib.rs"],
    );
    git(repo.path(), &["commit", "-q", "-m", "feat(core): add core"]);

    let failure = commit_failure(extract_head(repo.path(), GitExtractionLimits::default()));
    assert_eq!(failure.reason, "claim.unknown");
}

#[test]
fn malformed_or_non_canonical_base_mappings_are_reason_coded() {
    let cases = [
        ("malformed", "intent_conformance:\n  scope_mappings: [\n"),
        (
            "unversioned",
            "intent_conformance:\n  scope_mappings:\n    mappings:\n      core: [crates/core]\n",
        ),
        (
            "duplicate",
            "intent_conformance:\n  scope_mappings:\n    schema_version: 1\n    mappings:\n      core: [crates/core, crates/core]\n",
        ),
        (
            "unsorted",
            "intent_conformance:\n  scope_mappings:\n    schema_version: 1\n    mappings:\n      core: [src/core, crates/core]\n",
        ),
        (
            "wildcard",
            "intent_conformance:\n  scope_mappings:\n    schema_version: 1\n    mappings:\n      core: [crates/*]\n",
        ),
        (
            "invalid",
            "intent_conformance:\n  scope_mappings:\n    schema_version: 1\n    mappings:\n      core: [/crates/core]\n",
        ),
        (
            "unknown-key",
            "intent_conformance:\n  scope_mappings:\n    schema_version: 1\n    mappings:\n      core: [crates/core]\n    future: true\n",
        ),
    ];

    for (case, config) in cases {
        let repo = repository();
        commit_file(
            repo.path(),
            ".anvil.yaml",
            config,
            "docs: establish invalid mapping",
        );
        commit_file(
            repo.path(),
            "crates/core/src/lib.rs",
            "pub fn core() {}\n",
            "feat(core): change core",
        );

        let failure = commit_failure(extract_head(repo.path(), GitExtractionLimits::default()));
        assert_eq!(failure.reason, "claim.scope-mapping-invalid", "case {case}");
    }
}

#[test]
fn range_is_ancestry_checked_oid_sorted_and_keeps_per_commit_footprints() {
    let repo = repository();
    let base = commit_file(
        repo.path(),
        "docs/guide.md",
        "one\n",
        "docs(path:docs): add guide",
    );
    let second = commit_file(
        repo.path(),
        "docs/guide.md",
        "two\n",
        "docs(path:docs): update guide",
    );
    std::fs::rename(
        repo.path().join("docs/guide.md"),
        repo.path().join("docs/renamed.md"),
    )
    .expect("rename fixture");
    git(repo.path(), &["add", "-A"]);
    git(
        repo.path(),
        &["commit", "-q", "-m", "docs(path:docs): rename guide"],
    );
    let head = git(repo.path(), &["rev-parse", "HEAD"]);

    let outcome = GitExtractor::default().extract(
        repo.path(),
        GitSelection::Range {
            base: base.clone(),
            head: head.clone(),
        },
        &identity(repo.path()),
    );
    let GitExtractionOutcome::Evaluated(extraction) = outcome else {
        panic!("expected evaluated range: {outcome:?}");
    };
    assert_eq!(extraction.base_revision, base);
    assert_eq!(extraction.head_revision, head);
    let mut expected = vec![second, extraction.head_revision.clone()];
    expected.sort();
    let actual: Vec<_> = extraction
        .commits
        .iter()
        .map(|commit| match commit {
            GitCommitExtraction::Evaluated(commit) => commit.commit_revision.clone(),
            GitCommitExtraction::NotEvaluated(failure) => failure
                .commit_revision
                .clone()
                .expect("selected commit identity")
                .into(),
        })
        .collect();
    assert_eq!(actual, expected);
    let rename = extraction
        .commits
        .iter()
        .find_map(|commit| match commit {
            GitCommitExtraction::Evaluated(commit)
                if commit.commit_revision == extraction.head_revision =>
            {
                Some(&commit.coverage[0])
            }
            _ => None,
        })
        .expect("rename commit");
    assert_eq!(rename.status, GitChangeStatus::Renamed);
    assert_eq!(rename.raw_status, "R100");
    assert_eq!(rename.rename_score, Some(100));
    assert_eq!(rename.old_path.as_deref(), Some("docs/guide.md"));
    assert_eq!(rename.new_path, "docs/renamed.md");
}

#[allow(clippy::too_many_lines)]
#[test]
fn deterministic_extraction_budgets_are_reason_coded_and_keep_the_commit() {
    let repo = repository();
    let base = commit_file(
        repo.path(),
        "docs/one.md",
        "one\n",
        "docs(path:docs): add one",
    );
    commit_file(
        repo.path(),
        "docs/two.md",
        "two\n",
        "docs(path:docs): add two",
    );
    commit_file(
        repo.path(),
        "docs/three.md",
        "three\n",
        "docs(path:docs): add three",
    );
    let head = git(repo.path(), &["rev-parse", "HEAD"]);

    let mut limits = GitExtractionLimits {
        max_commits: 1,
        ..GitExtractionLimits::default()
    };
    let too_many_commits = GitExtractor::with_limits(limits.clone()).extract(
        repo.path(),
        GitSelection::Range {
            base: base.clone(),
            head: head.clone(),
        },
        &identity(repo.path()),
    );
    let GitExtractionOutcome::NotEvaluated(too_many_commits) = too_many_commits else {
        panic!("commit budget must fail at selection");
    };
    assert_eq!(too_many_commits.reason, "budget.commits");
    let commit_budget = too_many_commits.budget.expect("commit budget diagnostics");
    assert_eq!(commit_budget.configured_limit, 1);
    assert_eq!(commit_budget.commits, Some(2));
    assert!(
        commit_budget
            .raw_output_digest
            .as_deref()
            .is_some_and(|digest| digest.starts_with("sha256:"))
    );

    for index in 4..=6 {
        commit_file(
            repo.path(),
            &format!("docs/{index}.md"),
            &format!("{index}\n"),
            "docs(path:docs): extend range",
        );
    }
    let overflow_head = git(repo.path(), &["rev-parse", "HEAD"]);
    limits = GitExtractionLimits {
        max_commits: 0,
        ..GitExtractionLimits::default()
    };
    let overflow = GitExtractor::with_limits(limits.clone()).extract(
        repo.path(),
        GitSelection::Range {
            base,
            head: overflow_head,
        },
        &identity(repo.path()),
    );
    let GitExtractionOutcome::NotEvaluated(overflow) = overflow else {
        panic!("revision-list byte guard must preserve commit budget semantics");
    };
    assert_eq!(overflow.reason, "budget.commits");
    assert_eq!(overflow.observed, 3);
    assert_eq!(overflow.limit, Some(0));
    let commit_budget = overflow.budget.expect("overflow commit diagnostics");
    assert_eq!(commit_budget.configured_limit, 0);
    assert_eq!(commit_budget.commits, Some(3));
    assert!(commit_budget.raw_bytes.is_some_and(|bytes| bytes > 129));

    limits = GitExtractionLimits {
        max_records_per_commit: 0,
        ..GitExtractionLimits::default()
    };
    let records = commit_failure(extract_head(repo.path(), limits));
    assert_eq!(records.reason, "budget.records");
    let record_budget = records.budget.expect("record budget diagnostics");
    assert_eq!(record_budget.configured_limit, 0);
    assert_eq!(record_budget.records, Some(0));
    assert!(record_budget.raw_bytes.is_some_and(|bytes| bytes > 0));
    assert!(record_budget.raw_output_digest.is_some());

    limits = GitExtractionLimits {
        max_raw_bytes_per_commit: 1,
        ..GitExtractionLimits::default()
    };
    let raw = commit_failure(extract_head(repo.path(), limits));
    assert_eq!(raw.reason, "budget.raw-bytes");
    assert!(
        raw.raw_digest
            .as_deref()
            .is_some_and(|digest| digest.starts_with("sha256:"))
    );
    let raw_budget = raw.budget.expect("raw-byte budget diagnostics");
    assert_eq!(raw_budget.configured_limit, 1);
    assert!(raw_budget.raw_bytes.is_some_and(|bytes| bytes > 1));
    assert!(raw_budget.raw_output_digest.is_some());

    limits = GitExtractionLimits {
        max_decoded_bytes_per_commit: 1,
        ..GitExtractionLimits::default()
    };
    let decoded = commit_failure(extract_head(repo.path(), limits));
    assert_eq!(decoded.reason, "budget.decoded-bytes");
    let decoded_budget = decoded.budget.expect("decoded-byte budget diagnostics");
    assert_eq!(decoded_budget.configured_limit, 1);
    assert!(decoded_budget.decoded_bytes.is_some_and(|bytes| bytes > 1));
    assert_eq!(decoded_budget.records, Some(1));
    assert!(decoded_budget.raw_output_digest.is_some());

    let rename_repo = repository();
    commit_file(
        rename_repo.path(),
        "docs/old.md",
        "content\n",
        "docs(path:docs): add old",
    );
    std::fs::rename(
        rename_repo.path().join("docs/old.md"),
        rename_repo.path().join("docs/new.md"),
    )
    .expect("rename budget fixture");
    git(rename_repo.path(), &["add", "-A"]);
    git(
        rename_repo.path(),
        &["commit", "-q", "-m", "docs(path:docs): rename"],
    );
    limits = GitExtractionLimits {
        max_rename_sources: 0,
        max_rename_targets: 0,
        ..GitExtractionLimits::default()
    };
    let rename = commit_failure(extract_head(rename_repo.path(), limits));
    assert_eq!(rename.reason, "budget.rename-candidates");
    let rename_budget = rename.budget.expect("rename budget diagnostics");
    assert_eq!(rename_budget.configured_limit, 0);
    assert_eq!(rename_budget.records, Some(2));
    assert_eq!(rename_budget.rename_sources, Some(1));
    assert_eq!(rename_budget.rename_targets, Some(1));
    assert!(rename_budget.raw_bytes.is_some_and(|bytes| bytes > 0));
    assert!(rename_budget.raw_output_digest.is_some());
}

#[test]
fn elapsed_time_budgets_are_reason_coded() {
    let repo = repository();
    commit_file(repo.path(), "docs/one.md", "one\n", "docs: add one");

    let mut limits = GitExtractionLimits {
        git_timeout: Duration::ZERO,
        ..GitExtractionLimits::default()
    };
    assert_eq!(
        non_evaluation_reason(extract_head(repo.path(), limits)),
        "budget.git-timeout"
    );

    limits = GitExtractionLimits {
        run_timeout: Duration::ZERO,
        ..GitExtractionLimits::default()
    };
    assert_eq!(
        non_evaluation_reason(extract_head(repo.path(), limits)),
        "budget.run-timeout"
    );
}

#[cfg(unix)]
fn real_git_program() -> std::path::PathBuf {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|directory| directory.join("git"))
        .find(|candidate| candidate.is_file())
        .expect("Git executable on PATH")
}

#[cfg(unix)]
fn delayed_git_wrapper(directory: &Path, delayed_stage: &str) {
    use std::os::unix::fs::PermissionsExt;

    let real_git = real_git_program();
    let trigger = match delayed_stage {
        "revision" => " rev-parse --verify ",
        "ancestry" => " merge-base --is-ancestor ",
        _ => panic!("unknown delayed Git stage"),
    };
    let script = format!(
        "#!/bin/sh\ncase \" $* \" in\n  *'{trigger}'*) /bin/sleep 1 ;;\nesac\nexec '{}' \"$@\"\n",
        real_git.display()
    );
    let path = directory.join("git");
    std::fs::write(&path, script).expect("write delayed Git wrapper");
    let mut permissions = std::fs::metadata(&path)
        .expect("wrapper metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("make Git wrapper executable");
}

#[cfg(unix)]
fn shared_run_timeout_git_wrapper(directory: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let real_git = real_git_program();
    let marker = directory.join("identity-delayed");
    let script = format!(
        "#!/bin/sh\ncase \" $* \" in\n  *' rev-parse --path-format=absolute --show-toplevel '*)\n    if [ ! -e '{}' ]; then : > '{}'; /bin/sleep 0.4; fi\n    ;;\n  *' rev-parse --verify --end-of-options '*) /bin/sleep 0.4 ;;\nesac\nexec '{}' \"$@\"\n",
        marker.display(),
        marker.display(),
        real_git.display()
    );
    let path = directory.join("git");
    std::fs::write(&path, script).expect("write shared timeout Git wrapper");
    let mut permissions = std::fs::metadata(&path)
        .expect("wrapper metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("make Git wrapper executable");
}

#[cfg(unix)]
fn scripted_diff_git_wrapper(directory: &Path, mode: &str) {
    use std::os::unix::fs::PermissionsExt;

    let real_git = real_git_program();
    let raw_record = format!(
        ":000000 100644 {} {} A\\000docs/partial.md\\000",
        "0".repeat(40),
        "1".repeat(40)
    );
    let script = format!(
        "#!/bin/sh\ncase \" $* \" in\n  *' diff-tree '*' --no-renames '*)\n    if [ '{mode}' = preflight-timeout ]; then printf '%b' '{raw_record}'; /bin/sleep 1; exit 0; fi\n    ;;\n  *' diff-tree '*)\n    case '{mode}' in\n      final-timeout) printf '%b' '{raw_record}'; /bin/sleep 1; exit 0 ;;\n      endpoint-mismatch) printf '%b' '{raw_record}'; exit 0 ;;\n    esac\n    ;;\nesac\nexec '{}' \"$@\"\n",
        real_git.display()
    );
    let path = directory.join("git");
    std::fs::write(&path, script).expect("write scripted Git wrapper");
    let mut permissions = std::fs::metadata(&path)
        .expect("wrapper metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("make Git wrapper executable");
}

#[cfg(unix)]
fn descendant_holding_pipe_git_wrapper(directory: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let real_git = real_git_program();
    let script = format!(
        "#!/bin/sh\ncase \" $* \" in\n  *' diff-tree '*' --no-renames '*)\n    /bin/sleep 10 &\n    /bin/sleep 10\n    ;;\nesac\nexec '{}' \"$@\"\n",
        real_git.display()
    );
    let path = directory.join("git");
    std::fs::write(&path, script).expect("write descendant Git wrapper");
    let mut permissions = std::fs::metadata(&path)
        .expect("wrapper metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("make Git wrapper executable");
}

#[cfg(unix)]
fn nested_repository_race_git_wrapper(directory: &Path, nested: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let real_git = real_git_program();
    let script = format!(
        "#!/bin/sh\ncase \" $* \" in\n  *' rev-parse --path-format=absolute --show-toplevel '*)\n    '{}' \"$@\"\n    status=$?\n    '{}' -C '{}' init -q\n    exit $status\n    ;;\nesac\nexec '{}' \"$@\"\n",
        real_git.display(),
        real_git.display(),
        nested.display(),
        real_git.display()
    );
    let path = directory.join("git");
    std::fs::write(&path, script).expect("write nested-repository Git wrapper");
    let mut permissions = std::fs::metadata(&path)
        .expect("wrapper metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("make Git wrapper executable");
}

#[cfg(unix)]
#[test]
fn verified_canonical_worktree_remains_the_command_boundary() {
    let repo = repository();
    commit_file(repo.path(), "docs/base.md", "base\n", "docs: add base");
    let nested = repo.path().join("docs/nested");
    std::fs::create_dir_all(&nested).expect("create nested path");
    let expected = identity(repo.path());
    let wrapper = tempfile::tempdir().expect("wrapper directory");
    nested_repository_race_git_wrapper(wrapper.path(), &nested);

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "canonical_worktree_boundary_child",
            "--nocapture",
        ])
        .env("ANVIL_CONF_CANONICAL_REPO", &nested)
        .env("ANVIL_CONF_REPOSITORY_ID", &expected.repository_id)
        .env("ANVIL_CONF_WORKTREE_ID", &expected.canonical_worktree_id)
        .env("PATH", wrapper.path())
        .status()
        .expect("run canonical-worktree boundary child");
    assert!(
        status.success(),
        "canonical worktree boundary was not retained"
    );
}

#[cfg(unix)]
#[test]
fn canonical_worktree_boundary_child() {
    let Some(repository) = std::env::var_os("ANVIL_CONF_CANONICAL_REPO") else {
        return;
    };
    let expected = GitEvaluationIdentity {
        run_id: "run-conf-003".to_owned(),
        repository_id: std::env::var("ANVIL_CONF_REPOSITORY_ID").expect("repository identity"),
        canonical_worktree_id: std::env::var("ANVIL_CONF_WORKTREE_ID").expect("worktree identity"),
        run_started: Instant::now(),
    };
    let outcome = GitExtractor::default().extract(
        Path::new(&repository),
        GitSelection::Commit("HEAD".into()),
        &expected,
    );
    let GitExtractionOutcome::Evaluated(extraction) = outcome else {
        panic!("verified canonical worktree must remain authoritative: {outcome:?}");
    };
    assert_eq!(extraction.commits.len(), 1);
}

#[cfg(unix)]
#[test]
fn timeout_terminates_descendants_that_retain_git_output_pipes() {
    let repo = repository();
    commit_file(repo.path(), "docs/base.md", "base\n", "docs: add base");
    commit_file(repo.path(), "docs/head.md", "head\n", "docs: add head");
    let wrapper = tempfile::tempdir().expect("wrapper directory");
    descendant_holding_pipe_git_wrapper(wrapper.path());

    let started = Instant::now();
    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "descendant_pipe_timeout_child", "--nocapture"])
        .env("ANVIL_CONF_DESCENDANT_REPO", repo.path())
        .env("PATH", wrapper.path())
        .status()
        .expect("run descendant timeout child");
    assert!(status.success(), "descendant timeout invariant failed");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "descendant retaining a pipe blocked extraction for {:?}",
        started.elapsed()
    );
}

#[cfg(unix)]
#[test]
fn descendant_pipe_timeout_child() {
    let Some(repository) = std::env::var_os("ANVIL_CONF_DESCENDANT_REPO") else {
        return;
    };
    let limits = GitExtractionLimits {
        git_timeout: Duration::from_millis(100),
        run_timeout: Duration::from_secs(2),
        ..GitExtractionLimits::default()
    };
    let failure = commit_failure(GitExtractor::with_limits(limits).extract(
        Path::new(&repository),
        GitSelection::Commit("HEAD".into()),
        &identity(Path::new(&repository)),
    ));
    assert_eq!(failure.reason, "budget.git-timeout");
    assert_eq!(failure.stage, "diff-preflight");
}

#[cfg(unix)]
#[test]
fn diff_timeouts_report_stage_local_complete_records_and_endpoints_must_match() {
    let repo = repository();
    commit_file(repo.path(), "docs/base.md", "base\n", "docs: add base");
    commit_file(repo.path(), "docs/head.md", "head\n", "docs: add head");

    for mode in ["preflight-timeout", "final-timeout", "endpoint-mismatch"] {
        let wrapper = tempfile::tempdir().expect("wrapper directory");
        scripted_diff_git_wrapper(wrapper.path(), mode);
        let status = Command::new(std::env::current_exe().expect("current test executable"))
            .args(["--exact", "scripted_diff_failure_child", "--nocapture"])
            .env("ANVIL_CONF_DIFF_REPO", repo.path())
            .env("ANVIL_CONF_DIFF_MODE", mode)
            .env("PATH", wrapper.path())
            .status()
            .expect("run isolated diff child");
        assert!(status.success(), "{mode} invariant was not enforced");
    }
}

#[cfg(unix)]
#[test]
fn scripted_diff_failure_child() {
    let Some(repository) = std::env::var_os("ANVIL_CONF_DIFF_REPO") else {
        return;
    };
    let mode = std::env::var("ANVIL_CONF_DIFF_MODE").expect("diff wrapper mode");
    let limits = GitExtractionLimits {
        git_timeout: Duration::from_millis(100),
        run_timeout: Duration::from_secs(2),
        ..GitExtractionLimits::default()
    };
    let failure = commit_failure(GitExtractor::with_limits(limits).extract(
        Path::new(&repository),
        GitSelection::Commit("HEAD".into()),
        &identity(Path::new(&repository)),
    ));

    if mode == "endpoint-mismatch" {
        assert_eq!(failure.reason, "git.endpoint-mismatch");
        assert_eq!(failure.stage, "diff");
        return;
    }

    assert_eq!(failure.reason, "budget.git-timeout");
    assert_eq!(
        failure.stage,
        if mode == "preflight-timeout" {
            "diff-preflight"
        } else {
            "diff"
        }
    );
    let diagnostics = failure.budget.expect("timeout budget diagnostics");
    assert_eq!(diagnostics.records, Some(1));
    assert_eq!(diagnostics.decoded_bytes, Some("docs/partial.md".len()));
}

#[cfg(unix)]
#[test]
fn revision_and_ancestry_preserve_git_and_run_timeout_reasons_and_stages() {
    let repo = repository();
    let base = commit_file(repo.path(), "docs/base.md", "base\n", "docs: add base");
    let head = commit_file(repo.path(), "docs/head.md", "head\n", "docs: add head");

    for (budget, stage) in [
        ("git", "revision"),
        ("run", "revision"),
        ("git", "ancestry"),
        ("run", "ancestry"),
    ] {
        let wrapper = tempfile::tempdir().expect("wrapper directory");
        delayed_git_wrapper(wrapper.path(), stage);
        let status = Command::new(std::env::current_exe().expect("current test executable"))
            .args(["--exact", "stage_specific_timeout_child", "--nocapture"])
            .env("ANVIL_CONF_TIMEOUT_REPO", repo.path())
            .env("ANVIL_CONF_TIMEOUT_BASE", &base)
            .env("ANVIL_CONF_TIMEOUT_HEAD", &head)
            .env("ANVIL_CONF_TIMEOUT_BUDGET", budget)
            .env("ANVIL_CONF_TIMEOUT_STAGE", stage)
            .env("PATH", wrapper.path())
            .status()
            .expect("run isolated timeout child");
        assert!(
            status.success(),
            "{budget} timeout at {stage} lost its reason"
        );
    }
}

#[cfg(unix)]
#[test]
fn stage_specific_timeout_child() {
    let Some(repository) = std::env::var_os("ANVIL_CONF_TIMEOUT_REPO") else {
        return;
    };
    let budget = std::env::var("ANVIL_CONF_TIMEOUT_BUDGET").expect("timeout budget");
    let stage = std::env::var("ANVIL_CONF_TIMEOUT_STAGE").expect("timeout stage");
    let limits = GitExtractionLimits {
        git_timeout: if budget == "git" {
            Duration::from_millis(100)
        } else {
            Duration::from_secs(2)
        },
        run_timeout: if budget == "run" {
            Duration::from_millis(500)
        } else {
            Duration::from_secs(2)
        },
        ..GitExtractionLimits::default()
    };
    let selection = if stage == "revision" {
        GitSelection::Commit("HEAD".to_owned())
    } else {
        GitSelection::Range {
            base: std::env::var("ANVIL_CONF_TIMEOUT_BASE").expect("timeout base"),
            head: std::env::var("ANVIL_CONF_TIMEOUT_HEAD").expect("timeout head"),
        }
    };
    let failure = match GitExtractor::with_limits(limits).extract(
        Path::new(&repository),
        selection,
        &identity(Path::new(&repository)),
    ) {
        GitExtractionOutcome::NotEvaluated(failure) => failure,
        outcome @ GitExtractionOutcome::Evaluated(_) => {
            panic!("expected stage timeout, got {outcome:?}")
        }
    };
    assert_eq!(
        failure.reason,
        if budget == "git" {
            "budget.git-timeout"
        } else {
            "budget.run-timeout"
        }
    );
    assert_eq!(failure.stage, stage);
    let diagnostics = failure.budget.expect("timeout budget diagnostics");
    let expected_limit = if budget == "git" { 100 } else { 500 };
    assert_eq!(diagnostics.configured_limit, expected_limit);
    assert!(
        diagnostics
            .elapsed_millis
            .is_some_and(|elapsed| elapsed >= expected_limit)
    );
    assert!(diagnostics.raw_bytes.is_some());
    assert!(diagnostics.raw_output_digest.is_some());
}

#[cfg(unix)]
#[test]
fn identity_and_extraction_share_one_run_timeout() {
    let repo = repository();
    commit_file(repo.path(), "docs/base.md", "base\n", "docs: add base");
    let wrapper = tempfile::tempdir().expect("wrapper directory");
    shared_run_timeout_git_wrapper(wrapper.path());

    let status = Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "shared_run_timeout_child", "--nocapture"])
        .env("ANVIL_CONF_SHARED_TIMEOUT_REPO", repo.path())
        .env("PATH", wrapper.path())
        .status()
        .expect("run shared timeout child");
    assert!(
        status.success(),
        "identity and extraction reset the run budget"
    );
}

#[cfg(unix)]
#[test]
fn shared_run_timeout_child() {
    let Some(repository) = std::env::var_os("ANVIL_CONF_SHARED_TIMEOUT_REPO") else {
        return;
    };
    let extractor = GitExtractor::with_limits(GitExtractionLimits {
        git_timeout: Duration::from_secs(2),
        run_timeout: Duration::from_millis(600),
        ..GitExtractionLimits::default()
    });
    let identity = extractor
        .identity_for_repository(Path::new(&repository), "run-conf-011")
        .expect("identity stage remains within the shared budget");
    let outcome = extractor.extract_footprint(
        Path::new(&repository),
        GitSelection::Commit("HEAD".to_owned()),
        &identity,
    );
    let GitFootprintExtractionOutcome::NotEvaluated(failure) = outcome else {
        panic!("combined identity and extraction must exhaust one run budget: {outcome:?}");
    };
    assert_eq!(failure.reason, "budget.run-timeout");
    assert_eq!(failure.stage, "revision");
}

#[test]
fn merge_commit_footprint_is_against_its_first_parent_only() {
    let repo = repository();
    commit_file(
        repo.path(),
        "docs/base.md",
        "base\n",
        "docs(path:docs): add base",
    );
    let main_branch = git(repo.path(), &["symbolic-ref", "--short", "HEAD"]);
    git(repo.path(), &["checkout", "-q", "-b", "side"]);
    commit_file(
        repo.path(),
        "docs/side.md",
        "side\n",
        "docs(path:docs): add side",
    );
    git(repo.path(), &["checkout", "-q", &main_branch]);
    commit_file(
        repo.path(),
        "docs/main.md",
        "main\n",
        "docs(path:docs): add main",
    );
    git(
        repo.path(),
        &[
            "merge",
            "-q",
            "--no-ff",
            "side",
            "-m",
            "docs(path:docs): merge side",
        ],
    );

    let outcome = extract_head(repo.path(), GitExtractionLimits::default());
    let GitExtractionOutcome::Evaluated(extraction) = outcome else {
        panic!("merge extraction must be selected: {outcome:?}");
    };
    let GitCommitExtraction::Evaluated(commit) = &extraction.commits[0] else {
        panic!("merge commit must be evaluated");
    };
    let paths: Vec<_> = commit
        .coverage
        .iter()
        .map(|member| member.new_path.as_str())
        .collect();
    assert_eq!(paths, ["docs/side.md"]);
}

#[cfg(unix)]
#[test]
fn preserves_added_deleted_modified_renamed_and_type_changed_raw_records() {
    let repo = repository();
    for path in [
        "docs/modify.md",
        "docs/delete.md",
        "docs/rename.md",
        "docs/type.md",
    ] {
        let full_path = repo.path().join(path);
        std::fs::create_dir_all(full_path.parent().expect("fixture parent"))
            .expect("create fixture parent");
        std::fs::write(full_path, format!("{path}\n")).expect("write fixture");
    }
    git(repo.path(), &["add", "-A"]);
    git(
        repo.path(),
        &["commit", "-q", "-m", "docs(path:docs): add fixtures"],
    );

    std::fs::write(repo.path().join("docs/modify.md"), "changed\n").expect("modify fixture");
    std::fs::remove_file(repo.path().join("docs/delete.md")).expect("delete fixture");
    std::fs::rename(
        repo.path().join("docs/rename.md"),
        repo.path().join("docs/renamed.md"),
    )
    .expect("rename fixture");
    std::fs::remove_file(repo.path().join("docs/type.md")).expect("replace type fixture");
    std::os::unix::fs::symlink("modify.md", repo.path().join("docs/type.md"))
        .expect("create symlink fixture");
    std::fs::write(repo.path().join("docs/added.md"), "added\n").expect("add fixture");
    git(repo.path(), &["add", "-A"]);
    git(
        repo.path(),
        &["commit", "-q", "-m", "docs(path:docs): change fixtures"],
    );

    let outcome = extract_head(repo.path(), GitExtractionLimits::default());
    let GitExtractionOutcome::Evaluated(extraction) = outcome else {
        panic!("status extraction must be selected: {outcome:?}");
    };
    let GitCommitExtraction::Evaluated(commit) = &extraction.commits[0] else {
        panic!("status commit must be evaluated");
    };
    let status_for = |path: &str| {
        commit
            .coverage
            .iter()
            .find(|member| member.new_path == path)
            .map(|member| (member.status, member.raw_status.as_str()))
            .expect("coverage path")
    };
    assert_eq!(status_for("docs/added.md"), (GitChangeStatus::Added, "A"));
    assert_eq!(
        status_for("docs/delete.md"),
        (GitChangeStatus::Deleted, "D")
    );
    assert_eq!(
        status_for("docs/modify.md"),
        (GitChangeStatus::Modified, "M")
    );
    assert_eq!(
        status_for("docs/type.md"),
        (GitChangeStatus::TypeChanged, "T")
    );
    let renamed = commit
        .coverage
        .iter()
        .find(|member| member.new_path == "docs/renamed.md")
        .expect("rename coverage");
    assert_eq!(renamed.status, GitChangeStatus::Renamed);
    assert_eq!(renamed.raw_status, "R100");
    assert_eq!(renamed.rename_score, Some(100));
}

#[test]
fn preserves_gitlink_object_identity_and_type() {
    let submodule = repository();
    let old_object = commit_file(submodule.path(), "one.txt", "one\n", "test: add one");
    let new_object = commit_file(submodule.path(), "two.txt", "two\n", "test: add two");

    let repo = repository();
    commit_file(repo.path(), "docs/base.md", "base\n", "docs: add base");
    git(
        repo.path(),
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            "160000",
            &old_object,
            "vendor/sub",
        ],
    );
    git(
        repo.path(),
        &["commit", "-q", "-m", "docs(path:vendor): add gitlink"],
    );
    git(
        repo.path(),
        &[
            "update-index",
            "--cacheinfo",
            "160000",
            &new_object,
            "vendor/sub",
        ],
    );
    git(
        repo.path(),
        &["commit", "-q", "-m", "docs(path:vendor): update gitlink"],
    );

    let outcome = extract_head(repo.path(), GitExtractionLimits::default());
    let GitExtractionOutcome::Evaluated(extraction) = outcome else {
        panic!("gitlink extraction must be selected: {outcome:?}");
    };
    let GitCommitExtraction::Evaluated(commit) = &extraction.commits[0] else {
        panic!("gitlink commit must be evaluated");
    };
    let member = &commit.coverage[0];
    assert_eq!(member.status, GitChangeStatus::Modified);
    assert_eq!(member.old_object_type, GitObjectType::Gitlink);
    assert_eq!(member.new_object_type, GitObjectType::Gitlink);
    assert_eq!(member.old_object, old_object);
    assert_eq!(member.new_object, new_object);
}

#[cfg(unix)]
#[test]
fn invalid_utf8_path_is_reason_coded_not_evaluated() {
    let repo = repository();
    let blob = String::from_utf8(git_with_stdin(
        repo.path(),
        &["hash-object", "-w", "--stdin"],
        b"invalid path\n",
    ))
    .expect("blob id")
    .trim()
    .to_owned();
    let mut tree_entry = format!("100644 blob {blob}\t").into_bytes();
    tree_entry.extend_from_slice(b"bad-\xff.md\0");
    let tree = String::from_utf8(git_with_stdin(repo.path(), &["mktree", "-z"], &tree_entry))
        .expect("tree id")
        .trim()
        .to_owned();
    let commit = git(
        repo.path(),
        &["commit-tree", &tree, "-m", "docs(path:docs): invalid path"],
    );
    git(repo.path(), &["update-ref", "HEAD", &commit]);

    let failure = commit_failure(extract_head(repo.path(), GitExtractionLimits::default()));
    assert_eq!(failure.reason, "git.path-invalid-utf8");
}

#[test]
fn shallow_repository_is_reason_coded_not_evaluated() {
    let source = repository();
    commit_file(source.path(), "docs/one.md", "one\n", "docs: add one");
    commit_file(source.path(), "docs/two.md", "two\n", "docs: add two");

    let outer = tempfile::tempdir().expect("clone parent");
    let clone = outer.path().join("shallow");
    let source_url = format!("file://{}", source.path().display());
    let output = Command::new("git")
        .args(["clone", "-q", "--depth=1", &source_url])
        .arg(&clone)
        .output()
        .expect("create shallow clone");
    assert!(
        output.status.success(),
        "shallow clone failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let outcome = GitExtractor::default().extract(
        &clone,
        GitSelection::Commit("HEAD".to_owned()),
        &unbound_identity(),
    );
    assert_eq!(non_evaluation_reason(outcome), "repository.shallow");
}
