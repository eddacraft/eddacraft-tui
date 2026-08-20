import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const DISCLOSURE_START = '{/* RESPONSIBLE DISCLOSURE */}';
const DISCLOSURE_END = '{/* SEE ALSO */}';
const REPORTING_LINK = /href\s*=\s*["']mailto:security@eddacraft\.ai["']/i;
const STRONG_PGP_GUIDANCE = /\b(?:pgp|openpgp|gpg|fingerprint|security[- ]key|key file)\b/i;
const REPORTING_KEY_GUIDANCE = /\b(?:encrypt(?:ed|ion)?|public[- ]key|certificate)\b/i;
const REAL_FINGERPRINT = /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/i;
const FINGERPRINT_VALUE = /^[0-9a-f]{4}(?:[\s:-]*[0-9a-f]{4})*/i;
const FINGERPRINT_BOUNDARY = /^(?:$|\s|\p{P}+(?:$|\s))/u;
const DEGENERATE_FINGERPRINT = /^([0-9a-f])\1+$/i;

function hasInvalidFingerprint(securityPage) {
  const sourceText = securityPage
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, ' ')
    .replace(/<[^>]*>/g, ' ')
    .replace(/\s+/g, ' ');
  const labels = sourceText.matchAll(/\bfingerprint\b\s*:?\s*/gi);

  return [...labels].some((label) => {
    const tail = sourceText.slice(label.index + label[0].length);
    const candidate = tail.match(FINGERPRINT_VALUE);
    if (!candidate || !FINGERPRINT_BOUNDARY.test(tail.slice(candidate[0].length))) {
      return true;
    }

    const compact = candidate[0].replace(/[\s:-]/g, '');
    return !REAL_FINGERPRINT.test(compact) || DEGENERATE_FINGERPRINT.test(compact);
  });
}

function responsibleDisclosureSource(securityPage) {
  const start = securityPage.indexOf(DISCLOSURE_START);
  const end = securityPage.indexOf(DISCLOSURE_END, start + DISCLOSURE_START.length);
  if (start < 0 || end < 0) return null;
  return securityPage.slice(start, end);
}

export function findPublicTrustFailures(securityPage, publishedKeyExists) {
  const failures = [];
  const disclosure = responsibleDisclosureSource(securityPage);

  if (!disclosure || !REPORTING_LINK.test(disclosure)) {
    failures.push('missing working security reporting email');
  }
  if (!disclosure) {
    failures.push('missing responsible disclosure section');
    return failures;
  }

  const advertisesPgp =
    STRONG_PGP_GUIDANCE.test(securityPage) || REPORTING_KEY_GUIDANCE.test(disclosure);
  if (advertisesPgp && hasInvalidFingerprint(securityPage)) {
    failures.push('placeholder PGP fingerprint is published');
  }
  if (advertisesPgp && !publishedKeyExists) {
    failures.push('PGP guidance is published without the advertised key');
  }

  return failures;
}

function run() {
  const securityPage = readFileSync(new URL('../app/security/page.tsx', import.meta.url), 'utf8');
  const publishedKey = new URL('../public/.well-known/pgp-key.txt', import.meta.url);
  const failures = findPublicTrustFailures(securityPage, existsSync(publishedKey));

  if (failures.length > 0) {
    console.error(failures.join('\n'));
    process.exit(1);
  }

  console.log('website public trust contract: ok');
}

const invokedPath = process.argv[1];
if (invokedPath && import.meta.url === pathToFileURL(resolve(invokedPath)).href) {
  run();
}
