/**
 * Bundle Verifier - Signature verification for OPA policy bundles
 *
 * Implements signature verification for OPA policy bundles following
 * the OPA signature format specification.
 */

import { readFile, readdir, stat } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { join, resolve, sep, isAbsolute, normalize } from 'node:path';
import { createHash, createVerify, timingSafeEqual } from 'node:crypto';
import { createDebugger } from './utils/debug.js';

const debug = createDebugger('policy');

/**
 * Supported signature algorithms
 */
export type SignatureAlgorithm = 'RS256' | 'ES256' | 'Ed25519';

/**
 * Result of bundle verification
 */
export interface VerificationResult {
  /** Whether the bundle signature was successfully verified */
  verified: boolean;
  /** ID of the key that successfully verified the signature */
  keyId?: string;
  /** Errors encountered during verification */
  errors: string[];
  /** Per-file verification results */
  fileResults: Array<{
    file: string;
    verified: boolean;
    error?: string;
  }>;
}

/**
 * Public key configuration for signature verification
 */
export interface PublicKeyConfig {
  /** Unique identifier for this key */
  id: string;
  /** Signature algorithm */
  algorithm: SignatureAlgorithm;
  /** Public key in PEM format or base64 encoded */
  key: string;
  /** Source of the key */
  source: 'file' | 'inline' | 'env';
}

/**
 * Configuration for bundle verifier
 */
export interface BundleVerifierConfig {
  /** Public keys available for verification */
  keys: PublicKeyConfig[];
  /** Whether to require signatures (fail if no signature found) */
  require_signature: boolean;
  /** Allowed signature algorithms (default: all supported) */
  allowed_algorithms?: SignatureAlgorithm[];
}

/**
 * File entry in signature manifest
 */
interface SignatureFileEntry {
  /** File name/path relative to bundle root */
  name: string;
  /** Hash in format 'sha256:hexstring' */
  hash: string;
}

/**
 * A single signature block in the manifest
 */
interface SignatureBlock {
  /** Files covered by this signature */
  files: SignatureFileEntry[];
  /** Algorithm used for signing */
  algorithm: string;
  /** ID of the key used for signing */
  keyid: string;
  /** Base64-encoded signatures */
  signatures: string[];
}

/**
 * OPA signature manifest format (.signatures.json)
 */
export interface SignatureManifest {
  /** Array of signature blocks */
  signatures: SignatureBlock[];
}

/**
 * Default allowed algorithms
 */
const DEFAULT_ALLOWED_ALGORITHMS: SignatureAlgorithm[] = ['RS256', 'ES256', 'Ed25519'];

/**
 * Allowed environment variable name prefixes for key resolution.
 * Restricts env var access to prevent exfiltration of sensitive variables.
 */
const ALLOWED_ENV_VAR_PREFIXES = [
  'ANVIL_BUNDLE_',
  'ANVIL_POLICY_',
  'ANVIL_VERIFY_',
  'OPA_BUNDLE_',
  'OPA_VERIFY_',
];

/**
 * Signature file name in OPA bundles
 */
const SIGNATURES_FILE = '.signatures.json';

/**
 * Verifies signatures for OPA policy bundles
 */
export class BundleVerifier {
  private readonly keys: Map<string, PublicKeyConfig>;
  private readonly requireSignature: boolean;
  private readonly allowedAlgorithms: Set<SignatureAlgorithm>;

  constructor(config: BundleVerifierConfig) {
    this.keys = new Map();
    this.requireSignature = config.require_signature;
    this.allowedAlgorithms = new Set(config.allowed_algorithms ?? DEFAULT_ALLOWED_ALGORITHMS);

    // Add initial keys
    for (const key of config.keys) {
      this.keys.set(key.id, key);
    }
  }

  /**
   * Verify signatures for an OPA policy bundle
   * @param bundlePath - Path to the bundle directory or .tar.gz file
   * @returns Verification result with details
   */
  async verifyBundle(bundlePath: string): Promise<VerificationResult> {
    debug('verifying bundle', bundlePath);
    const result: VerificationResult = {
      verified: false,
      errors: [],
      fileResults: [],
    };

    // Check if bundle path exists
    if (!existsSync(bundlePath)) {
      debug('bundle path does not exist', bundlePath);
      result.errors.push(`Bundle path does not exist: ${bundlePath}`);
      return result;
    }

    // Extract signatures manifest
    const manifest = await this.extractSignatures(bundlePath);

    if (!manifest) {
      if (this.requireSignature) {
        debug('no signature manifest found but signatures required');
        result.errors.push('No signature manifest found and signatures are required');
        return result;
      }
      // No signatures found but not required - consider verified
      debug('no signature manifest, signatures not required - verified');
      result.verified = true;
      return result;
    }

    // Verify each signature block
    for (const sigBlock of manifest.signatures) {
      const blockResult = await this.verifySignatureBlock(bundlePath, sigBlock);

      if (blockResult.verified) {
        debug('bundle verified with key', sigBlock.keyid);
        result.verified = true;
        result.keyId = sigBlock.keyid;
        result.fileResults = blockResult.fileResults;
        return result;
      }

      // Accumulate errors from failed verification attempts
      result.errors.push(...blockResult.errors);
      result.fileResults = blockResult.fileResults;
    }

    if (!result.verified && manifest.signatures.length > 0) {
      result.errors.push('No signature block could be verified');
    }

    return result;
  }

  /**
   * Verify a single file against its expected hash
   * @param filePath - Absolute path to the file
   * @param expectedHash - Expected hash in format 'sha256:hexstring' or just hexstring
   * @returns True if hash matches
   */
  async verifyFile(filePath: string, expectedHash: string): Promise<boolean> {
    if (!existsSync(filePath)) {
      return false;
    }

    try {
      const content = await readFile(filePath);
      const actualHash = this.computeFileHash(content);

      // Parse expected hash (may have 'sha256:' prefix)
      const normalizedExpected = expectedHash.startsWith('sha256:')
        ? expectedHash.slice(7)
        : expectedHash;

      return this.timingSafeCompare(actualHash, normalizedExpected);
    } catch {
      return false;
    }
  }

  /**
   * Extract signature manifest from a bundle
   * @param bundlePath - Path to the bundle directory
   * @returns Parsed signature manifest or null if not found
   */
  async extractSignatures(bundlePath: string): Promise<SignatureManifest | null> {
    const signaturePath = join(bundlePath, SIGNATURES_FILE);

    if (!existsSync(signaturePath)) {
      return null;
    }

    try {
      const content = await readFile(signaturePath, 'utf-8');
      const manifest = JSON.parse(content) as SignatureManifest;

      // Validate manifest structure
      if (!this.isValidManifest(manifest)) {
        return null;
      }

      return manifest;
    } catch {
      return null;
    }
  }

  /**
   * Add a public key for verification
   * @param key - Public key configuration
   */
  addKey(key: PublicKeyConfig): void {
    this.keys.set(key.id, key);
  }

  /**
   * Remove a public key
   * @param keyId - ID of the key to remove
   * @returns True if key was removed
   */
  removeKey(keyId: string): boolean {
    return this.keys.delete(keyId);
  }

  /**
   * Verify a signature block against bundle files
   */
  private async verifySignatureBlock(
    bundlePath: string,
    sigBlock: SignatureBlock
  ): Promise<{
    verified: boolean;
    errors: string[];
    fileResults: VerificationResult['fileResults'];
  }> {
    const errors: string[] = [];
    const fileResults: VerificationResult['fileResults'] = [];

    // Check if algorithm is allowed
    const algorithm = sigBlock.algorithm as SignatureAlgorithm;
    if (!this.allowedAlgorithms.has(algorithm)) {
      errors.push(`Algorithm not allowed: ${sigBlock.algorithm}`);
      return { verified: false, errors, fileResults };
    }

    // Get the key for this signature block
    const keyConfig = this.keys.get(sigBlock.keyid);
    if (!keyConfig) {
      errors.push(`Unknown key ID: ${sigBlock.keyid}`);
      return { verified: false, errors, fileResults };
    }

    // Verify algorithm matches key configuration
    if (keyConfig.algorithm !== algorithm) {
      errors.push(
        `Algorithm mismatch: signature uses ${algorithm} but key ${sigBlock.keyid} is configured for ${keyConfig.algorithm}`
      );
      return { verified: false, errors, fileResults };
    }

    // First verify all file hashes
    const resolvedBundle = resolve(bundlePath);
    for (const fileEntry of sigBlock.files) {
      // Validate manifest file paths don't escape bundle directory
      if (isAbsolute(fileEntry.name) || normalize(fileEntry.name).startsWith('..')) {
        errors.push(`Unsafe path in manifest: ${fileEntry.name}`);
        fileResults.push({
          file: fileEntry.name,
          verified: false,
          error: 'Path traversal rejected',
        });
        continue;
      }
      const filePath = join(bundlePath, fileEntry.name);
      if (
        !resolve(filePath).startsWith(resolvedBundle + sep) &&
        resolve(filePath) !== resolvedBundle
      ) {
        errors.push(`Path escapes bundle directory: ${fileEntry.name}`);
        fileResults.push({
          file: fileEntry.name,
          verified: false,
          error: 'Path traversal rejected',
        });
        continue;
      }
      const verified = await this.verifyFile(filePath, fileEntry.hash);

      fileResults.push({
        file: fileEntry.name,
        verified,
        error: verified ? undefined : `Hash mismatch for ${fileEntry.name}`,
      });

      if (!verified) {
        errors.push(`File hash verification failed: ${fileEntry.name}`);
      }
    }

    // If any file hash failed, don't proceed with signature verification
    const allFilesVerified = fileResults.every((r) => r.verified);
    if (!allFilesVerified) {
      return { verified: false, errors, fileResults };
    }

    // Verify the signature itself
    // The signed data is the canonical JSON of the files array
    const signedData = this.canonicalizeFilesArray(sigBlock.files);

    for (const signature of sigBlock.signatures) {
      try {
        const isValid = this.verifySignature(signedData, signature, keyConfig);
        if (isValid) {
          // A valid signature only attests to the files listed in the
          // manifest. Reject the bundle if it ships any *extra* file the
          // signature does not cover — otherwise an attacker can smuggle
          // unsigned `.rego`/data files into an otherwise-valid bundle and
          // have them loaded as trusted policy.
          const uncovered = await this.findUncoveredFiles(bundlePath, sigBlock.files);
          if (uncovered.length > 0) {
            for (const item of uncovered) {
              fileResults.push({ file: item.file, verified: false, error: item.reason });
            }
            errors.push(
              `Bundle contains files not covered by the signature manifest: ${uncovered
                .map((u) => u.file)
                .join(', ')}`
            );
            return { verified: false, errors, fileResults };
          }
          return { verified: true, errors: [], fileResults };
        }
      } catch (error) {
        errors.push(
          `Signature verification error: ${error instanceof Error ? error.message : 'Unknown error'}`
        );
      }
    }

    errors.push('No valid signature found in signature block');
    return { verified: false, errors, fileResults };
  }

  /**
   * Enumerate the bundle directory and return every entry that the verifying
   * signature block does NOT cover (the signature file itself is always
   * excluded). Each result carries a reason:
   *  - `unsigned`: a regular file with no matching manifest entry.
   *  - `symlink`:  a symbolic link. Symlinks are rejected unconditionally —
   *    even if their name appears in the manifest — because the per-file hash
   *    check dereferences the link (hashing the target, which may live
   *    outside the bundle) and a symlinked directory would let unsigned
   *    files escape enumeration entirely.
   *
   * Non-directory bundle paths (e.g. a `.tar.gz` handed directly) cannot be
   * walked here and return an empty list. This is currently unreachable —
   * `extractSignatures` only resolves a manifest for directory bundles — but
   * is NOT a substitute for signature-coverage on tarballs if that path is
   * ever wired up.
   */
  private async findUncoveredFiles(
    bundlePath: string,
    signedFiles: SignatureFileEntry[]
  ): Promise<Array<{ file: string; reason: string }>> {
    let stats;
    try {
      stats = await stat(bundlePath);
    } catch {
      return [];
    }
    if (!stats.isDirectory()) {
      return [];
    }

    const signed = new Set(signedFiles.map((f) => normalize(f.name)));
    const uncovered: Array<{ file: string; reason: string }> = [];

    const walk = async (dir: string, relPrefix: string): Promise<void> => {
      const entries = await readdir(dir, { withFileTypes: true });
      for (const entry of entries) {
        const rel = relPrefix ? `${relPrefix}/${entry.name}` : entry.name;
        // Reject symlinks before any directory/file classification: a symlink
        // to a directory is NOT an `isDirectory()` entry, so its contents
        // would otherwise be skipped and never hash-checked.
        if (entry.isSymbolicLink()) {
          uncovered.push({ file: rel, reason: 'Symbolic links are not allowed in signed bundles' });
          continue;
        }
        if (entry.isDirectory()) {
          await walk(join(dir, entry.name), rel);
          continue;
        }
        if (rel === SIGNATURES_FILE) {
          continue;
        }
        if (!signed.has(normalize(rel))) {
          uncovered.push({ file: rel, reason: 'Unsigned file not covered by signature manifest' });
        }
      }
    };

    await walk(bundlePath, '');
    return uncovered;
  }

  /**
   * Verify a cryptographic signature
   */
  private verifySignature(
    data: string,
    signatureBase64: string,
    keyConfig: PublicKeyConfig
  ): boolean {
    const signature = Buffer.from(signatureBase64, 'base64');
    const publicKey = this.resolvePublicKey(keyConfig);

    switch (keyConfig.algorithm) {
      case 'RS256': {
        const verifier = createVerify('RSA-SHA256');
        verifier.update(data);
        return verifier.verify(publicKey, signature);
      }

      case 'ES256': {
        const verifier = createVerify('SHA256');
        verifier.update(data);
        return verifier.verify(
          {
            key: publicKey,
            dsaEncoding: 'ieee-p1363',
          },
          signature
        );
      }

      case 'Ed25519': {
        const verifier = createVerify('ed25519');
        verifier.update(data);
        return verifier.verify(publicKey, signature);
      }

      default:
        throw new Error(`Unsupported algorithm: ${keyConfig.algorithm}`);
    }
  }

  /**
   * Resolve a public key from its configuration
   */
  private resolvePublicKey(keyConfig: PublicKeyConfig): string {
    switch (keyConfig.source) {
      case 'inline':
        // Key is directly provided, may be PEM or base64
        if (keyConfig.key.includes('-----BEGIN')) {
          return keyConfig.key;
        }
        // Assume base64-encoded DER, convert to PEM
        return this.base64ToPem(keyConfig.key, keyConfig.algorithm);

      case 'env': {
        // Validate env var name is a safe identifier (alphanumeric + underscore)
        if (!/^[A-Z_][A-Z0-9_]*$/i.test(keyConfig.key)) {
          throw new Error(`Invalid environment variable name: ${keyConfig.key}`);
        }
        // Restrict to allowlisted prefixes to prevent exfiltration of sensitive env vars
        const isAllowed = ALLOWED_ENV_VAR_PREFIXES.some((prefix) =>
          keyConfig.key.startsWith(prefix)
        );
        if (!isAllowed) {
          throw new Error(
            `Environment variable '${keyConfig.key}' not in allowlist. ` +
              `Allowed prefixes: ${ALLOWED_ENV_VAR_PREFIXES.join(', ')}`
          );
        }
        const envValue = process.env[keyConfig.key];
        if (!envValue) {
          throw new Error(`Environment variable not found: ${keyConfig.key}`);
        }
        if (envValue.includes('-----BEGIN')) {
          return envValue;
        }
        return this.base64ToPem(envValue, keyConfig.algorithm);
      }

      case 'file':
        // Key is in keyConfig.key but needs to be read from file
        // For file sources, the key content should already be resolved
        // by the caller before creating the config
        if (keyConfig.key.includes('-----BEGIN')) {
          return keyConfig.key;
        }
        return this.base64ToPem(keyConfig.key, keyConfig.algorithm);

      default:
        throw new Error(`Unknown key source: ${keyConfig.source}`);
    }
  }

  /**
   * Convert base64-encoded key to PEM format
   */
  private base64ToPem(base64Key: string, algorithm: SignatureAlgorithm): string {
    const keyType = algorithm === 'RS256' ? 'RSA PUBLIC KEY' : 'PUBLIC KEY';
    const formatted = base64Key.match(/.{1,64}/g)?.join('\n') ?? base64Key;
    return `-----BEGIN ${keyType}-----\n${formatted}\n-----END ${keyType}-----`;
  }

  /**
   * Compute SHA-256 hash of file content
   */
  private computeFileHash(content: Buffer): string {
    return createHash('sha256').update(content).digest('hex');
  }

  /**
   * Timing-safe comparison of two hash strings
   */
  private timingSafeCompare(actual: string, expected: string): boolean {
    if (actual.length !== expected.length) {
      return false;
    }

    return timingSafeEqual(Buffer.from(actual, 'utf8'), Buffer.from(expected, 'utf8'));
  }

  /**
   * Canonicalize the files array for signature verification
   * Uses deterministic JSON serialization
   */
  private canonicalizeFilesArray(files: SignatureFileEntry[]): string {
    // Sort files by name for deterministic ordering
    const sorted = [...files].sort((a, b) => a.name.localeCompare(b.name));

    // Create canonical JSON representation
    return JSON.stringify(sorted);
  }

  /**
   * Validate that a manifest has the expected structure
   */
  private isValidManifest(manifest: unknown): manifest is SignatureManifest {
    if (typeof manifest !== 'object' || manifest === null) {
      return false;
    }

    const m = manifest as Record<string, unknown>;
    if (!Array.isArray(m.signatures)) {
      return false;
    }

    for (const sig of m.signatures) {
      if (typeof sig !== 'object' || sig === null) {
        return false;
      }

      const s = sig as Record<string, unknown>;
      if (!Array.isArray(s.files)) {
        return false;
      }
      if (typeof s.algorithm !== 'string') {
        return false;
      }
      if (typeof s.keyid !== 'string') {
        return false;
      }
      if (!Array.isArray(s.signatures)) {
        return false;
      }

      // Validate file entries
      for (const file of s.files) {
        if (typeof file !== 'object' || file === null) {
          return false;
        }
        const f = file as Record<string, unknown>;
        if (typeof f.name !== 'string' || typeof f.hash !== 'string') {
          return false;
        }
      }
    }

    return true;
  }
}

/**
 * Load a public key from a file
 * @param filePath - Path to the PEM or DER key file
 * @param id - Key identifier
 * @param algorithm - Signature algorithm
 * @returns PublicKeyConfig ready for use
 */
export async function loadKeyFromFile(
  filePath: string,
  id: string,
  algorithm: SignatureAlgorithm
): Promise<PublicKeyConfig> {
  const content = await readFile(filePath, 'utf-8');
  return {
    id,
    algorithm,
    key: content.trim(),
    source: 'file',
  };
}
