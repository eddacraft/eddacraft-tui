//! SPG-005: throughput baseline for the rayon-parallel anti-pattern scanner.
//!
//! RSCAN-005 called out `cargo bench -p anvil-bench --bench antipattern_scan`
//! as the CI guard for the parallel-scan claim in ADR-026; that bench did not
//! exist. This file closes that gap. Corpus v2 is 320 synthetic artifacts
//! spanning every `ArtifactKind` (source, pr-description, commit-message,
//! agent-output). Its source share is balanced across TypeScript, Rust, and
//! Python so the headline throughput is not a proxy for one input language.
//! The content exercises language-scoped regex hot paths, the `flags:"i"`
//! inline prefix, hand-coded PCRE post-filters, and rayon fan-out.
//!
//! Baselines recorded in `crates/anvil-bench/README.md` under the
//! `antipattern_scan` heading.

use std::time::Duration;

use anvil_checks::antipattern::{Artifact, ArtifactKind, scan_artifacts};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};

const CORPUS_SIZE: usize = 320;
const CORPUS_VERSION: u8 = 2;

/// Build a deterministic synthetic corpus that exercises every rule family
/// and every artifact kind the scanner supports:
///
/// - Source artifacts split evenly across `.ts`, `.rs`, and `.py`.
///   They cover representative language-specific rules plus shared
///   DD-001..003 debt rules and the hand-coded PCRE post-filters.
/// - `pr-description` and `agent-output` artifacts covering DD-004 and
///   RL-001/005 — exercises the flags:"i" inline prefix and the RL-001/005
///   post-filters.
/// - `commit-message` artifacts covering RL-002/005.
///
/// Each slot cycles through several distinct content strings so the
/// benchmark measures real match work rather than hitting a tiny content
/// cache — adversarial-reviewer M-4.
fn build_corpus() -> Vec<Artifact> {
    const TYPESCRIPT_SOURCE: &[&str] = &[
        "const v: any = fetchData();\n// TODO refactor soon\n",
        "// HACK force admin auth\nconst n = value!;\n",
        "try {\n  doWork();\n} catch (e) {}\n",
        "// temporary fix pending review\nconst q = payload!;\n",
        "// FIXME handle retry\nconst s: any = resp;\n",
        "const ok = 1;\nexport function clean(): void { return; }\n",
    ];
    const RUST_SOURCE: &[&str] = &[
        "// TODO refactor soon\npub fn load() {}\n",
        "// HACK bypass validation\npub fn save() {}\n",
        "// temporary compatibility shim\npub fn bridge() {}\n",
        "// FIXME handle retry\npub fn retry() {}\n",
        "// XXX bypass validation\npub fn validate() {}\n",
        "pub fn clean(value: usize) -> usize { value + 1 }\n",
    ];
    const PYTHON_SOURCE: &[&str] = &[
        "value = payload  # type: ignore\n# TODO refactor soon\n",
        "import os  # noqa\n# HACK bypass validation\n",
        "from service.config import *\n",
        "result = eval(user_input)\n",
        "os.system(command)\n# temporary compatibility shim\n",
        "def clean(value: int) -> int: return value + 1\n",
    ];
    const PR_DESC: &[&str] = &[
        "This is a pre-existing failure unrelated to my change.\n",
        "Tracked as follow-up in issue #42.\n",
        "All failures are unrelated.\n",
        "none of which were touched by this change.\n",
        "Will defer cleanup to the next cycle.\n",
        "All requirements addressed.\n",
    ];
    const COMMIT: &[&str] = &[
        "fix(api): patch auth bug\n\nRemaining edge case tracked as follow-up in issue #99.\n",
        "chore: bump deps\n\nAdditional cleanup deferred to next cycle.\n",
        "feat: add endpoint\n\nVerified via end-to-end smoke test.\n",
    ];
    const AGENT: &[&str] = &[
        "Pre-existing failure on this branch, investigating.\n",
        "All requirements met; moving on.\n",
        "Tool call succeeded; output logged.\n",
    ];

    let mut artifacts = Vec::with_capacity(CORPUS_SIZE);
    for i in 0..CORPUS_SIZE {
        let (kind, content, ext) = match i % 10 {
            0 | 1 => (
                ArtifactKind::Source,
                TYPESCRIPT_SOURCE[i % TYPESCRIPT_SOURCE.len()],
                ".ts",
            ),
            2 | 3 => (
                ArtifactKind::Source,
                RUST_SOURCE[i % RUST_SOURCE.len()],
                ".rs",
            ),
            4 | 5 => (
                ArtifactKind::Source,
                PYTHON_SOURCE[i % PYTHON_SOURCE.len()],
                ".py",
            ),
            6 | 7 => (ArtifactKind::PrDescription, PR_DESC[i % PR_DESC.len()], ""),
            8 => (ArtifactKind::CommitMessage, COMMIT[i % COMMIT.len()], ""),
            _ => (ArtifactKind::AgentOutput, AGENT[i % AGENT.len()], ""),
        };
        let reference = match kind {
            ArtifactKind::Source => format!("src/fixture_{i:04}{ext}"),
            ArtifactKind::PrDescription => format!("pr/{i}"),
            ArtifactKind::CommitMessage => format!("sha{i:08}"),
            ArtifactKind::AgentOutput => format!("session/{i}"),
        };
        artifacts.push(Artifact {
            kind,
            reference,
            content: content.to_string(),
        });
    }
    artifacts
}

fn assert_corpus_v2_shape(corpus: &[Artifact]) {
    fn has_extension(reference: &str, expected: &str) -> bool {
        std::path::Path::new(reference)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
    }

    let mut counts = [0_usize; 6];
    for artifact in corpus {
        match artifact.kind {
            ArtifactKind::Source if has_extension(&artifact.reference, "ts") => counts[0] += 1,
            ArtifactKind::Source if has_extension(&artifact.reference, "rs") => counts[1] += 1,
            ArtifactKind::Source if has_extension(&artifact.reference, "py") => counts[2] += 1,
            ArtifactKind::PrDescription => counts[3] += 1,
            ArtifactKind::CommitMessage => counts[4] += 1,
            ArtifactKind::AgentOutput => counts[5] += 1,
            ArtifactKind::Source => {
                panic!("corpus v2 contains an unexpected source extension")
            }
        }
    }
    assert_eq!(
        counts,
        [64, 64, 64, 64, 32, 32],
        "corpus v2 distribution changed; bump CORPUS_VERSION"
    );
}

fn corpus_content_identity(corpus: &[Artifact]) -> u64 {
    fn update(hash: &mut u64, bytes: &[u8]) {
        for byte in bytes {
            *hash ^= u64::from(*byte);
            *hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }

    fn update_field(hash: &mut u64, value: &str) {
        update(hash, &(value.len() as u64).to_le_bytes());
        update(hash, value.as_bytes());
    }

    let mut hash = 0xcbf2_9ce4_8422_2325;
    for artifact in corpus {
        let kind = match artifact.kind {
            ArtifactKind::Source => 0_u8,
            ArtifactKind::PrDescription => 1,
            ArtifactKind::CommitMessage => 2,
            ArtifactKind::AgentOutput => 3,
        };
        update(&mut hash, &[kind]);
        update_field(&mut hash, &artifact.reference);
        update_field(&mut hash, &artifact.content);
    }
    hash
}

fn bench_parallel_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("antipattern_scan");
    group.sample_size(30);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(CORPUS_SIZE as u64));

    let corpus = build_corpus();
    assert_corpus_v2_shape(&corpus);
    let corpus_identity = corpus_content_identity(&corpus);

    group.bench_function(
        format!("parallel_balanced_corpus_v{CORPUS_VERSION}_fnv1a_{corpus_identity:016x}"),
        |b| {
            b.iter(|| scan_artifacts(&corpus, None));
        },
    );

    group.finish();
}

criterion_group!(benches, bench_parallel_scan);
criterion_main!(benches);
