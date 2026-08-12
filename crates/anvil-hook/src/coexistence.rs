//! Hook-coexistence planning primitives (ADOPT-001).
//!
//! Pure functions that decide WHAT to write so Anvil's hooks fire
//! alongside a host hook manager (Husky / Lefthook / pre-commit
//! framework) instead of overwriting `.git/hooks/`.
//!
//! Detection precedence is owned by [`crate::detect_framework`];
//! this module assumes the framework has already been identified
//! and returns the install / uninstall plan for that framework.

use crate::framework::HookFramework;
use crate::shell::HookKind;

pub const MARKER_BEGIN: &str = "# >>> anvil-managed (do not edit) >>>";
pub const MARKER_END: &str = "# <<< anvil-managed <<<";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoexistenceFile {
    pub relative_path: String,
    pub initial_content: String,
    pub block: String,
    pub executable: bool,
    /// When true, Anvil owns the whole file. Install writes
    /// `initial_content` as the complete body (no marker region);
    /// uninstall deletes the file (`apply` returns empty).
    /// Marker-block files set this to `false`.
    pub fully_owned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoexistencePlan {
    pub framework: HookFramework,
    pub files: Vec<CoexistenceFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoexistenceError {
    #[error(
        "framework `{0}` is not supported by coexistence; use `anvil hook bootstrap` for the plain install path"
    )]
    UnsupportedFramework(&'static str),
}

pub fn plan_install(
    framework: HookFramework,
    kinds: &[HookKind],
) -> Result<CoexistencePlan, CoexistenceError> {
    let files = match framework {
        HookFramework::Husky => husky_files(kinds),
        HookFramework::Lefthook => lefthook_files(kinds),
        HookFramework::PreCommitFramework => pre_commit_framework_files(kinds),
        HookFramework::CargoHusky | HookFramework::CoreHooksPath | HookFramework::Plain => {
            return Err(CoexistenceError::UnsupportedFramework(framework.id()));
        }
    };
    Ok(CoexistencePlan { framework, files })
}

pub fn plan_uninstall(
    framework: HookFramework,
    kinds: &[HookKind],
) -> Result<CoexistencePlan, CoexistenceError> {
    let mut plan = plan_install(framework, kinds)?;
    for f in &mut plan.files {
        f.block.clear();
        if f.fully_owned {
            // Fully-owned managed files delete by emptying content.
            f.initial_content.clear();
        }
    }
    Ok(plan)
}

/// Apply a [`CoexistenceFile`] to a host file's current content.
///
/// `existing = Some(bytes)` — host file exists; merge or replace.
/// `existing = None` — host file does not exist; seed with
/// `initial_content` then insert the managed block. An empty
/// `block` removes the Anvil-managed region (uninstall path).
///
/// **Fully-owned files** ([`CoexistenceFile::fully_owned`]): install
/// writes `initial_content` as the complete file body (no markers).
/// Uninstall (empty `initial_content` after [`plan_uninstall`])
/// returns empty so the CLI can delete the path.
///
/// **Round-trip contract.** Install→uninstall returns the input
/// to its **canonical** byte form: trailing whitespace before the
/// marker is collapsed to a single `\n`; leading whitespace after
/// the marker is stripped. Input that already matches the
/// canonical form (the common case: text files ending in exactly
/// one `\n`) round-trips byte-exact. The
/// `install_then_uninstall_preserves_user_content_*` tests pin
/// both the byte-exact and canonical cases. Fully-owned managed
/// files round-trip to empty (deleted) on uninstall.
#[must_use]
pub fn apply(existing: Option<&str>, file: &CoexistenceFile) -> String {
    if file.fully_owned {
        // Whole-file ownership: install/refresh writes initial_content;
        // uninstall clears it and yields empty (delete).
        return file.initial_content.clone();
    }
    if existing.is_none() && file.block.is_empty() {
        return String::new();
    }
    let body: String = existing.map_or_else(|| file.initial_content.clone(), str::to_string);
    if let Some((before, after)) = split_at_markers(&body) {
        if file.block.is_empty() {
            return remove_marker_block(before, after);
        }
        return format!(
            "{before}{MARKER_BEGIN}\n{}\n{MARKER_END}{after}",
            file.block
        );
    }
    if file.block.is_empty() {
        return body;
    }
    append_marker_block(&body, &file.block)
}

fn split_at_markers(body: &str) -> Option<(&str, &str)> {
    let start = body.find(MARKER_BEGIN)?;
    let after_begin = start + MARKER_BEGIN.len();
    let end_offset = body[after_begin..].find(MARKER_END)?;
    let after_end = after_begin + end_offset + MARKER_END.len();
    Some((&body[..start], &body[after_end..]))
}

/// Remove the marker-bounded region and canonicalise trailing
/// whitespace. The round-trip contract is **canonical** byte
/// equality (single trailing newline, no leading whitespace
/// stripped) — `apply` documents this. Tests assert byte-exact
/// round-trip on canonical input.
fn remove_marker_block(before: &str, after: &str) -> String {
    let head = before.trim_end_matches('\n');
    let tail = after.trim_start_matches('\n');
    if head.is_empty() && tail.is_empty() {
        return String::new();
    }
    if head.is_empty() {
        let mut s = tail.to_string();
        if !s.ends_with('\n') {
            s.push('\n');
        }
        return s;
    }
    if tail.is_empty() {
        let mut s = head.to_string();
        s.push('\n');
        return s;
    }
    let mut s = String::with_capacity(head.len() + tail.len() + 2);
    s.push_str(head);
    s.push('\n');
    s.push_str(tail);
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// Append a marker block. Canonical insertion: body is normalised
/// to end with exactly one `\n`, then `MARKER_BEGIN\nblock\nMARKER_END\n`
/// is appended. This is exactly inverse to [`remove_marker_block`]
/// on canonical input, so install→uninstall is byte-exact for any
/// input whose original trailing-whitespace already matches the
/// canonical form (the common case).
fn append_marker_block(body: &str, block: &str) -> String {
    let trimmed = body.trim_end_matches('\n');
    let mut result = String::with_capacity(
        trimmed.len() + MARKER_BEGIN.len() + MARKER_END.len() + block.len() + 4,
    );
    if !trimmed.is_empty() {
        result.push_str(trimmed);
        result.push('\n');
    }
    result.push_str(MARKER_BEGIN);
    result.push('\n');
    result.push_str(block);
    result.push('\n');
    result.push_str(MARKER_END);
    result.push('\n');
    result
}

fn husky_files(kinds: &[HookKind]) -> Vec<CoexistenceFile> {
    kinds
        .iter()
        .map(|k| CoexistenceFile {
            relative_path: format!(".husky/{}", k.filename()),
            initial_content: HUSKY_INITIAL_HEADER.to_string(),
            block: husky_block(*k),
            executable: true,
            fully_owned: false,
        })
        .collect()
}

fn husky_block(k: HookKind) -> String {
    // Unavailable anvil must not abort the host Husky hook. Prefer a true
    // no-op (`if`/`fi`) over `|| exit 0` so user content after the managed
    // marker region still runs. When anvil is present, `exec` propagates its
    // exit status (same as the plain shell template's intent, ADR-038 §D-5).
    // Do not use `cmd && exec` — a missing binary makes that form exit
    // non-zero and blocks Git.
    format!(
        "if command -v anvil >/dev/null 2>&1; then\n  exec anvil hook {} \"$@\"\nfi",
        k.subcommand()
    )
}

fn lefthook_files(kinds: &[HookKind]) -> Vec<CoexistenceFile> {
    // The managed config file is fully Anvil-owned and listed via
    // `extends:` in the host `lefthook.yml`. We intentionally do
    // NOT inject the `extends:` key from a marker block: lefthook
    // supports at most one top-level `extends:` and many users
    // already have their own. The CLI consumer is expected to
    // append `.anvil-lefthook.yml` to the user's `extends:` list
    // (creating one if absent) and surface a confirmation prompt
    // — that is wired in the CLI follow-up step of ADOPT-001.
    let managed = CoexistenceFile {
        relative_path: ".anvil-lefthook.yml".to_string(),
        initial_content: lefthook_managed_initial(kinds),
        block: String::new(),
        executable: false,
        fully_owned: true,
    };
    let host = CoexistenceFile {
        relative_path: "lefthook.yml".to_string(),
        initial_content: LEFTHOOK_HOST_INITIAL.to_string(),
        block: LEFTHOOK_HOST_BLOCK.to_string(),
        executable: false,
        fully_owned: false,
    };
    vec![managed, host]
}

fn lefthook_managed_initial(kinds: &[HookKind]) -> String {
    use std::fmt::Write as _;
    let mut out = String::from(
        "# anvil-managed lefthook configuration.\n# Do not edit by hand — re-run `anvil hook bootstrap` to regenerate.\n\n",
    );
    for k in kinds {
        let _ = writeln!(
            out,
            "{filename}:\n  commands:\n    anvil:\n      run: anvil hook {sub}",
            filename = k.filename(),
            sub = k.subcommand(),
        );
    }
    out
}

fn pre_commit_framework_files(kinds: &[HookKind]) -> Vec<CoexistenceFile> {
    let managed = CoexistenceFile {
        relative_path: ".anvil-pre-commit-config.local.yaml".to_string(),
        initial_content: pre_commit_managed_initial(kinds),
        block: String::new(),
        executable: false,
        fully_owned: true,
    };
    let host = CoexistenceFile {
        relative_path: ".pre-commit-config.yaml".to_string(),
        initial_content: PRE_COMMIT_HOST_INITIAL.to_string(),
        block: PRE_COMMIT_HOST_BLOCK.to_string(),
        executable: false,
        fully_owned: false,
    };
    vec![managed, host]
}

fn pre_commit_managed_initial(kinds: &[HookKind]) -> String {
    use std::fmt::Write as _;
    let mut out = String::from(
        "# anvil-managed snippet for `.pre-commit-config.yaml`.\n# Do not edit by hand — re-run `anvil hook bootstrap` to regenerate.\n# pre-commit framework does not support config inclusion, so the\n# `repos:` entry below must be merged into your existing\n# `.pre-commit-config.yaml` `repos:` list manually.\n\nrepos:\n  - repo: local\n    hooks:\n",
    );
    for k in kinds {
        let _ = writeln!(
            out,
            "      - id: anvil-{filename}\n        name: anvil hook {sub}\n        entry: anvil hook {sub}\n        language: system\n        stages: [{stage}]\n        pass_filenames: false",
            filename = k.filename(),
            sub = k.subcommand(),
            stage = pre_commit_stage(*k),
        );
    }
    out
}

fn pre_commit_stage(k: HookKind) -> &'static str {
    match k {
        HookKind::PreCommit => "pre-commit",
        HookKind::PostCommit => "post-commit",
        HookKind::PrePush => "pre-push",
        HookKind::PostMerge => "post-merge",
        HookKind::PostRewrite => "post-rewrite",
    }
}

const HUSKY_INITIAL_HEADER: &str = "#!/usr/bin/env sh\n[ -f \"$(dirname -- \"$0\")/_/husky.sh\" ] && . \"$(dirname -- \"$0\")/_/husky.sh\"\n\n";

const LEFTHOOK_HOST_INITIAL: &str = "# Lefthook configuration — see https://lefthook.dev\n";

const LEFTHOOK_HOST_BLOCK: &str = "# Lefthook supports a single top-level `extends:` key. The anvil-\n# managed snippet lives in `.anvil-lefthook.yml`. Add it to your\n# existing `extends:` list (creating one if absent) so Lefthook\n# loads the anvil hooks alongside your own.";

const PRE_COMMIT_HOST_INITIAL: &str =
    "# pre-commit configuration — see https://pre-commit.com\nrepos: []\n";

const PRE_COMMIT_HOST_BLOCK: &str = "# pre-commit framework does not support config inclusion. The\n# anvil-managed snippet lives in `.anvil-pre-commit-config.local.yaml`.\n# Merge the `local` repo entry from that snippet into the top-level\n# repository list above to enable the anvil hooks. Running\n# `anvil hook bootstrap` regenerates the snippet but cannot edit\n# your existing list for you.";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::HookFramework;
    use crate::shell::HookKind;

    const ALL_HOOKS: &[HookKind] = &[
        HookKind::PreCommit,
        HookKind::PostCommit,
        HookKind::PrePush,
        HookKind::PostMerge,
        HookKind::PostRewrite,
    ];

    #[test]
    fn markers_are_distinct() {
        assert_ne!(MARKER_BEGIN, MARKER_END);
    }

    #[test]
    fn markers_start_with_comment_prefix() {
        assert!(MARKER_BEGIN.starts_with('#'));
        assert!(MARKER_END.starts_with('#'));
    }

    #[test]
    fn markers_mention_anvil_managed() {
        assert!(MARKER_BEGIN.contains("anvil-managed"));
        assert!(MARKER_END.contains("anvil-managed"));
    }

    #[test]
    fn plan_install_husky_emits_one_file_per_kind() {
        let plan = plan_install(HookFramework::Husky, ALL_HOOKS).unwrap();
        assert_eq!(plan.framework, HookFramework::Husky);
        assert_eq!(plan.files.len(), ALL_HOOKS.len());
        for (file, kind) in plan.files.iter().zip(ALL_HOOKS) {
            assert_eq!(file.relative_path, format!(".husky/{}", kind.filename()));
            assert!(file.executable);
            assert!(
                file.block
                    .contains(&format!("anvil hook {}", kind.subcommand()))
            );
        }
    }

    #[test]
    fn plan_install_husky_preserves_kind_order() {
        let custom = [HookKind::PrePush, HookKind::PreCommit];
        let plan = plan_install(HookFramework::Husky, &custom).unwrap();
        assert_eq!(plan.files[0].relative_path, ".husky/pre-push");
        assert_eq!(plan.files[1].relative_path, ".husky/pre-commit");
    }

    #[test]
    fn plan_install_lefthook_emits_managed_file_and_host_extends_block() {
        let plan = plan_install(HookFramework::Lefthook, ALL_HOOKS).unwrap();
        assert_eq!(plan.framework, HookFramework::Lefthook);
        assert_eq!(plan.files.len(), 2);
        let managed = &plan.files[0];
        assert_eq!(managed.relative_path, ".anvil-lefthook.yml");
        assert!(managed.block.is_empty());
        assert!(managed.initial_content.contains("pre-commit:"));
        assert!(managed.initial_content.contains("anvil hook pre-commit"));
        assert!(!managed.executable);
        let host = &plan.files[1];
        assert_eq!(host.relative_path, "lefthook.yml");
        // Lefthook supports at most one top-level `extends:`; the
        // host marker block is a doc-pointer comment, not raw YAML,
        // so it can never inject a second `extends:` key. The CLI
        // consumer is responsible for splicing
        // `.anvil-lefthook.yml` into the user's existing list.
        assert!(host.block.contains(".anvil-lefthook.yml"));
        assert!(!host.block.contains("extends:\n"));
    }

    #[test]
    fn plan_install_pre_commit_framework_emits_managed_file_and_doc_pointer() {
        let plan = plan_install(HookFramework::PreCommitFramework, ALL_HOOKS).unwrap();
        assert_eq!(plan.framework, HookFramework::PreCommitFramework);
        assert_eq!(plan.files.len(), 2);
        let managed = &plan.files[0];
        assert_eq!(managed.relative_path, ".anvil-pre-commit-config.local.yaml");
        assert!(managed.initial_content.contains("repos:"));
        assert!(managed.initial_content.contains("id: anvil-pre-commit"));
        assert!(managed.initial_content.contains("stages: [pre-commit]"));
        let host = &plan.files[1];
        assert_eq!(host.relative_path, ".pre-commit-config.yaml");
        assert!(host.block.contains(".anvil-pre-commit-config.local.yaml"));
        assert!(!host.block.contains("repos:"));
    }

    #[test]
    fn plan_install_unsupported_frameworks_error() {
        for fw in [
            HookFramework::CargoHusky,
            HookFramework::CoreHooksPath,
            HookFramework::Plain,
        ] {
            let err = plan_install(fw, ALL_HOOKS).unwrap_err();
            match err {
                CoexistenceError::UnsupportedFramework(id) => {
                    assert_eq!(id, fw.id());
                }
            }
        }
    }

    #[test]
    fn plan_uninstall_matches_install_files_with_empty_block() {
        for fw in [
            HookFramework::Husky,
            HookFramework::Lefthook,
            HookFramework::PreCommitFramework,
        ] {
            let install = plan_install(fw, ALL_HOOKS).unwrap();
            let uninstall = plan_uninstall(fw, ALL_HOOKS).unwrap();
            assert_eq!(install.files.len(), uninstall.files.len());
            for (i, u) in install.files.iter().zip(&uninstall.files) {
                assert_eq!(i.relative_path, u.relative_path);
                assert_eq!(i.executable, u.executable);
                assert_eq!(i.fully_owned, u.fully_owned);
                assert!(u.block.is_empty());
                if i.fully_owned {
                    // Fully-owned managed file: uninstall clears content
                    // so `apply` returns empty (delete).
                    assert!(
                        u.initial_content.is_empty(),
                        "fully-owned uninstall must clear initial_content for {}",
                        i.relative_path
                    );
                } else {
                    // Marker-block file: only the block is cleared.
                    assert_eq!(i.initial_content, u.initial_content);
                }
            }
        }
    }

    #[test]
    fn plan_uninstall_unsupported_framework_errors() {
        let err = plan_uninstall(HookFramework::Plain, ALL_HOOKS).unwrap_err();
        assert!(matches!(
            err,
            CoexistenceError::UnsupportedFramework("plain")
        ));
    }

    #[test]
    fn apply_creates_file_when_missing_with_initial_then_block() {
        let file = CoexistenceFile {
            relative_path: ".husky/pre-commit".into(),
            initial_content: "#!/usr/bin/env sh\n".into(),
            block: "anvil hook pre-commit \"$@\"".into(),
            executable: true,
            fully_owned: false,
        };
        let out = apply(None, &file);
        assert!(out.starts_with("#!/usr/bin/env sh\n"));
        assert!(out.contains(MARKER_BEGIN));
        assert!(out.contains("anvil hook pre-commit \"$@\""));
        assert!(out.contains(MARKER_END));
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn apply_appends_marker_block_when_file_exists_without_markers() {
        let file = CoexistenceFile {
            relative_path: ".husky/pre-commit".into(),
            initial_content: "fallback\n".into(),
            block: "managed-line".into(),
            executable: true,
            fully_owned: false,
        };
        let existing = "#!/usr/bin/env sh\necho user-hook\n";
        let out = apply(Some(existing), &file);
        assert!(out.starts_with(existing));
        assert!(out.contains(MARKER_BEGIN));
        assert!(out.contains("managed-line"));
        assert!(out.contains(MARKER_END));
    }

    #[test]
    fn apply_replaces_existing_marker_block_when_block_changes() {
        let file = CoexistenceFile {
            relative_path: "lefthook.yml".into(),
            initial_content: String::new(),
            block: "new-content".into(),
            executable: false,
            fully_owned: false,
        };
        let existing = format!("top\n\n{MARKER_BEGIN}\nold-content\n{MARKER_END}\nbottom\n");
        let out = apply(Some(&existing), &file);
        assert!(out.contains("new-content"));
        assert!(!out.contains("old-content"));
        assert!(out.contains("top\n"));
        assert!(out.contains("bottom\n"));
    }

    #[test]
    fn apply_removes_marker_block_on_uninstall_keeping_surrounds() {
        let file = CoexistenceFile {
            relative_path: "lefthook.yml".into(),
            initial_content: String::new(),
            block: String::new(),
            executable: false,
            fully_owned: false,
        };
        let existing = format!("top\n\n{MARKER_BEGIN}\nmanaged\n{MARKER_END}\nbottom\n");
        let out = apply(Some(&existing), &file);
        assert!(!out.contains(MARKER_BEGIN));
        assert!(!out.contains(MARKER_END));
        assert!(!out.contains("managed"));
        assert!(out.contains("top"));
        assert!(out.contains("bottom"));
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn apply_uninstall_on_file_without_markers_is_noop() {
        let file = CoexistenceFile {
            relative_path: "lefthook.yml".into(),
            initial_content: String::new(),
            block: String::new(),
            executable: false,
            fully_owned: false,
        };
        let existing = "unrelated content\n";
        let out = apply(Some(existing), &file);
        assert_eq!(out, existing);
    }

    #[test]
    fn apply_uninstall_on_missing_file_returns_empty() {
        let file = CoexistenceFile {
            relative_path: "lefthook.yml".into(),
            initial_content: "ignored".into(),
            block: String::new(),
            executable: false,
            fully_owned: false,
        };
        assert_eq!(apply(None, &file), "");
    }

    #[test]
    fn apply_creates_fully_owned_managed_lefthook_and_pre_commit_files() {
        // Regression: empty-block managed files previously returned ""
        // from `apply(None, …)`, so `.anvil-lefthook.yml` and
        // `.anvil-pre-commit-config.local.yaml` were never created.
        for fw in [HookFramework::Lefthook, HookFramework::PreCommitFramework] {
            let plan = plan_install(fw, ALL_HOOKS).unwrap();
            let managed = &plan.files[0];
            assert!(
                managed.fully_owned,
                "{fw:?} managed file must be fully_owned"
            );
            assert!(
                managed.block.is_empty(),
                "{fw:?} managed file uses empty block (content in initial_content)"
            );
            assert!(
                !managed.initial_content.is_empty(),
                "{fw:?} managed file must ship non-empty initial_content"
            );
            let out = apply(None, managed);
            assert_eq!(
                out, managed.initial_content,
                "{fw:?} apply(None) must emit the generated managed config"
            );
            for k in ALL_HOOKS {
                assert!(
                    out.contains(&format!("anvil hook {}", k.subcommand())),
                    "{fw:?} managed output missing hook {}",
                    k.subcommand()
                );
            }
        }
    }

    #[test]
    fn fully_owned_managed_install_then_uninstall_deletes_file() {
        for fw in [HookFramework::Lefthook, HookFramework::PreCommitFramework] {
            let install = plan_install(fw, ALL_HOOKS).unwrap();
            let uninstall = plan_uninstall(fw, ALL_HOOKS).unwrap();
            let managed_install = &install.files[0];
            let managed_uninstall = &uninstall.files[0];
            assert!(managed_install.fully_owned);
            assert!(managed_uninstall.fully_owned);
            let installed = apply(None, managed_install);
            assert!(!installed.is_empty(), "{fw:?} install must create content");
            let removed = apply(Some(&installed), managed_uninstall);
            assert_eq!(removed, "", "{fw:?} uninstall must delete fully-owned file");
        }
    }

    #[test]
    fn apply_is_idempotent_under_double_install() {
        for fw in [
            HookFramework::Husky,
            HookFramework::Lefthook,
            HookFramework::PreCommitFramework,
        ] {
            let plan = plan_install(fw, ALL_HOOKS).unwrap();
            for file in &plan.files {
                let first = apply(None, file);
                let second = apply(Some(&first), file);
                assert_eq!(
                    first, second,
                    "framework {fw:?} file {} not idempotent",
                    file.relative_path
                );
            }
        }
    }

    #[test]
    fn install_then_uninstall_is_byte_exact_for_canonical_input() {
        // Canonical input = ends with exactly one `\n`. This is
        // what `cargo fmt`, editors with "final newline", and the
        // existing repo policy all produce. Byte-exact round-trip
        // matters because the host file is git-tracked and even a
        // whitespace-only delta shows up in `git diff`.
        let install_file = CoexistenceFile {
            relative_path: "lefthook.yml".into(),
            initial_content: "user-config:\n  thing: 1\n".into(),
            block: "managed-snippet".into(),
            executable: false,
            fully_owned: false,
        };
        let uninstall_file = CoexistenceFile {
            block: String::new(),
            ..install_file.clone()
        };
        let user_existing = "user-config:\n  thing: 1\n";
        let installed = apply(Some(user_existing), &install_file);
        let uninstalled = apply(Some(&installed), &uninstall_file);
        assert_eq!(
            uninstalled, user_existing,
            "round-trip must be byte-exact for canonical input"
        );
    }

    #[test]
    fn install_then_uninstall_canonicalises_non_canonical_input() {
        // Non-canonical inputs (no trailing newline, multiple
        // trailing newlines) round-trip to canonical form. This is
        // documented on `apply` and is what makes the inverse safe
        // without storing per-install state.
        let install_file = CoexistenceFile {
            relative_path: "lefthook.yml".into(),
            initial_content: String::new(),
            block: "managed".into(),
            executable: false,
            fully_owned: false,
        };
        let uninstall_file = CoexistenceFile {
            block: String::new(),
            ..install_file.clone()
        };
        for raw in ["body", "body\n\n", "body\n\n\n"] {
            let installed = apply(Some(raw), &install_file);
            let uninstalled = apply(Some(&installed), &uninstall_file);
            assert_eq!(uninstalled, "body\n", "input {raw:?}");
        }
    }

    #[test]
    fn install_then_uninstall_preserves_user_content_around_marker_block() {
        let install_file = CoexistenceFile {
            relative_path: "lefthook.yml".into(),
            initial_content: "user-config:\n  thing: 1\n".into(),
            block: "managed-snippet".into(),
            executable: false,
            fully_owned: false,
        };
        let uninstall_file = CoexistenceFile {
            block: String::new(),
            ..install_file.clone()
        };
        let user_existing = "user-config:\n  thing: 1\n";
        let installed = apply(Some(user_existing), &install_file);
        let uninstalled = apply(Some(&installed), &uninstall_file);
        assert!(uninstalled.contains("user-config:"));
        assert!(uninstalled.contains("thing: 1"));
        assert!(!uninstalled.contains(MARKER_BEGIN));
        assert!(!uninstalled.contains(MARKER_END));
        assert!(!uninstalled.contains("managed-snippet"));
    }

    #[test]
    fn install_then_uninstall_returns_empty_when_only_marker_block_present() {
        let install_file = CoexistenceFile {
            relative_path: "lefthook.yml".into(),
            initial_content: String::new(),
            block: "extends:\n  - .anvil-lefthook.yml".into(),
            executable: false,
            fully_owned: false,
        };
        let uninstall_file = CoexistenceFile {
            block: String::new(),
            ..install_file.clone()
        };
        let installed = apply(None, &install_file);
        let uninstalled = apply(Some(&installed), &uninstall_file);
        assert_eq!(uninstalled, "");
    }

    #[test]
    fn husky_block_uses_command_v_guard_and_exec() {
        let plan = plan_install(HookFramework::Husky, &[HookKind::PreCommit]).unwrap();
        let block = &plan.files[0].block;
        assert!(block.contains("command -v anvil"));
        assert!(block.contains("exec anvil hook pre-commit"));
        assert!(block.contains("\"$@\""));
        // Unavailable-binary path must be a true no-op (if/fi), not
        // `cmd && exec` (exits non-zero when missing) and not a bare
        // `|| exit 0` after the guard (would skip user content after the
        // managed marker region in coexistence files).
        assert!(
            block.contains("if command -v anvil >/dev/null 2>&1; then"),
            "husky block must use if-guard no-op when anvil is absent; got: {block:?}"
        );
        assert!(
            !block.contains("&& exec"),
            "prefer if-guard over `cmd && exec` for exit-status fidelity; got: {block:?}"
        );
        assert!(
            !block.contains("|| exit 0"),
            "prefer if-guard over `|| exit 0` so post-marker user content can run; got: {block:?}"
        );
    }

    /// Build an applied Husky hook script and run it with a controlled PATH.
    ///
    /// Invoked via `/bin/sh` (not the shebang) so a restricted PATH that
    /// omits `/usr/bin` cannot break `#!/usr/bin/env sh` before the guard.
    #[cfg(unix)]
    fn run_generated_husky_hook(
        path: &std::path::Path,
        extra_tail: &str,
    ) -> (tempfile::TempDir, std::process::ExitStatus) {
        let plan = plan_install(HookFramework::Husky, &[HookKind::PreCommit]).unwrap();
        let mut contents = apply(None, &plan.files[0]);
        contents.push_str(extra_tail);
        let tmp = tempfile::tempdir().unwrap();
        let hook = tmp.path().join("pre-commit");
        std::fs::write(&hook, contents).unwrap();
        // Controlled bin dir first so a fake `anvil` wins; append core
        // system paths so the husky header's `dirname` still resolves.
        let path_value = format!("{}:/usr/bin:/bin", path.display());
        let status = std::process::Command::new("/bin/sh")
            .arg(&hook)
            .env("PATH", path_value)
            .status()
            .expect("spawn generated husky hook");
        (tmp, status)
    }

    #[cfg(unix)]
    fn write_fake_anvil(bin_dir: &std::path::Path, exit_code: i32) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(bin_dir).unwrap();
        let anvil = bin_dir.join("anvil");
        std::fs::write(&anvil, format!("#!/bin/sh\nexit {exit_code}\n")).unwrap();
        let mut perms = std::fs::metadata(&anvil).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&anvil, perms).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn husky_hook_exits_zero_when_anvil_unavailable() {
        // Regression: coexistence husky block used `cmd && exec`, so missing
        // anvil made the whole hook fail and blocked Git.
        let empty = tempfile::tempdir().unwrap();
        let (_tmp, status) = run_generated_husky_hook(empty.path(), "");
        assert!(
            status.success(),
            "hook must exit 0 when anvil is not on PATH; got {status}"
        );
    }

    #[test]
    #[cfg(unix)]
    fn husky_hook_runs_post_marker_user_content_when_anvil_unavailable() {
        // Coexistence must not `exit 0` from the managed block — user
        // commands after the marker region should still run.
        let empty = tempfile::tempdir().unwrap();
        let stamp_dir = tempfile::tempdir().unwrap();
        let stamp = stamp_dir.path().join("after-ran");
        let tail = format!("touch '{}'\n", stamp.display());
        let (_tmp, status) = run_generated_husky_hook(empty.path(), &tail);
        assert!(
            status.success(),
            "hook must exit 0 when anvil is absent; got {status}"
        );
        assert!(
            stamp.is_file(),
            "post-marker user content must run when anvil is unavailable"
        );
    }

    #[test]
    #[cfg(unix)]
    fn husky_hook_propagates_anvil_failure_exit_code() {
        let bin = tempfile::tempdir().unwrap();
        write_fake_anvil(bin.path(), 42);
        let (_tmp, status) = run_generated_husky_hook(bin.path(), "");
        assert_eq!(
            status.code(),
            Some(42),
            "hook must propagate anvil's non-zero exit; got {status}"
        );
    }

    #[test]
    #[cfg(unix)]
    fn husky_hook_propagates_anvil_success_exit_code() {
        let bin = tempfile::tempdir().unwrap();
        write_fake_anvil(bin.path(), 0);
        let (_tmp, status) = run_generated_husky_hook(bin.path(), "");
        assert!(
            status.success(),
            "hook must propagate anvil exit 0; got {status}"
        );
    }

    #[test]
    fn husky_initial_header_sources_husky_runtime_safely() {
        let plan = plan_install(HookFramework::Husky, &[HookKind::PreCommit]).unwrap();
        let header = &plan.files[0].initial_content;
        assert!(header.starts_with("#!/usr/bin/env sh\n"));
        assert!(header.contains("[ -f"));
        assert!(header.contains("husky.sh"));
    }

    #[test]
    fn lefthook_managed_file_lists_every_kind() {
        let managed = &plan_install(HookFramework::Lefthook, ALL_HOOKS)
            .unwrap()
            .files[0];
        for k in ALL_HOOKS {
            assert!(
                managed
                    .initial_content
                    .contains(&format!("{}:", k.filename())),
                "missing key for {k:?}"
            );
            assert!(
                managed
                    .initial_content
                    .contains(&format!("anvil hook {}", k.subcommand())),
                "missing run line for {k:?}"
            );
        }
    }

    #[test]
    fn pre_commit_managed_file_lists_every_kind_with_correct_stage() {
        let managed = &plan_install(HookFramework::PreCommitFramework, ALL_HOOKS)
            .unwrap()
            .files[0];
        for k in ALL_HOOKS {
            assert!(
                managed
                    .initial_content
                    .contains(&format!("id: anvil-{}", k.filename())),
                "missing id for {k:?}"
            );
            // Use the actual stage helper so a future divergence
            // between `filename()` and `pre_commit_stage()` (e.g.
            // adding `HookKind::CommitMsg` whose stage name differs
            // from its filename) trips this test.
            assert!(
                managed
                    .initial_content
                    .contains(&format!("stages: [{}]", pre_commit_stage(*k))),
                "missing stage for {k:?}"
            );
        }
    }

    #[test]
    fn pre_commit_host_block_does_not_inject_second_repos_key() {
        let host = &plan_install(HookFramework::PreCommitFramework, ALL_HOOKS)
            .unwrap()
            .files[1];
        assert!(!host.block.contains("repos:"));
        assert!(host.block.contains(".anvil-pre-commit-config.local.yaml"));
    }

    #[test]
    fn paths_are_forward_slash_separated() {
        for fw in [
            HookFramework::Husky,
            HookFramework::Lefthook,
            HookFramework::PreCommitFramework,
        ] {
            let plan = plan_install(fw, ALL_HOOKS).unwrap();
            for f in &plan.files {
                assert!(
                    !f.relative_path.contains('\\'),
                    "path `{}` must not contain backslash",
                    f.relative_path
                );
            }
        }
    }
}
