# ADR-045: Update Signing Scheme — Minisign

## Status

Proposed

## Date

2026-05-14

## Context

DISTRIB-001 requires that `anvil update` verify downloaded artefacts against a
published signature before replacing the running binary. Without this, a
compromised release CDN or hijacked GitHub Release asset could ship malicious
code into the developer workflow at the point of greatest trust: the user has
already chosen to upgrade.

Three forces shape the decision:

1. **Air-gapped guarantee.** Anvil targets developers in restricted-network
   environments. Verification must work without network access once the
   artefact and signature are local on disk. ADR-044 §9 makes DISTRIB-001 and
   DISTRIB-002 load-bearing for the `v0.7.0-beta` MCP-backend swap; the
   trust premise is the whole point.
2. **Operator simplicity.** Anvil is maintained by a small team. Key
   custody must be tractable. A long-lived asymmetric key pair with a clear
   rotation runbook is preferable to anything that requires hosted infra,
   OIDC provider plumbing, or a transparency-log SLA.
3. **Verifier blast radius.** The verifier ships inside `anvil`, runs on
   every update, and must not pull in heavy dependencies. A pure-Rust
   verifier with no TLS / no network requirement is preferred.

We must pick a scheme now because DISTRIB-001 verification code embeds the
public key, the CI release workflow must publish signatures, and the choice
is binding for the lifetime of the v0 series (the chosen scheme's public key
becomes part of every shipped binary).

## Decision

Use **minisign** (Ed25519 over BLAKE2b) for signing every release artefact
that `anvil update` may consume. Verification in Anvil uses the pure-Rust
`minisign-verify` crate.

### Concrete commitments

- **Algorithm:** Ed25519 / BLAKE2b, as implemented by the upstream `minisign`
  reference tool and the `minisign-verify` Rust crate.
- **Trusted public key:** embedded as a compile-time constant in
  `crates/anvil-cli/src/commands/update/signature.rs`. Stored in source so a
  CDN compromise cannot rewrite it.
- **Private key custody:** the long-lived signing private key
  (`anvil-release.key`) is generated offline by the release maintainer and
  stored encrypted in 1Password under the `eddacraft/anvil-release` vault.
  The unencrypted form is base64-encoded into the GitHub Actions secret
  `ANVIL_MINISIGN_PRIVATE_KEY` (org-scoped, restricted to release workflows
  on protected tags).
- **Signature format:** detached `.minisig` files alongside each release
  asset (e.g. `anvil-x86_64-unknown-linux-gnu.tar.gz.minisig`). The trusted
  comment in the signature carries `tag=<vX.Y.Z>;commit=<sha>;built=<date>`
  so a verified signature also binds the artefact to its release tag.
- **Coverage:** sign the installer scripts (`installer.sh`, `installer.ps1`)
  that the axoupdater library path consumes. The installers themselves
  SHA256-verify the binary tarballs they download — this gives a single
  signature check anchored at the script that already covers the binary
  chain of trust. Binaries are still signed individually for downstream
  consumers (Homebrew bottle verification, manual install).
- **Rotation:** the public key has no built-in expiry. The release runbook
  must rotate the key pair if compromise is suspected. A key-rotation event
  ships a minor release whose embedded public key is the new one; the prior
  release verifies its own update with the prior key, so users who update
  from N → N+1 cross the rotation boundary using N's key.

### Out of scope for this ADR

- Verification of artefacts shipped by Homebrew. Homebrew owns its own
  SHA256 verification via the formula; we delegate to it on the Homebrew
  install path. DISTRIB-003 (Homebrew automation) re-publishes the
  formula on every release with fresh SHAs.
- Verification of artefacts installed by Windows package managers (winget /
  scoop). These have their own integrity model; the Anvil minisign chain
  applies on the library-fallback path only.
- Transparency logging (Sigstore Rekor). Not required for the v0 trust
  premise; revisit if the threat model expands.

## Rationale

### Alternatives Considered

| Option                           | Pros                                                                                                                       | Cons                                                                                                                                                                                                                       |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Minisign (chosen)**            | Single static binary; pure-Rust verifier; air-gapped; tiny key footprint; used by Mozilla / OpenBSD / Nginx; zero PKI.     | Single long-lived key requires careful custody. No built-in transparency log. No revocation infra beyond key rotation.                                                                                                     |
| Cosign / Sigstore keyless        | Strong supply-chain story; keyless via OIDC; transparency log; widely adopted in container ecosystem.                       | Verification requires Fulcio + Rekor reachability by default — breaks the air-gapped guarantee. Verifier crate (`sigstore-rs`) pulls in heavy TLS / cert path code. Operator complexity high for a small team.            |
| Cosign with explicit static keys | Same crypto strength as keyless mode; static key fits the air-gapped model.                                                | Verifier crate still heavyweight. Buys little over minisign while costing dependency footprint.                                                                                                                            |
| GPG / OpenPGP                    | Long-standing tool ecosystem; familiar to many Linux distros.                                                              | Verifier libs are large; key model is fragile (subkeys, web of trust); historically poor user experience. Already used by Homebrew at the formula layer — duplicating it at the artefact layer adds no extra protection. |
| SHA256 checksums only            | Trivial to implement; cargo-dist installers already do this.                                                               | Not a signature. A CDN compromise can rewrite both the artefact and its checksum file. Does not bind release authority — answers "did this download correctly?" not "did the project author sign this?".               |
| SSH-signed (`ssh-keygen -Y`)    | Uses existing release-maintainer SSH key; format is well-specified.                                                        | Pure-Rust verifier ecosystem immature; tooling assumes shell scripts; less established for distribution than minisign.                                                                                                     |

The decisive factors:

- **Air-gapped.** Cosign keyless requires network for both Fulcio and Rekor
  unless we ship a vendored TUF root and pre-fetched bundles. This is
  feasible but expensive for what it buys.
- **Verifier weight.** `minisign-verify` is a single small Rust crate with
  no transitive TLS or async runtime. `sigstore-rs` is much larger.
- **Operator ergonomics.** Minisign's CLI is one tool, one command, one
  key file. The team can teach the rotation procedure on a single page.

The risk we accept by not using Sigstore: no transparency log means a key
compromise can produce signed-but-malicious releases that are hard to
distinguish from legitimate ones until users notice. Mitigation: the public
key is embedded in source and any release that ships under a new key
requires a source-visible diff to land first; users updating across that
boundary cross with the prior key.

## Consequences

- **Positive:** `anvil update` gains a real signature check that works
  offline once the artefact + `.minisig` pair is local. The verifier adds a
  single small dependency (`minisign-verify`). The release process gains a
  single new step — `minisign -S -s ANVIL_MINISIGN_PRIVATE_KEY -m <file>` —
  trivially scripted in CI. The Anvil binary carries its own root of trust
  in the embedded public key.
- **Negative:** Anvil now owns key custody. The release maintainer holds
  a long-lived private key and must follow the rotation runbook if
  compromise is suspected. There is no transparency log, so a stolen key
  cannot be detected by external observers.
- **Risks:**
  - Private key leak from GitHub Actions secret store.
  - Loss of the offline key copy (locks out future legitimate releases
    until rotation).
  - Verifier crate vulnerability (mitigated by minimal surface).
- **Mitigations:**
  - Org-scoped secret restricted to protected tag workflows; required
    reviewer on the release workflow file changes.
  - Two-of-two key backup: 1Password vault entry and an encrypted USB
    held by the release maintainer.
  - Pin `minisign-verify` to a known-audited version range; the verifier
    runs on every update so a regression is loud.

## References

- Related ADRs:
  - [ADR-044: MCP entry activation owned](044-mcp-entry-activation-owned.md)
    — §9 makes DISTRIB-001 / -002 load-bearing for the MCP-backend swap.
  - [ADR-025: Package manager distribution](025-package-manager-distribution.md)
    — defines Homebrew / winget / scoop as the package-manager surface
    that `anvil update` defers to.
- APS modules: DISTRIB-001, DISTRIB-003 (Homebrew formula automation).
- Operator runbook: [`docs/runbooks/release-signing.md`](../../docs/runbooks/release-signing.md).
- External:
  - Minisign reference: <https://jedisct1.github.io/minisign/>
  - `minisign-verify` crate: <https://crates.io/crates/minisign-verify>
  - Mozilla's signing policy (precedent for minisign in shipped software).
