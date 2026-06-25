#!/usr/bin/env bash
# Collect Rust workspace coverage for nightly assurance (CICD-006).
#
# Uses cargo-llvm-cov's --no-report split so nextest collection and report
# generation are separate phases. This avoids corrupt
# profraw merges when parallel nextest workers race on profile files and
# matches the upstream nextest/cargo-llvm-cov CI guidance.

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "${repo_root}"

json_output="${1:-coverage-rust.json}"
summary_output="${2:-coverage-rust-summary.txt}"
html_dir="${3:-target/llvm-cov}"

# Drop stale profiles/target instrumentation state. Required when re-running
# locally or when the restored target cache holds stale or non-instrumented
# artefacts that would corrupt the coverage merge.
cargo llvm-cov clean --workspace

# Serialise test execution for stable profraw merge. The strict rust-tests gate
# keeps nextest parallel; coverage is observability-only.
cargo llvm-cov --no-report nextest --workspace --test-threads 1

# nextest does not run doctests. llvm-cov --doc needs a nightly compiler for
# instrumented doctests; the stable rust-tests gate runs `cargo test --doc`
# separately without coverage merge. Keep nightly coverage nextest-only.

mkdir -p "$(dirname "${json_output}")" "${html_dir}"

cargo llvm-cov report --json --output-path "${json_output}"
# Write directly to file; piping to tee can SIGPIPE when the consumer closes early.
cargo llvm-cov report --summary-only --output-path "${summary_output}"
cargo llvm-cov report --html --output-dir "${html_dir}"