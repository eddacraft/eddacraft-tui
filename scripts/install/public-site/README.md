# Install site assets (`install.eddacraft.ai`)

The public install site is GitHub Pages on **`eddacraft/anvil`**, source `main`
/ `docs/` (custom domain `install.eddacraft.ai`).

This directory is the source of truth for the curated landing page. Do not leave
branding or messaging only on the public repo.

| Local file    | Published on Pages          | Purpose                                                       |
| ------------- | --------------------------- | ------------------------------------------------------------- |
| `index.html`  | `/` (`docs/index.html`)     | Landing page (brand + install methods)                        |
| `favicon.svg` | `/favicon.svg`              | Tab icon (ember brandmark on void)                            |
| `og-card.svg` | not served                  | Source used to generate `/og.png` (`docs/og.png`)             |
| `windows`     | `/windows` (`docs/windows`) | PowerShell short URL → latest `eddacraft-anvil-installer.ps1` |

The live page must stay on the **anvil** brand used at `eddacraft.ai`: lowercase
product name, ember (`#CC5500`) brandmark, Nordic Terminal tokens, and the
generation-time policy line. Do not restore "Anvil CLI" or cargo-dist default
copy. Keep `id="version-tag"` on an `<a>` and `id="release-date"` on a `<span>`
so `.github/workflows/stamp-install-page.yml` in `eddacraft/anvil` can rewrite
the baked release.

## Publish the landing page

From a checkout of **anvil-001** with `gh` authenticated for `eddacraft/anvil`:

```bash
bash scripts/release/publish-public-contents.sh \
  --repo eddacraft/anvil \
  --path docs/index.html \
  --file scripts/install/public-site/index.html \
  --message "fix(install): align landing page with anvil brand"

bash scripts/release/publish-public-contents.sh \
  --repo eddacraft/anvil \
  --path docs/favicon.svg \
  --file scripts/install/public-site/favicon.svg \
  --message "fix(install): use ember favicon on void"

# Optional: regenerate the social card, then publish it.
# rsvg-convert -w 1200 -h 630 scripts/install/public-site/og-card.svg -o /tmp/anvil-og.png
# bash scripts/release/publish-public-contents.sh \
#   --repo eddacraft/anvil --path docs/og.png --file /tmp/anvil-og.png \
#   --message "fix(install): refresh OG card"
```

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
