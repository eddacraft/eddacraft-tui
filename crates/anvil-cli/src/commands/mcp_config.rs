//! `anvil mcp-config` — generate MCP server configuration for AI editors.
//!
//! RCLI3-016 / MCPX compatibility surface. Produces scope-aware configuration
//! for clients in the shared agent registry. Stdio uses the managed first-wave
//! adapters; legacy HTTP remains limited to Claude Code and Cursor.

use std::fs;
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};
use serde_json::{Map, Value, json};

use crate::GlobalArgs;
use crate::activation::agent_registry::{AgentClientId, InstallScope, McpConfigKind};
use crate::commands::mcp_installer;
use crate::output::AlreadyReported;
use crate::util::atomic_write_nofollow;

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
    target: AgentClientId,

    /// Resolve a global (default) or project-local target path.
    #[arg(long, value_enum, default_value_t = InstallScope::Global)]
    scope: InstallScope,

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

    /// Override the selected scope root used to resolve the client config.
    /// Global scope defaults to the user home; project scope defaults to the
    /// current working directory.
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
    /// Anthropic Claude Code (`.claude.json`).
    ClaudeCode,
    /// Cursor (`.cursor/mcp.json`).
    Cursor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Transport {
    Stdio,
    Http,
}

/// Whether this parsed configuration command is guaranteed not to write.
///
/// clap rejects `--verify --write`; checking the write flag directly also
/// fails closed if an invalid argument state is ever constructed internally.
pub fn is_read_only_diagnostic(
    McpConfigArgs {
        target: _,
        scope: _,
        transport: _,
        port: _,
        write,
        verify: _,
        workspace: _,
        command: _,
        yes: _,
    }: &McpConfigArgs,
) -> bool {
    !*write
}

pub fn run(args: &McpConfigArgs, global: &GlobalArgs) -> Result<()> {
    let workspace = match &args.workspace {
        Some(p) => p.clone(),
        None if args.scope == InstallScope::Global => default_client_config_root()?,
        None => std::env::current_dir().context("resolving current directory")?,
    };
    let command_override = validate_command_override(args.command.as_deref())?;

    if args.transport == Transport::Stdio {
        return run_registry_stdio(
            args,
            global,
            &workspace,
            crate::activation::mcp_client::preferred_mcp_command(command_override),
        );
    }

    let target = legacy_target(args.target).with_context(|| {
        format!(
            "{} supports stdio configuration only; --transport http remains limited to claude-code and cursor",
            args.target.label()
        )
    })?;

    if args.verify {
        return run_verify(args, global, &workspace);
    }

    let value = build_config(target, args.transport, args.port, command_override);
    let entry_json = serde_json::to_string_pretty(&value)?;

    let config_path = workspace.join(relative_path_for(target));

    if !args.write {
        if global.json {
            println!("{entry_json}");
        } else {
            println!("# Preview — pass --write to install at the target path.");
            println!("# Target: {}", target_label(target));
            println!("# Path  : {}", config_path.display());
            println!("{entry_json}");
        }
        return Ok(());
    }

    write_target_config(target, &workspace, &value, args.yes, global)?;

    if global.json {
        println!(
            "{}",
            json!({
                "target": target_label(target),
                "path": config_path.display().to_string(),
                "wrote": true,
            })
        );
    } else {
        println!(
            "Wrote {} config for {} to {}",
            target_label(target),
            SERVER_NAME,
            config_path.display()
        );
    }
    Ok(())
}

fn run_registry_stdio(
    args: &McpConfigArgs,
    global: &GlobalArgs,
    workspace: &Path,
    command: &str,
) -> Result<()> {
    let adapter = args.target.entry();
    let path = adapter.mcp_path(args.scope, workspace).with_context(|| {
        format!(
            "{} does not support {}-scope MCP configuration",
            adapter.display_name,
            args.scope.label()
        )
    })?;

    if !args.write && !args.verify {
        let rendered = mcp_installer::preview(args.target, command)?;
        let toml_kind = matches!(
            adapter.mcp_kind,
            Some(McpConfigKind::CodexToml | McpConfigKind::GrokToml)
        );
        if global.json && toml_kind {
            // Issue #3947: for TOML-kind clients (codex, grok) the raw
            // preview is not JSON, so under `--json` the text travels
            // inside one document — the `config convert --stdout`
            // envelope pattern. JSON-kind previews already print one
            // valid document raw; that existing contract stays as-is.
            crate::output::json::print(&json!({
                "target": args.target.label(),
                "path": path.display().to_string(),
                "format": "toml",
                "config": rendered,
            }))?;
        } else {
            if !global.json {
                println!("# Preview — pass --write to install at the target path.");
                println!("# Target: {}", args.target.label());
                println!("# Path  : {}", path.display());
            }
            print!("{rendered}");
        }
        return Ok(());
    }

    let report = mcp_installer::install(
        args.target,
        args.scope,
        workspace,
        command,
        args.verify,
        false,
    )?;
    if global.json {
        println!(
            "{}",
            json!({
                "target": args.target.label(),
                "path": report.path.display().to_string(),
                "entry": report.entry,
                "wrote": report.wrote,
                "ok": true,
            })
        );
    } else if args.verify {
        println!("Resolved : {}", report.path.display());
        println!("Status   : ok");
    } else {
        println!(
            "Wrote {} config for {} to {}",
            args.target.label(),
            SERVER_NAME,
            report.path.display()
        );
    }
    Ok(())
}

fn legacy_target(client: AgentClientId) -> Option<Target> {
    match client {
        AgentClientId::ClaudeCode => Some(Target::ClaudeCode),
        AgentClientId::Cursor => Some(Target::Cursor),
        _ => None,
    }
}

fn run_verify(args: &McpConfigArgs, global: &GlobalArgs, workspace: &Path) -> Result<()> {
    let require_rust_stdio = args.transport == Transport::Stdio;
    let expected_command = if require_rust_stdio {
        validate_command_override(args.command.as_deref())?
    } else {
        None
    };
    let (config_path, entry) = verify_target_config(
        legacy_target(args.target).context("unsupported legacy MCP target")?,
        global,
        workspace,
        require_rust_stdio,
        expected_command,
    )?;

    if global.json {
        println!(
            "{}",
            json!({
                "target": args.target.label(),
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

pub(crate) fn default_client_config_root() -> Result<PathBuf> {
    crate::util::user_home_dir().context("could not determine home directory")
}

fn validate_command_override(command: Option<&str>) -> Result<Option<&str>> {
    match command.map(str::trim) {
        Some("") => bail!("--command must not be empty"),
        Some(command) => Ok(Some(command)),
        None => Ok(None),
    }
}

fn write_target_config(
    target: Target,
    workspace: &Path,
    value: &Value,
    yes: bool,
    global: &GlobalArgs,
) -> Result<PathBuf> {
    let config_path = workspace.join(relative_path_for(target));
    ensure_path_safe(workspace, &config_path, yes, global)?;

    // Re-validate after the initial check: directory creation and the
    // merge read still use path-based ops, so a concurrent parent swap
    // could race them. The write itself is no-follow / parent-fd pinned
    // via atomic_write_nofollow, which is what closes the final window.
    if let Some(parent) = config_path.parent() {
        crate::util::create_dir_all_nofollow(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    ensure_path_safe(workspace, &config_path, yes, global)?;

    let merged = merge_into_existing(target, &config_path, value)?;
    // Parent-fd-pinned rename: refuses a swapped/symlinked parent and
    // never writes the tempfile through a redirecting directory.
    atomic_write_nofollow(&config_path, format!("{merged}\n").as_bytes())
        .with_context(|| format!("writing {}", config_path.display()))?;

    Ok(config_path)
}

fn verify_target_config(
    target: Target,
    global: &GlobalArgs,
    workspace: &Path,
    require_rust_stdio: bool,
    expected_command: Option<&str>,
) -> Result<(PathBuf, Value)> {
    let config_path = workspace.join(relative_path_for(target));
    if !config_path.exists() {
        if global.json {
            eprintln!(
                "{}",
                json!({
                    "target": target_label(target),
                    "path": config_path.display().to_string(),
                    "error": "missing",
                })
            );
        } else {
            eprintln!(
                "No {} config found at {}",
                target_label(target),
                config_path.display()
            );
        }
        // The structured/human-readable message is already on stderr;
        // signal main to exit non-zero without printing again.
        return Err(AlreadyReported.into());
    }

    let raw = fs::read_to_string(&config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let parsed: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing {} as JSON", config_path.display()))?;

    let Some(entry) = extract_entry(target, &parsed) else {
        if global.json {
            eprintln!(
                "{}",
                json!({
                    "target": target_label(target),
                    "path": config_path.display().to_string(),
                    "error": "missing-entry",
                })
            );
        } else {
            eprintln!(
                "{} config at {} is missing the `{SERVER_NAME}` entry.",
                target_label(target),
                config_path.display()
            );
        }
        return Err(AlreadyReported.into());
    };

    if require_rust_stdio {
        validate_rust_stdio_entry(target, &config_path, &entry, expected_command, global)?;
    }

    Ok((config_path, entry))
}

fn validate_rust_stdio_entry(
    target: Target,
    config_path: &Path,
    entry: &Value,
    expected_command: Option<&str>,
    global: &GlobalArgs,
) -> Result<()> {
    let command_ok = entry
        .get("command")
        .and_then(Value::as_str)
        .is_some_and(|command| command_matches_expected(command, expected_command));
    let args_ok = entry.get("args") == Some(&json!(["mcp", "serve", "--stdio",]));
    let type_ok = match entry.get("type") {
        Some(value) => value.as_str() == Some("stdio"),
        None => !matches!(target, Target::ClaudeCode),
    };

    if command_ok && args_ok && type_ok {
        return Ok(());
    }

    let expected_command = crate::activation::mcp_client::preferred_mcp_command(expected_command);

    if global.json {
        eprintln!(
            "{}",
            json!({
                "target": target_label(target),
                "path": config_path.display().to_string(),
                "error": "malformed-entry",
                "expected": {
                    "command": expected_command,
                    "args": ["mcp", "serve", "--stdio"],
                    "type": "stdio",
                    "typeRequired": type_required(target),
                },
                "entry": entry,
            })
        );
    } else {
        eprintln!(
            "{} config at {} has a malformed `{SERVER_NAME}` entry; expected command `{}` with args `mcp serve --stdio` and type {}.",
            target_label(target),
            config_path.display(),
            expected_command,
            expected_type_label(target)
        );
    }

    Err(AlreadyReported.into())
}

fn type_required(target: Target) -> bool {
    matches!(target, Target::ClaudeCode)
}

fn expected_type_label(target: Target) -> &'static str {
    match target {
        Target::ClaudeCode => "`stdio`",
        Target::Cursor => "`stdio` when present",
    }
}

fn command_matches_expected(command: &str, expected_command: Option<&str>) -> bool {
    if command.trim().is_empty() {
        return false;
    }

    if let Some(expected_command) = expected_command {
        return command == expected_command;
    }

    command == crate::activation::mcp_client::PREFERRED_MCP_COMMAND
}

/// Build the editor-specific JSON value that goes on disk.
///
/// Both surviving targets (Claude Code, Cursor) share the `mcpServers` map
/// keyed by server name with `command` / `args` (for stdio) or `url` (for
/// http); Claude Code also carries `type: "stdio"` in the same map for its
/// user-scope config.
pub(crate) fn build_config(
    target: Target,
    transport: Transport,
    port: u16,
    command_override: Option<&str>,
) -> Value {
    let command = crate::activation::mcp_client::preferred_mcp_command(command_override);
    let entry = build_entry(target, transport, port, command);
    json!({
        "mcpServers": {
            SERVER_NAME: entry,
        }
    })
}

fn build_entry(target: Target, transport: Transport, port: u16, command: &str) -> Value {
    match (target, transport) {
        (Target::ClaudeCode, Transport::Stdio) => json!({
            "type": "stdio",
            "command": command,
            "args": ["mcp", "serve", "--stdio"],
            "env": {},
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
fn extract_entry(_target: Target, root: &Value) -> Option<Value> {
    root.get("mcpServers")
        .and_then(|m| m.get(SERVER_NAME))
        .cloned()
}

/// Merge the freshly-built entry into any existing config file at
/// `config_path`. If the file is missing or empty, we start from the
/// freshly-built shape. If the file exists but cannot be parsed as JSON,
/// we refuse — silently overwriting would clobber the user's other MCP
/// servers and unrelated editor settings. We never rewrite unrelated keys
/// when the merge succeeds — only the `mcpServers.anvil` (or `VSCode`
/// equivalent) leaf.
fn merge_into_existing(target: Target, config_path: &Path, fresh: &Value) -> Result<String> {
    let existing: Option<Value> = match fs::read_to_string(config_path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            return Err(anyhow::Error::from(e))
                .with_context(|| format!("reading existing config at {}", config_path.display()));
        }
        Ok(raw) if raw.trim().is_empty() => None,
        Ok(raw) => Some(serde_json::from_str(&raw).with_context(|| {
            format!(
                "existing config at {} is not valid JSON; refusing to overwrite (resolve or remove the file and re-run)",
                config_path.display()
            )
        })?),
    };

    let merged = match existing {
        None => fresh.clone(),
        Some(mut base) => {
            if !base.is_object() {
                bail!("existing config root is not an object; refusing to overwrite");
            }
            // Only merge the leaf we own. Preserve every other key in the
            // user's editor config — they may have other MCP servers
            // configured, settings unrelated to MCP, etc.
            let entry = extract_entry(target, fresh).unwrap_or(Value::Null);
            insert_entry(target, &mut base, entry)?;
            base
        }
    };

    Ok(serde_json::to_string_pretty(&merged)?)
}

fn insert_entry(_target: Target, root: &mut Value, entry: Value) -> Result<()> {
    let obj = ensure_object(root);
    let servers = obj
        .entry("mcpServers".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let Value::Object(map) = servers else {
        bail!("existing config has non-object `mcpServers`; refusing to overwrite");
    };
    map.insert(SERVER_NAME.to_string(), entry);
    Ok(())
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
        Target::ClaudeCode => PathBuf::from(".claude.json"),
        Target::Cursor => PathBuf::from(".cursor").join("mcp.json"),
    }
}

fn target_label(target: Target) -> &'static str {
    match target {
        Target::ClaudeCode => "claude-code",
        Target::Cursor => "cursor",
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
        assert_eq!(entry["type"], "stdio");
        assert_eq!(entry["command"], "anvil");
        assert_eq!(entry["args"][0], "mcp");
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
    fn merge_into_existing_refuses_when_existing_is_invalid_json() {
        // Silently overwriting a malformed JSON file would clobber the
        // user's other MCP servers and editor settings. The safer default
        // is to fail loudly so the user can resolve before re-running.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.json");
        fs::write(&path, "{ not valid json,,, }").unwrap();
        let fresh = build_config(Target::Cursor, Transport::Stdio, 0, None);
        let err = merge_into_existing(Target::Cursor, &path, &fresh)
            .expect_err("invalid JSON must error, not overwrite");
        assert!(
            err.to_string().contains("refusing to overwrite"),
            "error must explain why we refuse: {err}"
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
            PathBuf::from(".claude.json"),
        );
        assert_eq!(
            relative_path_for(Target::Cursor),
            PathBuf::from(".cursor").join("mcp.json"),
        );
    }
}
