use globset::{Glob, GlobMatcher};
use thiserror::Error;

use crate::policy::{BranchRule, Policy};

#[derive(Debug, Error)]
pub enum ResolveError {
    /// A branch pattern wasn't a valid glob expression.
    #[error("invalid branch pattern {pattern:?}: {source}")]
    InvalidPattern {
        pattern: String,
        #[source]
        source: globset::Error,
    },
}

impl Policy {
    /// Find the first [`BranchRule`] whose pattern matches `branch`.
    ///
    /// Match semantics use globset's `Glob` (POSIX-ish: `*` matches
    /// path-segment characters, `?` matches a single character,
    /// `{a,b}` alternatives). Patterns are compiled on every call;
    /// this is fine for the per-push frequency we expect. A caller
    /// that resolves many branches in a tight loop can compile
    /// matchers once via [`Policy::compile_matchers`].
    pub fn resolve(&self, branch: &str) -> Result<Option<&BranchRule>, ResolveError> {
        for rule in &self.branches {
            if compile_matcher(&rule.pattern)?.is_match(branch) {
                return Ok(Some(rule));
            }
        }
        Ok(None)
    }

    /// Compile all branch-rule patterns up-front. Returns one
    /// [`GlobMatcher`] per rule in declaration order, suitable for
    /// reuse across many resolutions.
    pub fn compile_matchers(&self) -> Result<Vec<GlobMatcher>, ResolveError> {
        self.branches
            .iter()
            .map(|r| compile_matcher(&r.pattern))
            .collect()
    }

    /// True when `commit` is at or before `cutoff_commit` in the
    /// caller-supplied ancestry list.
    ///
    /// The library does NOT shell out to git. The caller is expected
    /// to produce `ancestry` (e.g. via `git rev-list --first-parent
    /// HEAD`) and pass it in. Returns `false` when there is no
    /// `cutoff_commit` set on the policy.
    ///
    /// `ancestry` is the ordered sequence from newest to oldest. The
    /// helper asks: "is `commit` reachable from `cutoff_commit` going
    /// backward in time?" — answered by checking that `cutoff_commit`
    /// appears in `ancestry[..=position_of(commit)]`.
    pub fn commit_is_before_cutoff(&self, commit: &str, ancestry: &[&str]) -> bool {
        let Some(cutoff) = self.baseline.cutoff_commit.as_deref() else {
            return false;
        };
        let Some(commit_pos) = ancestry.iter().position(|a| *a == commit) else {
            return false;
        };
        let Some(cutoff_pos) = ancestry.iter().position(|a| *a == cutoff) else {
            return false;
        };
        // ancestry is newest-first, so a larger index is older. The
        // commit is "before or at the cutoff" when its index is
        // greater than or equal to the cutoff's index.
        commit_pos >= cutoff_pos
    }
}

fn compile_matcher(pattern: &str) -> Result<GlobMatcher, ResolveError> {
    Glob::new(pattern)
        .map(|g| g.compile_matcher())
        .map_err(|source| ResolveError::InvalidPattern {
            pattern: pattern.to_string(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{BaselineSection, OnNoWitness, Requirement};
    use anvil_config::ConfigFormat;
    use std::path::Path;

    fn fixture_policy() -> Policy {
        // `cutoff_commit` uses a hex shape so `Policy::validate` —
        // which enforces hex-SHA shape per MLP2-021's Council
        // follow-up — accepts the fixture. The cutoff-ancestry
        // tests below still compare by string equality so the
        // chosen hex value is opaque.
        Policy::parse(
            r"
baseline:
  cutoff_commit: c0ff00c0ff00c0ff00c0ff00c0ff00c0ff00c0ff
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
  - pattern: dependabot/*
    require: l4_only
    on_no_witness: validate_at_l4
  - pattern: 'release/*'
    require: l3_only
    on_no_witness: reject
  - pattern: '*'
    require: l4_or_l3
    on_no_witness: validate_at_l4
",
            ConfigFormat::Yaml,
            Path::new("<test>"),
        )
        .unwrap()
    }

    #[test]
    fn resolve_returns_first_matching_rule_in_declaration_order() {
        let p = fixture_policy();
        let r = p.resolve("main").unwrap().unwrap();
        assert_eq!(r.pattern, "main");
    }

    #[test]
    fn resolve_matches_glob_segment_wildcard() {
        let p = fixture_policy();
        let r = p.resolve("dependabot/cargo-bump").unwrap().unwrap();
        assert_eq!(r.pattern, "dependabot/*");
        assert_eq!(r.require, Requirement::L4Only);
    }

    #[test]
    fn resolve_falls_through_to_star_when_nothing_else_matches() {
        let p = fixture_policy();
        let r = p.resolve("feature/random").unwrap().unwrap();
        assert_eq!(r.pattern, "*");
        assert_eq!(r.require, Requirement::L4OrL3);
    }

    #[test]
    fn resolve_returns_none_when_no_pattern_matches() {
        // A policy with no `*` fallback.
        let p = Policy::parse(
            r"
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
",
            ConfigFormat::Yaml,
            Path::new("<test>"),
        )
        .unwrap();
        assert!(p.resolve("feature/x").unwrap().is_none());
    }

    #[test]
    fn resolve_respects_declaration_order_over_specificity() {
        // First match wins, even if a later rule is "more specific."
        let p = Policy::parse(
            r"
branches:
  - pattern: '*'
    require: l4_only
    on_no_witness: validate_at_l4
  - pattern: main
    require: l3_only
    on_no_witness: reject
",
            ConfigFormat::Yaml,
            Path::new("<test>"),
        )
        .unwrap();
        let r = p.resolve("main").unwrap().unwrap();
        // First rule (`*`) wins.
        assert_eq!(r.pattern, "*");
        assert_eq!(r.require, Requirement::L4Only);
    }

    #[test]
    fn resolve_propagates_invalid_pattern_as_typed_error() {
        let p = Policy {
            required_anvil_version: None,
            baseline: BaselineSection::default(),
            branches: vec![crate::policy::BranchRule {
                pattern: "[".to_string(), // unclosed character class
                require: Requirement::L4OrL3,
                on_no_witness: OnNoWitness::ValidateAtL4,
                on_block: crate::policy::OnBlock::Reject,
                on_warn: crate::policy::OnWarn::Allow,
            }],
        };
        let err = p.resolve("main").unwrap_err();
        assert!(matches!(err, ResolveError::InvalidPattern { .. }));
    }

    #[test]
    fn compile_matchers_returns_one_per_rule() {
        let p = fixture_policy();
        let matchers = p.compile_matchers().unwrap();
        assert_eq!(matchers.len(), p.branches.len());
        assert!(matchers[1].is_match("dependabot/foo"));
        assert!(!matchers[0].is_match("dependabot/foo"));
    }

    #[test]
    fn cutoff_accepts_commit_at_cutoff_sha() {
        let p = fixture_policy();
        // `c0ff00c0ff00c0ff00c0ff00c0ff00c0ff00c0ff` matches the fixture's `cutoff_commit`. The other
        // entries are opaque hex-shaped SHAs.
        let ancestry = [
            "aaaa01",
            "c0ff00c0ff00c0ff00c0ff00c0ff00c0ff00c0ff",
            "0deadbeef",
        ];
        assert!(p.commit_is_before_cutoff("c0ff00c0ff00c0ff00c0ff00c0ff00c0ff00c0ff", &ancestry));
    }

    #[test]
    fn cutoff_accepts_commit_older_than_cutoff() {
        let p = fixture_policy();
        let ancestry = [
            "aaaa01",
            "c0ff00c0ff00c0ff00c0ff00c0ff00c0ff00c0ff",
            "0deadbeef",
            "0badcafe",
        ];
        assert!(p.commit_is_before_cutoff("0deadbeef", &ancestry));
        assert!(p.commit_is_before_cutoff("0badcafe", &ancestry));
    }

    #[test]
    fn cutoff_rejects_commit_newer_than_cutoff() {
        let p = fixture_policy();
        let ancestry = [
            "aaaa01",
            "bbbb02",
            "c0ff00c0ff00c0ff00c0ff00c0ff00c0ff00c0ff",
            "0deadbeef",
        ];
        // `aaaa01` and `bbbb02` are newer than cutoff.
        assert!(!p.commit_is_before_cutoff("aaaa01", &ancestry));
        assert!(!p.commit_is_before_cutoff("bbbb02", &ancestry));
    }

    #[test]
    fn cutoff_returns_false_when_no_cutoff_pinned() {
        let p = Policy::parse(
            r"
branches:
  - pattern: main
    require: l4_or_l3
    on_no_witness: validate_at_l4
",
            ConfigFormat::Yaml,
            Path::new("<test>"),
        )
        .unwrap();
        let ancestry = ["aaaa01"];
        assert!(!p.commit_is_before_cutoff("aaaa01", &ancestry));
    }

    #[test]
    fn cutoff_returns_false_for_commit_not_in_ancestry() {
        let p = fixture_policy();
        let ancestry = ["aaaa01", "c0ff00c0ff00c0ff00c0ff00c0ff00c0ff00c0ff"];
        assert!(!p.commit_is_before_cutoff("ffff99", &ancestry));
    }
}
