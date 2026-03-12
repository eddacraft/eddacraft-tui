# Branching Strategy

## Branches

| Branch | Purpose                                              | Protection                         |
| ------ | ---------------------------------------------------- | ---------------------------------- |
| `main` | Production releases. Always deployable.              | PRs only from `dev`. Full CI gate. |
| `dev`  | Active development. All feature/fix PRs target here. | PRs required. Standard CI.         |

## Workflow

```
feature/xyz ──PR──► dev ──PR──► main (release)
fix/abc     ──PR──► dev ──PR──► main (release)
```

1. **Feature/fix work**: Branch from `dev`, PR back to `dev`
2. **Release**: PR from `dev` to `main` — triggers full CI including
   cross-platform (macOS/Windows)
3. **Hotfixes**: Branch from `main`, PR to `main`, then cherry-pick or merge
   back to `dev`

## CI Tiers

### PRs to `dev` (lightweight)

- Lint & Format
- Type Check
- Unit Tests (Linux, Node 20)
- Build (Linux, Node 20)
- E2E Tests (if changed)
- Security scans

### PRs to `main` (release gate)

All of the above PLUS:

- Cross-platform smoke tests (macOS + Windows)

### Nightly (`ci-nightly.yml`)

- Cross-platform: macOS + Windows
- Multi-version: Node 22 + 24
- Runs at 02:00 UTC / 10:00 AM Perth

## Why?

We burned 80k+ included GitHub Actions minutes in 9 days. The main culprits:

- macOS runners at **10x multiplier** on every push
- Windows runners at **2x multiplier** on every push
- No concurrency groups (5 agent pushes = 5 full CI runs)
- Node 22 matrix doubling Linux runs

This strategy saves ~60-70% of CI minutes while maintaining full coverage before
releases.
