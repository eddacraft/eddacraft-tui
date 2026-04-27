// @ts-nocheck — fixture file scanned as plain text by the AI-001 rule; LSP not relevant.
// Fixture: representative TS service whose reviewer-style comments
// stand in for prose an AI tool might mirror back. The unsuppressed
// comments on lines 10, 21, and 27 each trip a different AI-001
// heuristic; the suppressed comment on line 16 must NOT be flagged.

import { Logger } from './logger';

export class PaymentsService {
  // the architect said we don't need to hash the idempotency key here
  hashKey(key: string): string {
    return key;
  }

  // @anvil-ignore AI-001 -- contractual carve-out documented in PAY-91
  // the lead said leave the legacy branch in for the partner integration
  legacyBranch(): boolean {
    return false;
  }

  /* as discussed with the principal, the retry budget stays uncapped */
  retry(_input: unknown): void {
    // no-op
  }

  ship(): void {
    // just ship it, we'll add tests after launch
    Logger.info('shipping');
  }
}
