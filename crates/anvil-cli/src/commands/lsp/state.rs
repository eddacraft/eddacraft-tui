use std::collections::HashMap;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

const MAX_OPEN_DOCUMENTS: usize = 256;

#[derive(Debug, Clone)]
pub(super) struct ScanJob {
    pub uri: String,
    pub version: i64,
    pub text: String,
    pub content_hash: [u8; 32],
}

#[derive(Debug)]
struct Document {
    version: i64,
    text: String,
    content_hash: [u8; 32],
    last_scanned_hash: Option<[u8; 32]>,
    due: Option<Instant>,
    in_flight: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StaleVersion;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DocumentCapacityExceeded;

pub(super) struct DocumentStore {
    documents: HashMap<String, Document>,
    debounce: Duration,
}

impl DocumentStore {
    pub fn new(debounce: Duration) -> Self {
        Self {
            documents: HashMap::new(),
            debounce,
        }
    }

    pub fn open(
        &mut self,
        uri: &str,
        version: i64,
        text: &str,
        now: Instant,
    ) -> Result<(), DocumentCapacityExceeded> {
        if !self.documents.contains_key(uri) && self.documents.len() >= MAX_OPEN_DOCUMENTS {
            return Err(DocumentCapacityExceeded);
        }
        self.documents.insert(
            uri.to_string(),
            Document {
                version,
                text: text.to_string(),
                content_hash: content_hash(text),
                last_scanned_hash: None,
                due: Some(now + self.debounce),
                in_flight: false,
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
    ) -> Result<(), StaleVersion> {
        let document = self.documents.get_mut(uri).ok_or(StaleVersion)?;
        if version <= document.version {
            return Err(StaleVersion);
        }
        let hash = content_hash(text);
        document.version = version;
        document.text.clear();
        document.text.push_str(text);
        document.content_hash = hash;
        document.due = if document.last_scanned_hash == Some(hash) {
            None
        } else {
            Some(now + self.debounce)
        };
        Ok(())
    }

    pub fn close(&mut self, uri: &str) {
        self.documents.remove(uri);
    }

    pub fn take_due(&mut self, now: Instant) -> Vec<ScanJob> {
        let mut jobs = Vec::new();
        for (uri, document) in &mut self.documents {
            if document.in_flight || !document.due.is_some_and(|due| due <= now) {
                continue;
            }
            document.due = None;
            document.in_flight = true;
            jobs.push(ScanJob {
                uri: uri.clone(),
                version: document.version,
                text: document.text.clone(),
                content_hash: document.content_hash,
            });
        }
        jobs
    }

    pub fn finish(&mut self, job: &ScanJob) -> bool {
        let Some(document) = self.documents.get_mut(&job.uri) else {
            return false;
        };
        document.in_flight = false;
        if document.version != job.version || document.content_hash != job.content_hash {
            return false;
        }
        document.last_scanned_hash = Some(job.content_hash);
        true
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
    use std::time::{Duration, Instant};

    use super::DocumentStore;

    #[test]
    fn debounce_keeps_only_the_latest_document_version() {
        let started = Instant::now();
        let mut store = DocumentStore::new(Duration::from_millis(80));
        store
            .open("file:///src/main.rs", 1, "one", started)
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
        assert_eq!(jobs[0].text, "two");
    }

    #[test]
    fn stale_scan_results_never_become_publishable() {
        let started = Instant::now();
        let mut store = DocumentStore::new(Duration::from_millis(80));
        store
            .open("file:///src/main.rs", 1, "one", started)
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

        assert!(!store.finish(&first));
        let second = store
            .take_due(started + Duration::from_millis(170))
            .pop()
            .expect("replacement scan");
        assert_eq!(second.version, 2);
    }

    #[test]
    fn an_identical_completed_buffer_does_not_round_trip_twice() {
        let started = Instant::now();
        let mut store = DocumentStore::new(Duration::from_millis(80));
        store
            .open("file:///src/main.rs", 1, "same", started)
            .expect("document capacity");
        let first = store
            .take_due(started + Duration::from_millis(80))
            .pop()
            .expect("first scan");
        assert!(store.finish(&first));

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
}
