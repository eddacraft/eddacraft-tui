/**
 * Regression tests for issue #1826 findings
 * `fnd_sig-feat-library-255c3bcb97-7873_3d003d325a` /
 * `fnd_sig-feat-library-c6a8d0fc79-c38f_d649ce67fd`:
 * "Bundle signature verification accepts unsigned files in the bundle".
 *
 * A bundle whose signature manifest verifies must still be rejected if the
 * bundle directory contains files that are NOT covered by the verifying
 * signature block — otherwise an attacker can smuggle unsigned policy/data
 * files into an otherwise-valid bundle and have them loaded as trusted.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { BundleVerifier, type SignatureManifest } from './bundle-verifier.js';
import { mkdirSync, mkdtempSync, writeFileSync, symlinkSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { createHash, generateKeyPairSync, createSign } from 'node:crypto';
import { safeCleanup } from '../../../../tools/test-utils/safe-cleanup.js';

describe('BundleVerifier — unsigned files in bundle (issue #1826)', () => {
  let tempDir: string;
  let bundleDir: string;
  let verifier: BundleVerifier;
  let rsaKeyPair: { publicKey: string; privateKey: string };

  const signManifest = (files: { name: string; hash: string }[]): SignatureManifest => {
    const signedData = JSON.stringify([...files].sort((a, b) => a.name.localeCompare(b.name)));
    const signer = createSign('RSA-SHA256');
    signer.update(signedData);
    const signature = signer.sign(rsaKeyPair.privateKey, 'base64');
    return {
      signatures: [{ files, algorithm: 'RS256', keyid: 'test-rsa-key', signatures: [signature] }],
    };
  };

  const writeSigned = (name: string, content: string): { name: string; hash: string } => {
    writeFileSync(join(bundleDir, name), content);
    return { name, hash: 'sha256:' + createHash('sha256').update(content).digest('hex') };
  };

  beforeEach(() => {
    tempDir = mkdtempSync(join(tmpdir(), 'anvil-bundle-unsigned-test-'));
    bundleDir = join(tempDir, 'bundle');
    mkdirSync(bundleDir, { recursive: true });
    rsaKeyPair = generateKeyPairSync('rsa', {
      modulusLength: 2048,
      publicKeyEncoding: { type: 'spki', format: 'pem' },
      privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
    });
    verifier = new BundleVerifier({
      keys: [
        { id: 'test-rsa-key', algorithm: 'RS256', key: rsaKeyPair.publicKey, source: 'inline' },
      ],
      require_signature: true,
    });
  });

  afterEach(async () => {
    await safeCleanup(tempDir);
  });

  it('rejects a bundle containing a file not covered by the signature manifest', async () => {
    const signed = writeSigned('policy.rego', 'package test\nallow = true');
    // Attacker drops an extra file that is NOT in the manifest.
    writeFileSync(join(bundleDir, 'evil.rego'), 'package evil\nallow = true');
    writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(signManifest([signed])));

    const result = await verifier.verifyBundle(bundleDir);

    expect(result.verified).toBe(false);
    expect(
      result.errors.some(
        (e) => e.toLowerCase().includes('not covered') || e.toLowerCase().includes('unsigned')
      )
    ).toBe(true);
    expect(result.errors.some((e) => e.includes('evil.rego'))).toBe(true);
  });

  it('rejects an unsigned file smuggled into a subdirectory', async () => {
    const signed = writeSigned('policy.rego', 'package test\nallow = true');
    mkdirSync(join(bundleDir, 'nested'), { recursive: true });
    writeFileSync(join(bundleDir, 'nested', 'evil.rego'), 'package evil');
    writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(signManifest([signed])));

    const result = await verifier.verifyBundle(bundleDir);

    expect(result.verified).toBe(false);
  });

  // A symlink listed in the manifest whose target lives OUTSIDE the bundle:
  // the per-file hash check passes (it hashes the link target), the signature
  // is valid, but the bundle must still be rejected because the bytes that
  // actually get loaded are not the signed bundle's bytes.
  it.skipIf(process.platform === 'win32')(
    'rejects a manifest-listed entry that is actually a symlink to an outside file',
    async () => {
      const outsideContent = 'package outside\nallow = true';
      const outside = join(tempDir, 'outside.rego');
      writeFileSync(outside, outsideContent);
      symlinkSync(outside, join(bundleDir, 'policy.rego'));

      const entry = {
        name: 'policy.rego',
        hash: 'sha256:' + createHash('sha256').update(outsideContent).digest('hex'),
      };
      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(signManifest([entry])));

      const result = await verifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(false);
      // The per-file reason is recorded in fileResults; the top-level errors
      // array carries the generic "not covered by the signature manifest".
      expect(result.fileResults.some((r) => /symbolic link/i.test(r.error ?? ''))).toBe(true);
    }
  );

  it.skipIf(process.platform === 'win32')(
    'rejects an unsigned symlink smuggled into the bundle',
    async () => {
      const signed = writeSigned('policy.rego', 'package test\nallow = true');
      const outside = join(tempDir, 'secret.txt');
      writeFileSync(outside, 'secret');
      symlinkSync(outside, join(bundleDir, 'link.txt'));
      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(signManifest([signed])));

      const result = await verifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(false);
    }
  );

  it('still verifies a bundle whose files are all covered by the manifest', async () => {
    const f1 = writeSigned('policy.rego', 'package test\nallow = true');
    const f2 = writeSigned('data.json', '{"x":1}');
    writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(signManifest([f1, f2])));

    const result = await verifier.verifyBundle(bundleDir);

    expect(result.verified).toBe(true);
    expect(result.errors).toHaveLength(0);
  });
});
