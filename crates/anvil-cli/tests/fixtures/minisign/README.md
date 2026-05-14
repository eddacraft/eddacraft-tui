# Minisign Test Fixtures

These files are **TEST ONLY**. They are intentionally committed so the
signature-verification test suite is deterministic without each contributor
having to regenerate keys.

| File                  | Purpose                                                                |
| --------------------- | ---------------------------------------------------------------------- |
| `anvil-test.pub`      | Minisign public key (with comment header)                              |
| `anvil-test.pub.b64`  | Just the base64 line; mirrors `DEV_PUBLIC_KEY` in `signature.rs`       |
| `anvil-test.key`      | Unencrypted secret key — committed by design                           |
| `regenerate.sh`       | Regenerate the keypair (requires `cargo install rsign2`)               |

## Why is the secret key committed?

The corresponding public key is the development fallback used **only when
`ANVIL_RELEASE_PUBLIC_KEY` is unset at compile time**. Production builds
in CI inject the real release public key at compile time; the development
key has no production trust value.

The release-signing workflow (`.github/workflows/release-sign-artefacts.yml`)
refuses to run when the embedded public key is still the dev fallback (the
`is_using_dev_public_key()` guard runs as a release-readiness preflight).

See ADR-045 for the full rationale.

## Regenerating

```sh
cargo install rsign2
crates/anvil-cli/tests/fixtures/minisign/regenerate.sh
```

After regenerating, update the `DEV_PUBLIC_KEY` constant in
`crates/anvil-cli/src/commands/update/signature.rs` to match the new
`anvil-test.pub.b64` content, then re-run:

```sh
cargo test -p eddacraft-anvil --lib commands::update::signature
cargo test -p eddacraft-anvil --test update_resolution_chain
```
