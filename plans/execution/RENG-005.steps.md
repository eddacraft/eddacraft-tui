# RENG-005: Benchmark all ported checks vs JS

## Steps

1. [x] Add criterion 0.5 to workspace dependencies
2. [x] Configure bench target in anvil-checks Cargo.toml
3. [x] Implement secret scan benchmarks (small/medium/large/clean + entropy)
4. [x] Implement anti-pattern benchmarks (TS small/medium/large + HTML + clean)
5. [x] Implement command safety benchmarks (parse + compound + wrapped + rule matching)
6. [x] Run `cargo bench --package anvil-checks` — all groups pass

## Results (Rust-only, in-memory)

### Secret Scan

| Benchmark | Time |
| --- | --- |
| small_file (~50 lines, 2 secrets) | 2.74 ms |
| medium_file (~500 lines, 5 secrets) | 4.55 ms |
| large_file (~5000 lines, 10 secrets) | 59.8 ms |
| clean_file (~500 lines, 0 secrets) | 2.64 ms |
| entropy short_string (16 chars) | 249 ns |
| entropy long_string (64 chars) | 761 ns |

### Anti-Pattern Detection

| Benchmark | Time |
| --- | --- |
| typescript_small (~50 lines, 3 patterns) | 759 µs |
| typescript_medium (~500 lines, 10 patterns) | 1.54 ms |
| typescript_large (~5000 lines, 20 patterns) | 3.26 ms |
| html_file (~200 lines) | 7.42 ms |
| clean_file (~500 lines, 0 patterns) | 503 µs |

### Command Safety

| Benchmark | Time |
| --- | --- |
| simple_command (`git status`) | 272 ns |
| dangerous_command (`git push --force`) | 373 ns |
| compound_command (3 chained) | 1.57 µs |
| wrapped_command (`sudo bash -c ...`) | 675 ns |
| complex_pipeline (pipe + chain) | 1.90 µs |
| rule_matching (parse + match 36 rules) | 9.25 µs |
