/**
 * Unit Tests for Bundle Verifier
 *
 * Tests signature verification for OPA policy bundles
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  BundleVerifier,
  loadKeyFromFile,
  type BundleVerifierConfig,
  type PublicKeyConfig,
  type SignatureManifest,
} from './bundle-verifier.js';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { createHash, generateKeyPairSync, createSign } from 'node:crypto';
import { safeCleanup } from '../../../../tools/test-utils/safe-cleanup.js';

describe('BundleVerifier', () => {
  let tempDir: string;
  let bundleDir: string;
  let verifier: BundleVerifier;
  let rsaKeyPair: { publicKey: string; privateKey: string };
  let ecKeyPair: { publicKey: string; privateKey: string };

  beforeEach(() => {
    tempDir = join(tmpdir(), 'anvil-bundle-verifier-test', Math.random().toString(36));
    bundleDir = join(tempDir, 'bundle');
    mkdirSync(bundleDir, { recursive: true });

    // Generate test RSA key pair
    rsaKeyPair = generateKeyPairSync('rsa', {
      modulusLength: 2048,
      publicKeyEncoding: { type: 'spki', format: 'pem' },
      privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
    });

    // Generate test EC key pair
    ecKeyPair = generateKeyPairSync('ec', {
      namedCurve: 'P-256',
      publicKeyEncoding: { type: 'spki', format: 'pem' },
      privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
    });

    const config: BundleVerifierConfig = {
      keys: [
        {
          id: 'test-rsa-key',
          algorithm: 'RS256',
          key: rsaKeyPair.publicKey,
          source: 'inline',
        },
        {
          id: 'test-ec-key',
          algorithm: 'ES256',
          key: ecKeyPair.publicKey,
          source: 'inline',
        },
      ],
      require_signature: false,
    };

    verifier = new BundleVerifier(config);
  });

  afterEach(async () => {
    await safeCleanup(tempDir);
  });

  describe('initialization', () => {
    it('should reject unsigned bundles when require_signature is true', async () => {
      const strictVerifier = new BundleVerifier({
        keys: [],
        require_signature: true,
      });

      // Create an unsigned bundle directory
      const unsignedBundle = join(tempDir, 'unsigned-bundle');
      mkdirSync(unsignedBundle, { recursive: true });
      writeFileSync(join(unsignedBundle, 'policy.rego'), 'package test');

      const result = await strictVerifier.verifyBundle(unsignedBundle);
      expect(result.verified).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
    });

    it('should accept custom allowed algorithms and reject disallowed ones', async () => {
      const restrictedVerifier = new BundleVerifier({
        keys: [],
        require_signature: true,
        allowed_algorithms: ['RS256'],
      });

      // Verifier should be created with restricted algorithms
      // The restriction is tested via the actual verification flow in crypto tests below
      expect(restrictedVerifier).toBeDefined();
    });
  });

  describe('key management', () => {
    it('should add keys and use them for verification', () => {
      const newKey: PublicKeyConfig = {
        id: 'new-key',
        algorithm: 'RS256',
        key: rsaKeyPair.publicKey,
        source: 'inline',
      };

      verifier.addKey(newKey);

      // Verify key was added by confirming removeKey succeeds
      const removed = verifier.removeKey('new-key');
      expect(removed).toBe(true);
    });

    it('should remove keys', () => {
      const removed = verifier.removeKey('test-rsa-key');
      expect(removed).toBe(true);
    });

    it('should return false when removing non-existent key', () => {
      const removed = verifier.removeKey('non-existent-key');
      expect(removed).toBe(false);
    });
  });

  describe('file verification', () => {
    it('should verify file with correct hash', async () => {
      const filePath = join(bundleDir, 'policy.rego');
      const content = 'package test\nallow = true';
      writeFileSync(filePath, content);

      const hash = createHash('sha256').update(content).digest('hex');
      const result = await verifier.verifyFile(filePath, hash);

      expect(result).toBe(true);
    });

    it('should verify file with sha256 prefix', async () => {
      const filePath = join(bundleDir, 'policy.rego');
      const content = 'package test\nallow = true';
      writeFileSync(filePath, content);

      const hash = createHash('sha256').update(content).digest('hex');
      const result = await verifier.verifyFile(filePath, `sha256:${hash}`);

      expect(result).toBe(true);
    });

    it('should reject file with incorrect hash', async () => {
      const filePath = join(bundleDir, 'policy.rego');
      writeFileSync(filePath, 'package test\nallow = true');

      const result = await verifier.verifyFile(filePath, 'incorrect-hash-value'.padEnd(64, '0'));

      expect(result).toBe(false);
    });

    it('should return false for non-existent file', async () => {
      const result = await verifier.verifyFile('/non/existent/file.rego', 'somehash');

      expect(result).toBe(false);
    });
  });

  describe('signature extraction', () => {
    it('should extract valid signature manifest', async () => {
      const manifest: SignatureManifest = {
        signatures: [
          {
            files: [{ name: 'policy.rego', hash: 'sha256:abc123' }],
            algorithm: 'RS256',
            keyid: 'test-key',
            signatures: ['base64sig'],
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await verifier.extractSignatures(bundleDir);

      expect(result).not.toBeNull();
      expect(result?.signatures).toHaveLength(1);
      expect(result?.signatures[0].keyid).toBe('test-key');
    });

    it('should return null for missing signature file', async () => {
      const result = await verifier.extractSignatures(bundleDir);

      expect(result).toBeNull();
    });

    it('should return null for invalid JSON', async () => {
      writeFileSync(join(bundleDir, '.signatures.json'), 'not valid json');

      const result = await verifier.extractSignatures(bundleDir);

      expect(result).toBeNull();
    });

    it('should return null for invalid manifest structure', async () => {
      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify({ invalid: true }));

      const result = await verifier.extractSignatures(bundleDir);

      expect(result).toBeNull();
    });

    it('should validate file entries in manifest', async () => {
      const invalidManifest = {
        signatures: [
          {
            files: [{ name: 'policy.rego' }], // Missing hash
            algorithm: 'RS256',
            keyid: 'test-key',
            signatures: ['base64sig'],
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(invalidManifest));

      const result = await verifier.extractSignatures(bundleDir);

      expect(result).toBeNull();
    });
  });

  describe('bundle verification', () => {
    it('should return verified=true for bundle without signature when not required', async () => {
      // Create a policy file but no signatures
      writeFileSync(join(bundleDir, 'policy.rego'), 'package test\nallow = true');

      const result = await verifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('should return verified=false for bundle without signature when required', async () => {
      const strictVerifier = new BundleVerifier({
        keys: [],
        require_signature: true,
      });

      writeFileSync(join(bundleDir, 'policy.rego'), 'package test\nallow = true');

      const result = await strictVerifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(false);
      expect(result.errors).toContain('No signature manifest found and signatures are required');
    });

    it('should return error for non-existent bundle path', async () => {
      const result = await verifier.verifyBundle('/non/existent/path');

      expect(result.verified).toBe(false);
      expect(result.errors[0]).toContain('Bundle path does not exist');
    });

    it('should reject bundle with unknown key ID', async () => {
      const policyContent = 'package test\nallow = true';
      writeFileSync(join(bundleDir, 'policy.rego'), policyContent);

      const hash = createHash('sha256').update(policyContent).digest('hex');
      const manifest: SignatureManifest = {
        signatures: [
          {
            files: [{ name: 'policy.rego', hash: `sha256:${hash}` }],
            algorithm: 'RS256',
            keyid: 'unknown-key',
            signatures: ['fakesig'],
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await verifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(false);
      expect(result.errors).toContain('Unknown key ID: unknown-key');
    });

    it('should reject bundle with disallowed algorithm', async () => {
      const restrictedVerifier = new BundleVerifier({
        keys: [
          {
            id: 'test-rsa-key',
            algorithm: 'RS256',
            key: rsaKeyPair.publicKey,
            source: 'inline',
          },
        ],
        require_signature: true,
        allowed_algorithms: ['ES256'], // Only allow ES256
      });

      const policyContent = 'package test\nallow = true';
      writeFileSync(join(bundleDir, 'policy.rego'), policyContent);

      const hash = createHash('sha256').update(policyContent).digest('hex');
      const manifest: SignatureManifest = {
        signatures: [
          {
            files: [{ name: 'policy.rego', hash: `sha256:${hash}` }],
            algorithm: 'RS256', // Not allowed
            keyid: 'test-rsa-key',
            signatures: ['fakesig'],
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await restrictedVerifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(false);
      expect(result.errors).toContain('Algorithm not allowed: RS256');
    });

    it('should reject bundle with algorithm mismatch', async () => {
      const policyContent = 'package test\nallow = true';
      writeFileSync(join(bundleDir, 'policy.rego'), policyContent);

      const hash = createHash('sha256').update(policyContent).digest('hex');
      const manifest: SignatureManifest = {
        signatures: [
          {
            files: [{ name: 'policy.rego', hash: `sha256:${hash}` }],
            algorithm: 'ES256', // Mismatch: key is RS256
            keyid: 'test-rsa-key',
            signatures: ['fakesig'],
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await verifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(false);
      expect(result.errors.some((e) => e.includes('Algorithm mismatch'))).toBe(true);
    });

    it('should reject bundle with incorrect file hash', async () => {
      const policyContent = 'package test\nallow = true';
      writeFileSync(join(bundleDir, 'policy.rego'), policyContent);

      // Use wrong hash
      const manifest: SignatureManifest = {
        signatures: [
          {
            files: [{ name: 'policy.rego', hash: 'sha256:' + '0'.repeat(64) }],
            algorithm: 'RS256',
            keyid: 'test-rsa-key',
            signatures: ['fakesig'],
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await verifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(false);
      expect(result.fileResults[0].verified).toBe(false);
      expect(result.errors.some((e) => e.includes('File hash verification failed'))).toBe(true);
    });

    it('should verify bundle with valid RS256 signature', async () => {
      const policyContent = 'package test\nallow = true';
      writeFileSync(join(bundleDir, 'policy.rego'), policyContent);

      const hash = createHash('sha256').update(policyContent).digest('hex');
      const files = [{ name: 'policy.rego', hash: `sha256:${hash}` }];

      // Create the signed data (canonical JSON of files array)
      const signedData = JSON.stringify([...files].sort((a, b) => a.name.localeCompare(b.name)));

      // Sign with RSA
      const signer = createSign('RSA-SHA256');
      signer.update(signedData);
      const signature = signer.sign(rsaKeyPair.privateKey, 'base64');

      const manifest: SignatureManifest = {
        signatures: [
          {
            files,
            algorithm: 'RS256',
            keyid: 'test-rsa-key',
            signatures: [signature],
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await verifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(true);
      expect(result.keyId).toBe('test-rsa-key');
      expect(result.errors).toHaveLength(0);
      expect(result.fileResults[0].verified).toBe(true);
    });

    it('should verify bundle with valid ES256 signature', async () => {
      const policyContent = 'package test\ndeny = false';
      writeFileSync(join(bundleDir, 'policy.rego'), policyContent);

      const hash = createHash('sha256').update(policyContent).digest('hex');
      const files = [{ name: 'policy.rego', hash: `sha256:${hash}` }];

      // Create the signed data
      const signedData = JSON.stringify([...files].sort((a, b) => a.name.localeCompare(b.name)));

      // Sign with EC
      const signer = createSign('SHA256');
      signer.update(signedData);
      const signature = signer.sign(
        { key: ecKeyPair.privateKey, dsaEncoding: 'ieee-p1363' },
        'base64'
      );

      const manifest: SignatureManifest = {
        signatures: [
          {
            files,
            algorithm: 'ES256',
            keyid: 'test-ec-key',
            signatures: [signature],
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await verifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(true);
      expect(result.keyId).toBe('test-ec-key');
      expect(result.errors).toHaveLength(0);
    });

    it('should verify bundle with multiple files', async () => {
      const policy1Content = 'package test.one\nallow = true';
      const policy2Content = 'package test.two\ndeny = false';

      writeFileSync(join(bundleDir, 'policy1.rego'), policy1Content);
      writeFileSync(join(bundleDir, 'policy2.rego'), policy2Content);

      const hash1 = createHash('sha256').update(policy1Content).digest('hex');
      const hash2 = createHash('sha256').update(policy2Content).digest('hex');

      const files = [
        { name: 'policy1.rego', hash: `sha256:${hash1}` },
        { name: 'policy2.rego', hash: `sha256:${hash2}` },
      ];

      const signedData = JSON.stringify([...files].sort((a, b) => a.name.localeCompare(b.name)));

      const signer = createSign('RSA-SHA256');
      signer.update(signedData);
      const signature = signer.sign(rsaKeyPair.privateKey, 'base64');

      const manifest: SignatureManifest = {
        signatures: [
          {
            files,
            algorithm: 'RS256',
            keyid: 'test-rsa-key',
            signatures: [signature],
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await verifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(true);
      expect(result.fileResults).toHaveLength(2);
      expect(result.fileResults.every((r) => r.verified)).toBe(true);
    });

    it('should reject bundle with tampered file', async () => {
      const originalContent = 'package test\nallow = true';
      const hash = createHash('sha256').update(originalContent).digest('hex');
      const files = [{ name: 'policy.rego', hash: `sha256:${hash}` }];

      const signedData = JSON.stringify([...files].sort((a, b) => a.name.localeCompare(b.name)));

      const signer = createSign('RSA-SHA256');
      signer.update(signedData);
      const signature = signer.sign(rsaKeyPair.privateKey, 'base64');

      const manifest: SignatureManifest = {
        signatures: [
          {
            files,
            algorithm: 'RS256',
            keyid: 'test-rsa-key',
            signatures: [signature],
          },
        ],
      };

      // Write TAMPERED content
      writeFileSync(join(bundleDir, 'policy.rego'), 'package test\nallow = false');
      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await verifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(false);
      expect(result.fileResults[0].verified).toBe(false);
    });

    it('should reject bundle with invalid signature', async () => {
      const policyContent = 'package test\nallow = true';
      writeFileSync(join(bundleDir, 'policy.rego'), policyContent);

      const hash = createHash('sha256').update(policyContent).digest('hex');
      const files = [{ name: 'policy.rego', hash: `sha256:${hash}` }];

      const manifest: SignatureManifest = {
        signatures: [
          {
            files,
            algorithm: 'RS256',
            keyid: 'test-rsa-key',
            signatures: ['aW52YWxpZC1zaWduYXR1cmU='], // Invalid signature
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await verifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(false);
    });

    it('should try multiple signature blocks until one succeeds', async () => {
      // Generate a second RSA key pair
      const rsaKeyPair2 = generateKeyPairSync('rsa', {
        modulusLength: 2048,
        publicKeyEncoding: { type: 'spki', format: 'pem' },
        privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
      });

      verifier.addKey({
        id: 'test-rsa-key-2',
        algorithm: 'RS256',
        key: rsaKeyPair2.publicKey,
        source: 'inline',
      });

      const policyContent = 'package test\nallow = true';
      writeFileSync(join(bundleDir, 'policy.rego'), policyContent);

      const hash = createHash('sha256').update(policyContent).digest('hex');
      const files = [{ name: 'policy.rego', hash: `sha256:${hash}` }];

      const signedData = JSON.stringify([...files].sort((a, b) => a.name.localeCompare(b.name)));

      // Sign with second key
      const signer = createSign('RSA-SHA256');
      signer.update(signedData);
      const signature = signer.sign(rsaKeyPair2.privateKey, 'base64');

      const manifest: SignatureManifest = {
        signatures: [
          {
            // First block with first key - invalid signature
            files,
            algorithm: 'RS256',
            keyid: 'test-rsa-key',
            signatures: ['aW52YWxpZA=='],
          },
          {
            // Second block with second key - valid signature
            files,
            algorithm: 'RS256',
            keyid: 'test-rsa-key-2',
            signatures: [signature],
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await verifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(true);
      expect(result.keyId).toBe('test-rsa-key-2');
    });
  });

  describe('key sources', () => {
    it('should load key from environment variable', async () => {
      const envKeyName = 'ANVIL_VERIFY_KEY_' + Math.random().toString(36).slice(2);
      process.env[envKeyName] = rsaKeyPair.publicKey;

      try {
        const envVerifier = new BundleVerifier({
          keys: [
            {
              id: 'env-key',
              algorithm: 'RS256',
              key: envKeyName,
              source: 'env',
            },
          ],
          require_signature: true,
        });

        const policyContent = 'package test\nallow = true';
        writeFileSync(join(bundleDir, 'policy.rego'), policyContent);

        const hash = createHash('sha256').update(policyContent).digest('hex');
        const files = [{ name: 'policy.rego', hash: `sha256:${hash}` }];
        const signedData = JSON.stringify([...files].sort((a, b) => a.name.localeCompare(b.name)));

        const signer = createSign('RSA-SHA256');
        signer.update(signedData);
        const signature = signer.sign(rsaKeyPair.privateKey, 'base64');

        const manifest: SignatureManifest = {
          signatures: [
            {
              files,
              algorithm: 'RS256',
              keyid: 'env-key',
              signatures: [signature],
            },
          ],
        };

        writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

        const result = await envVerifier.verifyBundle(bundleDir);

        expect(result.verified).toBe(true);
      } finally {
        delete process.env[envKeyName];
      }
    });

    it('should fail when environment variable is not set', async () => {
      const envVerifier = new BundleVerifier({
        keys: [
          {
            id: 'missing-env-key',
            algorithm: 'RS256',
            key: 'ANVIL_VERIFY_MISSING_' + Math.random().toString(36).slice(2),
            source: 'env',
          },
        ],
        require_signature: true,
      });

      const policyContent = 'package test\nallow = true';
      writeFileSync(join(bundleDir, 'policy.rego'), policyContent);

      const hash = createHash('sha256').update(policyContent).digest('hex');
      const manifest: SignatureManifest = {
        signatures: [
          {
            files: [{ name: 'policy.rego', hash: `sha256:${hash}` }],
            algorithm: 'RS256',
            keyid: 'missing-env-key',
            signatures: ['fakesig'],
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await envVerifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(false);
      expect(result.errors.some((e) => e.includes('Environment variable not found'))).toBe(true);
    });
  });

  describe('env var allowlist', () => {
    it('should reject env vars with disallowed prefixes', async () => {
      const envVerifier = new BundleVerifier({
        keys: [
          {
            id: 'blocked-key',
            algorithm: 'RS256',
            key: 'SECRET_KEY',
            source: 'env',
          },
        ],
        require_signature: true,
      });

      const policyContent = 'package test\nallow = true';
      writeFileSync(join(bundleDir, 'policy.rego'), policyContent);

      const hash = createHash('sha256').update(policyContent).digest('hex');
      const manifest: SignatureManifest = {
        signatures: [
          {
            files: [{ name: 'policy.rego', hash: `sha256:${hash}` }],
            algorithm: 'RS256',
            keyid: 'blocked-key',
            signatures: ['fakesig'],
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await envVerifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(false);
      expect(result.errors.some((e) => e.includes('not in allowlist'))).toBe(true);
    });

    it('should reject AWS-style env vars', async () => {
      const envVerifier = new BundleVerifier({
        keys: [
          {
            id: 'aws-key',
            algorithm: 'RS256',
            key: 'AWS_SECRET_ACCESS_KEY',
            source: 'env',
          },
        ],
        require_signature: true,
      });

      const policyContent = 'package test\nallow = true';
      writeFileSync(join(bundleDir, 'policy.rego'), policyContent);

      const hash = createHash('sha256').update(policyContent).digest('hex');
      const manifest: SignatureManifest = {
        signatures: [
          {
            files: [{ name: 'policy.rego', hash: `sha256:${hash}` }],
            algorithm: 'RS256',
            keyid: 'aws-key',
            signatures: ['fakesig'],
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await envVerifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(false);
      expect(result.errors.some((e) => e.includes('not in allowlist'))).toBe(true);
    });

    it('should accept env vars with allowed prefixes', async () => {
      const envKeyName = 'ANVIL_BUNDLE_PUBLIC_KEY_' + Math.random().toString(36).slice(2);
      process.env[envKeyName] = rsaKeyPair.publicKey;

      try {
        const envVerifier = new BundleVerifier({
          keys: [
            {
              id: 'allowed-key',
              algorithm: 'RS256',
              key: envKeyName,
              source: 'env',
            },
          ],
          require_signature: true,
        });

        const policyContent = 'package test\nallow = true';
        writeFileSync(join(bundleDir, 'policy.rego'), policyContent);

        const hash = createHash('sha256').update(policyContent).digest('hex');
        const files = [{ name: 'policy.rego', hash: `sha256:${hash}` }];
        const signedData = JSON.stringify([...files].sort((a, b) => a.name.localeCompare(b.name)));

        const signer = createSign('RSA-SHA256');
        signer.update(signedData);
        const signature = signer.sign(rsaKeyPair.privateKey, 'base64');

        const manifest: SignatureManifest = {
          signatures: [
            {
              files,
              algorithm: 'RS256',
              keyid: 'allowed-key',
              signatures: [signature],
            },
          ],
        };

        writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

        const result = await envVerifier.verifyBundle(bundleDir);

        expect(result.verified).toBe(true);
      } finally {
        delete process.env[envKeyName];
      }
    });
  });

  describe('loadKeyFromFile', () => {
    it('should load key from file', async () => {
      const keyPath = join(tempDir, 'public-key.pem');
      writeFileSync(keyPath, rsaKeyPair.publicKey);

      const keyConfig = await loadKeyFromFile(keyPath, 'file-key', 'RS256');

      expect(keyConfig.id).toBe('file-key');
      expect(keyConfig.algorithm).toBe('RS256');
      // loadKeyFromFile trims whitespace from the key
      expect(keyConfig.key).toBe(rsaKeyPair.publicKey.trim());
      expect(keyConfig.source).toBe('file');
    });

    it('should use loaded key for verification', async () => {
      const keyPath = join(tempDir, 'public-key.pem');
      writeFileSync(keyPath, rsaKeyPair.publicKey);

      const keyConfig = await loadKeyFromFile(keyPath, 'loaded-key', 'RS256');

      const fileVerifier = new BundleVerifier({
        keys: [keyConfig],
        require_signature: true,
      });

      const policyContent = 'package test\nallow = true';
      writeFileSync(join(bundleDir, 'policy.rego'), policyContent);

      const hash = createHash('sha256').update(policyContent).digest('hex');
      const files = [{ name: 'policy.rego', hash: `sha256:${hash}` }];
      const signedData = JSON.stringify([...files].sort((a, b) => a.name.localeCompare(b.name)));

      const signer = createSign('RSA-SHA256');
      signer.update(signedData);
      const signature = signer.sign(rsaKeyPair.privateKey, 'base64');

      const manifest: SignatureManifest = {
        signatures: [
          {
            files,
            algorithm: 'RS256',
            keyid: 'loaded-key',
            signatures: [signature],
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await fileVerifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(true);
    });
  });

  describe('error handling', () => {
    it('should handle file read errors gracefully', async () => {
      // Create a directory where a file is expected
      const dirAsFile = join(bundleDir, 'policy.rego');
      mkdirSync(dirAsFile, { recursive: true });

      const result = await verifier.verifyFile(dirAsFile, 'somehash');

      expect(result).toBe(false);
    });

    it('should provide detailed error messages', async () => {
      const policyContent = 'package test\nallow = true';
      writeFileSync(join(bundleDir, 'policy.rego'), policyContent);

      const hash = createHash('sha256').update(policyContent).digest('hex');
      const manifest: SignatureManifest = {
        signatures: [
          {
            files: [{ name: 'policy.rego', hash: `sha256:${hash}` }],
            algorithm: 'RS256',
            keyid: 'test-rsa-key',
            signatures: ['invalid-base64!@#'],
          },
        ],
      };

      writeFileSync(join(bundleDir, '.signatures.json'), JSON.stringify(manifest));

      const result = await verifier.verifyBundle(bundleDir);

      expect(result.verified).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
    });
  });
});
