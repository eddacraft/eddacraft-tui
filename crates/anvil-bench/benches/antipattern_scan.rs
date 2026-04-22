//! SPG-005: throughput baseline for the rayon-parallel anti-pattern scanner.
//!
//! RSCAN-005 called out `cargo bench -p anvil-bench --bench antipattern_scan`
//! as the CI guard for the parallel-scan claim in ADR-026; that bench did not
//! exist. This file closes that gap. The corpus is 320 synthetic artifacts
//! spanning every `ArtifactKind` (source, pr-description, commit-message,
//! agent-output) with content that exercises the regex hot path, the
//! `flags:"i"` inline prefix, the hand-coded PCRE post-filters, and the
//! rayon fan-out.
//!
//! Baselines recorded in `crates/anvil-bench/README.md` under the
//! `antipattern_scan` heading.

use std::time::Duration;

use anvil_checks::antipattern::{Artifact, ArtifactKind, scan_artifacts};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};

const CORPUS_SIZE: usize = 320;

/// Build a deterministic synthetic corpus that exercises every rule family
/// and every artifact kind the scanner supports:
///
/// - Source artifacts (.ts) covering AP-*, DD-001..003, GS-001 — plain
///   regex matches and the hand-coded PCRE post-filters for DD-001/002/003
///   and GS-001.
/// - `pr-description` and `agent-output` artifacts covering DD-004 and
///   RL-001/005 — exercises the flags:"i" inline prefix and the RL-001/005
///   post-filters.
/// - `commit-message` artifacts covering RL-002/005.
///
/// Each slot cycles through several distinct content strings so the
/// benchmark measures real match work rather than hitting a tiny content
/// cache — adversarial-reviewer M-4.
fn build_corpus() -> Vec<Artifact> {
    const SOURCE: &[&str] = &[
        "const v: any = fetchData();\n// TODO refactor soon\n",
        "// HACK force admin auth\nconst n = value!;\n",
        "try {\n  doWork();\n} catch (e) {}\n",
        "// temporary fix pending review\nconst q = payload!;\n",
        "// FIXME handle retry\nconst s: any = resp;\n",
        "const ok = 1;\nexport function clean(): void { return; }\n",
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
            0..=5 => (ArtifactKind::Source, SOURCE[i % SOURCE.len()], ".ts"),
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

fn bench_parallel_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("antipattern_scan");
    group.sample_size(30);
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(CORPUS_SIZE as u64));

    let corpus = build_corpus();

    group.bench_function("parallel_mixed_corpus", |b| {
        b.iter(|| scan_artifacts(&corpus, None));
    });

    group.finish();
}

criterion_group!(benches, bench_parallel_scan);
criterion_main!(benches);
