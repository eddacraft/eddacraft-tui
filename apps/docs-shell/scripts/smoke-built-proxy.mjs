import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

import { SignJWT } from 'jose';

const privateDocsUrl = 'https://private.example';
const keyPair = await crypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, true, [
  'sign',
  'verify',
]);
const publicKey = Buffer.from(await crypto.subtle.exportKey('spki', keyPair.publicKey))
  .toString('base64')
  .match(/.{1,64}/g)
  .join('\n');

process.env.LICENSE_PUBLIC_KEY =
  '-----BEGIN PUBLIC KEY-----\n' + publicKey + '\n-----END PUBLIC KEY-----';
process.env.ANVIL_DOCS_URL = privateDocsUrl;
process.env.PUBLIC_DOCS_URL = 'https://public.example';
process.env.DOCS_UPSTREAM_SECRET = 'test-secret';

const upstreamRequests = [];
globalThis.fetch = async (input) => {
  upstreamRequests.push(String(input));
  return new Response('private docs', { status: 200 });
};

const require = createRequire(import.meta.url);
const { handler } = require('../.next/server/middleware.js');
const context = { waitUntil() {}, requestMeta: {} };

async function signLicence(claims) {
  return new SignJWT(claims)
    .setProtectedHeader({ alg: 'ES256' })
    .setSubject('bundle-smoke')
    .setIssuer('https://api.eddacraft.ai')
    .setAudience('anvil-cli')
    .setIssuedAt()
    .setExpirationTime('5m')
    .sign(keyPair.privateKey);
}

async function requestPrivateDocs(claims) {
  upstreamRequests.length = 0;
  const token = await signLicence(claims);
  const response = await handler(
    new Request('https://docs.eddacraft.ai/anvil/overview', {
      headers: { cookie: 'anvil-docs-session=' + token },
    }),
    context
  );
  return { response, upstreamRequests: [...upstreamRequests] };
}

for (const plan of ['beta', 'pro', 'enterprise']) {
  const { response, upstreamRequests: requests } = await requestPrivateDocs({
    plan,
    tier: plan,
  });
  assert.equal(response.status, 200, plan + ' licence must reach private docs');
  assert.deepEqual(requests, [privateDocsUrl + '/anvil/overview']);
}

const legacy = await requestPrivateDocs({ tier: 'pro' });
assert.equal(legacy.response.status, 200, 'legacy tier:pro licence must de-escalate to beta');
assert.deepEqual(legacy.upstreamRequests, [privateDocsUrl + '/anvil/overview']);

const denied = await requestPrivateDocs({ plan: 'free', tier: 'pro' });
assert.equal(denied.response.status, 302, 'free plan must be denied');
assert.equal(denied.upstreamRequests.length, 0, 'denied licence must not reach private docs');
assert.match(denied.response.headers.get('location') ?? '', /^\/auth\/login\?/);
assert.match(denied.response.headers.get('set-cookie') ?? '', /Max-Age=0/i);

console.log('built proxy entitlement smoke: passed');
