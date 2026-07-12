use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anvil_intercept::workspace_anchor::WorkspaceAnchor;
use thiserror::Error;

pub const MAX_ARTEFACT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug)]
pub struct Workspace {
    canonical_root: PathBuf,
    anchor: Mutex<WorkspaceAnchor>,
}

impl Workspace {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, WorkspaceReadError> {
        let canonical = std::fs::canonicalize(root.as_ref())
            .map_err(|source| WorkspaceReadError::InvalidRoot { source })?;
        let anchor = WorkspaceAnchor::open(&canonical)
            .map_err(|source| WorkspaceReadError::InvalidRoot { source })?;
        Ok(Self {
            canonical_root: canonical,
            anchor: Mutex::new(anchor),
        })
    }

    pub fn root(&self) -> &Path {
        &self.canonical_root
    }

    pub fn read(&self, relative: &Path) -> Result<Vec<u8>, WorkspaceReadError> {
        let display = relative.to_string_lossy().into_owned();
        let relative = relative
            .to_str()
            .ok_or_else(|| WorkspaceReadError::UnsafePath {
                path: display.clone(),
            })?;
        let anchor = self
            .anchor
            .lock()
            .map_err(|_| WorkspaceReadError::BoundaryUnavailable)?;
        let bytes = anchor
            .read_rel_capped(relative, MAX_ARTEFACT_BYTES as u64)
            .map_err(|source| classify_read_error(&display, source))?;
        Ok(bytes)
    }
}

fn classify_read_error(path: &str, source: io::Error) -> WorkspaceReadError {
    if source.kind() == io::ErrorKind::InvalidInput {
        return WorkspaceReadError::UnsafePath {
            path: path.to_owned(),
        };
    }
    if source.kind() == io::ErrorKind::NotFound {
        return WorkspaceReadError::Missing {
            path: path.to_owned(),
        };
    }
    if source.kind() == io::ErrorKind::FileTooLarge {
        return WorkspaceReadError::TooLarge {
            path: path.to_owned(),
            max_bytes: MAX_ARTEFACT_BYTES,
        };
    }
    if is_link_rejection(&source) {
        return WorkspaceReadError::Symlink {
            path: path.to_owned(),
        };
    }
    WorkspaceReadError::Unavailable {
        path: path.to_owned(),
        source,
    }
}

#[cfg(unix)]
fn is_link_rejection(error: &io::Error) -> bool {
    matches!(
        error.raw_os_error(),
        Some(code)
            if code == nix::errno::Errno::ELOOP as i32
                || code == nix::errno::Errno::ENOTDIR as i32
    )
}

#[cfg(windows)]
fn is_link_rejection(error: &io::Error) -> bool {
    error.raw_os_error() == Some(anvil_intercept_win32::read_safety::ERROR_CANT_RESOLVE_FILENAME)
}

#[derive(Debug, Error)]
pub enum WorkspaceReadError {
    #[error("workspace root is unavailable")]
    InvalidRoot { source: io::Error },
    #[error("artefact path is outside the workspace boundary: {path}")]
    UnsafePath { path: String },
    #[error("artefact path resolves through a link: {path}")]
    Symlink { path: String },
    #[error("artefact does not exist: {path}")]
    Missing { path: String },
    #[error("artefact exceeds the {max_bytes}-byte dashboard limit: {path}")]
    TooLarge { path: String, max_bytes: usize },
    #[error("workspace read boundary is unavailable")]
    BoundaryUnavailable,
    #[error("artefact is unavailable: {path}")]
    Unavailable { path: String, source: io::Error },
}

impl WorkspaceReadError {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidRoot { .. } => "invalid-workspace-root",
            Self::UnsafePath { .. } => "unsafe-artefact-path",
            Self::Symlink { .. } => "symlinked-artefact-path",
            Self::Missing { .. } => "artefact-not-found",
            Self::TooLarge { .. } => "artefact-too-large",
            Self::BoundaryUnavailable => "workspace-boundary-unavailable",
            Self::Unavailable { .. } => "artefact-unavailable",
        }
    }
}
