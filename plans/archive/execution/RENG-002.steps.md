# RENG-002: Port anti-pattern detection to Rust

## Steps

1. [x] Define anti-pattern types (Warning, AntiPattern, WarningResult, WarningSummary, Location, Suppression)
2. [x] Implement 13 pattern definitions (AP-001 through AP-013) with regex, severity, file extensions
3. [x] Implement scanner (line-by-line regex, file extension filtering, allowlist, suppression via @anvil-ignore)
4. [x] Implement check runner (severity scoring, pass/fail threshold, warning fingerprinting)
5. [x] Add unit tests with parity fixtures
6. [x] Verify `cargo test` passes, `cargo clippy` clean
