// Mixed fixture exercising AP-003 (any), AP-001 (eslint-disable), and
// DD-001 (untracked TODO) so the spike's parity assertion covers both
// the standard regex path and an SPG-003 PCRE-rewrite rule.
const value: any = input;

/* eslint-disable */
function legacyShim(payload: any) {
  // TODO refactor later
  return payload;
}
