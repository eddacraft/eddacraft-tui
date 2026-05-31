use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use anvil_intercept_rules::{
    ChangeKind, LaunchReasoningPatternRule, RegistryDecision, RuleInput, RuleRegistry,
    SecretDetectionRule,
};
use anvil_kernel_types::{Diagnostic, Mode};

#[cfg(test)]
use anvil_intercept_rules::{InterceptRule, RuleDecision};

pub const CONTENT_SIZE_CAP_BYTES: u64 = 1024 * 1024;
pub const CONTENT_SIZE_CAP_BYTES_USIZE: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnforcementDecision {
    Allow { affected_paths: Vec<PathBuf> },
    Interrupt(InterruptDecision),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterruptDecision {
    pub rule_id: String,
    pub message: String,
    pub line: Option<u32>,
    pub affected_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
pub struct ProposedChange<'a> {
    pub path: &'a Path,
    pub change_kind: ChangeKind,
    pub content: Option<&'a [u8]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: PathBuf,
    pub change_kind: ChangeKind,
}

#[derive(Debug)]
pub struct EnforcementPipeline {
    registry: RuleRegistry,
}

impl EnforcementPipeline {
    #[must_use]
    pub fn new(registry: RuleRegistry) -> Self {
        Self { registry }
    }

    /// Pure evaluation only. Control-lane callers emit delivered telemetry after
    /// their decision send succeeds.
    #[must_use]
    pub fn evaluate_filesystem_changes(&self, changes: &[FileChange]) -> EnforcementDecision {
        evaluate_filesystem_changes(&self.registry, changes)
    }

    /// Pure evaluation only. Control-lane callers emit delivered telemetry after
    /// their decision send succeeds.
    #[must_use]
    pub fn evaluate_proposed_changes(&self, changes: &[ProposedChange<'_>]) -> EnforcementDecision {
        evaluate_proposed_changes(&self.registry, changes)
    }

    #[must_use]
    pub fn diagnostics_for_proposed_changes(
        &self,
        changes: &[ProposedChange<'_>],
        mode: &Mode,
    ) -> Vec<Diagnostic> {
        diagnostics_for_proposed_changes(&self.registry, changes, mode)
    }

    #[must_use]
    pub fn diagnostics_for_proposed_changes_with_limit(
        &self,
        changes: &[ProposedChange<'_>],
        mode: &Mode,
        limit: usize,
    ) -> Vec<Diagnostic> {
        diagnostics_for_proposed_changes_with_limit(&self.registry, changes, mode, limit)
    }
}

impl Default for EnforcementPipeline {
    fn default() -> Self {
        Self::new(default_rule_registry())
    }
}

#[must_use]
pub fn default_rule_registry() -> RuleRegistry {
    RuleRegistry::with_rules(vec![
        Box::<SecretDetectionRule>::default(),
        Box::<LaunchReasoningPatternRule>::default(),
    ])
    .expect("default intercept rules have unique ids")
}

impl FileChange {
    #[must_use]
    pub fn created(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            change_kind: ChangeKind::Created,
        }
    }

    #[must_use]
    pub fn modified(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            change_kind: ChangeKind::Modified,
        }
    }

    #[must_use]
    pub fn removed(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            change_kind: ChangeKind::Removed,
        }
    }
}

#[must_use]
pub fn evaluate_proposed_changes(
    registry: &RuleRegistry,
    changes: &[ProposedChange<'_>],
) -> EnforcementDecision {
    let affected_paths = changes
        .iter()
        .map(|change| change.path.to_path_buf())
        .collect::<Vec<_>>();
    for change in changes {
        let content = if registry.any_needs_content() {
            evaluation_content(change.change_kind, change.content)
        } else {
            None
        };
        if let Some(interrupt) = evaluate_one(
            registry,
            change.path,
            change.change_kind,
            content,
            &affected_paths,
        ) {
            return EnforcementDecision::Interrupt(interrupt);
        }
    }

    EnforcementDecision::Allow { affected_paths }
}

#[must_use]
pub fn evaluate_filesystem_changes(
    registry: &RuleRegistry,
    changes: &[FileChange],
) -> EnforcementDecision {
    let affected_paths = changes
        .iter()
        .map(|change| change.path.clone())
        .collect::<Vec<_>>();
    for change in changes {
        let content = match read_content_for_change(registry, change, &affected_paths) {
            Ok(content) => content,
            Err(interrupt) => return EnforcementDecision::Interrupt(interrupt),
        };
        if let Some(interrupt) = evaluate_one(
            registry,
            &change.path,
            change.change_kind,
            content.as_deref(),
            &affected_paths,
        ) {
            return EnforcementDecision::Interrupt(interrupt);
        }
    }

    EnforcementDecision::Allow { affected_paths }
}

#[must_use]
pub fn diagnostics_for_proposed_changes(
    registry: &RuleRegistry,
    changes: &[ProposedChange<'_>],
    mode: &Mode,
) -> Vec<Diagnostic> {
    diagnostics_for_proposed_changes_with_limit(registry, changes, mode, usize::MAX)
}

#[must_use]
pub fn diagnostics_for_proposed_changes_with_limit(
    registry: &RuleRegistry,
    changes: &[ProposedChange<'_>],
    mode: &Mode,
    limit: usize,
) -> Vec<Diagnostic> {
    if limit == 0 {
        return Vec::new();
    }
    for change in changes {
        let content = if registry.any_needs_content() {
            evaluation_content(change.change_kind, change.content)
        } else {
            None
        };
        let input = RuleInput {
            path: change.path,
            change_kind: change.change_kind,
            content,
        };
        let diagnostics = registry.diagnostics_with_limit(&input, mode, limit);
        if !diagnostics.is_empty() {
            return diagnostics;
        }
    }

    Vec::new()
}

fn evaluate_one(
    registry: &RuleRegistry,
    path: &Path,
    change_kind: ChangeKind,
    content: Option<&[u8]>,
    affected_paths: &[PathBuf],
) -> Option<InterruptDecision> {
    let input = RuleInput {
        path,
        change_kind,
        content,
    };
    match registry.evaluate(&input) {
        RegistryDecision::Allow => None,
        RegistryDecision::Interrupt(reason) => Some(InterruptDecision {
            rule_id: reason.rule_id,
            message: reason.message,
            // `InterruptReason.line` is `NonZeroU32`-typed to make the
            // 1-based invariant unrepresentable; `InterruptDecision` keeps
            // the wire-facing `Option<u32>`, so widen at the boundary.
            line: reason.line.map(std::num::NonZeroU32::get),
            affected_paths: affected_paths.to_vec(),
        }),
    }
}

fn read_content_for_change(
    registry: &RuleRegistry,
    change: &FileChange,
    affected_paths: &[PathBuf],
) -> Result<Option<Vec<u8>>, InterruptDecision> {
    if !registry.any_needs_content() || change.change_kind == ChangeKind::Removed {
        return Ok(None);
    }

    let metadata =
        fs::metadata(&change.path).map_err(|err| read_failure(change, affected_paths, &err))?;
    if !metadata.is_file() || metadata.len() > CONTENT_SIZE_CAP_BYTES {
        return Ok(None);
    }

    let mut file =
        File::open(&change.path).map_err(|err| read_failure(change, affected_paths, &err))?;
    let mut content = Vec::new();
    file.by_ref()
        .take(CONTENT_SIZE_CAP_BYTES + 1)
        .read_to_end(&mut content)
        .map_err(|err| read_failure(change, affected_paths, &err))?;
    if content.len() as u64 > CONTENT_SIZE_CAP_BYTES {
        return Ok(None);
    }
    if content.contains(&0) {
        return Ok(None);
    }
    Ok(Some(content))
}

fn read_failure(
    change: &FileChange,
    affected_paths: &[PathBuf],
    err: &std::io::Error,
) -> InterruptDecision {
    InterruptDecision {
        rule_id: "anvil-intercept.read-content".to_string(),
        message: format!(
            "failed to read changed file {}: {err}",
            change.path.display()
        ),
        line: None,
        affected_paths: affected_paths.to_vec(),
    }
}

fn evaluation_content(change_kind: ChangeKind, content: Option<&[u8]>) -> Option<&[u8]> {
    if change_kind == ChangeKind::Removed {
        return None;
    }
    let content = content?;
    if content.contains(&0) {
        return None;
    }
    Some(content)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::{Arc, Mutex};

    use super::*;

    type SeenInputs = Arc<Mutex<Vec<(PathBuf, ChangeKind, Option<Vec<u8>>)>>>;

    struct RecordingRule {
        id: &'static str,
        needs_content: bool,
        interrupt_on: Option<&'static str>,
        seen: SeenInputs,
    }

    impl RecordingRule {
        fn new(id: &'static str) -> (Self, SeenInputs) {
            let seen = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    id,
                    needs_content: false,
                    interrupt_on: None,
                    seen: Arc::clone(&seen),
                },
                seen,
            )
        }

        fn needing_content(mut self) -> Self {
            self.needs_content = true;
            self
        }

        fn interrupting_on(mut self, needle: &'static str) -> Self {
            self.interrupt_on = Some(needle);
            self
        }
    }

    impl InterceptRule for RecordingRule {
        fn rule_id(&self) -> &str {
            self.id
        }

        fn needs_content(&self) -> bool {
            self.needs_content
        }

        fn evaluate(&self, input: &RuleInput<'_>) -> RuleDecision {
            self.seen.lock().unwrap().push((
                input.path.to_path_buf(),
                input.change_kind,
                input.content.map(<[u8]>::to_vec),
            ));

            if let (Some(needle), Some(content)) = (self.interrupt_on, input.content)
                && String::from_utf8_lossy(content).contains(needle)
            {
                return RuleDecision::interrupt(self.id, format!("matched {needle}"));
            }

            RuleDecision::Allow
        }
    }

    #[test]
    fn proposed_content_uses_registry_and_returns_affected_path_on_interrupt() {
        let (rule, seen) = RecordingRule::new("content-rule");
        let registry = RuleRegistry::with_rules(vec![Box::new(
            rule.needing_content().interrupting_on("deny"),
        )])
        .expect("registry");
        let path = Path::new("src/lib.rs");

        let decision = evaluate_proposed_changes(
            &registry,
            &[ProposedChange {
                path,
                change_kind: ChangeKind::Modified,
                content: Some(b"deny this change"),
            }],
        );

        assert_eq!(
            decision,
            EnforcementDecision::Interrupt(InterruptDecision {
                rule_id: "content-rule".to_string(),
                message: "matched deny".to_string(),
                line: None,
                affected_paths: vec![path.to_path_buf()],
            })
        );
        assert_eq!(seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn first_violation_short_circuits_later_changes() {
        let (rule, seen) = RecordingRule::new("content-rule");
        let registry = RuleRegistry::with_rules(vec![Box::new(
            rule.needing_content().interrupting_on("deny"),
        )])
        .expect("registry");

        let decision = evaluate_proposed_changes(
            &registry,
            &[
                ProposedChange {
                    path: Path::new("a.rs"),
                    change_kind: ChangeKind::Modified,
                    content: Some(b"deny"),
                },
                ProposedChange {
                    path: Path::new("b.rs"),
                    change_kind: ChangeKind::Modified,
                    content: Some(b"deny"),
                },
            ],
        );

        assert_eq!(
            decision,
            EnforcementDecision::Interrupt(InterruptDecision {
                rule_id: "content-rule".to_string(),
                message: "matched deny".to_string(),
                line: None,
                affected_paths: vec![PathBuf::from("a.rs"), PathBuf::from("b.rs")],
            })
        );
        assert_eq!(seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn binary_content_skips_content_rules() {
        let (rule, seen) = RecordingRule::new("content-rule");
        let registry = RuleRegistry::with_rules(vec![Box::new(
            rule.needing_content().interrupting_on("deny"),
        )])
        .expect("registry");

        let decision = evaluate_proposed_changes(
            &registry,
            &[ProposedChange {
                path: Path::new("asset.bin"),
                change_kind: ChangeKind::Modified,
                content: Some(b"deny\0payload"),
            }],
        );

        assert_eq!(
            decision,
            EnforcementDecision::Allow {
                affected_paths: vec![PathBuf::from("asset.bin")]
            }
        );
        assert!(seen.lock().unwrap().is_empty());
    }

    #[test]
    fn proposed_content_is_evaluated_even_when_larger_than_daemon_read_cap() {
        let (rule, seen) = RecordingRule::new("content-rule");
        let registry = RuleRegistry::with_rules(vec![Box::new(
            rule.needing_content().interrupting_on("deny"),
        )])
        .expect("registry");
        let cap = usize::try_from(CONTENT_SIZE_CAP_BYTES).expect("cap fits usize");
        let mut content = vec![b'a'; cap + 1];
        content.extend_from_slice(b"deny");

        let decision = evaluate_proposed_changes(
            &registry,
            &[ProposedChange {
                path: Path::new("large-proposed.rs"),
                change_kind: ChangeKind::Modified,
                content: Some(&content),
            }],
        );

        assert!(matches!(decision, EnforcementDecision::Interrupt(_)));
        assert_eq!(seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn removed_files_pass_only_path_based_rules() {
        let (path_rule, path_seen) = RecordingRule::new("path-rule");
        let (content_rule, content_seen) = RecordingRule::new("content-rule");
        let registry = RuleRegistry::with_rules(vec![
            Box::new(path_rule),
            Box::new(content_rule.needing_content().interrupting_on("deny")),
        ])
        .expect("registry");

        let decision = evaluate_proposed_changes(
            &registry,
            &[ProposedChange {
                path: Path::new("deleted.rs"),
                change_kind: ChangeKind::Removed,
                content: Some(b"deny"),
            }],
        );

        assert_eq!(
            decision,
            EnforcementDecision::Allow {
                affected_paths: vec![PathBuf::from("deleted.rs")]
            }
        );
        assert_eq!(path_seen.lock().unwrap()[0].2, None);
        assert!(content_seen.lock().unwrap().is_empty());
    }

    #[test]
    fn file_change_path_reads_content_from_disk() {
        let temp = tempfile::tempdir().expect("tempdir");
        let file = temp.path().join("changed.rs");
        fs::write(&file, b"deny from disk").expect("write fixture");
        let (rule, seen) = RecordingRule::new("content-rule");
        let registry = RuleRegistry::with_rules(vec![Box::new(
            rule.needing_content().interrupting_on("deny"),
        )])
        .expect("registry");

        let decision =
            evaluate_filesystem_changes(&registry, &[FileChange::modified(file.clone())]);

        assert!(matches!(decision, EnforcementDecision::Interrupt(_)));
        assert_eq!(seen.lock().unwrap()[0].2, Some(b"deny from disk".to_vec()));
    }

    #[test]
    fn file_change_path_applies_one_megabyte_cap() {
        let temp = tempfile::tempdir().expect("tempdir");
        let file = temp.path().join("large.rs");
        let cap = usize::try_from(CONTENT_SIZE_CAP_BYTES).expect("cap fits usize");
        fs::write(&file, vec![b'a'; cap + 1]).expect("write fixture");
        let (rule, seen) = RecordingRule::new("content-rule");
        let registry =
            RuleRegistry::with_rules(vec![Box::new(rule.needing_content().interrupting_on("a"))])
                .expect("registry");

        let decision =
            evaluate_filesystem_changes(&registry, &[FileChange::modified(file.clone())]);

        assert_eq!(
            decision,
            EnforcementDecision::Allow {
                affected_paths: vec![file]
            }
        );
        assert!(seen.lock().unwrap().is_empty());
    }

    #[test]
    fn file_change_path_interrupts_when_content_cannot_be_read() {
        let temp = tempfile::tempdir().expect("tempdir");
        let file = temp.path().join("missing.rs");
        let (rule, _seen) = RecordingRule::new("content-rule");
        let registry =
            RuleRegistry::with_rules(vec![Box::new(rule.needing_content())]).expect("registry");

        let decision =
            evaluate_filesystem_changes(&registry, &[FileChange::modified(file.clone())]);

        let EnforcementDecision::Interrupt(interrupt) = decision else {
            panic!("expected read failure interrupt");
        };
        assert_eq!(interrupt.rule_id, "anvil-intercept.read-content");
        assert!(interrupt.message.contains(&file.display().to_string()));
        assert_eq!(interrupt.affected_paths, vec![file]);
    }
}
