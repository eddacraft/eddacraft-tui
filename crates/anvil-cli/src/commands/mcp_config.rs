//! `anvil mcp-config` — generate MCP server configuration for AI editors.
//!
//! RCLI3-016. Produces editor-specific JSON for `claude-code`, `cursor`,
//! `windsurf`, and `vscode` so the RTAI launch demo runbook
//! (`plans/specs/2026-04-26-rtai-demo-runbook.md`) has a one-command install
//! step before Cursor / Claude Code can consume the daemon.

use std::fs;
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};
use serde_json::{Map, Value, json};

use crate::GlobalArgs;

/// Default HTTP port advertised when the user picks `--transport http` but
/// does not pin a port. Matches the daemon default chosen by INTD.
const DEFAULT_HTTP_PORT: u16 = 7616;

/// Server entry name written into every editor config. Stable so re-running
/// `--write` updates the existing entry rather than duplicating it.
const SERVER_NAME: &str = "anvil";

#[derive(Debug, Args)]
pub struct McpConfigArgs {
    /// Editor / agent target whose config format we emit.
    #[arg(long, value_enum)]
    target: Target,

    /// Transport to advertise. `stdio` spawns the daemon as a child process;
    /// `http` points the editor at a running daemon's HTTP endpoint.
    #[arg(long, value_enum, default_value_t = Transport::Stdio)]
    transport: Transport,

    /// Port to use when `--transport http` is selected. Ignored for stdio.
    #[arg(long, default_value_t = DEFAULT_HTTP_PORT)]
    port: u16,

    /// Write the generated config to the target's well-known path. Without
    /// this flag, the config is printed to stdout for review.
    #[arg(long)]
    write: bool,

    /// Resolve the target's config path, print its current entry, and parse
    /// it. Exits non-zero if the file is missing, unparseable, or has no
    /// `anvil` entry. Does not write.
    #[arg(long, conflicts_with = "write")]
    verify: bool,

    /// Override the workspace root used to resolve target-local config
    /// paths (`.claude/`, `.cursor/`, `.windsurf/`, `.vscode/`). Defaults to
    /// the current working directory.
    #[arg(long)]
    workspace: Option<PathBuf>,

    /// Override the command path used in `stdio` configs. Defaults to
    /// `anvil`, relying on the editor's PATH. Useful in tests and unusual
    /// deployments where `anvil` is not on PATH.
    #[arg(long, hide = true)]
    command: Option<String>,

    /// Skip the "outside workspace root" confirmation prompt and proceed.
    /// Required for non-interactive callers writing to a custom path.
    #[arg(long)]
    yes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Target {
    /// Anthropic Claude Code (`.claude/mcp.json`).
    ClaudeCode,
    /// Cursor (`.cursor/mcp.json`).
    Cursor,
    /// Windsurf (`.windsurf/mcp.json`).
    Windsurf,
    /// `VSCode` workspace settings (`.vscode/settings.json`, `mcp.servers` key).
    Vscode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Transport {
    Stdio,
    Http,
}

pub fn run(args: &McpConfigArgs, global: &GlobalArgs) -> Result<()> {
    let workspace = match &args.workspace {
        Some(p) => p.clone(),
        None => std::env::current_dir().context("resolving current directory")?,
    };

    if args.verify {
        return run_verify(args, global, &workspace);
    }

    let value = build_config(
        args.target,
        args.transport,
        args.port,
        args.command.as_deref(),
    );
    let entry_json = serde_json::to_string_pretty(&value)?;

    if !args.write {
        if global.json {
            println!("{entry_json}");
        } else {
            println!("# Preview — pass --write to install at the target path.");
            println!("# Target: {}", target_label(args.target));
            println!("# Path  : {}", relative_path_for(args.target).display());
            println!("{entry_json}");
        }
        return Ok(());
    }

    let config_path = workspace.join(relative_path_for(args.target));
    ensure_path_safe(&workspace, &config_path, args.yes, global)?;

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }

    let merged = merge_into_existing(args.target, &config_path, &value)?;
    fs::write(&config_path, format!("{merged}\n"))
        .with_context(|| format!("writing {}", config_path.display()))?;

    if global.json {
        println!(
            "{}",
            json!({
                "target": target_label(args.target),
                "path": config_path.display().to_string(),
                "wrote": true,
            })
        );
    } else {
        println!(
            "Wrote {} config for {} to {}",
            target_label(args.target),
            SERVER_NAME,
            config_path.display()
        );
    }
    Ok(())
}

fn run_verify(args: &McpConfigArgs, global: &GlobalArgs, workspace: &Path) -> Result<()> {
    let config_path = workspace.join(relative_path_for(args.target));
    if !config_path.exists() {
        if global.json {
            eprintln!(
                "{}",
                json!({
                    "target": target_label(args.target),
                    "path": config_path.display().to_string(),
                    "error": "missing",
                })
            );
        } else {
            eprintln!(
                "No {} config found at {}",
                target_label(args.target),
                config_path.display()
            );
        }
        bail!("config not found");
    }

    let raw = fs::read_to_string(&config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let parsed: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing {} as JSON", config_path.display()))?;

    let Some(entry) = extract_entry(args.target, &parsed) else {
        if global.json {
            eprintln!(
                "{}",
                json!({
                    "target": target_label(args.target),
                    "path": config_path.display().to_string(),
                    "error": "missing-entry",
                })
            );
        } else {
            eprintln!(
                "{} config at {} is missing the `{SERVER_NAME}` entry.",
                target_label(args.target),
                config_path.display()
            );
        }
        bail!("anvil entry missing");
    };

    if global.json {
        println!(
            "{}",
            json!({
                "target": target_label(args.target),
                "path": config_path.display().to_string(),
                "entry": entry,
                "ok": true,
            })
        );
    } else {
        println!("Resolved : {}", config_path.display());
        println!("Entry    :");
        println!("{}", serde_json::to_string_pretty(&entry)?);
        println!("Status   : ok");
    }
    Ok(())
}

/// Build the editor-specific JSON value that goes on disk.
///
/// Note the `VSCode` shape diverges: it nests the entry under `mcp.servers`
/// with a `type` field (`stdio`/`sse`) per the `VSCode` MCP convention. The
/// other targets share a `mcpServers` map keyed by server name with
/// `command` / `args` (for stdio) or `url` (for http).
pub(crate) fn build_config(
    target: Target,
    transport: Transport,
    port: u16,
    command_override: Option<&str>,
) -> Value {
    let command = command_override.unwrap_or("anvil");
    let entry = build_entry(target, transport, port, command);
    match target {
        Target::ClaudeCode | Target::Cursor | Target::Windsurf => {
            json!({
                "mcpServers": {
                    SERVER_NAME: entry,
                }
            })
        }
        Target::Vscode => {
            json!({
                "mcp": {
                    "servers": {
                        SERVER_NAME: entry,
                    }
                }
            })
        }
    }
}

fn build_entry(target: Target, transport: Transport, port: u16, command: &str) -> Value {
    match (target, transport) {
        (Target::Vscode, Transport::Stdio) => json!({
            "type": "stdio",
            "command": command,
            "args": ["mcp", "serve", "--stdio"],
        }),
        (Target::Vscode, Transport::Http) => json!({
            "type": "sse",
            "url": format!("http://127.0.0.1:{port}/mcp"),
        }),
        (_, Transport::Stdio) => json!({
            "command": command,
            "args": ["mcp", "serve", "--stdio"],
            "env": {},
        }),
        (_, Transport::Http) => json!({
            "url": format!("http://127.0.0.1:{port}/mcp"),
            "env": {},
        }),
    }
}

/// Pluck the existing `anvil` entry out of an on-disk config so `--verify`
/// can report it and `--write` can do an idempotent merge.
fn extract_entry(target: Target, root: &Value) -> Option<Value> {
    match target {
        Target::ClaudeCode | Target::Cursor | Target::Windsurf => root
            .get("mcpServers")
            .and_then(|m| m.get(SERVER_NAME))
            .cloned(),
        Target::Vscode => root
            .get("mcp")
            .and_then(|m| m.get("servers"))
            .and_then(|s| s.get(SERVER_NAME))
            .cloned(),
    }
}

/// Merge the freshly-built entry into any existing config file at
/// `config_path`. If the file is missing or unparseable, we start from the
/// freshly-built shape so corrupt files do not block the install. We never
/// rewrite unrelated keys — only the `mcpServers.anvil` (or `VSCode`
/// equivalent) leaf.
fn merge_into_existing(target: Target, config_path: &Path, fresh: &Value) -> Result<String> {
    let existing: Option<Value> = match fs::read_to_string(config_path) {
        Ok(raw) if raw.trim().is_empty() => None,
        Ok(raw) => serde_json::from_str(&raw).ok(),
        Err(_) => None,
    };

    let merged = match existing {
        None => fresh.clone(),
        Some(mut base) => {
            // Only merge the leaf we own. Preserve every other key in the
            // user's editor config — they may have other MCP servers
            // configured, settings unrelated to MCP, etc.
            let entry = extract_entry(target, fresh).unwrap_or(Value::Null);
            insert_entry(target, &mut base, entry);
            base
        }
    };

    Ok(serde_json::to_string_pretty(&merged)?)
}

fn insert_entry(target: Target, root: &mut Value, entry: Value) {
    match target {
        Target::ClaudeCode | Target::Cursor | Target::Windsurf => {
            let obj = ensure_object(root);
            let servers = obj
                .entry("mcpServers".to_string())
                .or_insert_with(|| Value::Object(Map::new()));
            if let Value::Object(map) = servers {
                map.insert(SERVER_NAME.to_string(), entry);
            }
        }
        Target::Vscode => {
            let obj = ensure_object(root);
            let mcp = obj
                .entry("mcp".to_string())
                .or_insert_with(|| Value::Object(Map::new()));
            if let Value::Object(mcp_map) = mcp {
                let servers = mcp_map
                    .entry("servers".to_string())
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Value::Object(map) = servers {
                    map.insert(SERVER_NAME.to_string(), entry);
                }
            }
        }
    }
}

fn ensure_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    value.as_object_mut().expect("just ensured object")
}

/// Resolve `path` through symlinks where possible, then check it lives
/// under `root`. If not, refuse (or prompt, for interactive callers).
///
/// We `canonicalize` `root` once and then walk the path lexically because
/// `path` may not exist yet (we are about to create the parent dirs). For
/// any prefix that *does* exist (a real `.cursor/` symlinked into a sibling
/// repo, say) we canonicalize it and re-check containment.
fn ensure_path_safe(root: &Path, path: &Path, confirmed: bool, global: &GlobalArgs) -> Result<()> {
    let canonical_root = match root.canonicalize() {
        Ok(p) => p,
        Err(_) => normalise(root),
    };
    let canonical_path = nearest_existing_canonical(path);

    if canonical_path.starts_with(&canonical_root) {
        return Ok(());
    }

    if confirmed {
        return Ok(());
    }

    // Non-interactive callers must opt in via --yes; refusing is the safer
    // default than silently writing outside the workspace.
    let interactive = !global.json
        && !global.no_tui
        && std::io::stdin().is_terminal()
        && std::io::stderr().is_terminal();
    if !interactive {
        bail!(
            "refusing to write outside the workspace root\n  workspace: {}\n  target  : {}\n  rerun with --yes to override",
            canonical_root.display(),
            canonical_path.display(),
        );
    }

    let mut stderr = std::io::stderr();
    writeln!(
        stderr,
        "Target path is outside the workspace root.\n  workspace: {}\n  target  : {}",
        canonical_root.display(),
        canonical_path.display(),
    )?;
    write!(stderr, "Continue anyway? [y/N] ")?;
    stderr.flush()?;
    let mut line = String::new();
    let stdin = std::io::stdin();
    let mut locked = stdin.lock();
    let n = locked.read_line(&mut line)?;
    if n == 0 {
        bail!("no confirmation received; refusing to write");
    }
    let answer = line.trim().to_ascii_lowercase();
    if answer == "y" || answer == "yes" {
        Ok(())
    } else {
        bail!("user declined; not writing");
    }
}

/// Walk back up `path` until we find a component that exists, then
/// `canonicalize` it. Used for path-safety checks on files that have not
/// been created yet.
fn nearest_existing_canonical(path: &Path) -> PathBuf {
    let mut cursor: Option<&Path> = Some(path);
    while let Some(p) = cursor {
        if let Ok(canonical) = p.canonicalize() {
            // Reattach any tail components below the existing prefix so the
            // result still represents the would-be target file.
            if let Ok(rest) = path.strip_prefix(p) {
                return canonical.join(rest);
            }
            return canonical;
        }
        cursor = p.parent();
    }
    normalise(path)
}

/// Lexical normalisation for paths that cannot be canonicalised (target
/// not on disk yet, parent missing, etc). Resolves `..` and `.` without
/// touching the filesystem so we still catch `../../etc/passwd`-style
/// escapes.
fn normalise(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn relative_path_for(target: Target) -> PathBuf {
    match target {
        Target::ClaudeCode => PathBuf::from(".claude").join("mcp.json"),
        Target::Cursor => PathBuf::from(".cursor").join("mcp.json"),
        Target::Windsurf => PathBuf::from(".windsurf").join("mcp.json"),
        Target::Vscode => PathBuf::from(".vscode").join("settings.json"),
    }
}

fn target_label(target: Target) -> &'static str {
    match target {
        Target::ClaudeCode => "claude-code",
        Target::Cursor => "cursor",
        Target::Windsurf => "windsurf",
        Target::Vscode => "vscode",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_anvil_entry(target: Target, value: &Value) {
        let entry = extract_entry(target, value).expect("anvil entry present");
        assert!(
            entry.is_object(),
            "expected anvil entry to be an object, got {entry}"
        );
    }

    #[test]
    fn claude_code_stdio_shape() {
        let v = build_config(Target::ClaudeCode, Transport::Stdio, 0, Some("anvil"));
        assert!(v.get("mcpServers").is_some(), "claude-code uses mcpServers");
        let entry = extract_entry(Target::ClaudeCode, &v).unwrap();
        assert_eq!(entry["command"], "anvil");
        assert_eq!(entry["args"][0], "mcp");
        assert!(entry.get("type").is_none(), "claude-code has no type field");
        // Round-trip parse — the file we write must be valid JSON.
        let raw = serde_json::to_string_pretty(&v).unwrap();
        let _: Value = serde_json::from_str(&raw).unwrap();
    }

    #[test]
    fn cursor_stdio_shape() {
        let v = build_config(Target::Cursor, Transport::Stdio, 0, None);
        assert_anvil_entry(Target::Cursor, &v);
        let entry = extract_entry(Target::Cursor, &v).unwrap();
        assert_eq!(entry["command"], "anvil");
        let raw = serde_json::to_string_pretty(&v).unwrap();
        let _: Value = serde_json::from_str(&raw).unwrap();
    }

    #[test]
    fn windsurf_stdio_shape() {
        let v = build_config(Target::Windsurf, Transport::Stdio, 0, None);
        assert_anvil_entry(Target::Windsurf, &v);
        let raw = serde_json::to_string_pretty(&v).unwrap();
        let _: Value = serde_json::from_str(&raw).unwrap();
    }

    #[test]
    fn vscode_stdio_shape_has_type_field() {
        // VSCode's MCP convention is type-tagged (stdio / sse) and sits
        // under `mcp.servers`, not the shared `mcpServers` map.
        let v = build_config(Target::Vscode, Transport::Stdio, 0, None);
        assert!(v.get("mcp").is_some(), "vscode nests under mcp.servers");
        assert!(
            v.get("mcpServers").is_none(),
            "vscode must not use the shared mcpServers key"
        );
        let entry = extract_entry(Target::Vscode, &v).unwrap();
        assert_eq!(entry["type"], "stdio");
        assert_eq!(entry["command"], "anvil");
        let raw = serde_json::to_string_pretty(&v).unwrap();
        let _: Value = serde_json::from_str(&raw).unwrap();
    }

    #[test]
    fn vscode_http_uses_sse_type() {
        let v = build_config(Target::Vscode, Transport::Http, 7616, None);
        let entry = extract_entry(Target::Vscode, &v).unwrap();
        assert_eq!(entry["type"], "sse");
        assert!(
            entry["url"].as_str().unwrap().contains("7616"),
            "url should embed the chosen port",
        );
    }

    #[test]
    fn http_transport_emits_url() {
        let v = build_config(Target::ClaudeCode, Transport::Http, 9999, None);
        let entry = extract_entry(Target::ClaudeCode, &v).unwrap();
        assert!(entry["url"].as_str().unwrap().ends_with(":9999/mcp"));
    }

    #[test]
    fn merge_preserves_unrelated_keys() {
        // A user with their own MCP servers configured must not lose them
        // when we install the anvil entry.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".cursor").join("mcp.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let existing = json!({
            "mcpServers": {
                "other": { "command": "other-bin", "args": [] }
            },
            "unrelated": { "keep": true }
        });
        fs::write(&path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();

        let fresh = build_config(Target::Cursor, Transport::Stdio, 0, None);
        let merged_str = merge_into_existing(Target::Cursor, &path, &fresh).unwrap();
        let merged: Value = serde_json::from_str(&merged_str).unwrap();

        assert!(
            merged["mcpServers"]["other"].is_object(),
            "kept other server"
        );
        assert_eq!(merged["unrelated"]["keep"], true);
        assert!(
            merged["mcpServers"][SERVER_NAME].is_object(),
            "added anvil entry"
        );
    }

    #[test]
    fn normalise_resolves_parent_dir() {
        let p = normalise(Path::new("/tmp/foo/../bar"));
        assert_eq!(p, PathBuf::from("/tmp/bar"));
    }

    #[test]
    fn relative_paths_per_target() {
        assert_eq!(
            relative_path_for(Target::ClaudeCode),
            PathBuf::from(".claude").join("mcp.json"),
        );
        assert_eq!(
            relative_path_for(Target::Cursor),
            PathBuf::from(".cursor").join("mcp.json"),
        );
        assert_eq!(
            relative_path_for(Target::Windsurf),
            PathBuf::from(".windsurf").join("mcp.json"),
        );
        assert_eq!(
            relative_path_for(Target::Vscode),
            PathBuf::from(".vscode").join("settings.json"),
        );
    }
}
