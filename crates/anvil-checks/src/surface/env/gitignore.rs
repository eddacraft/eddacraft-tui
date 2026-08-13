//! `.gitignore` hygiene for `.env` files (SURFENV-002).
//!
//! Surfaces two structural concerns:
//!
//! 1. A sensitive `.env`-shaped file is present on disk but is **not**
//!    matched by any `.gitignore` rule — the file will get committed by
//!    accident.
//! 2. The repository has *no* `.gitignore` at all while sensitive `.env`
//!    files exist on disk.
//!
//! Both fire as warnings (not blocks) per the SURFENV scope: structural
//! observations, not deal-breakers.
//!
//! Suppression follows [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md):
//! a `# @anvil-ignore SURFENV-002 -- <reason>` directive in the first
//! handful of lines of the offending `.env` file marks the finding as
//! suppressed (the file is committed deliberately — e.g. a frozen test
//! fixture). Per-file granularity is the right unit because `.gitignore`
//! itself is not the right place to add per-rule directives.

use std::path::{Component, Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::surface::env::suppression::resolve_file_header_suppression;

/// Rule ID for the SURFENV-002 `.gitignore` hygiene check.
pub const SURFENV_002_RULE_ID: &str = "SURFENV-002";

/// One `.gitignore` hygiene finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitignoreFinding {
    /// Repo-relative path of the unprotected `.env`-shaped file.
    pub file: PathBuf,
    /// `.gitignore` line that ought to cover this file. Surfaces in
    /// the operator-facing message so the fix is copy-paste.
    pub suggested_pattern: String,
    /// Why the file was flagged.
    pub kind: GitignoreFindingKind,
    /// `true` when a SURFENV-002 suppression directive was found in the
    /// file's header lines.
    pub suppressed: bool,
    pub suppression_reason: Option<String>,
}

/// Why the file was flagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitignoreFindingKind {
    /// `.gitignore` exists but does not match this file's basename.
    UnignoredEnvFile,
    /// No `.gitignore` exists in the repo at all.
    MissingGitignore,
}

/// Filenames that do **not** belong in `.gitignore` even though they
/// match `is_env_file`. The canonical "commit me" templates
/// (`.env.example`, `.env.sample`, `.env.template`) are joined here
/// by Next.js-style `.env*.example` (e.g. `.env.local.example`,
/// `.env.production.example`) — these are documentation templates,
/// not real env files. `.envrc` is a direnv shell script that's
/// almost always committed and would never leak secrets if used as
/// documented.
fn is_intentionally_committed(filename: &str) -> bool {
    if filename == ".envrc" || filename == ".env.sample" || filename == ".env.template" {
        return true;
    }
    // `.env*.example` covers `.env.example`, `.env.local.example`,
    // `.env.production.example`, etc. Any filename that starts with
    // `.env` and ends with `.example` is treated as a template.
    filename.starts_with(".env") && filename.ends_with(".example")
}

/// Check `.gitignore` hygiene for a set of `.env`-shaped files.
///
/// `env_files` is the discovery output (paths to every file
/// `is_env_file` matched in the repo). `gitignore_text` is the contents
/// of the repository's root `.gitignore`, or `None` if none exists.
/// `read_file` is supplied by the caller so we don't hard-code a
/// filesystem dependency in the surface library — the caller has already
/// loaded each env file's content for the SURFENV-001 scan and can hand
/// the same content back here.
#[must_use]
pub fn check_gitignore_hygiene(
    env_files: &[(PathBuf, String)],
    gitignore_text: Option<&str>,
) -> Vec<GitignoreFinding> {
    let patterns = gitignore_text
        .map(extract_ignore_patterns)
        .unwrap_or_default();
    let gitignore_present = gitignore_text.is_some();

    let mut findings = Vec::new();
    for (path, content) in env_files {
        let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if is_intentionally_committed(filename) {
            continue;
        }

        if path_is_effectively_ignored(path, &patterns) {
            continue;
        }

        let kind = if gitignore_present {
            GitignoreFindingKind::UnignoredEnvFile
        } else {
            GitignoreFindingKind::MissingGitignore
        };
        let (suppressed, reason) = resolve_file_header_suppression(content, SURFENV_002_RULE_ID);
        findings.push(GitignoreFinding {
            file: path.clone(),
            suggested_pattern: suggest_pattern(filename),
            kind,
            suppressed,
            suppression_reason: reason,
        });
    }

    findings
}

fn suggest_pattern(filename: &str) -> String {
    // The minimum pattern that covers this file without over-matching:
    // `.env.local` -> `.env.local`, `.env.production` -> `.env.production`.
    // For the bare `.env` filename, `.env` is correct. Callers that want
    // a single line covering everything add `.env*` themselves.
    filename.to_string()
}

/// One parsed gitignore pattern.
///
/// The matcher implements the subset of gitignore semantics that
/// matters for SURFENV-002:
///
/// - Bare names (`.env`) and basename globs (`.env*`, `*.local`,
///   `.env.*.local`) match the file's basename anywhere in the tree.
/// - Path-anchored patterns (`apps/web/.env.local`) match the
///   relative path exactly.
/// - `**`-bearing patterns (`**/.env.local`, `apps/**/.env.local`)
///   match path-component sequences correctly — `**` matches zero or
///   more components, `*` matches within one component.
/// - Negation (`!.env.staging`) is preserved — copilot review caught
///   that dropping it produced false-negatives on rules like
///   `.env*\n!.env.staging` that re-include a specific file. Coverage
///   is now resolved by walking patterns in source order with
///   last-match-wins, matching the gitignore spec.
///
/// Council review caught two false-negatives in the prior hand-rolled
/// matcher (multi-glob patterns like `.env.*.local`, and any pattern
/// containing path separators). The implementation now compiles each
/// pattern to a regex once at parse time, which keeps the matcher
/// honest while still allocating only on `extract_ignore_patterns`.
/// We avoid the `globset` crate to stay zero-new-deps; the regex
/// dependency is already in the workspace.
#[derive(Debug, Clone)]
struct GitignorePattern {
    /// `true` when the pattern body contains a path separator —
    /// drives whether the regex matches against the full relative
    /// path or just the basename.
    has_path_segments: bool,
    /// `true` for `!`-prefixed patterns. A negated match flips a
    /// previously-ignored path back to "not ignored" per gitignore
    /// last-match-wins semantics.
    negated: bool,
    regex: Regex,
}

/// Walk `patterns` in source order against `path` and resolve final
/// ignore state per gitignore last-match-wins semantics. Returns
/// `true` when the path is effectively ignored — i.e. a positive
/// pattern matched and no later negation re-included it.
fn path_is_effectively_ignored(path: &Path, patterns: &[GitignorePattern]) -> bool {
    let mut ignored = false;
    for pattern in patterns {
        if pattern.matches(path) {
            ignored = !pattern.negated;
        }
    }
    ignored
}

impl GitignorePattern {
    fn matches(&self, path: &Path) -> bool {
        if self.has_path_segments {
            // Path-aware match: compare against the relative path with
            // forward slashes (the gitignore spec is `/`-only; on
            // Windows the discovery layer normalises).
            let path_str = path.to_string_lossy();
            // Normalise Windows-style separators.
            let normalised = if path_str.contains('\\') {
                path_str.replace('\\', "/")
            } else {
                path_str.into_owned()
            };
            self.regex.is_match(&normalised)
        } else {
            // Basename match — the pattern can match the file
            // anywhere in the tree.
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| self.regex.is_match(n))
        }
    }
}

fn extract_ignore_patterns(gitignore: &str) -> Vec<GitignorePattern> {
    gitignore
        .lines()
        .filter_map(|raw| {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            // Negation: `!`-prefixed lines re-include a previously
            // ignored path. We keep them as `negated: true` so the
            // last-match-wins resolver can flip the state back.
            let (negated, after_bang) = match line.strip_prefix('!') {
                Some(rest) => (true, rest),
                None => (false, line),
            };
            // Strip the trailing `/` directory marker (we match files,
            // not dirs). Leading `/` (root anchor) is preserved as a
            // signal that the pattern is path-anchored — the regex
            // builder treats the leading-slash form as "match from
            // root only" rather than basename-anywhere.
            let trimmed_dir = after_bang.trim_end_matches('/');
            let (body, root_anchored) = match trimmed_dir.strip_prefix('/') {
                Some(rest) => (rest, true),
                None => (trimmed_dir, false),
            };
            if body.is_empty() {
                return None;
            }
            compile_pattern(body, root_anchored, negated)
        })
        .collect()
}

fn compile_pattern(body: &str, root_anchored: bool, negated: bool) -> Option<GitignorePattern> {
    // A pattern is "path-segmented" when it contains a `/` after the
    // anchor strip — that means it's expressing a path constraint, and
    // the matcher should run against the full relative path.
    let has_path_segments = body.contains('/') || root_anchored;
    let regex = build_regex(body, has_path_segments, root_anchored)?;
    Some(GitignorePattern {
        has_path_segments,
        negated,
        regex,
    })
}

fn build_regex(body: &str, has_path_segments: bool, root_anchored: bool) -> Option<Regex> {
    let mut pattern = String::with_capacity(body.len() + 16);
    pattern.push('^');
    // Path-segmented but NOT root-anchored patterns may match at any
    // depth (gitignore semantics for non-anchored multi-segment
    // patterns). Patterns starting with `**/` already encode this; we
    // add the prefix only when needed.
    if has_path_segments && !root_anchored && !body.starts_with("**/") {
        pattern.push_str("(?:.*/)?");
    }

    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'*' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                    // `**` — zero-or-more path segments. When followed
                    // by `/`, consume the slash too so `**/.env.local`
                    // matches both `.env.local` (zero segments) and
                    // `apps/api/.env.local` (two segments).
                    if i + 2 < bytes.len() && bytes[i + 2] == b'/' {
                        pattern.push_str("(?:.*/)?");
                        i += 3;
                    } else {
                        pattern.push_str(".*");
                        i += 2;
                    }
                } else {
                    // Single `*` — match within one path component.
                    pattern.push_str("[^/]*");
                    i += 1;
                }
            }
            b'?' => {
                pattern.push_str("[^/]");
                i += 1;
            }
            // Regex metachars that need escaping.
            b'.' | b'+' | b'(' | b')' | b'|' | b'[' | b']' | b'{' | b'}' | b'^' | b'$' | b'\\' => {
                pattern.push('\\');
                pattern.push(c as char);
                i += 1;
            }
            _ => {
                pattern.push(c as char);
                i += 1;
            }
        }
    }
    pattern.push('$');
    Regex::new(&pattern).ok()
}

/// Convenience wrapper for callers that have a directory tree on disk
/// and just want to bundle discovery + check. Walks `repo_root` for
/// every `is_env_file` match (non-recursive into ignored directories
/// is the caller's job), reads each file, then runs the check.
///
/// `env_paths` may be repository-relative or absolute. Absolute paths
/// are resolved for reading, then stripped to `repo_root`-relative
/// form before matching and before they appear on findings. An
/// absolute path outside `repo_root` is `InvalidInput`.
pub fn check_gitignore_hygiene_for_paths(
    repo_root: &Path,
    env_paths: &[PathBuf],
) -> std::io::Result<Vec<GitignoreFinding>> {
    let gitignore_path = repo_root.join(".gitignore");
    let gitignore_text = match std::fs::read_to_string(&gitignore_path) {
        Ok(text) => Some(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => return Err(err),
    };

    let mut env_files = Vec::with_capacity(env_paths.len());
    for path in env_paths {
        let absolute = if path.is_absolute() {
            path.clone()
        } else {
            repo_root.join(path)
        };
        // Resolve matching/finding path from the read location, not the
        // raw input, so `repo/../outside.env` cannot be read as outside
        // and then reported as a repo-relative `../outside.env`.
        let relative = repo_relative_env_path(repo_root, &absolute)?;
        // Distinguish "file gone" (discovery list is stale — skip it,
        // surfacing nothing is correct because there's nothing to
        // warn about) from real I/O failures (permission denied,
        // symlink loop). The latter must propagate so callers can
        // surface them to the operator instead of silently treating
        // an unreadable file as empty content (which would also
        // disable any SURFENV-002 header-suppression directive
        // inside it). Council + copilot review both flagged this.
        let content = match std::fs::read_to_string(&absolute) {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(err),
        };
        env_files.push((relative, content));
    }

    Ok(check_gitignore_hygiene(
        &env_files,
        gitignore_text.as_deref(),
    ))
}

/// Convert a resolved env path to the repo-relative form the matcher
/// and `GitignoreFinding.file` expect. Relative paths are kept as-is
/// when they stay inside `repo_root`; absolute paths must sit under
/// `repo_root` with no `..` escape.
fn repo_relative_env_path(repo_root: &Path, path: &Path) -> std::io::Result<PathBuf> {
    let outside = || {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "env path {} is outside repository root {}",
                path.display(),
                repo_root.display()
            ),
        )
    };
    let stripped = if path.is_absolute() {
        path.strip_prefix(repo_root).map_err(|_| outside())?
    } else {
        path
    };
    if stripped
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(outside());
    }
    Ok(stripped.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::{
        GitignoreFindingKind, SURFENV_002_RULE_ID, check_gitignore_hygiene,
        check_gitignore_hygiene_for_paths, is_intentionally_committed,
    };
    use std::path::{Path, PathBuf};

    fn env_file(path: &str, content: &str) -> (PathBuf, String) {
        (PathBuf::from(path), content.to_string())
    }

    #[test]
    fn flags_unignored_env_file() {
        let files = vec![env_file(".env.local", "FOO=bar\n")];
        let findings = check_gitignore_hygiene(&files, Some("node_modules/\n"));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, GitignoreFindingKind::UnignoredEnvFile);
        assert_eq!(findings[0].suggested_pattern, ".env.local");
        assert!(!findings[0].suppressed);
    }

    #[test]
    fn ignored_env_file_does_not_fire() {
        let files = vec![env_file(".env.local", "FOO=bar\n")];
        let gitignore = "node_modules/\n.env*\n";
        let findings = check_gitignore_hygiene(&files, Some(gitignore));
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn anchored_gitignore_pattern_still_matches() {
        let files = vec![env_file(".env", "FOO=bar\n")];
        let gitignore = "/.env\n";
        let findings = check_gitignore_hygiene(&files, Some(gitignore));
        assert!(findings.is_empty());
    }

    #[test]
    fn missing_gitignore_yields_dedicated_kind() {
        let files = vec![env_file(".env", "FOO=bar\n")];
        let findings = check_gitignore_hygiene(&files, None);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, GitignoreFindingKind::MissingGitignore);
    }

    #[test]
    fn intentionally_committed_filenames_are_skipped() {
        assert!(is_intentionally_committed(".env.example"));
        assert!(is_intentionally_committed(".env.sample"));
        assert!(is_intentionally_committed(".env.template"));
        assert!(is_intentionally_committed(".envrc"));
        assert!(!is_intentionally_committed(".env"));
        assert!(!is_intentionally_committed(".env.local"));

        let files = vec![
            env_file(".env.example", "FOO=bar\n"),
            env_file(".envrc", "export FOO=bar\n"),
        ];
        let findings = check_gitignore_hygiene(&files, Some(""));
        assert!(findings.is_empty());
    }

    #[test]
    fn header_directive_suppresses_finding() {
        let directive = format!("# @anvil-ignore {SURFENV_002_RULE_ID} -- frozen replay fixture\n");
        let files = vec![env_file(".env.local", &format!("{directive}FOO=bar\n"))];
        let findings = check_gitignore_hygiene(&files, Some("node_modules/\n"));
        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert!(f.suppressed);
        assert_eq!(
            f.suppression_reason.as_deref(),
            Some("frozen replay fixture")
        );
    }

    #[test]
    fn directive_for_other_rule_does_not_suppress() {
        let files = vec![env_file(
            ".env.local",
            "# @anvil-ignore SURFENV-001 -- unrelated\nFOO=bar\n",
        )];
        let findings = check_gitignore_hygiene(&files, Some("node_modules/\n"));
        assert_eq!(findings.len(), 1);
        assert!(!findings[0].suppressed);
    }

    #[test]
    fn directive_after_header_window_does_not_suppress() {
        // SURFENV-002 directives must appear in the file header — a
        // line buried 30 lines deep can't be the file's "I committed
        // this on purpose" announcement.
        let buried = format!(
            "{}# @anvil-ignore {SURFENV_002_RULE_ID} -- buried\nFOO=bar\n",
            "PRELUDE=value\n".repeat(20),
        );
        let files = vec![env_file(".env.local", &buried)];
        let findings = check_gitignore_hygiene(&files, Some("node_modules/\n"));
        assert_eq!(findings.len(), 1);
        assert!(!findings[0].suppressed);
    }

    #[test]
    fn negation_pattern_does_not_count_as_coverage() {
        // `.gitignore` lines starting with `!` *re-include* paths;
        // they don't ignore them, so they must not satisfy SURFENV-002.
        let files = vec![env_file(".env", "FOO=bar\n")];
        let gitignore = "!.env\n";
        let findings = check_gitignore_hygiene(&files, Some(gitignore));
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn negation_re_includes_path_after_broad_ignore() {
        // Real-world pattern: ignore all `.env*` then re-include
        // `.env.staging` because that one is committed deliberately.
        // The .env.staging IS effectively unignored (the negation
        // wins — last-match-wins gitignore semantics), so SURFENV-002
        // must still fire on it. Copilot review caught the prior
        // matcher silently dropping `!`-lines, which made this case a
        // false negative.
        let files = vec![
            env_file(".env.staging", "FOO=bar\n"),
            env_file(".env.local", "FOO=bar\n"),
        ];
        let gitignore = ".env*\n!.env.staging\n";
        let findings = check_gitignore_hygiene(&files, Some(gitignore));
        assert_eq!(findings.len(), 1, "got {findings:?}");
        assert_eq!(findings[0].file, std::path::Path::new(".env.staging"));
    }

    #[test]
    fn last_match_wins_ordering() {
        // If a later positive pattern follows a negation, the file
        // is again effectively ignored. Confirms last-match-wins.
        let files = vec![env_file(".env.staging", "FOO=bar\n")];
        let gitignore = ".env*\n!.env.staging\n.env.staging\n";
        let findings = check_gitignore_hygiene(&files, Some(gitignore));
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn dot_env_local_example_is_intentionally_committed() {
        // Next.js convention: `.env.local.example` documents the
        // shape of `.env.local` for new contributors. Treat it as a
        // template (no SURFENV-002 finding) even when no broad
        // gitignore covers it. Copilot review flagged the earlier
        // hard-coded allowlist as incomplete here.
        let files = vec![
            env_file(".env.local.example", "FOO=\n"),
            env_file(".env.production.example", "FOO=\n"),
        ];
        let findings = check_gitignore_hygiene(&files, Some(""));
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn dotenv_glob_covers_multiple_envs() {
        let files = vec![
            env_file(".env", "FOO=bar\n"),
            env_file(".env.local", "FOO=bar\n"),
            env_file(".env.production", "FOO=bar\n"),
        ];
        let findings = check_gitignore_hygiene(&files, Some(".env*\n"));
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn double_star_prefix_pattern_covers_basename_at_any_depth() {
        // Council/security/adversarial all flagged this: `**/.env.local`
        // is a real-world monorepo pattern that the prior matcher
        // silently failed on, producing false-positive findings on
        // every package's env file.
        let files = vec![
            env_file("apps/web/.env.local", "FOO=bar\n"),
            env_file("apps/api/.env.local", "FOO=bar\n"),
            env_file(".env.local", "FOO=bar\n"),
        ];
        let findings = check_gitignore_hygiene(&files, Some("**/.env.local\n"));
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn nested_glob_pattern_covers_nested_paths() {
        // `apps/**/.env.local` should match files at any depth under
        // `apps/`, not just direct children. Real-world pattern in
        // Vercel/Next.js monorepos.
        let files = vec![
            env_file("apps/web/.env.local", "FOO=bar\n"),
            env_file("apps/api/v2/.env.local", "FOO=bar\n"),
        ];
        let findings = check_gitignore_hygiene(&files, Some("apps/**/.env.local\n"));
        assert!(findings.is_empty(), "got {findings:?}");

        // Sibling outside apps/ is NOT covered by this pattern.
        let outside = vec![env_file("services/.env.local", "FOO=bar\n")];
        let findings = check_gitignore_hygiene(&outside, Some("apps/**/.env.local\n"));
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn explicit_path_pattern_matches_only_that_path() {
        let files = vec![
            env_file("apps/web/.env.local", "FOO=bar\n"),
            env_file("apps/api/.env.local", "FOO=bar\n"),
        ];
        // Anchored to `apps/web/` only.
        let findings = check_gitignore_hygiene(&files, Some("/apps/web/.env.local\n"));
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].file,
            std::path::Path::new("apps/api/.env.local")
        );
    }

    #[test]
    fn multi_glob_basename_pattern_matches_all_variants() {
        // Council finding: `.env.*.local` (a real Next.js gitignore
        // entry) was a silent false-negative in the prior matcher.
        let files = vec![
            env_file(".env.development.local", "FOO=bar\n"),
            env_file(".env.production.local", "FOO=bar\n"),
        ];
        let findings = check_gitignore_hygiene(&files, Some(".env.*.local\n"));
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn dotenv_dot_star_glob_covers_variants_but_not_bare_env() {
        // `.env.*` matches `.env.local` but not bare `.env`; the
        // pattern matcher strips the `*` and requires the prefix.
        let files = vec![
            env_file(".env", "FOO=bar\n"),
            env_file(".env.local", "FOO=bar\n"),
        ];
        let findings = check_gitignore_hygiene(&files, Some(".env.*\n"));
        // Only `.env` should remain unignored.
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, std::path::Path::new(".env"));
    }

    #[test]
    fn wrapper_absolute_path_matches_root_anchored_gitignore() {
        // The wrapper accepts absolute env paths for reading. Those
        // must be stripped to repo-relative before matching, or a
        // root-anchored rule such as `/.env.local` fails to cover
        // `/tmp/repo/.env.local` and findings leak absolute paths.
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo_root = tmp.path();
        std::fs::write(repo_root.join(".gitignore"), "/.env.local\n").expect("write gitignore");
        std::fs::write(repo_root.join(".env.local"), "FOO=bar\n").expect("write .env.local");
        std::fs::write(repo_root.join(".env.production"), "FOO=bar\n")
            .expect("write .env.production");

        let env_paths = vec![
            repo_root.join(".env.local"),
            repo_root.join(".env.production"),
        ];
        let findings = check_gitignore_hygiene_for_paths(repo_root, &env_paths)
            .expect("wrapper should accept in-repo absolute paths");

        assert_eq!(findings.len(), 1, "got {findings:?}");
        assert_eq!(findings[0].file, Path::new(".env.production"));
        assert!(
            !findings[0].file.is_absolute(),
            "finding path must stay repo-relative, got {}",
            findings[0].file.display()
        );
    }

    #[test]
    fn wrapper_rejects_absolute_path_outside_repo_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo_root = tmp.path();
        std::fs::write(repo_root.join(".gitignore"), "/.env.local\n").expect("write gitignore");

        let outside = tempfile::tempdir().expect("outside dir");
        let outsider = outside.path().join(".env.local");
        std::fs::write(&outsider, "FOO=bar\n").expect("write outside env");

        let err = check_gitignore_hygiene_for_paths(repo_root, &[outsider])
            .expect_err("absolute path outside repo_root must fail");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn wrapper_rejects_parent_dir_escape() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo_root = tmp.path();
        std::fs::write(repo_root.join(".gitignore"), "/.env.local\n").expect("write gitignore");

        let outside = tempfile::tempdir().expect("outside dir");
        let outsider = outside.path().join(".env.local");
        std::fs::write(&outsider, "FOO=bar\n").expect("write outside env");

        let escaped = repo_root
            .join("..")
            .join(outside.path().file_name().expect("outside name"))
            .join(".env.local");
        let err = check_gitignore_hygiene_for_paths(repo_root, &[escaped])
            .expect_err("lexical .. escape must fail");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);

        let relative_escape = PathBuf::from("..")
            .join(outside.path().file_name().expect("outside name"))
            .join(".env.local");
        let err = check_gitignore_hygiene_for_paths(repo_root, &[relative_escape])
            .expect_err("relative .. escape must fail");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }
}
