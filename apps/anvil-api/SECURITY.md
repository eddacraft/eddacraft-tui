# Security: TOKEN_PEPPER Rotation

## Overview

`TOKEN_PEPPER` is a server-side secret mixed into token hashes before storage.
It ensures that even if the database is compromised, token hashes cannot be
reversed or matched against known inputs without the pepper value.

## How It Works

Tokens are hashed as `SHA-256(token + TOKEN_PEPPER)` before being stored in the
database. During validation, the same pepper is applied to the incoming token
and the result is compared against the stored hash.

## Rotation Steps

1. **Set the new pepper**: Add `TOKEN_PEPPER_NEW` to the environment alongside
   the existing `TOKEN_PEPPER`.
2. **Re-hash all tokens**: Run a migration that reads each token record,
   re-computes the hash using `TOKEN_PEPPER_NEW`, and updates the stored hash.
3. **Swap environment variables**: Replace `TOKEN_PEPPER` with the new value and
   remove `TOKEN_PEPPER_NEW`.

## Zero-Downtime Rotation

To avoid invalidating active tokens during migration:

1. Deploy code that validates against **both** `TOKEN_PEPPER` and
   `TOKEN_PEPPER_NEW` (try the new pepper first, fall back to the old one).
2. Run the re-hash migration in batches while the dual-validation code is live.
3. Once all hashes have been migrated, deploy code that only validates against
   the new pepper and remove the old pepper from the environment.

## Important Notes

- Never log or expose the pepper value.
- Store the pepper in a secrets manager, not in source control.
- Coordinate rotation with any read replicas or cached token lookups.
- Test the dual-pepper validation path before running in production.
