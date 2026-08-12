//! Install and verify the beta Anvil Agent Skill bundle (SKPKG / ADR-106).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use serde::Serialize;
use serde_json::json;

use crate::GlobalArgs;
use crate::activation::agent_registry::{AgentClientId, InstallScope};
use crate::activation::detect_agents::RealDetectionEnv;
use crate::commands::skill_state::{
    self, DEFAULT_SKILL_NAME, MANIFEST_NAME, ManagedManifest, SKILL_MD, TOOL_REFERENCE,
    expected_developer_functions_manifest,
};

const SKILL_NAME: &str = DEFAULT_SKILL_NAME;

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
    /// Client to install into. Required for non-interactive installation;
    /// repeat to select more than one. Scripted fleets must enumerate every
    /// destination.
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
    /// The destination as a user should see it, rendered once at construction.
    ///
    /// CIB-282: this was a `PathBuf`, which serde serialises as the underlying
    /// string. The human branch stripped the Windows NT-extended prefix while
    /// `--json` beside it emitted `\\?\C:\...` verbatim — one surface, two path
    /// styles, the split CIB-237 was filed to end. Rendering here rather than at
    /// each print site is what stops the two branches drifting apart again.
    path: String,
    status: &'static str,
}

impl TargetReport {
    fn new(clients: Vec<&'static str>, destination: &Path, status: &'static str) -> Self {
        let path = destination.to_string_lossy();
        Self {
            clients,
            path: crate::display_path::strip_verbatim_prefix(&path).into_owned(),
            status,
        }
    }
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
        reports.push(TargetReport::new(clients, &destination, status));
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
            // CIB-237 / CIB-282: already rendered by `TargetReport::new`, so
            // this branch and `--json` above cannot disagree about the path.
            println!(
                "{} [{}] — {}",
                report.path,
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
    if path_exists_nofollow(destination)? {
        validate_managed_state(destination)?;
    }
    Ok(())
}

fn install_bundle(destination: &Path) -> Result<&'static str> {
    ensure_safe_destination(destination)?;
    let current = if path_exists_nofollow(destination)? {
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
    create_dir_all_nofollow(parent)
        .with_context(|| format!("creating {}", crate::display_path::shown(parent)))?;

    // Hold a no-follow directory handle for the parent before staging so a
    // concurrent symlink swap of a path component cannot redirect mkdir/write.
    let mut staging = create_staging_dir(parent).with_context(|| {
        format!(
            "staging managed skill beside {}",
            crate::display_path::shown(destination)
        )
    })?;

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

    // Revalidate the commit path under no-follow discipline immediately before
    // the rename (closes the residual check-then-use window on the destination).
    ensure_safe_destination(destination)?;
    ensure_safe_destination(parent)?;
    replace_directory(staging.path(), destination)?;
    staging.defuse();
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
            "managed skill at {} is valid but not the bundle shipped by this anvil version",
            crate::display_path::shown(destination)
        );
    }
    Ok(())
}

fn validate_managed_state(destination: &Path) -> Result<ManagedManifest> {
    let manifest_path = destination.join(MANIFEST_NAME);
    ensure_safe_destination(&manifest_path)?;
    if !path_exists_nofollow(&manifest_path)? {
        bail!(
            "refusing to overwrite unmanaged skill directory {}; move it outside the skills directory tree or choose another scope",
            crate::display_path::shown(destination)
        );
    }
    let raw_bytes = read_regular_file_nofollow(&manifest_path)
        .with_context(|| format!("reading {}", crate::display_path::shown(&manifest_path)))?;
    let raw = String::from_utf8(raw_bytes).with_context(|| {
        format!(
            "managed manifest {} is not valid UTF-8",
            crate::display_path::shown(&manifest_path)
        )
    })?;
    let manifest: ManagedManifest = serde_json::from_str(&raw).with_context(|| {
        format!(
            "parsing managed manifest {}",
            crate::display_path::shown(&manifest_path)
        )
    })?;
    if manifest.schema_version != 1 || manifest.skill != SKILL_NAME {
        bail!(
            "managed manifest {} has unsupported schema or skill identity; refusing to overwrite",
            crate::display_path::shown(&manifest_path)
        );
    }
    for (relative, expected_hash) in &manifest.files {
        if !skill_state::is_safe_relative_path(relative) {
            bail!("managed manifest contains unsafe relative path `{relative}`");
        }
        // CIB-237: manifest keys are `/`-separated; component-wise joining
        // keeps the path in the error message natively separated.
        let path = crate::display_path::join_relative(destination, relative);
        ensure_safe_destination(&path)?;
        let bytes = read_regular_file_nofollow(&path).with_context(|| {
            format!(
                "managed skill file {} is missing or modified; refusing to overwrite",
                crate::display_path::shown(&path)
            )
        })?;
        if skill_state::sha256(&bytes) != *expected_hash {
            bail!(
                "managed skill file {} was modified; refusing to overwrite user changes",
                crate::display_path::shown(&path)
            );
        }
    }
    if skill_state::bundle_digest(&manifest.files) != manifest.bundle_digest {
        bail!(
            "managed manifest {} has an invalid bundle digest; refusing to overwrite",
            crate::display_path::shown(&manifest_path)
        );
    }
    validate_no_unmanaged_entries(destination, &manifest.files)?;
    Ok(manifest)
}

fn validate_no_unmanaged_entries(
    destination: &Path,
    managed_files: &BTreeMap<String, String>,
) -> Result<()> {
    let mut allowed_files = managed_files
        .keys()
        .map(PathBuf::from)
        .collect::<BTreeSet<_>>();
    allowed_files.insert(PathBuf::from(MANIFEST_NAME));

    let mut allowed_directories = BTreeSet::new();
    for file in &allowed_files {
        let mut parent = file.parent();
        while let Some(path) = parent {
            if path.as_os_str().is_empty() {
                break;
            }
            allowed_directories.insert(path.to_path_buf());
            parent = path.parent();
        }
    }

    let mut directories = vec![destination.to_path_buf()];
    while let Some(directory) = directories.pop() {
        ensure_safe_destination(&directory)?;
        let entries = fs::read_dir(&directory).with_context(|| {
            format!(
                "inspecting managed skill directory {}",
                crate::display_path::shown(&directory)
            )
        })?;
        for entry in entries {
            let entry = entry.with_context(|| {
                format!(
                    "inspecting managed skill directory {}",
                    crate::display_path::shown(&directory)
                )
            })?;
            let path = entry.path();
            let relative = path.strip_prefix(destination).with_context(|| {
                format!(
                    "checking managed skill entry {}",
                    crate::display_path::shown(&path)
                )
            })?;
            let metadata = fs::symlink_metadata(&path).with_context(|| {
                format!(
                    "inspecting managed skill entry {}",
                    crate::display_path::shown(&path)
                )
            })?;

            let allowed = if metadata.file_type().is_symlink() {
                false
            } else if metadata.is_dir() {
                if allowed_directories.contains(relative) {
                    directories.push(path.clone());
                    true
                } else {
                    false
                }
            } else if metadata.is_file() {
                allowed_files.contains(relative)
            } else {
                false
            };

            if !allowed {
                bail!(
                    "managed skill directory contains unmanaged entry {}; move it outside the skills directory tree before retrying",
                    crate::display_path::shown(&path)
                );
            }
        }
    }
    Ok(())
}

fn expected_manifest() -> ManagedManifest {
    expected_developer_functions_manifest()
}

fn write_staged_file(destination: &Path, relative: &str, content: &str) -> Result<()> {
    // Installer-controlled relatives are either MANIFEST_NAME or paths already
    // known safe; still refuse traversal before joining.
    if relative != MANIFEST_NAME && !skill_state::is_safe_relative_path(relative) {
        bail!("refusing to write managed skill file with unsafe relative path `{relative}`");
    }
    let path = crate::display_path::join_relative(destination, relative);
    write_regular_file_nofollow(&path, content.as_bytes()).with_context(|| {
        format!(
            "writing managed skill file {}",
            crate::display_path::shown(&path)
        )
    })
}

fn replace_directory(staging: &Path, destination: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        replace_directory_nofollow(staging, destination)
    }
    #[cfg(not(unix))]
    {
        if let Some(parent) = destination.parent() {
            ensure_safe_destination(parent)?;
        }
        ensure_safe_destination(staging)?;
        replace_directory_with(staging, destination, |from, to| fs::rename(from, to))
    }
}

#[cfg(any(test, not(unix)))]
fn replace_directory_with(
    staging: &Path,
    destination: &Path,
    mut rename: impl FnMut(&Path, &Path) -> io::Result<()>,
) -> Result<()> {
    if !destination.exists() {
        return rename(staging, destination).with_context(|| {
            format!(
                "committing staged skill bundle to {}",
                crate::display_path::shown(destination)
            )
        });
    }

    let parent = destination
        .parent()
        .context("managed skill destination has no parent directory")?;
    let backup_root = tempfile::Builder::new()
        .prefix(".anvil-skill-backup-")
        .tempdir_in(parent)
        .with_context(|| {
            format!(
                "preparing rollback beside {}",
                crate::display_path::shown(destination)
            )
        })?;
    let backup = backup_root.path().join("previous");
    rename(destination, &backup).with_context(|| {
        format!(
            "moving {} into rollback storage",
            crate::display_path::shown(destination)
        )
    })?;

    if let Err(commit_error) = rename(staging, destination) {
        if let Err(rollback_error) = rename(&backup, destination) {
            let retained = backup_root.keep();
            bail!(
                "committing staged skill bundle to {} failed: {}; rollback also failed: {}; the previous bundle is retained at {}",
                crate::display_path::shown(destination),
                commit_error,
                rollback_error,
                crate::display_path::shown(&retained.join("previous"))
            );
        }
        return Err(commit_error).with_context(|| {
            format!(
                "committing staged skill bundle to {}; the previous bundle was restored",
                crate::display_path::shown(destination)
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

/// Walk every path component with `symlink_metadata` and refuse symlinks.
///
/// This is an early advisory check used for clear error messages. Install and
/// write paths must not rely on it alone: they use no-follow openat/mkdirat
/// operations so a concurrent component swap cannot redirect the write.
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
                crate::display_path::shown(&cursor)
            );
        }
    }
    Ok(())
}

/// Create every missing directory component without following symlinks.
fn create_dir_all_nofollow(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        return Ok(());
    }

    #[cfg(unix)]
    {
        create_dir_all_nofollow_unix(path)
    }
    #[cfg(not(unix))]
    {
        fs::create_dir_all(path)
            .with_context(|| format!("creating {}", crate::display_path::shown(path)))?;
        ensure_safe_destination(path)?;
        Ok(())
    }
}

/// Write a regular file without following a final-component symlink and without
/// walking intermediate components through symlinks (Unix openat ladder).
fn write_regular_file_nofollow(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if path.file_name().is_none() {
        bail!("managed skill file path has no file name");
    }
    create_dir_all_nofollow(parent)?;

    #[cfg(unix)]
    {
        let leaf = path
            .file_name()
            .context("managed skill file path has no file name")?;
        write_regular_file_nofollow_unix(parent, leaf, content)
    }
    #[cfg(not(unix))]
    {
        ensure_safe_destination(path)?;
        fs::write(path, content).map_err(Into::into)
    }
}

/// Read a regular file without following a final-component symlink.
fn read_regular_file_nofollow(path: &Path) -> Result<Vec<u8>> {
    #[cfg(unix)]
    {
        read_regular_file_nofollow_unix(path)
    }
    #[cfg(not(unix))]
    {
        ensure_safe_destination(path)?;
        fs::read(path).map_err(Into::into)
    }
}

/// Existence check that refuses symlink path components (and a symlink leaf).
fn path_exists_nofollow(path: &Path) -> Result<bool> {
    ensure_safe_destination(path)?;
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                bail!(
                    "refusing to install managed skill through symlinked path {}",
                    crate::display_path::shown(path)
                );
            }
            Ok(true)
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => {
            Err(err).with_context(|| format!("inspecting {}", crate::display_path::shown(path)))
        }
    }
}

struct StagingDir {
    path: PathBuf,
    active: bool,
}

impl StagingDir {
    fn path(&self) -> &Path {
        &self.path
    }

    fn defuse(&mut self) {
        self.active = false;
    }
}

impl Drop for StagingDir {
    fn drop(&mut self) {
        if self.active {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn create_staging_dir(parent: &Path) -> Result<StagingDir> {
    create_dir_all_nofollow(parent)?;

    #[cfg(unix)]
    {
        create_staging_dir_unix(parent)
    }
    #[cfg(not(unix))]
    {
        let staging = tempfile::Builder::new()
            .prefix(".anvil-skill-stage-")
            .tempdir_in(parent)
            .with_context(|| {
                format!(
                    "staging managed skill under {}",
                    crate::display_path::shown(parent)
                )
            })?;
        let path = staging.keep();
        ensure_safe_destination(&path)?;
        Ok(StagingDir { path, active: true })
    }
}

#[cfg(unix)]
fn create_dir_all_nofollow_unix(path: &Path) -> Result<()> {
    use std::os::fd::{AsFd, OwnedFd};
    use std::path::Component;

    use nix::fcntl::{OFlag, open};
    use nix::sys::stat::Mode;

    let dir_flags = OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_CLOEXEC;
    let nofollow_dir_flags = dir_flags | OFlag::O_NOFOLLOW;

    let mut components = path.components();
    let mut dirfd: OwnedFd = match components.next() {
        Some(Component::RootDir) => open(Path::new("/"), dir_flags, Mode::empty())
            .map_err(io::Error::from)
            .with_context(|| format!("opening {}", crate::display_path::shown(Path::new("/"))))?,
        Some(Component::CurDir) => open(Path::new("."), dir_flags, Mode::empty())
            .map_err(io::Error::from)
            .with_context(|| "opening current directory")?,
        Some(Component::Normal(name)) => {
            // Relative first component must not follow a symlink either.
            open_or_mkdir_component(None, name, nofollow_dir_flags, nofollow_dir_flags)?
        }
        Some(Component::Prefix(_)) => {
            bail!(
                "refusing managed skill path with Windows prefix {}",
                crate::display_path::shown(path)
            )
        }
        Some(Component::ParentDir) => {
            bail!(
                "refusing managed skill path with parent-dir component {}",
                crate::display_path::shown(path)
            )
        }
        None => return Ok(()),
    };

    for component in components {
        match component {
            Component::Normal(name) => {
                dirfd = open_or_mkdir_component(
                    Some(dirfd.as_fd()),
                    name,
                    nofollow_dir_flags,
                    nofollow_dir_flags,
                )?;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                bail!(
                    "refusing managed skill path with parent-dir component {}",
                    crate::display_path::shown(path)
                )
            }
            other => {
                bail!(
                    "refusing managed skill path with unsupported component {other:?} in {}",
                    crate::display_path::shown(path)
                )
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
fn open_or_mkdir_component(
    parent: Option<std::os::fd::BorrowedFd<'_>>,
    name: &std::ffi::OsStr,
    open_flags_existing: nix::fcntl::OFlag,
    open_flags_created: nix::fcntl::OFlag,
) -> Result<std::os::fd::OwnedFd> {
    use nix::errno::Errno;
    use nix::fcntl::{OFlag, open, openat};
    use nix::sys::stat::{Mode, mkdirat};

    let try_open = |flags: OFlag| -> std::result::Result<std::os::fd::OwnedFd, Errno> {
        match parent {
            Some(dirfd) => openat(dirfd, name, flags, Mode::empty()),
            None => open(Path::new(name), flags, Mode::empty()),
        }
    };

    match try_open(open_flags_existing) {
        Ok(fd) => Ok(fd),
        Err(Errno::ENOENT) => {
            match parent {
                Some(dirfd) => mkdirat(dirfd, name, Mode::from_bits_truncate(0o755))
                    .map_err(io::Error::from)
                    .with_context(|| {
                        format!(
                            "creating directory component {}",
                            crate::display_path::shown(Path::new(name))
                        )
                    })?,
                None => {
                    fs::create_dir(name).with_context(|| {
                        format!(
                            "creating directory component {}",
                            crate::display_path::shown(Path::new(name))
                        )
                    })?;
                }
            }
            try_open(open_flags_created)
                .map_err(io::Error::from)
                .with_context(|| {
                    format!(
                        "opening created directory component {}",
                        crate::display_path::shown(Path::new(name))
                    )
                })
        }
        Err(err) if is_nofollow_symlink_err(err) => bail!(
            "refusing to install managed skill through symlinked path {}",
            crate::display_path::shown(Path::new(name))
        ),
        Err(err) if is_non_directory_component_err(err) => bail!(
            "refusing managed skill path component that is not a real directory (symlink or non-directory) {}",
            crate::display_path::shown(Path::new(name))
        ),
        Err(err) => Err(io::Error::from(err)).with_context(|| {
            format!(
                "opening directory component {}",
                crate::display_path::shown(Path::new(name))
            )
        }),
    }
}

#[cfg(unix)]
fn open_dir_nofollow_unix(path: &Path) -> Result<std::os::fd::OwnedFd> {
    use std::os::fd::{AsFd, OwnedFd};
    use std::path::Component;

    use nix::fcntl::{OFlag, open, openat};
    use nix::sys::stat::Mode;

    let dir_flags = OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_CLOEXEC;
    let nofollow_dir_flags = dir_flags | OFlag::O_NOFOLLOW;

    let mut components = path.components();
    let mut dirfd: OwnedFd = match components.next() {
        Some(Component::RootDir) => open(Path::new("/"), dir_flags, Mode::empty())
            .map_err(io::Error::from)
            .with_context(|| format!("opening {}", crate::display_path::shown(Path::new("/"))))?,
        Some(Component::CurDir) => open(Path::new("."), dir_flags, Mode::empty())
            .map_err(io::Error::from)
            .with_context(|| "opening current directory")?,
        Some(Component::Normal(name)) => open(Path::new(name), nofollow_dir_flags, Mode::empty())
            .map_err(|err| map_symlink_open_error(err, Path::new(name)))
            .with_context(|| {
                format!(
                    "opening directory component {}",
                    crate::display_path::shown(Path::new(name))
                )
            })?,
        Some(Component::ParentDir) => {
            bail!(
                "refusing managed skill path with parent-dir component {}",
                crate::display_path::shown(path)
            )
        }
        Some(Component::Prefix(_)) => {
            bail!(
                "refusing managed skill path with Windows prefix {}",
                crate::display_path::shown(path)
            )
        }
        None => bail!("cannot open empty path as directory"),
    };

    for component in components {
        match component {
            Component::Normal(name) => {
                dirfd = openat(dirfd.as_fd(), name, nofollow_dir_flags, Mode::empty())
                    .map_err(|err| map_symlink_open_error(err, Path::new(name)))
                    .with_context(|| {
                        format!(
                            "opening directory component {}",
                            crate::display_path::shown(Path::new(name))
                        )
                    })?;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                bail!(
                    "refusing managed skill path with parent-dir component {}",
                    crate::display_path::shown(path)
                )
            }
            other => {
                bail!(
                    "refusing managed skill path with unsupported component {other:?} in {}",
                    crate::display_path::shown(path)
                )
            }
        }
    }
    Ok(dirfd)
}

#[cfg(unix)]
fn map_symlink_open_error(err: nix::errno::Errno, component: &Path) -> anyhow::Error {
    use nix::errno::Errno;
    match err {
        // With O_DIRECTORY|O_NOFOLLOW, a symlink may surface as ELOOP or
        // ENOTDIR depending on platform; a regular file is ENOTDIR.
        Errno::ELOOP => anyhow::anyhow!(
            "refusing to install managed skill through symlinked path {}",
            crate::display_path::shown(component)
        ),
        Errno::ENOTDIR => anyhow::anyhow!(
            "refusing managed skill path component that is not a real directory (symlink or non-directory) {}",
            crate::display_path::shown(component)
        ),
        other => anyhow::Error::from(io::Error::from(other)),
    }
}

#[cfg(unix)]
fn is_nofollow_symlink_err(err: nix::errno::Errno) -> bool {
    use nix::errno::Errno;
    // Final-component O_NOFOLLOW open of a symlink typically returns ELOOP.
    matches!(err, Errno::ELOOP)
}

#[cfg(unix)]
fn is_non_directory_component_err(err: nix::errno::Errno) -> bool {
    use nix::errno::Errno;
    // O_DIRECTORY|O_NOFOLLOW on a symlink often returns ENOTDIR on Linux; a
    // regular-file component returns ENOTDIR as well. Both are refusals.
    matches!(err, Errno::ENOTDIR)
}


#[cfg(unix)]
fn write_regular_file_nofollow_unix(
    parent: &Path,
    leaf: &std::ffi::OsStr,
    content: &[u8],
) -> Result<()> {
    use std::io::Write;
    use std::os::fd::AsFd;

    use nix::fcntl::{OFlag, openat};
    use nix::sys::stat::Mode;

    let dirfd = open_dir_nofollow_unix(parent)?;
    let flags =
        OFlag::O_CREAT | OFlag::O_TRUNC | OFlag::O_WRONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC;
    let fd = match openat(dirfd.as_fd(), leaf, flags, Mode::from_bits_truncate(0o644)) {
        Ok(fd) => fd,
        Err(err) if is_nofollow_symlink_err(err) => {
            bail!(
                "refusing to install managed skill through symlinked path {}",
                crate::display_path::shown(Path::new(leaf))
            )
        }
        Err(err) => {
            return Err(io::Error::from(err)).with_context(|| {
                format!(
                    "opening managed skill file {}",
                    crate::display_path::shown(Path::new(leaf))
                )
            });
        }
    };
    let mut file = fs::File::from(fd);
    file.write_all(content)?;
    file.flush()?;
    Ok(())
}

#[cfg(unix)]
fn read_regular_file_nofollow_unix(path: &Path) -> Result<Vec<u8>> {
    use std::io::Read;
    use std::os::fd::AsFd;

    use nix::fcntl::{OFlag, openat};
    use nix::sys::stat::{Mode, SFlag, fstat};

    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let leaf = path
        .file_name()
        .context("managed skill file path has no file name")?;
    let dirfd = open_dir_nofollow_unix(parent)?;
    let flags = OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC;
    let fd = match openat(dirfd.as_fd(), leaf, flags, Mode::empty()) {
        Ok(fd) => fd,
        Err(err) if is_nofollow_symlink_err(err) => {
            bail!(
                "refusing to install managed skill through symlinked path {}",
                crate::display_path::shown(path)
            )
        }
        Err(err) => {
            return Err(io::Error::from(err))
                .with_context(|| format!("opening {}", crate::display_path::shown(path)));
        }
    };
    let st = fstat(&fd).map_err(io::Error::from)?;
    let kind = SFlag::from_bits_truncate(st.st_mode);
    if !kind.contains(SFlag::S_IFREG) {
        bail!(
            "managed skill path {} is not a regular file",
            crate::display_path::shown(path)
        );
    }
    let mut file = fs::File::from(fd);
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(unix)]
fn create_staging_dir_unix(parent: &Path) -> Result<StagingDir> {
    use std::os::fd::AsFd;
    use std::time::{SystemTime, UNIX_EPOCH};

    use nix::errno::Errno;
    use nix::sys::stat::{Mode, mkdirat};

    let dirfd = open_dir_nofollow_unix(parent)?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    for attempt in 0..32u32 {
        let name = format!(
            ".anvil-skill-stage-{}-{}-{attempt}",
            std::process::id(),
            nanos
        );
        match mkdirat(
            dirfd.as_fd(),
            name.as_str(),
            Mode::from_bits_truncate(0o755),
        ) {
            Ok(()) => {
                return Ok(StagingDir {
                    path: parent.join(name),
                    active: true,
                });
            }
            Err(Errno::EEXIST) => {}
            Err(err) => {
                return Err(io::Error::from(err)).with_context(|| {
                    format!(
                        "creating staging directory under {}",
                        crate::display_path::shown(parent)
                    )
                });
            }
        }
    }
    bail!(
        "could not allocate a unique staging directory under {}",
        crate::display_path::shown(parent)
    )
}

#[cfg(unix)]
fn replace_directory_nofollow(staging: &Path, destination: &Path) -> Result<()> {
    use nix::fcntl::renameat;

    let parent = destination
        .parent()
        .context("managed skill destination has no parent directory")?;
    let staging_parent = staging
        .parent()
        .context("staged skill bundle has no parent directory")?;
    if staging_parent != parent {
        bail!(
            "staged skill bundle parent {} does not match destination parent {}",
            crate::display_path::shown(staging_parent),
            crate::display_path::shown(parent)
        );
    }
    let staging_name = staging
        .file_name()
        .context("staged skill bundle has no file name")?;
    let dest_name = destination
        .file_name()
        .context("managed skill destination has no file name")?;

    // Pin the parent with O_NOFOLLOW so renames cannot be redirected through a
    // swapped intermediate component.
    let parent_fd = open_dir_nofollow_unix(parent)?;
    if !destination_exists_nofollow(&parent_fd, dest_name, destination)? {
        renameat(&parent_fd, staging_name, &parent_fd, dest_name)
            .map_err(io::Error::from)
            .with_context(|| {
                format!(
                    "committing staged skill bundle to {}",
                    crate::display_path::shown(destination)
                )
            })?;
        return Ok(());
    }

    commit_with_backup_nofollow(&parent_fd, parent, staging_name, dest_name, destination)
}

#[cfg(unix)]
fn destination_exists_nofollow(
    parent_fd: &std::os::fd::OwnedFd,
    dest_name: &std::ffi::OsStr,
    destination: &Path,
) -> Result<bool> {
    use nix::errno::Errno;
    use nix::fcntl::AtFlags;
    use nix::sys::stat::{SFlag, fstatat};

    match fstatat(parent_fd, dest_name, AtFlags::AT_SYMLINK_NOFOLLOW) {
        Ok(st) => {
            let kind = SFlag::from_bits_truncate(st.st_mode);
            if kind.contains(SFlag::S_IFLNK) {
                bail!(
                    "refusing to install managed skill through symlinked path {}",
                    crate::display_path::shown(destination)
                );
            }
            Ok(true)
        }
        Err(Errno::ENOENT) => Ok(false),
        Err(err) => Err(io::Error::from(err))
            .with_context(|| format!("inspecting {}", crate::display_path::shown(destination))),
    }
}

#[cfg(unix)]
fn allocate_backup_dir(
    parent_fd: &std::os::fd::OwnedFd,
    parent: &Path,
    destination: &Path,
) -> Result<String> {
    use std::os::fd::AsFd;
    use std::time::{SystemTime, UNIX_EPOCH};

    use nix::errno::Errno;
    use nix::sys::stat::{Mode, mkdirat};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    for attempt in 0..32u32 {
        let name = format!(
            ".anvil-skill-backup-{}-{}-{attempt}",
            std::process::id(),
            nanos
        );
        match mkdirat(
            parent_fd.as_fd(),
            name.as_str(),
            Mode::from_bits_truncate(0o755),
        ) {
            Ok(()) => return Ok(name),
            Err(Errno::EEXIST) => {}
            Err(err) => {
                return Err(io::Error::from(err)).with_context(|| {
                    format!(
                        "preparing rollback beside {}",
                        crate::display_path::shown(destination)
                    )
                });
            }
        }
    }
    bail!(
        "could not allocate rollback directory beside {}",
        crate::display_path::shown(parent)
    )
}

#[cfg(unix)]
fn commit_with_backup_nofollow(
    parent_fd: &std::os::fd::OwnedFd,
    parent: &Path,
    staging_name: &std::ffi::OsStr,
    dest_name: &std::ffi::OsStr,
    destination: &Path,
) -> Result<()> {
    use std::ffi::OsStr;
    use std::os::fd::AsFd;

    use nix::fcntl::{OFlag, openat, renameat};
    use nix::sys::stat::Mode;

    let backup_name = allocate_backup_dir(parent_fd, parent, destination)?;
    let backup_path = parent.join(&backup_name);
    let backup_previous = OsStr::new("previous");

    let backup_fd = openat(
        parent_fd.as_fd(),
        backup_name.as_str(),
        OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    )
    .map_err(io::Error::from)
    .with_context(|| {
        format!(
            "opening rollback directory {}",
            crate::display_path::shown(&backup_path)
        )
    })?;

    renameat(parent_fd, dest_name, &backup_fd, backup_previous)
        .map_err(io::Error::from)
        .with_context(|| {
            format!(
                "moving {} into rollback storage",
                crate::display_path::shown(destination)
            )
        })?;

    if let Err(commit_error) = renameat(parent_fd, staging_name, parent_fd, dest_name) {
        if let Err(rollback_error) = renameat(&backup_fd, backup_previous, parent_fd, dest_name) {
            // Leave the backup directory in place for recovery.
            bail!(
                "committing staged skill bundle to {} failed: {}; rollback also failed: {}; the previous bundle is retained at {}",
                crate::display_path::shown(destination),
                io::Error::from(commit_error),
                io::Error::from(rollback_error),
                crate::display_path::shown(&backup_path.join("previous"))
            );
        }
        let _ = fs::remove_dir_all(&backup_path);
        return Err(io::Error::from(commit_error)).with_context(|| {
            format!(
                "committing staged skill bundle to {}; the previous bundle was restored",
                crate::display_path::shown(destination)
            )
        });
    }

    if let Err(error) = fs::remove_dir_all(&backup_path) {
        eprintln!(
            "warning: installed the managed skill but could not remove its rollback directory: {error}"
        );
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

    #[cfg(unix)]
    #[test]
    fn create_dir_all_nofollow_refuses_symlink_component_without_creating_outside() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let link = root.path().join("agents");
        symlink(outside.path(), &link).unwrap();

        let target = link.join("skills");
        let error = create_dir_all_nofollow(&target).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("symlinked path")
                || message.contains("symlink")
                || message.contains("not a real directory"),
            "unexpected error: {message}"
        );
        assert!(
            !outside.path().join("skills").exists(),
            "must not create directories on the far side of a symlink component"
        );
    }

    #[cfg(unix)]
    fn error_chain(error: &anyhow::Error) -> String {
        format!("{error:#}")
    }

    #[cfg(unix)]
    #[test]
    fn write_regular_file_nofollow_refuses_symlink_leaf_without_writing_through() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let victim = outside.path().join("captured");
        fs::write(&victim, b"safe").unwrap();

        let stage = root.path().join("stage");
        fs::create_dir(&stage).unwrap();
        let link = stage.join("SKILL.md");
        symlink(&victim, &link).unwrap();

        let error = write_staged_file(&stage, "SKILL.md", "pwned").unwrap_err();
        let message = error_chain(&error);
        assert!(
            message.contains("symlinked path")
                || message.contains("symlink")
                || message.contains("Too many levels of symbolic links"),
            "unexpected error: {message}"
        );
        assert_eq!(fs::read_to_string(&victim).unwrap(), "safe");
    }

    #[cfg(unix)]
    #[test]
    fn write_regular_file_nofollow_refuses_symlink_parent_component() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let agents = root.path().join("agents");
        symlink(outside.path(), &agents).unwrap();

        let destination = agents.join("skills");
        let error =
            write_regular_file_nofollow(&destination.join("SKILL.md"), b"pwned").unwrap_err();
        let message = error_chain(&error);
        assert!(
            message.contains("symlinked path")
                || message.contains("symlink")
                || message.contains("not a real directory"),
            "unexpected error: {message}"
        );
        assert!(
            !outside.path().join("skills").exists(),
            "must not create files under a redirected symlink parent"
        );
        assert!(
            !outside.path().join("skills/SKILL.md").exists(),
            "must not write through a redirected symlink parent"
        );
    }

    #[cfg(unix)]
    #[test]
    fn replace_directory_nofollow_refuses_symlink_parent() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        // Staging lives on the far side of the symlink so the path strings share
        // a parent component; openat(O_NOFOLLOW) on that parent must refuse.
        let parent_link = root.path().join("skills");
        symlink(outside.path(), &parent_link).unwrap();
        fs::create_dir(outside.path().join(".anvil-skill-stage-test")).unwrap();
        fs::write(
            outside.path().join(".anvil-skill-stage-test/SKILL.md"),
            "next",
        )
        .unwrap();

        let staging = parent_link.join(".anvil-skill-stage-test");
        let destination = parent_link.join("anvil-developer-functions");
        let error = replace_directory(&staging, &destination).unwrap_err();
        let message = error_chain(&error);
        assert!(
            message.contains("symlinked path")
                || message.contains("symlink")
                || message.contains("not a real directory"),
            "unexpected error: {message}"
        );
        assert!(
            !outside.path().join("anvil-developer-functions").exists(),
            "must not commit a skill bundle through a symlinked parent"
        );
    }
}
