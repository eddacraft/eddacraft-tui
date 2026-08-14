# acknowledgements-starter

A drop-in third-party-attribution pipeline. A dispatcher reads a `[[blocks]]`
array from `attribution.toml` and routes each block to an ecosystem-specific
driver — Rust ([`cargo-about`](https://github.com/EmbarkStudios/cargo-about)),
Node ([`license-checker`](https://github.com/davglass/license-checker)), Go
([`go-licenses`](https://github.com/google/go-licenses)), Python
([`pip-licenses`](https://github.com/raimon49/pip-licenses)), and
hand-maintained bundled binaries — splicing each driver's output between
`BEGIN`/`END` marker comments in a target markdown file (typically
`ACKNOWLEDGEMENTS.md`). Hand-curated content above, between, and below the
markers is preserved verbatim.

This kit is licensed under the [Apache License 2.0](./LICENSE).

> **This repository is a read-only mirror.** The canonical source lives in a
> private upstream repository and is force-pushed here whenever it changes.
> Direct commits to `main` here will be overwritten on the next sync. Please
> open issues against [`eddacraft/anvil`](https://github.com/eddacraft/anvil) —
> or, if you don't have access, file an issue here and the maintainer will
> mirror it upstream.

## Adopting the kit

As a tracked subtree (recommended — easy to pull updates). Use a **per-kit
prefix** so this kit is its own independently tracked subtree:

```bash
git subtree add --prefix tools/starters/acknowledgements \
  https://github.com/eddacraft/acknowledgements-starter.git main --squash
```

> Don't shorten this to `--prefix tools/starters`. `git subtree add` makes the
> prefix directory the tracked subtree as a whole — using the parent would lock
> the entire `tools/starters/` directory to this one kit, and a second starter
> (`logging-starter`, etc.) could not be added at the same parent. Keep one
> subtree per kit.

To pull future updates:

```bash
git subtree pull --prefix tools/starters/acknowledgements \
  https://github.com/eddacraft/acknowledgements-starter.git main --squash
```

Or just `cp -r` the directory in if you don't want subtree tracking.

### Tracking latest vs pinning a release

`main` always holds the latest kit and is force-pushed on every change — adopt
or pull from `main` (above) to track the bleeding edge.

To pin a **specific, immutable version** instead, adopt or pull from a release
tag (`vX.Y.Z`). Use the current latest on the
[Releases page](https://github.com/eddacraft/acknowledgements-starter/releases)
— do not copy an old tag from an earlier README. This release is `v1.2.0`:

```bash
git subtree add --prefix tools/starters/acknowledgements \
  https://github.com/eddacraft/acknowledgements-starter.git v1.2.0 --squash
```

Release tags are append-only — a published `vX.Y.Z` never changes. Each release
has a GitHub Release with notes drawn from [`CHANGELOG.md`](./CHANGELOG.md);
**watch this repository's releases** to be notified when the kit updates.
Versions follow [SemVer](https://semver.org/): a major bump signals a breaking
change to the kit's contract. `v1.0.0` and `v1.1.x` remain available for
history, but they are not the recommended pin: `v1.0.0` can silently delete
curated prose, and `v1.1.x` drops most Rust copyright notices.

## Design history

The starter kit grew out of the Rust-CLI attribution pipeline shipped with the
`eddacraft/anvil` project. The multi-ecosystem roadmap (CycloneDX intermediate,
bundled-binaries support, multi-block markers) is tracked in that repository.
This mirror is purely the kit; consult anvil for the "why".

---

The remainder of this file is the kit's contract documentation, mirrored
verbatim from upstream:
