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

## Design history

The starter kit grew out of the Rust-CLI attribution pipeline shipped with the
`eddacraft/anvil` project. The multi-ecosystem roadmap (CycloneDX intermediate,
bundled-binaries support, multi-block markers) is tracked in that repository.
This mirror is purely the kit; consult anvil for the "why".

---

The remainder of this file is the kit's contract documentation, mirrored
verbatim from upstream:
