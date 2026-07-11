//! GBASE-001 merge-base tree reader and base-graph producer (ADR-105 §1).
//!
//! Produces the full symbol graph of a merge-base commit's **committed tree**
//! by reading git objects — never a working tree. A dirty working tree would
//! poison a shared artefact, so the base is read exclusively from committed
//! blobs (`ls-tree -r` + `cat-file --batch`, the `l4_engine.rs` batched
//! object-read pattern reused verbatim, **zero new dependencies**), parsed by
//! tree-sitter through the injected [`KernelSymbolParser`] (ADR-067: the parser
//! links into the CLI binary, not the resident daemon crate — the
//! `daemon_dep_boundary` guard stays green), and assembled into an in-memory
//! base graph via the existing `anvil-graph-cache` insertion/re-resolution
//! path.
//!
//! ## Determinism
//!
//! Same merge-base sha ⇒ identical graph. The file list is sorted before the
//! parse loop, so synthetic-node id allocation is stable; the parser is
//! deterministic; and the re-resolution passes use `BTreeSet` ordering. No
//! wall-clock, no randomness enters the output.
//!
//! ## Scope
//!
//! This is GBASE-001 only: resolve the base commit, walk its tree, parse it,
//! build the graph, and emit a deterministic one-line summary. Persisting the
//! base to a content-addressed store, the proactive ref-watch trigger, and
//! refcount GC are out of scope here.
//!
//! Unix-only, mirroring [`crate::intercept_symbol_parser`]: the parser
//! injection surface it depends on is `cfg(unix)` (the inherited ADR-070
//! Windows gap).
#![cfg(unix)]

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anvil_intercept::save_time::SymbolParser;
use anvil_kernel::graph::{
    SymbolGraph, re_resolve_calls, re_resolve_imports, re_resolve_reexports, update_file,
};
use anvil_kernel::parser::languages::Language;
use anvil_kernel_types::{CallSite, ImportEdge, ReexportEdge};
use serde::Serialize;

use crate::intercept_symbol_parser::KernelSymbolParser;

/// Upper bound on a single committed blob the producer will materialise, in
/// bytes. The `git cat-file --batch` header carries the object size before the
/// body, so an over-cap blob is discarded at parse time and never copied into
/// the returned per-blob buffer. Note the bound this does — and does not —
/// give: `wait_with_output()` still buffers the child's **whole batch stdout**
/// (including any over-cap body) once, so the guard caps the per-blob copy,
/// not the subprocess's peak transcript; a streaming two-phase
/// (`cat-file -s` then fetch) read is a graduation-gate follow-up if
/// whole-tree transcripts prove heavy on large monorepos. An over-cap blob is
/// treated as "skipped" (no symbols), never an error: the base stays a
/// best-effort, non-fatal artefact. 8 MiB comfortably clears any real source
/// file.
const MAX_BLOB_BYTES: u64 = 8 * 1024 * 1024;

/// Deterministic one-line summary of a produced base graph (GBASE-001).
///
/// Serialised as JSON to stdout by the `anvil graph-base build` harness. Every
/// field is a pure function of the merge-base commit's committed tree, so the
/// same sha yields byte-identical JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BaseGraphSummary {
    /// The resolved merge-base commit sha the graph was built from.
    pub merge_base: String,
    /// Distinct source files that contributed at least one symbol.
    pub file_count: usize,
    /// Total resident symbol nodes (parsed symbols plus any synthetic
    /// module/external nodes the graph materialises for edge tracking).
    pub symbol_count: usize,
    /// Total resident graph edges (imports, re-exports, calls).
    pub edge_count: usize,
}

/// A typed producer failure. All variants are **non-fatal** at the call site:
/// ADR-105 §6 mandates "unresolvable merge-base ⇒ skip production, serve cold",
/// so a caller degrades rather than aborts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseGraphError {
    /// No base commit can be resolved — no default branch, no merge-base, or a
    /// detached / base-less topology. Non-fatal: the caller serves cold.
    NoBasePossible(String),
    /// A git plumbing invocation failed (spawn error, non-zero exit, or
    /// unparseable output). `op` names the git operation for triage.
    Git { op: String, detail: String },
    /// The provided or resolved merge-base was not a valid hex commit sha.
    InvalidSha(String),
}

impl std::fmt::Display for BaseGraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BaseGraphError::NoBasePossible(reason) => {
                write!(f, "no merge-base could be resolved: {reason}")
            }
            BaseGraphError::Git { op, detail } => {
                write!(f, "git {op} failed: {detail}")
            }
            BaseGraphError::InvalidSha(sha) => {
                write!(f, "not a valid commit sha: {sha}")
            }
        }
    }
}

impl std::error::Error for BaseGraphError {}

/// A 40- or 64-hex-char lowercase object name (SHA-1 or SHA-256).
fn is_hex_object_name(sha: &str) -> bool {
    (sha.len() == 40 || sha.len() == 64) && sha.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Run a git plumbing command in `repo_root`, returning trimmed stdout on
/// success or a typed [`BaseGraphError::Git`] otherwise.
fn git_stdout(repo_root: &Path, op: &str, args: &[&str]) -> Result<String, BaseGraphError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .map_err(|e| BaseGraphError::Git {
            op: op.to_string(),
            detail: format!("spawn failed: {e}"),
        })?;
    if !output.status.success() {
        return Err(BaseGraphError::Git {
            op: op.to_string(),
            detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Resolve the repository's default branch ref (e.g. `origin/main`).
///
/// Prefers `origin/HEAD` (`git rev-parse --abbrev-ref origin/HEAD`); when that
/// is unset (no remote HEAD symref) falls back to `default_branch` if the
/// caller supplied one and it resolves to a real commit. A completely
/// unresolvable default branch is a [`BaseGraphError::NoBasePossible`] — the
/// non-fatal "serve cold" posture, not a hard error.
pub fn resolve_default_branch(
    repo_root: &Path,
    default_branch: Option<&str>,
) -> Result<String, BaseGraphError> {
    if let Ok(head) = git_stdout(
        repo_root,
        "rev-parse origin/HEAD",
        &["rev-parse", "--abbrev-ref", "origin/HEAD"],
    ) && !head.is_empty()
    {
        return Ok(head);
    }
    // origin/HEAD is unset. Fall back to a configured default branch only when
    // it resolves to a real commit — a configured name that does not exist is
    // no better than no default at all.
    if let Some(branch) = default_branch
        && git_stdout(
            repo_root,
            "rev-parse default branch",
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("{branch}^{{commit}}"),
            ],
        )
        .is_ok()
    {
        return Ok(branch.to_string());
    }
    Err(BaseGraphError::NoBasePossible(
        "origin/HEAD is unset and no resolvable default branch was configured".to_string(),
    ))
}

/// Resolve the merge-base commit sha to build a base graph from.
///
/// - An explicit `merge_base` (an operator-supplied `--merge-base <sha>`) is
///   used directly after validating it names a real commit — it *is* the base
///   key, no branch resolution needed.
/// - Otherwise the default branch is resolved (see [`resolve_default_branch`])
///   and `git merge-base HEAD <default>` gives the base. When the branch has an
///   upstream (`@{upstream}` set), the merge-base against the upstream refines
///   the result — the ADR-105 §6 "`@{upstream}` refinement when set" rule —
///   otherwise default-branch keying stands (covering upstream-less local
///   branches, the majority).
///
/// Every unresolvable path returns [`BaseGraphError::NoBasePossible`]: the base
/// is a best-effort artefact and a caller must be able to serve cold.
pub fn resolve_base_commit(
    repo_root: &Path,
    merge_base: Option<&str>,
    default_branch: Option<&str>,
) -> Result<String, BaseGraphError> {
    if let Some(explicit) = merge_base {
        if !is_hex_object_name(explicit) {
            return Err(BaseGraphError::InvalidSha(explicit.to_string()));
        }
        // Confirm the sha names a real commit in this repo before trusting it.
        git_stdout(
            repo_root,
            "rev-parse explicit merge-base",
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("{explicit}^{{commit}}"),
            ],
        )
        .map_err(|_| {
            BaseGraphError::NoBasePossible(format!(
                "explicit merge-base {explicit} does not name a commit in this repository"
            ))
        })?;
        return Ok(explicit.to_string());
    }

    let default_ref = resolve_default_branch(repo_root, default_branch)?;
    let base = git_stdout(
        repo_root,
        "merge-base",
        &["merge-base", "HEAD", &default_ref],
    )
    .map_err(|e| BaseGraphError::NoBasePossible(format!("HEAD..{default_ref}: {e}")))?;
    if base.is_empty() {
        return Err(BaseGraphError::NoBasePossible(format!(
            "HEAD and {default_ref} share no merge-base"
        )));
    }

    // `@{upstream}` refinement, only when the branch actually tracks one.
    if let Ok(upstream) = git_stdout(
        repo_root,
        "rev-parse @{upstream}",
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
    ) && !upstream.is_empty()
        && let Ok(refined) = git_stdout(
            repo_root,
            "merge-base @{upstream}",
            &["merge-base", "HEAD", &upstream],
        )
        && !refined.is_empty()
    {
        return Ok(refined);
    }

    Ok(base)
}

/// A committed tree entry the producer cares about: a source blob's path and
/// object id.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeBlob {
    path: String,
    oid: String,
}

/// Enumerate the committed tree at `sha` via `git ls-tree -r -z`, keeping only
/// blob entries whose extension the kernel parser supports.
///
/// `-z` NUL-delimits records so paths containing newlines or spaces survive
/// intact. Each record is `<mode> SP <type> SP <oid> TAB <path>`; gitlink
/// (submodule) entries are `commit`-typed and dropped. The returned list is
/// sorted by path for deterministic downstream id allocation.
fn enumerate_tree(repo_root: &Path, sha: &str) -> Result<Vec<TreeBlob>, BaseGraphError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["ls-tree", "-r", "-z", sha])
        .output()
        .map_err(|e| BaseGraphError::Git {
            op: "ls-tree".to_string(),
            detail: format!("spawn failed: {e}"),
        })?;
    if !output.status.success() {
        return Err(BaseGraphError::Git {
            op: "ls-tree".to_string(),
            detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    let mut blobs = Vec::new();
    for record in output.stdout.split(|&b| b == 0) {
        if record.is_empty() {
            continue;
        }
        // Split meta (before TAB) from path (after TAB). A TAB in a path is
        // impossible in ls-tree's `-z` output — the first TAB is the delimiter.
        let Some(tab) = record.iter().position(|&b| b == b'\t') else {
            continue;
        };
        let meta = std::str::from_utf8(&record[..tab]).map_err(|e| BaseGraphError::Git {
            op: "ls-tree".to_string(),
            detail: format!("non-utf8 tree metadata: {e}"),
        })?;
        // Path bytes may be non-UTF-8 in theory; such a path can never match a
        // supported extension we care about, so a lossy conversion is safe here
        // and only used for parser/path identity.
        let path = String::from_utf8_lossy(&record[tab + 1..]).into_owned();

        // meta = `<mode> <type> <oid>`.
        let mut fields = meta.split(' ');
        let (Some(mode), Some(obj_type), Some(oid)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if obj_type != "blob" {
            continue; // gitlink / tree — not source content.
        }
        if mode == "120000" {
            // A symlink blob's body is the link TARGET PATH, not source
            // content — never base content, even when the link name carries a
            // supported extension.
            continue;
        }
        if Language::from_path(Path::new(&path)).is_none() {
            continue; // unsupported extension / binary / docs — skipped.
        }
        blobs.push(TreeBlob {
            path,
            oid: oid.to_string(),
        });
    }
    blobs.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(blobs)
}

/// Batch-read committed blob bodies by object id through a single
/// `git cat-file --batch` child process (the `l4_engine.rs` streaming pattern:
/// a writer thread feeds oids while the reader drains length-prefixed bodies,
/// so a full stdout pipe cannot deadlock a full stdin pipe).
///
/// Returns a vec aligned with `oids`. A slot is `None` when the object is
/// missing, is not a blob, or exceeds [`MAX_BLOB_BYTES`] (the size guard, read
/// from the batch header before the body is buffered). A batch-level framing or
/// I/O failure is a typed [`BaseGraphError::Git`].
fn read_blobs_batch(
    repo_root: &Path,
    oids: &[&str],
) -> Result<Vec<Option<Vec<u8>>>, BaseGraphError> {
    if oids.is_empty() {
        return Ok(Vec::new());
    }

    let mut child = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| BaseGraphError::Git {
            op: "cat-file --batch".to_string(),
            detail: format!("spawn failed: {e}"),
        })?;

    let queries: Vec<String> = oids.iter().map(|oid| format!("{oid}\n")).collect();
    let expected = queries.len();
    let mut stdin = child.stdin.take().expect("piped stdin");
    let writer = std::thread::spawn(move || {
        for q in &queries {
            if stdin.write_all(q.as_bytes()).is_err() {
                break;
            }
        }
        drop(stdin);
    });

    let output_res = child.wait_with_output();
    let _ = writer.join();
    let output = output_res.map_err(|e| BaseGraphError::Git {
        op: "cat-file --batch".to_string(),
        detail: format!("wait failed: {e}"),
    })?;
    if !output.status.success() {
        return Err(BaseGraphError::Git {
            op: "cat-file --batch".to_string(),
            detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    parse_batch_stdout(&output.stdout, expected).ok_or_else(|| BaseGraphError::Git {
        op: "cat-file --batch (parse)".to_string(),
        detail: "unparseable --batch stream framing".to_string(),
    })
}

/// Parse the streaming `git cat-file --batch` stdout into `expected` entries.
///
/// Adapted from `l4_engine::parse_batch_stdout`. Each record is either a hit
/// header `<oid> SP <type> SP <size> LF <size bytes> LF` or a miss `<oid> SP
/// missing LF`. A blob within [`MAX_BLOB_BYTES`] is returned; a miss, a
/// non-blob type, or an over-cap blob is `None` (its body still consumed to
/// keep the cursor aligned). Any framing error returns `None` for the whole
/// batch — the caller degrades to a typed error rather than reading garbage as
/// source bytes.
fn parse_batch_stdout(stdout: &[u8], expected: usize) -> Option<Vec<Option<Vec<u8>>>> {
    let mut out: Vec<Option<Vec<u8>>> = Vec::with_capacity(expected);
    let mut cursor = 0usize;
    while out.len() < expected {
        let rel = stdout.get(cursor..)?.iter().position(|&b| b == b'\n')?;
        let header = &stdout[cursor..cursor + rel];
        cursor += rel + 1;
        let header_str = std::str::from_utf8(header).ok()?;
        if header_str.ends_with(" missing") || header_str.ends_with(" ambiguous") {
            out.push(None);
            continue;
        }
        // Hit form: exactly three whitespace-free space-separated fields.
        let parts: Vec<&str> = header_str.split(' ').collect();
        if parts.len() != 3 {
            return None;
        }
        let obj_type = parts[1];
        let size: usize = parts[2].parse().ok()?;
        let body_end = cursor.checked_add(size)?;
        if body_end > stdout.len() {
            return None;
        }
        if obj_type == "blob" && (size as u64) <= MAX_BLOB_BYTES {
            out.push(Some(stdout[cursor..body_end].to_vec()));
        } else {
            // Non-blob or over-cap: discard the body, keep the cursor aligned.
            out.push(None);
        }
        cursor = body_end;
        if stdout.get(cursor) != Some(&b'\n') {
            return None;
        }
        cursor += 1;
    }
    Some(out)
}

/// Build the base symbol graph of the merge-base commit `sha`'s committed tree
/// and return its deterministic summary.
///
/// Walks the committed tree (`enumerate_tree`), batch-reads the blobs
/// (`read_blobs_batch`), parses each supported file through the injected
/// [`KernelSymbolParser`], and assembles the graph via the `anvil-graph-cache`
/// `update_file` insertion path plus the forward-reference re-resolution passes
/// (`re_resolve_imports`/`re_resolve_reexports`/`re_resolve_calls`) — exactly
/// the cold-scan build the daemon's warm path mirrors. Never touches the
/// working tree.
pub fn build_base_graph(repo_root: &Path, sha: &str) -> Result<BaseGraphSummary, BaseGraphError> {
    if !is_hex_object_name(sha) {
        return Err(BaseGraphError::InvalidSha(sha.to_string()));
    }
    let blobs = enumerate_tree(repo_root, sha)?;
    let oids: Vec<&str> = blobs.iter().map(|b| b.oid.as_str()).collect();
    let bodies = read_blobs_batch(repo_root, &oids)?;

    let parser = KernelSymbolParser::new();
    let mut graph = SymbolGraph::new();
    // Forward-reference accumulators: a file that imports/calls a target parsed
    // later cannot resolve at insert time, so re-resolve over the full set once
    // every file is resident (the daemon cold-path contract).
    let mut all_imports: Vec<ImportEdge> = Vec::new();
    let mut all_reexports: Vec<ReexportEdge> = Vec::new();
    let mut all_calls: Vec<(String, CallSite)> = Vec::new();

    for (blob, body) in blobs.iter().zip(bodies) {
        let Some(bytes) = body else { continue };
        let Some(file_symbols) = parser.parse(Path::new(&blob.path), &bytes) else {
            continue; // unsupported/unparseable/non-utf8 → safely skipped.
        };
        all_imports.extend(file_symbols.imports.iter().cloned());
        all_reexports.extend(file_symbols.reexports.iter().cloned());
        for call in &file_symbols.calls {
            all_calls.push((file_symbols.file.clone(), call.clone()));
        }
        update_file(&mut graph, file_symbols);
    }

    re_resolve_imports(&mut graph, &all_imports);
    re_resolve_reexports(&mut graph, &all_reexports);
    re_resolve_calls(&mut graph, &all_calls);

    let stats = graph.stats();
    Ok(BaseGraphSummary {
        merge_base: sha.to_string(),
        file_count: stats.files,
        symbol_count: stats.node_count,
        edge_count: stats.edge_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Run a git command in `root`, asserting success.
    fn git(root: &Path, args: &[&str]) -> std::process::Output {
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .expect("git available");
        assert!(out.status.success(), "git {args:?} failed: {out:?}");
        out
    }

    /// Initialise a fresh repo with deterministic identity/config so commit
    /// shas depend only on content + the fixed author, and gpg signing can
    /// never block the commit in a sandbox.
    fn init_repo() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["config", "user.email", "test@example.com"]);
        git(&root, &["config", "user.name", "Test"]);
        git(&root, &["config", "commit.gpgsign", "false"]);
        (tmp, root)
    }

    /// Write `path` (creating parents) with `content` under `root`.
    fn write_file(root: &Path, path: &str, content: &[u8]) {
        let full = root.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(full, content).unwrap();
    }

    fn head_sha(root: &Path) -> String {
        String::from_utf8(git(root, &["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_string()
    }

    /// A two-file TypeScript fixture with one cross-file relative import
    /// (`a.ts` imports `b.ts`), committed. Returns `(tmp, root, sha)`.
    fn commit_two_file_ts_fixture() -> (TempDir, PathBuf, String) {
        let (tmp, root) = init_repo();
        write_file(
            &root,
            "src/a.ts",
            b"import { b } from './b';\nexport function a() { return b(); }\n",
        );
        write_file(&root, "src/b.ts", b"export function b() { return 2; }\n");
        git(&root, &["add", "."]);
        git(&root, &["commit", "-q", "-m", "fixture"]);
        let sha = head_sha(&root);
        (tmp, root, sha)
    }

    /// (a) A known committed tree builds a deterministic base graph: exact
    /// symbol/edge/file counts, and two runs over the same sha produce
    /// byte-identical summaries. The counts are pinned so a regression in the
    /// tree walk, parse, or edge-resolution passes is visible.
    #[test]
    fn builds_deterministic_base_graph_from_committed_tree() {
        let (_tmp, root, sha) = commit_two_file_ts_fixture();

        let first = build_base_graph(&root, &sha).expect("base graph builds");
        let second = build_base_graph(&root, &sha).expect("base graph builds again");

        assert_eq!(
            first, second,
            "same sha must produce an identical summary (determinism)"
        );
        assert_eq!(first.merge_base, sha);
        // Two source files, each contributing one exported function.
        assert_eq!(first.file_count, 2, "summary: {first:?}");
        // The two exported functions `a` and `b`.
        assert_eq!(first.symbol_count, 2, "summary: {first:?}");
        // Two resident edges: the `a.ts -> b.ts` relative import, and the
        // `a() -> b()` call edge that resolves through it.
        assert_eq!(first.edge_count, 2, "summary: {first:?}");
    }

    /// (b) The core "never reads a working tree" guarantee: editing a file on
    /// disk *after* the commit must not change the base graph, because the
    /// producer reads the committed blob, not the working tree. The edit adds a
    /// whole new function and a new file; if the working tree leaked in,
    /// `symbol_count`/`file_count` would rise.
    #[test]
    fn ignores_dirty_working_tree() {
        let (_tmp, root, sha) = commit_two_file_ts_fixture();
        let committed = build_base_graph(&root, &sha).expect("base graph builds");

        // Dirty the working tree: add an uncommitted exported function.
        write_file(
            &root,
            "src/b.ts",
            b"export function b() { return 2; }\nexport function sneaky() { return 9; }\n",
        );
        // And an entirely new, uncommitted source file.
        write_file(&root, "src/c.ts", b"export function c() { return 3; }\n");

        let after_edit = build_base_graph(&root, &sha).expect("base graph builds");
        assert_eq!(
            committed, after_edit,
            "the base graph must reflect the committed blob, never the working tree"
        );
    }

    /// (c) An unresolvable default branch is a typed `NoBasePossible`, never a
    /// panic. A fresh repo with a commit but no `origin/HEAD` and no configured
    /// default cannot resolve a base.
    #[test]
    fn unresolvable_default_branch_is_typed_error_not_panic() {
        let (_tmp, root) = init_repo();
        write_file(&root, "src/a.ts", b"export function a() {}\n");
        git(&root, &["add", "."]);
        git(&root, &["commit", "-q", "-m", "one"]);

        let err = resolve_base_commit(&root, None, None)
            .expect_err("no origin/HEAD, no default → NoBasePossible");
        assert!(
            matches!(err, BaseGraphError::NoBasePossible(_)),
            "expected NoBasePossible, got {err:?}"
        );
    }

    /// (c') A configured default branch that *does* resolve lets the base
    /// resolve via `merge-base HEAD <default>`. Here the default branch is the
    /// current branch itself, so the merge-base is HEAD.
    #[test]
    fn configured_default_branch_resolves_merge_base() {
        let (_tmp, root) = init_repo();
        write_file(&root, "src/a.ts", b"export function a() {}\n");
        git(&root, &["add", "."]);
        git(&root, &["commit", "-q", "-m", "one"]);
        let head = head_sha(&root);

        let base = resolve_base_commit(&root, None, Some("main"))
            .expect("configured default branch resolves");
        assert_eq!(base, head, "merge-base of HEAD and main (==HEAD) is HEAD");
    }

    /// (c'') An explicit `--merge-base <sha>` short-circuits branch resolution
    /// but is validated against the repo; a well-formed sha that names no
    /// commit degrades to `NoBasePossible`, and a malformed one to
    /// `InvalidSha`.
    #[test]
    fn explicit_merge_base_is_validated() {
        let (_tmp, root, sha) = commit_two_file_ts_fixture();

        assert_eq!(
            resolve_base_commit(&root, Some(&sha), None).expect("real sha accepted"),
            sha
        );
        assert!(matches!(
            resolve_base_commit(&root, Some("HEAD"), None),
            Err(BaseGraphError::InvalidSha(_))
        ));
        assert!(matches!(
            resolve_base_commit(&root, Some(&"a".repeat(40)), None),
            Err(BaseGraphError::NoBasePossible(_))
        ));
    }

    /// (d) Unsupported and binary files are skipped: only the parser-supported
    /// source blob contributes to the graph. A markdown doc, a committed binary
    /// blob, and a lockfile must leave no trace in the summary.
    #[test]
    fn skips_unsupported_and_binary_files() {
        let (_tmp, root) = init_repo();
        write_file(&root, "src/only.ts", b"export function only() {}\n");
        write_file(&root, "README.md", b"# docs\n");
        write_file(&root, "pnpm-lock.yaml", b"lockfileVersion: 9\n");
        write_file(&root, "logo.bin", b"\x00\x01\x02\xff\xfe binary \x00");
        // A committed symlink whose NAME carries a supported extension: its
        // blob body is the link target path, not source content — the mode
        // filter (120000) must skip it before the parser ever sees it.
        std::os::unix::fs::symlink("src/only.ts", root.join("alias.ts")).unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-q", "-m", "mixed"]);
        let sha = head_sha(&root);

        let summary = build_base_graph(&root, &sha).expect("base graph builds");
        assert_eq!(
            summary.file_count, 1,
            "only the real .ts file is parseable; the symlink blob is skipped: {summary:?}"
        );
        assert_eq!(summary.symbol_count, 1, "just `only`: {summary:?}");
        assert_eq!(summary.edge_count, 0, "no imports: {summary:?}");
    }

    /// The tree walk itself keeps only supported blobs and sorts by path, so
    /// the parse order (and thus synthetic-id allocation) is deterministic.
    #[test]
    fn enumerate_tree_filters_and_sorts() {
        let (_tmp, root) = init_repo();
        write_file(&root, "z.ts", b"export const z = 1;\n");
        write_file(&root, "a.ts", b"export const a = 1;\n");
        write_file(&root, "notes.txt", b"ignore me\n");
        git(&root, &["add", "."]);
        git(&root, &["commit", "-q", "-m", "sort"]);
        let sha = head_sha(&root);

        let blobs = enumerate_tree(&root, &sha).expect("enumerate");
        let paths: Vec<&str> = blobs.iter().map(|b| b.path.as_str()).collect();
        assert_eq!(paths, vec!["a.ts", "z.ts"], "sorted, .txt filtered out");
    }

    /// (e) Warm-start-latency acceptance criterion (ADR-105 §11): base load
    /// sits on the cold-start critical path, so it must complete within a
    /// budget. This asserts a **fixture-scale** upper bound rather than a
    /// precise SLA — a deterministic fixture is worth more than flaky
    /// wall-clock precision. The budget is deliberately generous (5 s) to
    /// absorb cold tree-sitter grammar init, git process spawns, and loaded CI
    /// runners; it is a coarse regression tripwire that catches an accidental
    /// O(n^2) walk or a per-file git spawn, not a performance gate. A real
    /// N-worktree latency budget lands with the §11 graduation gate.
    #[test]
    fn base_build_meets_fixture_latency_budget() {
        // A small multi-file fixture: enough to exercise the batch read and the
        // re-resolution passes, cheap enough to stay well under budget.
        let (_tmp, root) = init_repo();
        for i in 0..12 {
            write_file(
                &root,
                &format!("src/mod{i}.ts"),
                format!("export function f{i}() {{ return {i}; }}\n").as_bytes(),
            );
        }
        git(&root, &["add", "."]);
        git(&root, &["commit", "-q", "-m", "many"]);
        let sha = head_sha(&root);

        let start = std::time::Instant::now();
        let summary = build_base_graph(&root, &sha).expect("base graph builds");
        let elapsed = start.elapsed();

        assert_eq!(summary.file_count, 12, "summary: {summary:?}");
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "fixture base build took {elapsed:?}, over the 5s coarse budget",
        );
    }

    /// A blob over the size guard is skipped, not buffered or errored — the
    /// `parse_batch_stdout` cap holds and keeps subsequent records aligned.
    #[test]
    fn parse_batch_skips_over_cap_blob_but_stays_aligned() {
        // First record: a "blob" whose declared size exceeds MAX_BLOB_BYTES.
        // Its body is present in the stream and must be consumed, not returned.
        let over = MAX_BLOB_BYTES + 1;
        let big_body = vec![b'x'; usize::try_from(over).unwrap()];
        let mut stream = format!("deadbeef blob {over}\n").into_bytes();
        stream.extend_from_slice(&big_body);
        stream.push(b'\n');
        stream.extend_from_slice(b"cafebabe blob 2\nok\n");

        let parsed = parse_batch_stdout(&stream, 2).expect("framed correctly");
        assert_eq!(parsed.len(), 2);
        assert!(parsed[0].is_none(), "over-cap blob skipped");
        assert_eq!(parsed[1].as_deref(), Some(b"ok".as_ref()));
    }

    /// A missing object surfaces as `None` in its slot without derailing the
    /// batch — the alignment invariant the graph build relies on.
    #[test]
    fn read_blobs_batch_reports_missing_object() {
        let (_tmp, root, _sha) = commit_two_file_ts_fixture();
        // A real oid alongside a bogus one.
        let real = String::from_utf8(git(&root, &["rev-parse", "HEAD:src/b.ts"]).stdout)
            .unwrap()
            .trim()
            .to_string();
        let bogus = "0".repeat(40);
        let bodies = read_blobs_batch(&root, &[real.as_str(), bogus.as_str()]).unwrap();
        assert_eq!(bodies.len(), 2);
        assert!(bodies[0].is_some(), "real blob resolves");
        assert!(bodies[1].is_none(), "bogus oid is a miss");
    }
}
