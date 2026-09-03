type OtpContentionObserver = {
  query: (
    text: string,
    values: readonly unknown[]
  ) => Promise<{ rows: Array<{ waiting_claimants: number | string }> }>;
};

type ContentionOptions = {
  minimumWaitingClaimants: number;
  maximumObservations: number;
};

/**
 * Observe PostgreSQL lock waits caused by the production OTP UPDATE. Every
 * iteration is a fresh database observation; there is no elapsed-time sleep
 * that could be mistaken for evidence of concurrency.
 */
export async function waitForOtpClaimContention(
  observer: OtpContentionObserver,
  claimsStartedAfter: string,
  { minimumWaitingClaimants, maximumObservations }: ContentionOptions
): Promise<{ observations: number; waitingClaimants: number }> {
  let greatestWaitingCount = 0;

  for (let observation = 1; observation <= maximumObservations; observation += 1) {
    const result = await observer.query(
      `
        SELECT count(*)::int AS waiting_claimants
        FROM pg_stat_activity
        WHERE pid <> pg_backend_pid()
          AND datname = current_database()
          AND usename = current_user
          AND state = 'active'
          AND wait_event_type = 'Lock'
          AND query_start >= $1::timestamptz
          AND query LIKE '%UPDATE otp_codes%'
          AND query LIKE '%SET attempts = attempts + 1%'
      `,
      [claimsStartedAfter]
    );
    const waitingClaimants = Number(result.rows[0]?.waiting_claimants ?? 0);
    if (!Number.isInteger(waitingClaimants) || waitingClaimants < 0) {
      throw new Error('PostgreSQL returned an invalid OTP lock-wait count');
    }

    greatestWaitingCount = Math.max(greatestWaitingCount, waitingClaimants);
    if (waitingClaimants >= minimumWaitingClaimants) {
      return { observations: observation, waitingClaimants };
    }
  }

  throw new Error(
    `PostgreSQL observed ${greatestWaitingCount} waiting OTP claimants; required ${minimumWaitingClaimants}`
  );
}
