# docs-shell local agent notes

Follow the repository-wide [agent contract](../../AGENTS.md), then read this
component's [README](README.md) and [architecture map](ARCHITECTURE.md).

- Preserve the live topology: `apps/docs-shell` fronts `apps/anvil-docs-private`
  and `apps/docs-public`; the former rollback host `apps/docs-site` has been
  deleted.
- Treat BAUTH's [auth as-built](../../docs/architecture/auth-as-built.md) as the
  authentication authority. Do not redefine licence, OAuth, or tier semantics
  here.
- Keep the recorded DOCRB/DSITE ownership gap visible; local changes do not
  alter sibling APS status.
- Validate changes with the narrow `@eddacraft/docs-shell` test, typecheck, and
  build commands in the README, plus repository-required documentation checks
  when Markdown changes.
