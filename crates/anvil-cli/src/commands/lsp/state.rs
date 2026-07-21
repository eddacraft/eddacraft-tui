use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

const MAX_OPEN_DOCUMENTS: usize = 256;
const MAX_TOTAL_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(super) struct ScanJob {
    pub uri: String,
    pub relative_path: PathBuf,
    pub version: i64,
    pub text: Arc<str>,
    pub content_hash: [u8; 32],
    generation: u64,
    pub cancelled: Arc<AtomicBool>,
}

#[derive(Debug)]
struct Document {
    version: i64,
    relative_path: PathBuf,
    text: Arc<str>,
    content_hash: [u8; 32],
    last_scanned_hash: Option<[u8; 32]>,
    due: Option<Instant>,
    in_flight: bool,
    generation: u64,
    cancellation: Arc<AtomicBool>,
    retained_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DocumentCapacityExceeded;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChangeError {
    StaleVersion,
    CapacityExceeded,
}

pub(super) struct DocumentStore {
    documents: HashMap<String, Document>,
    debounce: Duration,
    total_bytes: usize,
    next_generation: u64,
}

impl DocumentStore {
    pub fn new(debounce: Duration) -> Self {
        Self {
            documents: HashMap::new(),
            debounce,
            total_bytes: 0,
            next_generation: 1,
        }
    }

    pub fn open(
        &mut self,
        uri: &str,
        relative_path: PathBuf,
        version: i64,
        text: &str,
        now: Instant,
    ) -> Result<(), DocumentCapacityExceeded> {
        let replaced_bytes = self
            .documents
            .get(uri)
            .map_or(0, |document| document.retained_bytes);
        let retained_bytes = uri
            .len()
            .saturating_add(relative_path.to_string_lossy().len())
            .saturating_add(text.len());
        let next_total = self
            .total_bytes
            .saturating_sub(replaced_bytes)
            .saturating_add(retained_bytes);
        if text.len() > super::protocol::MAX_DOCUMENT_BYTES
            || next_total > MAX_TOTAL_DOCUMENT_BYTES
            || (!self.documents.contains_key(uri) && self.documents.len() >= MAX_OPEN_DOCUMENTS)
        {
            return Err(DocumentCapacityExceeded);
        }
        let generation = self.next_generation;
        self.next_generation = self.next_generation.wrapping_add(1).max(1);
        self.total_bytes = next_total;
        if let Some(previous) = self.documents.get(uri) {
            previous.cancellation.store(true, Ordering::Release);
        }
        self.documents.insert(
            uri.to_string(),
            Document {
                version,
                relative_path,
                text: Arc::from(text),
                content_hash: content_hash(text),
                last_scanned_hash: None,
                due: Some(now + self.debounce),
                in_flight: false,
                generation,
                cancellation: Arc::new(AtomicBool::new(false)),
                retained_bytes,
            },
        );
        Ok(())
    }

    pub fn change(
        &mut self,
        uri: &str,
        version: i64,
        text: &str,
        now: Instant,
    ) -> Result<(), ChangeError> {
        let document = self
            .documents
            .get_mut(uri)
            .ok_or(ChangeError::StaleVersion)?;
        let fixed_bytes = document.retained_bytes.saturating_sub(document.text.len());
        let retained_bytes = fixed_bytes.saturating_add(text.len());
        if version <= document.version {
            return Err(ChangeError::StaleVersion);
        }
        if text.len() > super::protocol::MAX_DOCUMENT_BYTES
            || self
                .total_bytes
                .saturating_sub(document.retained_bytes)
                .saturating_add(retained_bytes)
                > MAX_TOTAL_DOCUMENT_BYTES
        {
            return Err(ChangeError::CapacityExceeded);
        }
        let hash = content_hash(text);
        self.total_bytes = self
            .total_bytes
            .saturating_sub(document.retained_bytes)
            .saturating_add(retained_bytes);
        document.version = version;
        document.text = Arc::from(text);
        document.retained_bytes = retained_bytes;
        document.content_hash = hash;
        document.cancellation.store(true, Ordering::Release);
        document.cancellation = Arc::new(AtomicBool::new(false));
        document.generation = self.next_generation;
        self.next_generation = self.next_generation.wrapping_add(1).max(1);
        document.in_flight = false;
        document.due = if document.last_scanned_hash == Some(hash) {
            None
        } else {
            Some(now + self.debounce)
        };
        Ok(())
    }

    pub fn close(&mut self, uri: &str) {
        if let Some(document) = self.documents.remove(uri) {
            document.cancellation.store(true, Ordering::Release);
            self.total_bytes = self.total_bytes.saturating_sub(document.retained_bytes);
        }
    }

    pub fn close_all(&mut self) {
        for document in self.documents.values() {
            document.cancellation.store(true, Ordering::Release);
        }
        self.documents.clear();
        self.total_bytes = 0;
    }

    #[cfg(test)]
    pub fn take_due(&mut self, now: Instant) -> Vec<ScanJob> {
        self.take_due_bounded(now, usize::MAX)
    }

    pub fn take_due_bounded(&mut self, now: Instant, limit: usize) -> Vec<ScanJob> {
        let mut jobs = Vec::new();
        for (uri, document) in &mut self.documents {
            if jobs.len() >= limit {
                break;
            }
            if document.in_flight || document.due.is_none_or(|due| due > now) {
                continue;
            }
            document.due = None;
            document.in_flight = true;
            jobs.push(ScanJob {
                uri: uri.clone(),
                relative_path: document.relative_path.clone(),
                version: document.version,
                text: document.text.clone(),
                content_hash: document.content_hash,
                generation: document.generation,
                cancelled: Arc::clone(&document.cancellation),
            });
        }
        jobs
    }

    pub fn finish(&mut self, job: &ScanJob, successful: bool) -> bool {
        let Some(document) = self.documents.get_mut(&job.uri) else {
            return false;
        };
        document.in_flight = false;
        if document.generation != job.generation
            || document.version != job.version
            || document.content_hash != job.content_hash
        {
            return false;
        }
        if successful {
            document.last_scanned_hash = Some(job.content_hash);
        }
        true
    }

    pub fn retry(&mut self, job: &ScanJob, now: Instant) {
        if let Some(document) = self.documents.get_mut(&job.uri)
            && document.generation == job.generation
        {
            document.in_flight = false;
            document.due = Some(now + Duration::from_millis(10));
        }
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        self.documents
            .values()
            .filter(|document| !document.in_flight)
            .filter_map(|document| document.due)
            .min()
    }
}

fn content_hash(text: &str) -> [u8; 32] {
    Sha256::digest(text.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};

    use super::{ChangeError, DocumentStore};

    #[test]
    fn debounce_keeps_only_the_latest_document_version() {
        let started = Instant::now();
        let mut store = DocumentStore::new(Duration::from_millis(80));
        store
            .open("file:///src/main.rs", "main.rs".into(), 1, "one", started)
            .expect("document capacity");
        store
            .change(
                "file:///src/main.rs",
                2,
                "two",
                started + Duration::from_millis(20),
            )
            .expect("newer version");

        assert!(
            store
                .take_due(started + Duration::from_millis(99))
                .is_empty()
        );
        let jobs = store.take_due(started + Duration::from_millis(100));

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].version, 2);
        assert_eq!(jobs[0].text.as_ref(), "two");
    }

    #[test]
    fn change_distinguishes_stale_versions_from_capacity_rejection() {
        let started = Instant::now();
        let mut store = DocumentStore::new(Duration::ZERO);
        store
            .open("file:///src/main.rs", "main.rs".into(), 1, "one", started)
            .expect("document capacity");

        assert_eq!(
            store.change("file:///src/main.rs", 1, "stale", started),
            Err(ChangeError::StaleVersion)
        );
        assert_eq!(
            store.change(
                "file:///src/main.rs",
                2,
                &"x".repeat(super::super::protocol::MAX_DOCUMENT_BYTES + 1),
                started,
            ),
            Err(ChangeError::CapacityExceeded)
        );
    }

    #[test]
    fn stale_scan_results_never_become_publishable() {
        let started = Instant::now();
        let mut store = DocumentStore::new(Duration::from_millis(80));
        store
            .open("file:///src/main.rs", "main.rs".into(), 1, "one", started)
            .expect("document capacity");
        let first = store
            .take_due(started + Duration::from_millis(80))
            .pop()
            .expect("first scan");
        store
            .change(
                "file:///src/main.rs",
                2,
                "two",
                started + Duration::from_millis(90),
            )
            .expect("newer version");

        assert!(!store.finish(&first, true));
        let second = store
            .take_due(started + Duration::from_millis(170))
            .pop()
            .expect("replacement scan");
        assert_eq!(second.version, 2);
    }

    #[test]
    fn replacement_scan_is_not_serialised_behind_a_stale_result() {
        let started = Instant::now();
        let mut store = DocumentStore::new(Duration::from_millis(80));
        store
            .open("file:///src/main.rs", "main.rs".into(), 1, "one", started)
            .unwrap();
        let stale = store
            .take_due(started + Duration::from_millis(80))
            .pop()
            .unwrap();
        store
            .change(
                "file:///src/main.rs",
                2,
                "two",
                started + Duration::from_millis(90),
            )
            .unwrap();

        let replacement = store
            .take_due(started + Duration::from_millis(170))
            .pop()
            .expect("new version launches without waiting for stale completion");
        assert_eq!(replacement.version, 2);
        assert!(stale.cancelled.load(Ordering::Acquire));
        assert!(!store.finish(&stale, true));
    }

    #[test]
    fn an_identical_completed_buffer_does_not_round_trip_twice() {
        let started = Instant::now();
        let mut store = DocumentStore::new(Duration::from_millis(80));
        store
            .open("file:///src/main.rs", "main.rs".into(), 1, "same", started)
            .expect("document capacity");
        let first = store
            .take_due(started + Duration::from_millis(80))
            .pop()
            .expect("first scan");
        assert!(store.finish(&first, true));

        store
            .change(
                "file:///src/main.rs",
                2,
                "same",
                started + Duration::from_millis(90),
            )
            .expect("newer version");

        assert!(store.take_due(started + Duration::from_secs(1)).is_empty());
    }

    #[test]
    fn close_and_reopen_rejects_the_old_lifetime_result() {
        let started = Instant::now();
        let mut store = DocumentStore::new(Duration::from_millis(80));
        store
            .open("file:///src/main.rs", "main.rs".into(), 1, "same", started)
            .unwrap();
        let old = store
            .take_due(started + Duration::from_millis(80))
            .pop()
            .unwrap();
        store.close("file:///src/main.rs");
        store
            .open("file:///src/main.rs", "main.rs".into(), 1, "same", started)
            .unwrap();
        assert!(!store.finish(&old, true));
    }

    #[test]
    fn a_failed_scan_is_not_cached_as_completed() {
        let started = Instant::now();
        let mut store = DocumentStore::new(Duration::from_millis(80));
        store
            .open("file:///src/main.rs", "main.rs".into(), 1, "same", started)
            .unwrap();
        let failed = store
            .take_due(started + Duration::from_millis(80))
            .pop()
            .unwrap();
        assert!(store.finish(&failed, false));
        store
            .change("file:///src/main.rs", 2, "same", started)
            .unwrap();
        assert_eq!(store.take_due(started + Duration::from_millis(80)).len(), 1);
    }
}
