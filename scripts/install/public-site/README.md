# Install site assets (`install.eddacraft.ai`)

The public install site is GitHub Pages on **`eddacraft/anvil`**, source `main`
/ `docs/` (custom domain `install.eddacraft.ai`).

| Public path                 | Purpose                                                       |
| --------------------------- | ------------------------------------------------------------- |
| `/` (`docs/index.html`)     | Landing page (curated per release)                            |
| `/windows` (`docs/windows`) | PowerShell short URL → latest `eddacraft-anvil-installer.ps1` |

## Publish `/windows`

From a checkout of **anvil-001** with `gh` authenticated for `eddacraft/anvil`:

```bash
bash scripts/release/publish-public-contents.sh \
  --repo eddacraft/anvil \
  --path docs/windows \
  --file scripts/install/public-site/windows \
  --message "fix(install): refresh /windows forwarder to latest installer"
```

After Pages rebuilds (usually under a minute):

```bash
curl -fsSL https://install.eddacraft.ai/windows | head
# PowerShell: irm https://install.eddacraft.ai/windows | iex
```

Do **not** put HTML at `/windows` — that path must stay a PowerShell script so
`irm … | iex` works.
