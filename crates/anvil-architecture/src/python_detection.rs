//! Python entry-point detection (PYLAN-005).
//!
//! Declared scripts (PEP 621 / setup.cfg / best-effort setup.py) and
//! `__main__` guards → [`EntryPoint`]s. No import resolution (PYLAN-006).

use std::path::Path;

use crate::types::{DetectionConfidence, EntryPoint, EntryPointType};
use crate::util::relative_slash;

/// Detect Python entry points under `workspace_root`.
///
/// Combines declared script targets (`pyproject.toml`, `setup.cfg`, `setup.py`)
/// with `__main__`-guarded modules discovered by a bounded walk that skips
/// VCS, build, and virtualenv/cache directories. Returns entry points with
/// workspace-root-relative, forward-slash paths, sorted and de-duplicated by
/// path (when a file is reached from more than one source, the most
/// informative — a declared script over a bare `__main__` guard — wins).
#[must_use]
pub fn detect_python_entry_points(workspace_root: &Path) -> Vec<EntryPoint> {
    // (rank, EntryPoint) — rank breaks ties at the same path so dedup keeps the
    // most informative source deterministically: declared scripts (0) over
    // best-effort setup.py (1) over a bare `__main__` guard (2).
    let mut ranked: Vec<(u8, EntryPoint)> = Vec::new();

    collect_pyproject_scripts(workspace_root, &mut ranked);
    collect_setup_cfg_scripts(workspace_root, &mut ranked);
    collect_setup_py_scripts(workspace_root, &mut ranked);
    collect_main_guards(workspace_root, &mut ranked);

    // Deterministic order: by path, then by rank (lowest rank — most
    // informative — survives dedup), then by a total tiebreak on the remaining
    // fields so two distinct entries at the same path+rank (e.g. the same file
    // declared in both `[project.scripts]` and `[project.gui-scripts]`) always
    // order the same way regardless of insertion order or sort stability — no
    // baseline churn across toolchains.
    ranked.sort_by(|a, b| {
        a.1.path
            .cmp(&b.1.path)
            .then(a.0.cmp(&b.0))
            .then_with(|| entry_type_ord(&a.1.entry_type).cmp(&entry_type_ord(&b.1.entry_type)))
            .then_with(|| a.1.exports.cmp(&b.1.exports))
    });
    ranked.dedup_by(|a, b| a.1.path == b.1.path);
    ranked.into_iter().map(|(_, ep)| ep).collect()
}

/// Stable ordinal for tiebreaking entry points at the same path+rank, so the
/// dedup survivor is deterministic. `EntryPointType` is not `Ord`; this gives a
/// fixed order without imposing a semantic ranking on the type elsewhere.
fn entry_type_ord(t: &EntryPointType) -> u8 {
    match t {
        EntryPointType::Package => 0,
        EntryPointType::Application => 1,
        EntryPointType::Http => 2,
        EntryPointType::Api => 3,
        EntryPointType::Cli => 4,
        EntryPointType::Worker => 5,
        EntryPointType::Test => 6,
        EntryPointType::Unknown => 7,
    }
}

/// A parsed `name = module[:object] [extras]` entry-point target, reduced to
/// the importable module path and the optional attribute (its `exports`).
struct EntryTarget {
    module: String,
    object: Option<String>,
}

/// Parse the right-hand side of an entry-point line (`module:object [extras]`).
///
/// Returns `None` unless `module` is a dotted chain of ASCII Python identifiers
/// and `object` (if present) is likewise dotted identifiers — this rejects
/// malformed targets (`..secret`, `pkg..cli`, `mod:a:b`) before they reach the
/// filesystem, so no `//`-bearing or workspace-escaping path is ever probed or
/// emitted. The `[extras]` suffix and any trailing whitespace are stripped; the
/// `object` (after `:`) becomes the entry's single export.
fn parse_entry_target(value: &str) -> Option<EntryTarget> {
    // Drop a PEP 508 `[extra]` suffix and anything after whitespace.
    let value = value.trim();
    let value = value.split('[').next().unwrap_or(value).trim();
    let value = value.split_whitespace().next().unwrap_or("").trim();
    if value.is_empty() {
        return None;
    }
    let (module, object) = match value.split_once(':') {
        Some((m, o)) => {
            let o = o.trim();
            (m.trim(), (!o.is_empty()).then(|| o.to_string()))
        }
        None => (value, None),
    };
    if !is_dotted_identifier(module) {
        return None;
    }
    if let Some(obj) = &object
        && !is_dotted_identifier(obj)
    {
        return None;
    }
    Some(EntryTarget {
        module: module.to_string(),
        object,
    })
}

/// Whether `s` is a non-empty `.`-separated chain of ASCII Python identifiers
/// (`[A-Za-z_][A-Za-z0-9_]*`). Used to gate manifest-declared module / object
/// references before any path construction.
fn is_dotted_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.split('.').all(|seg| {
            let mut chars = seg.chars();
            match chars.next() {
                Some(c) if c == '_' || c.is_ascii_alphabetic() => {
                    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
                }
                _ => false,
            }
        })
}

/// Resolve a dotted module path to an existing `.py` file under the workspace,
/// trying the flat layout then the `src/` layout, module-as-file then
/// module-as-package (`__init__.py`). Returns the workspace-relative path of
/// the first candidate that exists on disk.
fn resolve_module_file(workspace_root: &Path, module: &str) -> Option<String> {
    let rel = module.replace('.', "/");
    let candidates = [
        format!("{rel}.py"),
        format!("{rel}/__init__.py"),
        format!("src/{rel}.py"),
        format!("src/{rel}/__init__.py"),
    ];
    for candidate in candidates {
        let abs = workspace_root.join(&candidate);
        if abs.is_file()
            && let Some(slashed) = relative_slash(workspace_root, &abs)
        {
            return Some(slashed);
        }
    }
    None
}

/// Push a declared script target if it resolves to a file on disk.
fn push_script(
    workspace_root: &Path,
    value: &str,
    entry_type: EntryPointType,
    confidence: DetectionConfidence,
    rank: u8,
    out: &mut Vec<(u8, EntryPoint)>,
) {
    let Some(target) = parse_entry_target(value) else {
        return;
    };
    let Some(path) = resolve_module_file(workspace_root, &target.module) else {
        return;
    };
    out.push((
        rank,
        EntryPoint {
            path,
            entry_type,
            confidence,
            exports: target.object.map(|o| vec![o]),
        },
    ));
}

// =============================================================================
// pyproject.toml — PEP 621 `[project.scripts]` / `[project.gui-scripts]`
// =============================================================================

#[derive(serde::Deserialize)]
struct PyProject {
    project: Option<PyProjectProject>,
}

#[derive(serde::Deserialize)]
struct PyProjectProject {
    #[serde(default)]
    scripts: std::collections::BTreeMap<String, String>,
    #[serde(default, rename = "gui-scripts")]
    gui_scripts: std::collections::BTreeMap<String, String>,
}

fn collect_pyproject_scripts(workspace_root: &Path, out: &mut Vec<(u8, EntryPoint)>) {
    let Ok(text) = std::fs::read_to_string(workspace_root.join("pyproject.toml")) else {
        return;
    };
    let Ok(parsed) = toml::from_str::<PyProject>(&text) else {
        return;
    };
    let Some(project) = parsed.project else {
        return;
    };
    for value in project.scripts.values() {
        push_script(
            workspace_root,
            value,
            EntryPointType::Cli,
            DetectionConfidence::High,
            0,
            out,
        );
    }
    for value in project.gui_scripts.values() {
        push_script(
            workspace_root,
            value,
            EntryPointType::Application,
            DetectionConfidence::High,
            0,
            out,
        );
    }
}

// =============================================================================
// setup.cfg — `[options.entry_points]` console_scripts / gui_scripts
// =============================================================================

/// Parse `setup.cfg`'s `[options.entry_points]` section. The section value is
/// an INI key whose body is an indented list of `name = module:object` lines:
///
/// ```ini
/// [options.entry_points]
/// console_scripts =
///     foo = pkg.cli:main
/// gui_scripts =
///     bar = pkg.gui:main
/// ```
fn collect_setup_cfg_scripts(workspace_root: &Path, out: &mut Vec<(u8, EntryPoint)>) {
    let Ok(text) = std::fs::read_to_string(workspace_root.join("setup.cfg")) else {
        return;
    };

    let mut in_entry_points = false;
    // Which list we are accumulating into (Cli for console_scripts, Application
    // for gui_scripts), set when we see the key, cleared at the next key/section.
    let mut current: Option<EntryPointType> = None;

    for raw in text.lines() {
        let line = raw.trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        // Section header.
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_entry_points = trimmed == "[options.entry_points]";
            current = None;
            continue;
        }
        if !in_entry_points {
            continue;
        }

        // A non-indented line inside the section is a key (`console_scripts =`).
        let is_indented = line.starts_with(' ') || line.starts_with('\t');
        if !is_indented {
            current = match trimmed.split('=').next().map(str::trim) {
                Some("console_scripts") => Some(EntryPointType::Cli),
                Some("gui_scripts") => Some(EntryPointType::Application),
                _ => None,
            };
            // An inline `console_scripts = foo = pkg:main` is malformed for the
            // list form; the indented-entry form is what setuptools documents.
            continue;
        }

        // An indented `name = module:object` entry under the active key.
        if let Some(entry_type) = current.clone()
            && let Some((_name, value)) = trimmed.split_once('=')
        {
            let confidence = DetectionConfidence::High;
            push_script(workspace_root, value, entry_type, confidence, 0, out);
        }
    }
}

// =============================================================================
// setup.py — best-effort `console_scripts` / `gui_scripts` string literals
// =============================================================================

/// Best-effort extraction of console/GUI scripts from `setup.py`. setup.py is
/// arbitrary Python and we never execute it, so this tokenises the source
/// (skipping `#` comments and treating string contents as opaque) and looks for
/// the documented `entry_points` shape — a `console_scripts` / `gui_scripts`
/// key followed by a `[ ... ]` list of `"name = module:object"` strings —
/// assigning [`Medium`](DetectionConfidence::Medium) confidence. Because
/// comments and string contents are tokenised away, a commented-out example or
/// a `]`/`[` inside a string can neither hijack the key match nor truncate the
/// list. A target that does not resolve to a file is dropped.
fn collect_setup_py_scripts(workspace_root: &Path, out: &mut Vec<(u8, EntryPoint)>) {
    let Ok(text) = std::fs::read_to_string(workspace_root.join("setup.py")) else {
        return;
    };
    let tokens = tokenize_py(&text);

    // Find each `console_scripts` / `gui_scripts` key (bare word or string key),
    // then collect the string literals of the `[...]` list that follows it.
    let mut i = 0;
    while i < tokens.len() {
        let entry_type = match &tokens[i] {
            PyTok::Ident(w) | PyTok::Str(w) if w == "console_scripts" => Some(EntryPointType::Cli),
            PyTok::Ident(w) | PyTok::Str(w) if w == "gui_scripts" => {
                Some(EntryPointType::Application)
            }
            _ => None,
        };
        let Some(entry_type) = entry_type else {
            i += 1;
            continue;
        };
        // Advance to the opening `[` that introduces the list (skipping the
        // `:`/`=` separator). Bail if anything other than a separator intervenes.
        let mut j = i + 1;
        while matches!(&tokens.get(j), Some(PyTok::Punct(':' | '='))) {
            j += 1;
        }
        if !matches!(tokens.get(j), Some(PyTok::Punct('['))) {
            i += 1;
            continue;
        }
        // Collect string literals until the matching `]` (bracket-depth aware).
        let mut depth = 1usize;
        j += 1;
        while j < tokens.len() && depth > 0 {
            match &tokens[j] {
                PyTok::Punct('[') => depth += 1,
                PyTok::Punct(']') => depth -= 1,
                PyTok::Str(literal) if depth == 1 => {
                    if let Some((_name, value)) = literal.split_once('=') {
                        push_script(
                            workspace_root,
                            value,
                            entry_type.clone(),
                            DetectionConfidence::Medium,
                            1,
                            out,
                        );
                    }
                }
                _ => {}
            }
            j += 1;
        }
        i = j;
    }
}

/// A coarse Python token: enough to scope entry-point extraction without a real
/// parser. String contents are captured opaquely; comments and whitespace are
/// dropped.
enum PyTok {
    Ident(String),
    Str(String),
    Punct(char),
}

/// Tokenise Python source coarsely: `#` comments to end-of-line are dropped,
/// single/double-quoted strings (including triple-quoted) become [`PyTok::Str`]
/// with their raw inner text, `[`/`]`/`{`/`}`/`:`/`=` become [`PyTok::Punct`],
/// and identifier-ish runs become [`PyTok::Ident`]. Everything else is skipped.
/// No escape handling — entry-point strings never contain escaped quotes.
fn tokenize_py(text: &str) -> Vec<PyTok> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '"' || c == '\'' {
            // Triple-quoted?
            let triple = i + 2 < chars.len() && chars[i + 1] == c && chars[i + 2] == c;
            let (delim_len, start) = if triple { (3, i + 3) } else { (1, i + 1) };
            let mut j = start;
            let mut content = String::new();
            while j < chars.len() {
                let matches_delim = if triple {
                    j + 2 < chars.len() && chars[j] == c && chars[j + 1] == c && chars[j + 2] == c
                } else {
                    chars[j] == c
                };
                if matches_delim {
                    break;
                }
                content.push(chars[j]);
                j += 1;
            }
            out.push(PyTok::Str(content));
            i = j + delim_len;
        } else if c == '_' || c.is_alphanumeric() {
            let mut word = String::new();
            while i < chars.len() && (chars[i] == '_' || chars[i].is_alphanumeric()) {
                word.push(chars[i]);
                i += 1;
            }
            out.push(PyTok::Ident(word));
        } else {
            if matches!(c, '[' | ']' | '{' | '}' | ':' | '=') {
                out.push(PyTok::Punct(c));
            }
            i += 1;
        }
    }
    out
}

// =============================================================================
// `if __name__ == "__main__":` guard scan
// =============================================================================

fn collect_main_guards(workspace_root: &Path, out: &mut Vec<(u8, EntryPoint)>) {
    let walker = ignore::WalkBuilder::new(workspace_root)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .filter_entry(|e| {
            if e.file_type().is_some_and(|ft| ft.is_dir()) {
                let name = e.file_name().to_string_lossy();
                return !is_pruned_dir(&name);
            }
            true
        })
        .build();

    for entry in walker.filter_map(Result::ok) {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("py") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        if !has_main_guard(&content) {
            continue;
        }
        if let Some(rel) = relative_slash(workspace_root, path) {
            out.push((
                2,
                EntryPoint {
                    path: rel,
                    entry_type: EntryPointType::Application,
                    confidence: DetectionConfidence::High,
                    exports: None,
                },
            ));
        }
    }
}

/// Directories never worth walking for entry points: VCS, build output (the
/// same cross-language set `validator::collect_source_files` prunes), and Python
/// virtualenv / tool caches. `venv`/`build`/`dist` are pruned by convention even
/// though a project could in theory name a real package directory that way — the
/// false-negative risk is accepted to keep the walk off vendored trees, matching
/// the validator. `env` is deliberately NOT pruned (too common a real package
/// name); a virtualenv is overwhelmingly `.venv`/`venv`.
fn is_pruned_dir(name: &str) -> bool {
    matches!(
        name,
        "node_modules"
            | ".git"
            | ".hg"
            | ".svn"
            | "dist"
            | "build"
            | "target"
            | ".venv"
            | "venv"
            | "__pycache__"
            | ".tox"
            | ".nox"
            | ".mypy_cache"
            | ".pytest_cache"
            | ".ruff_cache"
            | ".eggs"
            | "site-packages"
    ) || name.ends_with(".egg-info")
}

/// Whether `content` contains an executable `if __name__ == "__main__":` guard
/// (either operand order, single or double quotes). Lines inside triple-quoted
/// strings (docstrings) are skipped so guard text quoted in documentation does
/// not produce a false positive. A guard whose text sits inside a single-line
/// string is still not matched, because such a line does not begin with `if `
/// after trimming.
fn has_main_guard(content: &str) -> bool {
    // Track whether the current line begins inside a triple-quoted string.
    let mut in_triple: Option<char> = None;
    for line in content.lines() {
        if in_triple.is_none() && is_guard_line(line) {
            return true;
        }
        update_triple_state(line, &mut in_triple);
    }
    false
}

/// Whether a single line, considered as code, is a `__main__` guard.
fn is_guard_line(line: &str) -> bool {
    let l = line.trim();
    if !l.starts_with("if ") || !l.contains("__name__") || !l.contains("__main__") {
        return false;
    }
    let compact: String = l.chars().filter(|c| !c.is_whitespace()).collect();
    let compact = compact.replace('\'', "\"");
    compact.contains("if__name__==\"__main__\":") || compact.contains("if\"__main__\"==__name__:")
}

/// Toggle `in_triple` for each `'''` / `"""` delimiter encountered on `line`
/// (approximate: no escape handling, sufficient to skip docstring bodies).
fn update_triple_state(line: &str, in_triple: &mut Option<char>) {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if (c == '"' || c == '\'') && i + 2 < chars.len() && chars[i + 1] == c && chars[i + 2] == c
        {
            match *in_triple {
                Some(open) if open == c => *in_triple = None,
                None => *in_triple = Some(c),
                Some(_) => {} // inside a different-quote triple; ignore
            }
            i += 3;
            continue;
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, rel: &str, body: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    fn paths(entries: &[EntryPoint]) -> Vec<&str> {
        entries.iter().map(|e| e.path.as_str()).collect()
    }

    fn find<'a>(entries: &'a [EntryPoint], path: &str) -> &'a EntryPoint {
        entries
            .iter()
            .find(|e| e.path == path)
            .unwrap_or_else(|| panic!("no entry for {path} in {:?}", paths(entries)))
    }

    #[test]
    fn pyproject_console_script_resolves_to_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "pyproject.toml",
            "[project]\nname = \"app\"\n\n[project.scripts]\nappcli = \"app.cli:main\"\n",
        );
        write(tmp.path(), "app/cli.py", "def main():\n    pass\n");

        let entries = detect_python_entry_points(tmp.path());
        assert_eq!(paths(&entries), ["app/cli.py"]);
        let ep = find(&entries, "app/cli.py");
        assert_eq!(ep.entry_type, EntryPointType::Cli);
        assert_eq!(ep.confidence, DetectionConfidence::High);
        assert_eq!(ep.exports.as_deref(), Some(&["main".to_string()][..]));
    }

    #[test]
    fn pyproject_gui_script_is_application() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "pyproject.toml",
            "[project]\nname = \"app\"\n\n[project.gui-scripts]\nappgui = \"app.gui:run\"\n",
        );
        write(tmp.path(), "app/gui.py", "def run():\n    pass\n");

        let entries = detect_python_entry_points(tmp.path());
        let ep = find(&entries, "app/gui.py");
        assert_eq!(ep.entry_type, EntryPointType::Application);
        assert_eq!(ep.confidence, DetectionConfidence::High);
    }

    #[test]
    fn script_resolves_src_layout_and_package_init() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "pyproject.toml",
            "[project]\nname = \"pkg\"\n\n[project.scripts]\np = \"pkg:main\"\n",
        );
        // src/ layout, module-as-package (__init__.py).
        write(tmp.path(), "src/pkg/__init__.py", "def main():\n    pass\n");

        let entries = detect_python_entry_points(tmp.path());
        assert_eq!(paths(&entries), ["src/pkg/__init__.py"]);
    }

    #[test]
    fn unresolved_script_target_is_dropped() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "pyproject.toml",
            "[project]\nname = \"app\"\n\n[project.scripts]\nghost = \"nope.missing:main\"\n",
        );
        let entries = detect_python_entry_points(tmp.path());
        assert!(
            entries.is_empty(),
            "a script whose module resolves to no file is dropped, got {:?}",
            paths(&entries)
        );
    }

    #[test]
    fn setup_cfg_console_and_gui_scripts() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "setup.cfg",
            "[metadata]\nname = app\n\n[options.entry_points]\nconsole_scripts =\n    cli = app.cli:main\ngui_scripts =\n    gui = app.gui:main\n",
        );
        write(tmp.path(), "app/cli.py", "def main():\n    pass\n");
        write(tmp.path(), "app/gui.py", "def main():\n    pass\n");

        let entries = detect_python_entry_points(tmp.path());
        assert_eq!(paths(&entries), ["app/cli.py", "app/gui.py"]);
        assert_eq!(find(&entries, "app/cli.py").entry_type, EntryPointType::Cli);
        assert_eq!(
            find(&entries, "app/gui.py").entry_type,
            EntryPointType::Application
        );
    }

    #[test]
    fn setup_cfg_ignores_entries_outside_entry_points_section() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "setup.cfg",
            "[options]\nconsole_scripts =\n    cli = app.cli:main\n",
        );
        write(tmp.path(), "app/cli.py", "def main():\n    pass\n");
        // `console_scripts` under [options], not [options.entry_points] → ignored.
        assert!(detect_python_entry_points(tmp.path()).is_empty());
    }

    #[test]
    fn setup_py_best_effort_is_medium_confidence() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "setup.py",
            "from setuptools import setup\nsetup(\n    name=\"app\",\n    entry_points={\n        \"console_scripts\": [\n            \"cli = app.cli:main\",\n        ],\n    },\n)\n",
        );
        write(tmp.path(), "app/cli.py", "def main():\n    pass\n");

        let entries = detect_python_entry_points(tmp.path());
        let ep = find(&entries, "app/cli.py");
        assert_eq!(ep.entry_type, EntryPointType::Cli);
        assert_eq!(ep.confidence, DetectionConfidence::Medium);
    }

    #[test]
    fn main_guard_detected_double_and_single_quote() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "scripts/run.py",
            "def go():\n    pass\n\nif __name__ == \"__main__\":\n    go()\n",
        );
        write(
            tmp.path(),
            "scripts/run2.py",
            "if __name__=='__main__':\n    pass\n",
        );

        let entries = detect_python_entry_points(tmp.path());
        assert_eq!(paths(&entries), ["scripts/run.py", "scripts/run2.py"]);
        assert!(
            entries
                .iter()
                .all(|e| e.entry_type == EntryPointType::Application
                    && e.confidence == DetectionConfidence::High)
        );
    }

    #[test]
    fn comment_mentioning_main_is_not_a_guard() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "a.py",
            "# this module has no if __name__ == \"__main__\" guard\nx = 1\n",
        );
        assert!(detect_python_entry_points(tmp.path()).is_empty());
    }

    #[test]
    fn declared_script_wins_over_main_guard_at_same_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "pyproject.toml",
            "[project]\nname = \"app\"\n\n[project.scripts]\nc = \"app.cli:main\"\n",
        );
        // The script target file ALSO carries a __main__ guard.
        write(
            tmp.path(),
            "app/cli.py",
            "def main():\n    pass\n\nif __name__ == \"__main__\":\n    main()\n",
        );

        let entries = detect_python_entry_points(tmp.path());
        assert_eq!(paths(&entries), ["app/cli.py"], "deduped by path");
        let ep = find(&entries, "app/cli.py");
        assert_eq!(ep.entry_type, EntryPointType::Cli, "declared script wins");
        assert_eq!(ep.exports.as_deref(), Some(&["main".to_string()][..]));
    }

    #[test]
    fn virtualenv_and_pycache_dirs_are_pruned() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            ".venv/lib/dep.py",
            "if __name__ == \"__main__\":\n    pass\n",
        );
        write(
            tmp.path(),
            "__pycache__/cached.py",
            "if __name__ == \"__main__\":\n    pass\n",
        );
        write(
            tmp.path(),
            "real.py",
            "if __name__ == \"__main__\":\n    pass\n",
        );

        let entries = detect_python_entry_points(tmp.path());
        assert_eq!(
            paths(&entries),
            ["real.py"],
            "entry points inside .venv / __pycache__ are pruned"
        );
    }

    #[test]
    fn no_python_is_empty_not_an_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), "README.md", "# not python\n");
        assert!(detect_python_entry_points(tmp.path()).is_empty());
    }

    #[test]
    fn deterministic_across_runs() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "pyproject.toml",
            "[project]\nname = \"app\"\n\n[project.scripts]\nz = \"app.z:main\"\na = \"app.a:main\"\n",
        );
        write(tmp.path(), "app/z.py", "def main():\n    pass\n");
        write(tmp.path(), "app/a.py", "def main():\n    pass\n");
        write(
            tmp.path(),
            "tool.py",
            "if __name__ == \"__main__\":\n    pass\n",
        );

        let first = detect_python_entry_points(tmp.path());
        let second = detect_python_entry_points(tmp.path());
        assert_eq!(paths(&first), paths(&second));
        assert_eq!(paths(&first), ["app/a.py", "app/z.py", "tool.py"]);
    }

    #[test]
    fn entry_target_strips_extras_and_keeps_object() {
        let t = parse_entry_target("pkg.mod:func [gui]").unwrap();
        assert_eq!(t.module, "pkg.mod");
        assert_eq!(t.object.as_deref(), Some("func"));

        let t2 = parse_entry_target("pkg.mod").unwrap();
        assert_eq!(t2.module, "pkg.mod");
        assert_eq!(t2.object, None);

        assert!(parse_entry_target("   ").is_none());
    }

    // --- Council regression tests (PYLAN-005 review) ---------------------------

    #[test]
    fn malformed_module_targets_are_rejected() {
        // `..secret`, empty segments, leading dots, and multi-colon objects must
        // never reach the filesystem or produce a `//`-bearing path.
        assert!(parse_entry_target("..secret:main").is_none());
        assert!(parse_entry_target("pkg..cli:main").is_none());
        assert!(parse_entry_target(".cli:main").is_none());
        assert!(parse_entry_target("pkg.cli:attr:nested").is_none());
        assert!(parse_entry_target("9pkg:main").is_none(), "leading digit");
        // A valid dotted object (attribute chain) is kept.
        let t = parse_entry_target("pkg.cli:obj.method").unwrap();
        assert_eq!(t.object.as_deref(), Some("obj.method"));
    }

    #[test]
    fn dotdot_module_does_not_emit_workspace_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "pyproject.toml",
            "[project]\nname = \"app\"\n\n[project.scripts]\nbad = \"..secret:main\"\n",
        );
        // Even with a same-named file present, the malformed target is dropped.
        write(tmp.path(), "src/secret.py", "def main():\n    pass\n");
        assert!(detect_python_entry_points(tmp.path()).is_empty());
    }

    #[test]
    fn setup_py_bracket_in_string_does_not_truncate_list() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "setup.py",
            "setup(entry_points={\"console_scripts\": [\n    \"bad = app.trick:main]oops\",\n    \"real = app.cli:main\",\n]})\n",
        );
        write(tmp.path(), "app/cli.py", "def main():\n    pass\n");
        // The `]` inside the first (malformed) string must not truncate the list;
        // the real, resolvable entry that follows is still found.
        let entries = detect_python_entry_points(tmp.path());
        assert_eq!(paths(&entries), ["app/cli.py"]);
    }

    #[test]
    fn setup_py_comment_does_not_hijack_extraction() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "setup.py",
            "# console_scripts = [\"decoy = decoy.cli:main\"]\nsetup(entry_points={\"console_scripts\": [\"real = app.cli:main\"]})\n",
        );
        write(tmp.path(), "app/cli.py", "def main():\n    pass\n");
        let entries = detect_python_entry_points(tmp.path());
        assert_eq!(
            paths(&entries),
            ["app/cli.py"],
            "the commented-out decoy must not win the key match"
        );
    }

    #[test]
    fn main_guard_inside_docstring_is_not_detected() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "doc.py",
            "def describe():\n    \"\"\"\n    Use like:\n    if __name__ == \"__main__\":\n        run()\n    \"\"\"\n    return 1\n",
        );
        assert!(
            detect_python_entry_points(tmp.path()).is_empty(),
            "guard text inside a docstring must not produce an entry point"
        );
    }

    #[test]
    fn real_guard_after_docstring_is_still_detected() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "m.py",
            "\"\"\"module docstring mentioning if __name__ == \"__main__\" inline\"\"\"\n\ndef go():\n    pass\n\nif __name__ == \"__main__\":\n    go()\n",
        );
        let entries = detect_python_entry_points(tmp.path());
        assert_eq!(paths(&entries), ["m.py"]);
    }

    #[test]
    fn same_file_in_scripts_and_gui_scripts_is_deterministic() {
        // The same module declared in both [project.scripts] and
        // [project.gui-scripts] yields two rank-0 entries at the same path; the
        // total comparator must pick the same survivor every run.
        let tmp = tempfile::TempDir::new().unwrap();
        write(
            tmp.path(),
            "pyproject.toml",
            "[project]\nname = \"app\"\n\n[project.scripts]\nc = \"app.main:cli\"\n\n[project.gui-scripts]\ng = \"app.main:gui\"\n",
        );
        write(tmp.path(), "app/main.py", "def cli():\n    pass\n");

        let first = detect_python_entry_points(tmp.path());
        let second = detect_python_entry_points(tmp.path());
        assert_eq!(first.len(), 1);
        assert_eq!(paths(&first), ["app/main.py"]);
        // Survivor is stable across runs (type + exports identical each time).
        assert_eq!(first[0].entry_type, second[0].entry_type);
        assert_eq!(first[0].exports, second[0].exports);
        // Application (gui, ord 1) sorts before Cli (ord 4) at equal path+rank.
        assert_eq!(first[0].entry_type, EntryPointType::Application);
    }

    #[test]
    fn env_directory_is_not_pruned() {
        let tmp = tempfile::TempDir::new().unwrap();
        // `env` is a plausible real package dir, unlike `.venv`/`venv`.
        write(
            tmp.path(),
            "env/settings.py",
            "if __name__ == \"__main__\":\n    pass\n",
        );
        let entries = detect_python_entry_points(tmp.path());
        assert_eq!(paths(&entries), ["env/settings.py"]);
    }
}
