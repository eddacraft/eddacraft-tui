# @eddacraft/transactional

React Email templates for eddacraft transactional emails. Used by the anvil API
to send authentication and waitlist communications.

## Status

Active

## Templates

| Template                | Description                                |
| ----------------------- | ------------------------------------------ |
| `beta-invite`           | Beta programme invitation email            |
| `otp-code`              | One-time password for authentication       |
| `waitlist-confirmation` | Waitlist sign-up confirmation              |
| `waitlist-migration`    | Notification when migrating waitlist users |

## Usage

```ts
import { BetaInviteEmail, OtpCodeEmail } from '@eddacraft/transactional';
```

## Consumers

- `apps/anvil-api` (sends emails via Resend/React Email)

## Development

```bash
# Preview templates in the browser
pnpm --filter @eddacraft/transactional dev

# Build
pnpm --filter @eddacraft/transactional build
```
