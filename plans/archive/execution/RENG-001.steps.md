# RENG-001: Port secret scan to Rust

## Steps

1. [x] Create `anvil-checks` crate with workspace config
2. [x] Define secret pattern types and catalogue (18 patterns)
3. [x] Implement allowlist and code-pattern filter
4. [x] Implement Shannon entropy detector
5. [x] Implement pattern matcher (regex scan per line)
6. [x] Implement secret check runner (file scanning, deduplication, scoring)
7. [x] Implement git history scanner
8. [x] Add unit tests with parity fixtures (13 tests passing)
9. [x] Verify `cargo test` passes, `cargo clippy` clean
