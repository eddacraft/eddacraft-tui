# Continuous-improvement closeout

Before the final response on non-trivial work, run `pnpm ci-log:append`
(pending by default). `Improvement: none` is valid.

Do not skip the note because it looks unrelated to the feature PR — the
pending queue is PR-independent (CIB-191). Check `pnpm ci-log:status`, not
the tracked log, to see whether the practice is alive.

Full procedure: `docs/guides/continuous-improvement-log.md`.
The MUST also lives in `AGENTS.md` Operating Rules.
