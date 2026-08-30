//! SDT-004: the shared secret-pattern set must be compiled at service
//! construction, never inside a request.
//!
//! `DEFAULT_COMPILED_PATTERNS` is a `LazyLock`. Whoever touches it first pays
//! the whole catalogue's regex compilation — measured at 76 ms release and
//! 496 ms debug once the vendored tier-1 provider rules joined it, against
//! 1.5 ms / 16 ms for the 21 built-ins alone. If that first toucher is a
//! `scan_buffer` request, the cost lands inside the 2s `SCAN_BUFFER_TIMEOUT`,
//! which is how it first surfaced: as a CI timeout in the conformance suite,
//! not as a benchmark regression — the benchmark harness force-initialises the
//! lazy in its own warm-up, so it could never see this.
//!
//! **This file must contain exactly one test.** Rust builds one binary per
//! `tests/*.rs`, and the assertion is about which code touched the lazy
//! *first* in the process. A second test here could warm it and make this one
//! pass vacuously.

use std::sync::LazyLock;
use std::time::Instant;

use anvil_checks::secret::patterns::DEFAULT_COMPILED_PATTERNS;
use anvil_intercept::midedit::ScanBufferService;

#[test]
fn constructing_the_service_compiles_the_pattern_set_off_the_request_path() {
    let service = ScanBufferService::default();

    // Already forced by construction, so this is a pointer read. Left unforced
    // it is a full catalogue compile — two orders of magnitude slower than the
    // bound below even in a debug build on a loaded machine.
    let start = Instant::now();
    LazyLock::force(&DEFAULT_COMPILED_PATTERNS);
    let elapsed = start.elapsed();

    assert!(
        elapsed < std::time::Duration::from_millis(5),
        "the pattern set was not compiled at service construction: forcing it \
         afterwards took {elapsed:?}, so the first scan_buffer would have paid \
         that inside the {:?} budget",
        anvil_intercept::midedit::SCAN_BUFFER_TIMEOUT
    );
    drop(service);
}
