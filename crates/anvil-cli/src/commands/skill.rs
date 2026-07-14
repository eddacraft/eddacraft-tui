//! Install and verify the beta Anvil Agent Skill bundle (SKPKG / ADR-106).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::GlobalArgs;
use crate::activation::agent_registry::{AgentClientId, InstallScope};
use crate::activation::detect_agents::RealDetectionEnv;
const SKILL_NAME: &str = "anvil-developer-functions";
const SOURCE_COMMIT: &str = "ef5b34c5f424c9de4292406405e4bedfb603a65a";
const SKILL_MD: &str = include_str!("../../assets/skills/anvil-developer-functions/SKILL.md");
const TOOL_REFERENCE: &str =
    include_str!("../../assets/skills/anvil-developer-functions/references/tool-reference.md");
const _BUNDLE_PROVENANCE: &str =
    include_str!("../../assets/skills/anvil-developer-functions/bundle-provenance.json");
const MANIFEST_NAME: &str = ".anvil-managed.json";

#[derive(Debug, Args)]
pub struct SkillArgs {
    #[command(subcommand)]
    command: SkillCommand,
}

#[derive(Debug, Subcommand)]
enum SkillCommand {
    /// Install the bundled Anvil developer-functions skill.
    Install(SkillInstallArgs),
}

#[derive(Debug, Args)]
struct SkillInstallArgs {
    /// Client to install into. Repeat to select more than one.
    #[arg(long, value_enum)]
    client: Vec<AgentClientId>,

    /// Install globally (default) or into the current project.
    #[arg(long, value_enum)]
    scope: Option<InstallScope>,

    /// Override the selected scope root. Primarily useful for automation.
    #[arg(long)]
    workspace: Option<PathBuf>,

    /// Verify the managed bundle without writing.
    #[arg(long, conflicts_with = "dry_run")]
    verify: bool,

    /// Preview resolved destinations without writing.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetReport {
    clients: Vec<&'static str>,
    path: PathBuf,
    status: &'static str,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManagedManifest {
    schema_version: u32,
    skill: String,
    source_commit: String,
    anvil_version: String,
    bundle_digest: String,
    files: BTreeMap<String, String>,
}

pub fn run(args: &SkillArgs, global: &GlobalArgs) -> Result<()> {
    match &args.command {
        SkillCommand::Install(install) => run_install(install, global),
    }
}

fn run_install(args: &SkillInstallArgs, global: &GlobalArgs) -> Result<()> {
    let interactive =
        !global.json && !global.no_tui && io::stdin().is_terminal() && io::stderr().is_terminal();
    let scope = resolve_scope(args.scope, interactive)?;
    let root = match &args.workspace {
        Some(path) => path.clone(),
        None if scope == InstallScope::Global => {
            crate::util::user_home_dir().context("could not determine home directory")?
        }
        None => std::env::current_dir().context("resolving project directory")?,
    };
    let clients = resolve_clients(&args.client, scope, interactive)?;
    let mut destinations: BTreeMap<PathBuf, Vec<&'static str>> = BTreeMap::new();
    for client in clients {
        let entry = *client.entry();
        let Some(skill_root) = entry.skill_root(scope, &root) else {
            bail!(
                "{} does not publish a documented {}-scope skill location",
                entry.display_name,
                scope.label()
            );
        };
        destinations
            .entry(skill_root.join(SKILL_NAME))
            .or_default()
            .push(entry.label());
    }

    let mut reports = Vec::new();
    for (destination, clients) in destinations {
        let status = if args.verify {
            verify_bundle(&destination)?;
            "verified"
        } else if args.dry_run {
            preview_bundle(&destination)?;
            "would install"
        } else {
            install_bundle(&destination)?
        };
        reports.push(TargetReport {
            clients,
            path: destination,
            status,
        });
    }

    if global.json {
        println!(
            "{}",
            json!({
                "scope": scope.label(),
                "dryRun": args.dry_run,
                "verify": args.verify,
                "targets": reports,
            })
        );
    } else {
        for report in reports {
            println!(
                "{} [{}] — {}",
                report.path.display(),
                report.clients.join(", "),
                report.status
            );
        }
    }
    Ok(())
}

fn resolve_scope(requested: Option<InstallScope>, interactive: bool) -> Result<InstallScope> {
    if let Some(scope) = requested {
        return Ok(scope);
    }
    if !interactive {
        return Ok(InstallScope::Global);
    }

    eprint!("Install scope [G]lobal/[p]roject (default global): ");
    io::stderr().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    match answer.trim().to_ascii_lowercase().as_str() {
        "" | "g" | "global" => Ok(InstallScope::Global),
        "p" | "project" => Ok(InstallScope::Project),
        other => bail!("unknown scope `{other}`; choose global or project"),
    }
}

fn resolve_clients(
    requested: &[AgentClientId],
    scope: InstallScope,
    interactive: bool,
) -> Result<Vec<AgentClientId>> {
    if !requested.is_empty() {
        return Ok(requested.to_vec());
    }

    let env = RealDetectionEnv;
    let detected = AgentClientId::all()
        .iter()
        .filter(|entry| entry.supports_skill(scope) && entry.detected(&env))
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    if detected.is_empty() {
        bail!("no supported agent client was strongly detected; pass one or more --client values");
    }
    if !interactive {
        bail!(
            "detected {}; non-interactive installation requires explicit --client",
            detected
                .iter()
                .map(|client| client.label())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    eprintln!("Detected skill-capable clients:");
    for (index, client) in detected.iter().enumerate() {
        eprintln!("  {}. {}", index + 1, client.entry().display_name);
    }
    eprint!("Install into [all] or comma-separated numbers (default all): ");
    io::stderr().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    let answer = answer.trim();
    if answer.is_empty() || answer.eq_ignore_ascii_case("all") {
        return Ok(detected);
    }

    let mut selected = BTreeSet::new();
    for token in answer.split(',').map(str::trim) {
        let index = token
            .parse::<usize>()
            .with_context(|| format!("invalid client selection `{token}`"))?;
        let client = detected
            .get(index.saturating_sub(1))
            .copied()
            .with_context(|| format!("client selection {index} is out of range"))?;
        selected.insert(client);
    }
    Ok(selected.into_iter().collect())
}

fn preview_bundle(destination: &Path) -> Result<()> {
    ensure_safe_destination(destination)?;
    if destination.exists() {
        validate_managed_state(destination)?;
    }
    Ok(())
}

fn install_bundle(destination: &Path) -> Result<&'static str> {
    ensure_safe_destination(destination)?;
    let current = if destination.exists() {
        Some(validate_managed_state(destination)?)
    } else {
        None
    };
    let expected = expected_manifest();
    if current.as_ref() == Some(&expected) {
        return Ok("already installed");
    }

    let parent = destination
        .parent()
        .context("managed skill destination has no parent directory")?;
    ensure_safe_destination(parent)?;
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    let staging = tempfile::Builder::new()
        .prefix(".anvil-skill-stage-")
        .tempdir_in(parent)
        .with_context(|| format!("staging managed skill beside {}", destination.display()))?;

    write_staged_file(staging.path(), "SKILL.md", SKILL_MD)?;
    write_staged_file(
        staging.path(),
        "references/tool-reference.md",
        TOOL_REFERENCE,
    )?;
    let manifest = format!("{}\n", serde_json::to_string_pretty(&expected)?);
    write_staged_file(staging.path(), MANIFEST_NAME, &manifest)?;
    let staged_manifest = validate_managed_state(staging.path())?;
    if staged_manifest != expected {
        bail!("staged managed skill bundle failed integrity verification");
    }

    replace_directory(staging.path(), destination)?;
    Ok(if current.is_some() {
        "updated"
    } else {
        "installed"
    })
}

fn verify_bundle(destination: &Path) -> Result<()> {
    ensure_safe_destination(destination)?;
    let actual = validate_managed_state(destination)?;
    let expected = expected_manifest();
    if actual != expected {
        bail!(
            "managed skill at {} is valid but not the bundle shipped by this Anvil version",
            destination.display()
        );
    }
    Ok(())
}

fn validate_managed_state(destination: &Path) -> Result<ManagedManifest> {
    let manifest_path = destination.join(MANIFEST_NAME);
    ensure_safe_destination(&manifest_path)?;
    if !manifest_path.exists() {
        bail!(
            "refusing to overwrite unmanaged skill directory {}; move it aside or choose another scope",
            destination.display()
        );
    }
    let raw = fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let manifest: ManagedManifest = serde_json::from_str(&raw)
        .with_context(|| format!("parsing managed manifest {}", manifest_path.display()))?;
    if manifest.schema_version != 1 || manifest.skill != SKILL_NAME {
        bail!(
            "managed manifest {} has unsupported schema or skill identity; refusing to overwrite",
            manifest_path.display()
        );
    }
    for (relative, expected_hash) in &manifest.files {
        let relative_path = Path::new(relative);
        if relative_path.as_os_str().is_empty()
            || relative_path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            bail!("managed manifest contains unsafe relative path `{relative}`");
        }
        let path = destination.join(relative_path);
        ensure_safe_destination(&path)?;
        let bytes = fs::read(&path).with_context(|| {
            format!(
                "managed skill file {} is missing or modified; refusing to overwrite",
                path.display()
            )
        })?;
        if sha256(&bytes) != *expected_hash {
            bail!(
                "managed skill file {} was modified; refusing to overwrite user changes",
                path.display()
            );
        }
    }
    if bundle_digest(&manifest.files) != manifest.bundle_digest {
        bail!(
            "managed manifest {} has an invalid bundle digest; refusing to overwrite",
            manifest_path.display()
        );
    }
    Ok(manifest)
}

fn expected_manifest() -> ManagedManifest {
    let files = BTreeMap::from([
        ("SKILL.md".to_string(), sha256(SKILL_MD.as_bytes())),
        (
            "references/tool-reference.md".to_string(),
            sha256(TOOL_REFERENCE.as_bytes()),
        ),
    ]);
    ManagedManifest {
        schema_version: 1,
        skill: SKILL_NAME.to_string(),
        source_commit: SOURCE_COMMIT.to_string(),
        anvil_version: env!("CARGO_PKG_VERSION").to_string(),
        bundle_digest: bundle_digest(&files),
        files,
    }
}

fn bundle_digest(files: &BTreeMap<String, String>) -> String {
    let mut digest = Sha256::new();
    for (relative, file_digest) in files {
        digest.update(relative.as_bytes());
        digest.update([0]);
        digest.update(file_digest.as_bytes());
        digest.update([0]);
    }
    hex::encode(digest.finalize())
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn write_staged_file(destination: &Path, relative: &str, content: &str) -> Result<()> {
    let path = destination.join(relative);
    ensure_safe_destination(&path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(&path, content.as_bytes())
        .with_context(|| format!("writing managed skill file {}", path.display()))
}

fn replace_directory(staging: &Path, destination: &Path) -> Result<()> {
    replace_directory_with(staging, destination, |from, to| fs::rename(from, to))
}

fn replace_directory_with(
    staging: &Path,
    destination: &Path,
    mut rename: impl FnMut(&Path, &Path) -> io::Result<()>,
) -> Result<()> {
    if !destination.exists() {
        return rename(staging, destination).with_context(|| {
            format!(
                "committing staged skill bundle to {}",
                destination.display()
            )
        });
    }

    let parent = destination
        .parent()
        .context("managed skill destination has no parent directory")?;
    let backup_root = tempfile::Builder::new()
        .prefix(".anvil-skill-backup-")
        .tempdir_in(parent)
        .with_context(|| format!("preparing rollback beside {}", destination.display()))?;
    let backup = backup_root.path().join("previous");
    rename(destination, &backup)
        .with_context(|| format!("moving {} into rollback storage", destination.display()))?;

    if let Err(commit_error) = rename(staging, destination) {
        if let Err(rollback_error) = rename(&backup, destination) {
            let retained = backup_root.keep();
            bail!(
                "committing staged skill bundle to {} failed: {}; rollback also failed: {}; the previous bundle is retained at {}",
                destination.display(),
                commit_error,
                rollback_error,
                retained.join("previous").display()
            );
        }
        return Err(commit_error).with_context(|| {
            format!(
                "committing staged skill bundle to {}; the previous bundle was restored",
                destination.display()
            )
        });
    }

    if let Err(error) = backup_root.close() {
        eprintln!(
            "warning: installed the managed skill but could not remove its rollback directory: {error}"
        );
    }
    Ok(())
}

fn ensure_safe_destination(destination: &Path) -> Result<()> {
    let mut cursor = PathBuf::new();
    for component in destination.components() {
        cursor.push(component);
        let Ok(metadata) = fs::symlink_metadata(&cursor) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            bail!(
                "refusing to install managed skill through symlinked path {}",
                cursor.display()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_bundle_commit_restores_the_previous_directory() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("skill");
        let staging = root.path().join("staging");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("version"), "previous").unwrap();
        fs::create_dir(&staging).unwrap();
        fs::write(staging.join("version"), "next").unwrap();
        let mut calls = 0;

        let error = replace_directory_with(&staging, &destination, |from, to| {
            calls += 1;
            if calls == 2 {
                return Err(io::Error::other("injected commit failure"));
            }
            fs::rename(from, to)
        })
        .unwrap_err();

        assert!(error.to_string().contains("previous bundle was restored"));
        assert_eq!(
            fs::read_to_string(destination.join("version")).unwrap(),
            "previous"
        );
        assert_eq!(fs::read_to_string(staging.join("version")).unwrap(), "next");
    }
}
