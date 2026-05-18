# acknowledgements-starter

A drop-in third-party-attribution pipeline. Wraps
[`cargo-about`](https://github.com/EmbarkStudios/cargo-about) and splices its
output between `BEGIN`/`END` marker comments in a target markdown file
(typically `ACKNOWLEDGEMENTS.md`). Hand-curated content above and below the
markers is preserved verbatim.

> **This repository is a read-only mirror.** The canonical source lives in a
> private upstream repository and is force-pushed here whenever it changes.
> Direct commits to `main` here will be overwritten on the next sync. Please
> open issues against [`eddacraft/anvil`](https://github.com/eddacraft/anvil) —
> or, if you don't have access, file an issue here and the maintainer will
> mirror it upstream.

## Adopting the kit

This mirror is namespaced — its root contains a single `acknowledgements/`
directory holding the kit files. That means you point `git subtree` at the
parent directory you want the kit to live under, and the `acknowledgements/`
folder names itself. If you ever adopt a second starter kit alongside this one,
it can do the same trick under its own name and the two won't collide.

As a tracked subtree (recommended — easy to pull updates):

```bash
git subtree add --prefix tools/starters \
  https://github.com/eddacraft/acknowledgements-starter.git main --squash
```

Result: the kit lands at `tools/starters/acknowledgements/*`.

To pull future updates:

```bash
git subtree pull --prefix tools/starters \
  https://github.com/eddacraft/acknowledgements-starter.git main --squash
```

Or just `cp -r` the `acknowledgements/` directory in if you don't want subtree
tracking.

## Documentation

The kit's full contract — marker syntax, idempotency guarantees, the `--check`
exit-code semantics, strict licence-field enforcement, the single-source licence
allow-list pattern, atomic-write / empty-output / marker-count gates, and the
configuration reference — lives in
[`acknowledgements/README.md`](./acknowledgements/README.md). That file is
mirrored verbatim from upstream and is the canonical reference.

## Design history

The starter kit grew out of the Rust-CLI attribution pipeline shipped with the
`eddacraft/anvil` project. The multi-ecosystem roadmap (CycloneDX intermediate,
bundled-binaries support, multi-block markers) lives in the
`attribution-pipeline-v3` APS module in that repository. This mirror is purely
the kit; consult anvil for the "why".
