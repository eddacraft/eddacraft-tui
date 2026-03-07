# RENG-003: Port command safety check to Rust

## Steps

1. [x] Define command safety types (rules, parsed commands, analysis results)
2. [x] Implement command parser (tokenisation, wrapper unwrapping, compound splitting)
3. [x] Implement default git rules (17 rules)
4. [x] Implement default filesystem rules (19 rules)
5. [x] Implement rule matcher (specificity scoring, flag/arg/condition matching)
6. [x] Implement command safety check runner (orchestration, scoring)
7. [x] Add unit tests with parity fixtures (36 tests passing)
8. [x] Verify `cargo test` passes, `cargo clippy` clean
