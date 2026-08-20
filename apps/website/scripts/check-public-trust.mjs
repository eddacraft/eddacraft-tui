import { existsSync, readFileSync } from 'node:fs';

const securityPage = readFileSync(new URL('../app/security/page.tsx', import.meta.url), 'utf8');
const publishedKey = new URL('../public/.well-known/pgp-key.txt', import.meta.url);

const failures = [];

if (!securityPage.includes('mailto:security@eddacraft.ai')) {
  failures.push('missing working security reporting email');
}

const advertisesPgp =
  securityPage.includes('For encrypted communications') ||
  securityPage.includes('Fingerprint:') ||
  securityPage.includes('pgp-key.txt');

if (advertisesPgp && /Fingerprint:\s*(?:XXXX\s*){10}/.test(securityPage)) {
  failures.push('placeholder PGP fingerprint is published');
}

if (advertisesPgp && !existsSync(publishedKey)) {
  failures.push('PGP guidance is published without the advertised key');
}

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exit(1);
}

console.log('website public trust contract: ok');
