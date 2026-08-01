# Release operations

`eddacraft-tui` is maintained and released from the `eddacraft/anvil-001`
monorepo. The authoritative procedure is the
[`eddacraft-tui` crate release operator runbook][release-runbook].

Release preparation lands on `main` through a pull request. After merge, an
operator tags the merged commit as `eddacraft-tui-vX.Y.Z`; the canonical
[`publish-eddacraft-tui` workflow][publish-workflow] validates, publishes, and
updates the public mirror.

This file is packaged with the crate and mirrored as a pointer only. The public
mirror is not a release authority: do not publish from it, push bare `vX.Y.Z`
tags, or use its retired standalone token-driven release flow.

[release-runbook]: https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/eddacraft-tui-release.md
[publish-workflow]: https://github.com/eddacraft/anvil-001/blob/main/.github/workflows/publish-eddacraft-tui.yml
