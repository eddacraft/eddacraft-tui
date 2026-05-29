# json-render fixture specs

These three files are **verbatim copies** of the `@eddacraft/render` template
dashboard specs at `packages/libs/render/specs/`:

- `gate-summary.dashboard.json`
- `watch-session.dashboard.json`
- `architecture-health.dashboard.json`

They are vendored here so `eddacraft-tui` stays self-contained when mirrored out
of the monorepo (ADR-047) and its tests do not reach across package boundaries.

They exercise the json-render parser (`tests/json_render_specs.rs`) for
round-trip fidelity and base-catalogue validation. Keeping the vendored copies
in sync with the web source — and asserting catalogue parity against
`@eddacraft/render` — is owned by **TUIDASH-010** (catalogue parity), not this
work item. Refresh by re-copying from `packages/libs/render/specs/` when the
templates change.
