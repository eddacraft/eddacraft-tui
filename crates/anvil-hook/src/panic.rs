use std::panic::PanicHookInfo;

/// Path component (under `~/.local/state/anvil/`) where panic
/// reports are appended. Pinned by ADR-038 §D-7. The CLI consumer
/// is responsible for resolving this against the actual state-dir
/// path; the library only exposes the filename so test fixtures can
/// reuse it.
pub const PANIC_LOG_FILE: &str = "intercept-panic.log";

/// A structured record of a hook-time panic.
///
/// Built by [`format_panic_report`] from a [`PanicHookInfo`]; the
/// CLI wrapper writes the `log_text` to the panic log file and
/// passes the structured fields through to a witness append.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanicReport {
    /// Short panic message extracted from the `PanicHookInfo`
    /// payload (the `panic!("...")` string, or
    /// `"<non-string panic payload>"` when the payload isn't a
    /// `&str`/`String`).
    pub message: String,
    /// Source-location string in the form `"file:line:col"` when
    /// available, otherwise `"unknown"`. Useful for the witness
    /// record without leaking the full backtrace to stderr.
    pub location: String,
    /// Multi-line block suitable for appending to the panic log
    /// file. Includes a UTC timestamp placeholder (`{ts}`) the
    /// caller substitutes before writing — keeping `chrono` out of
    /// this crate's dependency surface.
    pub log_text: String,
}

/// Format a [`PanicHookInfo`] into a [`PanicReport`].
///
/// Pure function (no I/O, no allocation beyond the report itself)
/// so tests can drive it with synthetic panics without spawning a
/// child process.
pub fn format_panic_report(info: &PanicHookInfo<'_>) -> PanicReport {
    let payload = info.payload();
    let message = if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    };
    let location = info.location().map_or_else(
        || "unknown".to_string(),
        |loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()),
    );
    let log_text =
        format!("--- anvil hook panic @ {{ts}} ---\nmessage: {message}\nlocation: {location}\n");
    PanicReport {
        message,
        location,
        log_text,
    }
}

/// Build the actual `std::panic::set_hook` payload.
///
/// The closure captures a sink (`Fn(PanicReport) + Send + Sync +
/// 'static`) that the CLI wrapper supplies; the sink is responsible
/// for the side-effects ADR-038 §D-7 demands:
///
/// 1. Append `report.log_text` to `intercept-panic.log`.
/// 2. Emit the one-line stderr message: `anvil: hook errored (anvil
///    doctor for details)`. The library already builds that line
///    via [`crate::Verdict::InternalError`]; the sink wires the
///    suppression log.
/// 3. Append a witness line tagged `L3:{status:"error",
///    reason:"panic", log_path:"..."}` — also the sink's job.
///
/// The hook itself does NOT call `std::process::exit(0)` — the
/// caller decides when to exit so panic handling integrates with
/// the same exit-code rendering ([`crate::render_verdict`]) used
/// for clean verdicts. (Exiting from inside the panic hook would
/// also skip Drop on running stack frames, which the witness
/// writer's lock needs.)
pub fn panic_catcher_hook<F>(sink: F) -> Box<dyn Fn(&PanicHookInfo<'_>) + Send + Sync + 'static>
where
    F: Fn(PanicReport) + Send + Sync + 'static,
{
    Box::new(move |info| {
        let report = format_panic_report(info);
        sink(report);
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::MutexGuard;

    /// `std::panic::set_hook` mutates process-wide state, so every
    /// test that installs a custom hook must serialize against the
    /// others. Without this guard, Rust's parallel test runner can
    /// race two tests' `set_hook` + `take_hook` pairs and leak a
    /// stale hook into a third test.
    static PANIC_HOOK_LOCK: Mutex<()> = Mutex::new(());

    fn lock_panic_hook() -> MutexGuard<'static, ()> {
        // Recover from a poisoned lock — if a prior test panicked
        // *while holding* the guard, the data is still fine (it's
        // just `()`).
        match PANIC_HOOK_LOCK.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    #[test]
    fn panic_log_filename_is_pinned() {
        // ADR-038 §D-7 names this file explicitly. Don't drift.
        assert_eq!(PANIC_LOG_FILE, "intercept-panic.log");
    }

    #[test]
    fn format_extracts_static_str_message() {
        let _guard = lock_panic_hook();
        let captured: Arc<Mutex<Option<PanicReport>>> = Arc::new(Mutex::new(None));
        let cap = Arc::clone(&captured);
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let prev = panic::take_hook();
            panic::set_hook(Box::new(move |info| {
                *cap.lock().unwrap() = Some(format_panic_report(info));
            }));
            let _ = panic::catch_unwind(|| panic!("boom"));
            panic::set_hook(prev);
        }));
        assert!(result.is_ok());
        let report = captured.lock().unwrap().clone().expect("hook fired");
        assert_eq!(report.message, "boom");
        assert!(
            report.location.contains(':'),
            "expected file:line:col, got {}",
            report.location
        );
    }

    #[test]
    fn format_extracts_string_message() {
        let _guard = lock_panic_hook();
        let captured: Arc<Mutex<Option<PanicReport>>> = Arc::new(Mutex::new(None));
        let cap = Arc::clone(&captured);
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let prev = panic::take_hook();
            panic::set_hook(Box::new(move |info| {
                *cap.lock().unwrap() = Some(format_panic_report(info));
            }));
            let _ = panic::catch_unwind(|| {
                let s: String = "owned-payload".to_string();
                panic!("{}", s);
            });
            panic::set_hook(prev);
        }));
        assert!(result.is_ok());
        let report = captured.lock().unwrap().clone().expect("hook fired");
        assert_eq!(report.message, "owned-payload");
    }

    #[test]
    fn format_handles_non_string_payload() {
        let _guard = lock_panic_hook();
        let captured: Arc<Mutex<Option<PanicReport>>> = Arc::new(Mutex::new(None));
        let cap = Arc::clone(&captured);
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let prev = panic::take_hook();
            panic::set_hook(Box::new(move |info| {
                *cap.lock().unwrap() = Some(format_panic_report(info));
            }));
            let _ = panic::catch_unwind(|| panic::panic_any(42u32));
            panic::set_hook(prev);
        }));
        assert!(result.is_ok());
        let report = captured.lock().unwrap().clone().expect("hook fired");
        assert_eq!(report.message, "<non-string panic payload>");
    }

    #[test]
    fn log_text_contains_timestamp_placeholder() {
        let _guard = lock_panic_hook();
        // The library can't include a wall-clock timestamp without
        // pulling in `chrono`; expose `{ts}` for the caller to
        // substitute. Pin the placeholder so substitution can't
        // silently break.
        let captured: Arc<Mutex<Option<PanicReport>>> = Arc::new(Mutex::new(None));
        let cap = Arc::clone(&captured);
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let prev = panic::take_hook();
            panic::set_hook(Box::new(move |info| {
                *cap.lock().unwrap() = Some(format_panic_report(info));
            }));
            let _ = panic::catch_unwind(|| panic!("x"));
            panic::set_hook(prev);
        }));
        assert!(result.is_ok());
        let report = captured.lock().unwrap().clone().expect("hook fired");
        assert!(report.log_text.contains("{ts}"));
        assert!(report.log_text.contains("message: x"));
        assert!(report.log_text.contains("location:"));
    }

    #[test]
    fn panic_catcher_hook_forwards_to_sink() {
        let _guard = lock_panic_hook();
        let calls: Arc<Mutex<Vec<PanicReport>>> = Arc::new(Mutex::new(Vec::new()));
        let calls_for_sink = Arc::clone(&calls);
        let hook = panic_catcher_hook(move |report| calls_for_sink.lock().unwrap().push(report));
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            let prev = panic::take_hook();
            panic::set_hook(hook);
            let _ = panic::catch_unwind(|| panic!("sink-test"));
            panic::set_hook(prev);
        }));
        assert!(result.is_ok());
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].message, "sink-test");
    }
}
