# eddacraft-tui

> **This repository is a read-only mirror.** The canonical source for
> `eddacraft-tui` lives inside the Anvil project repository. Direct commits and
> source PRs against `main` here are not accepted — `main` is force-pushed by
> the mirror automation whenever the canonical tree changes, and any local
> commits will be overwritten on the next sync.

## How to depend on this crate

Depend on the **crates.io release**, not on git `main`:

```toml
[dependencies]
eddacraft-tui = "0.5"
```

`main` is mirror-managed and may be force-pushed at any time. Release tags
(`eddacraft-tui-v*`) are append-only and never rewritten by the mirror job — pin
to a tag if you need a frozen reference.

## How to report bugs or propose changes

Issues are open on this mirror — file them here and a maintainer will triage and
forward into the canonical project as needed. Source PRs opened against this
mirror are **auto-closed** by the
[`pr-redirect.yml`](.github/workflows/pr-redirect.yml) workflow with a link to
the contribution guide; auto-close protects your work, because `main` here is
force-pushed by automation on every canonical change and any local commits would
be silently overwritten on the next sync. If a maintainer accepts a change you
proposed, they will port it into the canonical tree and the next mirror sync
will carry it out — credit goes via a `Co-Authored-By:` trailer on the ported
commit.

## Why this layout

Anvil is the load-bearing consumer of `eddacraft-tui`. Hosting the canonical
source in the Anvil monorepo lets widget and consumer changes ship atomically
without a publish-then-bump round trip; mirroring out and publishing to
crates.io preserves the public trust surface for external users. See
[`CONTRIBUTING.md`](CONTRIBUTING.md) "External contributors" section for the
contribution path that applies from this mirror.

---
