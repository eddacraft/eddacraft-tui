use std::path::PathBuf;
use std::time::Instant;

/// A coalesced batch of file changes after debouncing.
#[derive(Debug, Clone)]
pub struct ChangeBatch {
    pub changes: Vec<FileChange>,
    pub received_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Created,
    Modified,
    Removed,
}
