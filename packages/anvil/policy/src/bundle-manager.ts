/**
 * OPA Bundle Manager - Download, cache, and manage OPA policy bundles
 *
 * Handles downloading, caching, and validating OPA policy bundles from remote
 * servers. Bundles are tarball files containing .rego policy files and optional
 * data.json files.
 */

import {
  existsSync,
  mkdirSync,
  unlinkSync,
  readFileSync,
  createWriteStream,
  rmSync,
} from 'node:fs';
import { readFile, writeFile, rm, mkdir } from 'node:fs/promises';
import { join, resolve, sep, isAbsolute } from 'node:path';
import { homedir } from 'node:os';
import { createHash, createVerify } from 'node:crypto';
import https from 'node:https';
import http from 'node:http';
import { pipeline } from 'node:stream/promises';
import { createGunzip } from 'node:zlib';
import { extract } from 'tar';
import { createDebugger } from './utils/debug.js';

const debug = createDebugger('policy');

/**
 * Default cache directory for policy bundles
 */
const DEFAULT_CACHE_DIR = join(homedir(), '.anvil', 'policy-cache', 'bundles');

/**
 * Default refresh interval: 5 minutes
 */
const DEFAULT_REFRESH_INTERVAL_MS = 5 * 60 * 1000;

/**
 * Prefix for environment variables that may be referenced as bundle
 * credentials. Bundle config is workspace-controlled input, so credential
 * references are confined to an operator-owned namespace — otherwise a
 * malicious config could name any process secret (e.g. `GITHUB_TOKEN`) and
 * exfiltrate it in the Authorization header of a download it also controls.
 */
const BUNDLE_AUTH_ENV_PREFIX = 'ANVIL_BUNDLE_';

/**
 * Operator-owned escape hatch: a comma-separated list of additional
 * environment variable names that may be referenced as bundle credentials.
 * Only the process environment (operator-controlled) can set this, so a
 * workspace bundle config cannot widen its own credential access.
 *
 * Despite carrying the credential prefix, this variable can never itself be
 * referenced as a credential (self-allowlisting included): its value
 * enumerates the operator's other trusted credential names, which must not
 * be sendable to a config-controlled URL.
 */
const BUNDLE_AUTH_ENV_ALLOWLIST_VAR = 'ANVIL_BUNDLE_AUTH_ENV_ALLOWLIST';

/**
 * Suffix for the operator-owned host binding of a bundle credential. When
 * `<CREDENTIAL_ENV>_HOST` is set, the credential is only ever attached to
 * requests whose host matches the binding (hostname, `host:port`, or an
 * origin URL), so a config-controlled URL cannot redirect it elsewhere.
 * An origin-form binding (with a protocol) pins the whole origin — the
 * protocol and the port it implies (explicit, or the protocol default);
 * bare `host` / `host:port` forms match the hostname, plus the port only
 * when one is declared.
 *
 * The suffix is reserved: credential env names may never end in `_HOST`
 * (the allowlist cannot lift this), otherwise a credential named
 * `ANVIL_BUNDLE_FOO_HOST` would silently double as the host binding for
 * `ANVIL_BUNDLE_FOO`.
 */
const BUNDLE_AUTH_HOST_BINDING_SUFFIX = '_HOST';

/**
 * Authentication configuration for bundle downloads
 */
export interface BundleAuthConfig {
  /** Authentication type */
  type: 'basic' | 'bearer';
  /** Username for basic auth */
  username?: string;
  /**
   * Environment variable name containing the password for basic auth.
   * Must start with `ANVIL_BUNDLE_` or be listed in the operator-owned
   * `ANVIL_BUNDLE_AUTH_ENV_ALLOWLIST` environment variable.
   */
  password_env?: string;
  /**
   * Environment variable name containing the token for bearer auth.
   * Must start with `ANVIL_BUNDLE_` or be listed in the operator-owned
   * `ANVIL_BUNDLE_AUTH_ENV_ALLOWLIST` environment variable.
   */
  token_env?: string;
}

/**
 * Configuration for a single policy bundle
 */
export interface BundleConfig {
  /** Unique name for this bundle */
  name: string;
  /** URL to download the bundle from */
  url: string;
  /** How often to check for updates (ms) */
  refresh_interval_ms?: number;
  /** Public key for signature verification (PEM format or path to key file) */
  signature_key?: string;
  /** Expected SHA256 checksum of the bundle */
  checksum?: string;
  /** HTTP headers to include in download request */
  headers?: Record<string, string>;
  /** Authentication configuration */
  auth?: BundleAuthConfig;
}

/**
 * Cache index entry for a downloaded bundle
 */
export interface BundleCacheEntry {
  /** Bundle name */
  name: string;
  /** Original download URL */
  url: string;
  /** Local path to extracted bundle */
  path: string;
  /** When the bundle was downloaded */
  downloaded_at: number;
  /** When to check for updates */
  expires_at: number;
  /** SHA256 checksum of the downloaded tarball */
  checksum: string;
  /** Size in bytes of the downloaded tarball */
  size_bytes: number;
  /** Whether signature was verified */
  signature_verified: boolean;
  /** ETag from server for conditional requests */
  etag?: string;
  /** Last-Modified header from server */
  last_modified?: string;
}

/**
 * Cache index structure
 */
interface BundleCacheIndex {
  version: number;
  entries: Record<string, BundleCacheEntry>;
  last_sync: number;
}

/**
 * Bundle manager configuration
 */
export interface BundleManagerConfig {
  /** Directory to store cached bundles */
  cacheDir?: string;
  /** Bundle configurations */
  bundles?: BundleConfig[];
  /** Whether to verify signatures when available */
  verifySignatures?: boolean;
  /** Connection timeout in ms */
  timeoutMs?: number;
}

/**
 * Result of a bundle sync operation
 */
export interface BundleSyncResult {
  /** Bundle name */
  name: string;
  /** Whether sync was successful */
  success: boolean;
  /** Whether bundle was updated */
  updated: boolean;
  /** Error message if failed */
  error?: string;
  /** Path to the bundle if successful */
  path?: string;
}

/**
 * Manages OPA policy bundle download, caching, and updates
 */
export class BundleManager {
  private readonly cacheDir: string;
  private readonly bundles: Map<string, BundleConfig>;
  private readonly verifySignatures: boolean;
  private readonly timeoutMs: number;

  private index: BundleCacheIndex | null = null;
  private indexDirty = false;
  private readonly indexPath: string;

  constructor(config: BundleManagerConfig = {}) {
    this.cacheDir = config.cacheDir || DEFAULT_CACHE_DIR;
    this.indexPath = join(this.cacheDir, 'index.json');
    this.verifySignatures = config.verifySignatures ?? true;
    this.timeoutMs = config.timeoutMs ?? 30000;

    this.bundles = new Map();
    if (config.bundles) {
      for (const bundle of config.bundles) {
        this.assertSafeBundleName(bundle.name);
        this.assertSafeBundleAuth(bundle.auth);
        this.bundles.set(bundle.name, bundle);
      }
    }
  }

  /**
   * Reject bundle names that could escape the cache directory when joined
   * into a filesystem path. The name is used to build `bundleDir` and temp
   * file paths that are then passed to recursive `rmSync`/`mkdirSync`, so an
   * unsanitised name containing path separators or `..` could delete or
   * create directories outside the cache.
   */
  private assertSafeBundleName(name: string): void {
    if (
      !name ||
      name === '.' ||
      name === '..' ||
      name.includes('/') ||
      name.includes('\\') ||
      name.includes('\0') ||
      isAbsolute(name)
    ) {
      throw new Error(`Invalid bundle name: ${JSON.stringify(name)}`);
    }
    // Defence in depth: ensure the name cannot resolve outside the cache dir.
    const base = resolve(this.cacheDir);
    const resolved = resolve(base, name);
    if (resolved !== base && !resolved.startsWith(base + sep)) {
      throw new Error(`Invalid bundle name (path escape): ${JSON.stringify(name)}`);
    }
  }

  /**
   * Whether a stored cache-entry path is confined to the cache directory.
   * Used to reject paths from a tampered on-disk index before they are
   * handed back to callers as a trusted bundle directory.
   */
  private isWithinCacheDir(candidate: string): boolean {
    const base = resolve(this.cacheDir);
    const resolved = resolve(candidate);
    return resolved === base || resolved.startsWith(base + sep);
  }

  /**
   * Whether a cache-entry path is safe to hand back as a trusted bundle
   * directory: confined to the cache dir and matching the expected
   * `cacheDir/<name>` location. A tampered index can pair a safe name with
   * an out-of-cache path; downloadBundle must not return that path on the
   * unexpired-cache or 304 success branches.
   */
  private isTrustedBundleCachePath(candidate: string, expectedBundleDir: string): boolean {
    return this.isWithinCacheDir(candidate) && resolve(candidate) === resolve(expectedBundleDir);
  }

  /**
   * Add or update a bundle configuration
   */
  addBundle(config: BundleConfig): void {
    this.assertSafeBundleName(config.name);
    this.assertSafeBundleAuth(config.auth);
    this.bundles.set(config.name, config);
  }

  /**
   * Remove a bundle configuration
   */
  removeBundle(name: string): boolean {
    return this.bundles.delete(name);
  }

  /**
   * Get all configured bundle names
   */
  getBundleNames(): string[] {
    return Array.from(this.bundles.keys());
  }

  /**
   * Sync all configured bundles, downloading or updating as needed
   */
  async syncAll(): Promise<BundleSyncResult[]> {
    const results: BundleSyncResult[] = [];

    for (const name of this.bundles.keys()) {
      const result = await this.downloadBundle(name);
      results.push(result);
    }

    return results;
  }

  /**
   * Download or update a specific bundle
   */
  async downloadBundle(name: string): Promise<BundleSyncResult> {
    const config = this.bundles.get(name);
    if (!config) {
      return {
        name,
        success: false,
        updated: false,
        error: `Bundle configuration not found: ${name}`,
      };
    }

    try {
      this.assertSafeBundleName(name);
      await this.ensureCacheDir();
      const index = await this.loadIndex();

      let existingEntry: BundleCacheEntry | undefined = index.entries[name];
      const bundleDir = join(this.cacheDir, name);

      // Drop a tampered index entry whose path escapes the cache or does not
      // match the expected bundle directory so refresh paths redownload
      // instead of returning an untrusted path.
      if (existingEntry && !this.isTrustedBundleCachePath(existingEntry.path, bundleDir)) {
        debug(`Bundle ${name} cache entry path is untrusted, ignoring: ${existingEntry.path}`);
        delete index.entries[name];
        this.indexDirty = true;
        existingEntry = undefined;
        await this.saveIndex();
      }

      // Check if we have a valid cached bundle that hasn't expired
      if (existingEntry && existsSync(bundleDir)) {
        if (Date.now() < existingEntry.expires_at) {
          debug(`Bundle ${name} is still valid, skipping download`);
          return {
            name,
            success: true,
            updated: false,
            // Prefer the canonical cache location over the stored path.
            path: bundleDir,
          };
        }
      }

      // Validate URL: enforce HTTPS (allow localhost for development/testing)
      const parsedBundleUrl = new URL(config.url);
      const isLocalhost =
        parsedBundleUrl.hostname === 'localhost' || parsedBundleUrl.hostname === '127.0.0.1';
      if (parsedBundleUrl.protocol !== 'https:' && !isLocalhost) {
        throw new Error(`Bundle URL must use HTTPS: ${config.url}`);
      }

      // Download the bundle
      debug(`Downloading bundle ${name} from ${config.url}`);
      const tempFile = join(this.cacheDir, `${name}.tar.gz.tmp`);

      try {
        const downloadResult = await this.downloadFile(
          config.url,
          tempFile,
          config.headers,
          existingEntry?.etag,
          existingEntry?.last_modified,
          config.auth
        );

        // Handle 304 Not Modified — only trust a path already validated above
        // (existingEntry is cleared when the stored path escapes the cache).
        if (downloadResult.notModified && existingEntry) {
          if (
            !this.isTrustedBundleCachePath(existingEntry.path, bundleDir) ||
            !existsSync(bundleDir)
          ) {
            throw new Error(`Bundle ${name} returned 304 but cache path is untrusted or missing`);
          }
          // Update expiration time
          existingEntry.expires_at =
            Date.now() + (config.refresh_interval_ms || DEFAULT_REFRESH_INTERVAL_MS);
          this.indexDirty = true;
          await this.saveIndex();

          return {
            name,
            success: true,
            updated: false,
            path: bundleDir,
          };
        }

        // Verify checksum if provided
        const actualChecksum = await this.computeChecksum(tempFile);
        if (config.checksum && actualChecksum !== config.checksum) {
          throw new Error(
            `Checksum mismatch for bundle ${name}: expected ${config.checksum}, got ${actualChecksum}`
          );
        }

        // Verify signature if key is provided
        let signatureVerified = false;
        if (config.signature_key && this.verifySignatures) {
          signatureVerified = await this.verifySignature(
            tempFile,
            `${tempFile}.sig`,
            config.signature_key
          );
          if (!signatureVerified) {
            throw new Error(`Signature verification failed for bundle ${name}`);
          }
        }

        // Remove old bundle directory
        if (existsSync(bundleDir)) {
          rmSync(bundleDir, { recursive: true, force: true });
        }

        // Extract bundle
        mkdirSync(bundleDir, { recursive: true });
        await this.extractBundle(tempFile, bundleDir);

        // Update cache index
        const stats = readFileSync(tempFile);
        const entry: BundleCacheEntry = {
          name,
          url: config.url,
          path: bundleDir,
          downloaded_at: Date.now(),
          expires_at: Date.now() + (config.refresh_interval_ms || DEFAULT_REFRESH_INTERVAL_MS),
          checksum: actualChecksum,
          size_bytes: stats.length,
          signature_verified: signatureVerified,
          etag: downloadResult.etag,
          last_modified: downloadResult.lastModified,
        };

        index.entries[name] = entry;
        index.last_sync = Date.now();
        this.indexDirty = true;
        await this.saveIndex();

        // Clean up temp file
        if (existsSync(tempFile)) {
          unlinkSync(tempFile);
        }

        debug(`Bundle ${name} downloaded and extracted to ${bundleDir}`);

        return {
          name,
          success: true,
          updated: true,
          path: bundleDir,
        };
      } finally {
        // Clean up temp file on error
        if (existsSync(tempFile)) {
          try {
            unlinkSync(tempFile);
          } catch {
            // Ignore cleanup errors
          }
        }
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      debug(`Failed to download bundle ${name}: ${message}`);
      return {
        name,
        success: false,
        updated: false,
        error: message,
      };
    }
  }

  /**
   * Get the cached path for a bundle, returning null if not cached
   */
  async getBundle(name: string): Promise<string | null> {
    const index = await this.loadIndex();
    const entry = index.entries[name];

    if (!entry) {
      return null;
    }

    // Defence in depth: a tampered on-disk index could store a path that
    // escapes the cache directory under a safe-looking bundle name. The
    // returned path is handed to the policy loader as a trusted bundle
    // directory, so never return one that is not confined to the cache dir.
    if (!this.isWithinCacheDir(entry.path)) {
      debug(`Bundle ${name} cache entry path escapes cache dir, refusing: ${entry.path}`);
      return null;
    }

    if (!existsSync(entry.path)) {
      // Cache entry exists but files are missing
      delete index.entries[name];
      this.indexDirty = true;
      await this.saveIndex();
      return null;
    }

    return entry.path;
  }

  /**
   * Get cache entry metadata for a bundle
   */
  async getBundleEntry(name: string): Promise<BundleCacheEntry | null> {
    const index = await this.loadIndex();
    return index.entries[name] || null;
  }

  /**
   * Invalidate a specific bundle cache, removing downloaded files
   */
  async invalidateBundle(name: string): Promise<boolean> {
    // Guard against a tampered on-disk index supplying an escaping name that
    // would target `rmSync` outside the cache directory.
    this.assertSafeBundleName(name);
    const index = await this.loadIndex();
    const entry = index.entries[name];

    if (!entry) {
      return false;
    }

    // Remove bundle directory
    const bundleDir = join(this.cacheDir, name);
    if (existsSync(bundleDir)) {
      try {
        rmSync(bundleDir, { recursive: true, force: true });
      } catch (error) {
        debug(`Failed to remove bundle directory ${bundleDir}`, error);
      }
    }

    // Remove from index
    delete index.entries[name];
    this.indexDirty = true;
    await this.saveIndex();

    return true;
  }

  /**
   * Clear all cached bundles
   */
  async clearCache(): Promise<void> {
    try {
      await rm(this.cacheDir, { recursive: true, force: true });
      this.index = null;
      this.indexDirty = false;
      debug('Bundle cache cleared');
    } catch (error) {
      debug('Failed to clear bundle cache', error);
    }
  }

  /**
   * Get cache statistics
   */
  async getCacheStats(): Promise<{
    bundleCount: number;
    totalSizeBytes: number;
    lastSync: number;
  }> {
    const index = await this.loadIndex();
    const entries = Object.values(index.entries);

    return {
      bundleCount: entries.length,
      totalSizeBytes: entries.reduce((sum, e) => sum + e.size_bytes, 0),
      lastSync: index.last_sync,
    };
  }

  /**
   * Ensure cache directory exists
   */
  private async ensureCacheDir(): Promise<void> {
    if (!existsSync(this.cacheDir)) {
      await mkdir(this.cacheDir, { recursive: true });
    }
  }

  /**
   * Load cache index from disk
   */
  private async loadIndex(): Promise<BundleCacheIndex> {
    if (this.index) {
      return this.index;
    }

    try {
      const content = await readFile(this.indexPath, 'utf-8');
      this.index = JSON.parse(content) as BundleCacheIndex;

      // Validate structure
      if (!this.index.version || !this.index.entries) {
        throw new Error('Invalid index structure');
      }
    } catch (error) {
      debug('Cache index missing or corrupted, creating new one', error);
      this.index = {
        version: 1,
        entries: {},
        last_sync: 0,
      };
      this.indexDirty = true;
    }

    return this.index;
  }

  /**
   * Save cache index to disk
   */
  private async saveIndex(): Promise<void> {
    if (!this.indexDirty || !this.index) {
      return;
    }

    await this.ensureCacheDir();
    await writeFile(this.indexPath, JSON.stringify(this.index, null, 2), 'utf-8');
    this.indexDirty = false;
  }

  /**
   * Reject auth configurations that reference environment variables outside
   * the trusted credential namespace. Bundle config is workspace-controlled,
   * so an unrestricted `password_env`/`token_env` lets a malicious config
   * select any process secret (e.g. `GITHUB_TOKEN`) and exfiltrate it to a
   * config-controlled URL. Only operator-owned names are accepted: the
   * `ANVIL_BUNDLE_` prefix, or an explicit entry in the operator-owned
   * `ANVIL_BUNDLE_AUTH_ENV_ALLOWLIST` environment variable.
   */
  private assertSafeBundleAuth(auth: BundleAuthConfig | undefined): void {
    if (!auth) {
      return;
    }
    if (auth.password_env) {
      this.assertAuthorisedAuthEnvName(auth.password_env);
    }
    if (auth.token_env) {
      this.assertAuthorisedAuthEnvName(auth.token_env);
    }
  }

  private assertAuthorisedAuthEnvName(envName: string): void {
    // A whitespace-padded name would validate as one name but read a
    // different `process.env` key when the header is built; refuse it
    // outright rather than silently trimming.
    if (envName !== envName.trim()) {
      throw new Error(
        `Bundle auth environment variable ${JSON.stringify(envName)} is not authorised for ` +
          `bundle credentials: names must not contain leading or trailing whitespace.`
      );
    }

    // The allowlist variable carries the credential prefix but can never be
    // used as a credential itself (self-allowlisting included): its value
    // enumerates the operator's other trusted credential names, which must
    // not be sendable to a config-controlled URL.
    if (envName === BUNDLE_AUTH_ENV_ALLOWLIST_VAR) {
      throw new Error(
        `Bundle auth environment variable "${BUNDLE_AUTH_ENV_ALLOWLIST_VAR}" is not authorised ` +
          `for bundle credentials: the allowlist variable itself can never be used as a credential.`
      );
    }

    // Names ending in the host-binding suffix are reserved, otherwise a
    // credential named `ANVIL_BUNDLE_FOO_HOST` would silently double as the
    // host binding for `ANVIL_BUNDLE_FOO`. The allowlist cannot lift this.
    if (envName.endsWith(BUNDLE_AUTH_HOST_BINDING_SUFFIX)) {
      throw new Error(
        `Bundle auth environment variable ${JSON.stringify(envName)} is not authorised for ` +
          `bundle credentials: names ending in "${BUNDLE_AUTH_HOST_BINDING_SUFFIX}" are ` +
          `reserved for operator-declared host bindings.`
      );
    }

    if (envName.startsWith(BUNDLE_AUTH_ENV_PREFIX)) {
      return;
    }
    const allowlist = (process.env[BUNDLE_AUTH_ENV_ALLOWLIST_VAR] || '')
      .split(',')
      .map((entry) => entry.trim())
      .filter((entry) => entry.length > 0);
    if (allowlist.includes(envName)) {
      return;
    }
    throw new Error(
      `Bundle auth environment variable ${JSON.stringify(envName)} is not authorised for ` +
        `bundle credentials: names must start with "${BUNDLE_AUTH_ENV_PREFIX}" or be listed ` +
        `in the operator-owned ${BUNDLE_AUTH_ENV_ALLOWLIST_VAR} environment variable.`
    );
  }

  /**
   * Enforce the operator-declared host binding for a credential, if one is
   * set. `<CREDENTIAL_ENV>_HOST` lives in the process environment, which a
   * workspace bundle config cannot modify — so even an authorised credential
   * cannot be sent to an attacker-chosen URL when the operator has bound it
   * to its intended bundle host. The error never includes the secret value.
   */
  private assertCredentialBoundToHost(envName: string, requestUrl: URL): void {
    const bindingVar = `${envName}${BUNDLE_AUTH_HOST_BINDING_SUFFIX}`;
    const binding = process.env[bindingVar];
    if (!binding) {
      return;
    }

    const isOriginForm = binding.includes('://');
    let bound: URL;
    try {
      // Accept a bare hostname, `host:port`, or a full origin URL.
      bound = new URL(isOriginForm ? binding : `https://${binding}`);
    } catch {
      // Fail closed: an unparseable binding must not degrade into "send the
      // credential anywhere".
      throw new Error(
        `Bundle auth credential ${envName} has an unparseable host binding in ${bindingVar}: ` +
          `expected a hostname, host:port, or origin URL. Refusing to send the credential.`
      );
    }

    const hostnameMatches = bound.hostname.toLowerCase() === requestUrl.hostname.toLowerCase();
    const requestPort = requestUrl.port || (requestUrl.protocol === 'https:' ? '443' : '80');
    let allowed = hostnameMatches;
    if (isOriginForm) {
      // An origin-form binding pins the whole origin: the protocol and the
      // port it implies (explicit, or the protocol default). Otherwise
      // "https://example.com" would let the credential go to
      // http://example.com or to any port on the same host.
      const boundPort = bound.port || (bound.protocol === 'https:' ? '443' : '80');
      allowed = allowed && bound.protocol === requestUrl.protocol && boundPort === requestPort;
    } else {
      // Bare `host` / `host:port` binding: hostname always, port only when
      // one is declared.
      allowed = allowed && (!bound.port || bound.port === requestPort);
    }
    if (!allowed) {
      const boundDescription = isOriginForm ? bound.origin : bound.host;
      throw new Error(
        `Bundle auth credential ${envName} is bound to "${boundDescription}" via ${bindingVar} ` +
          `and will not be sent to "${requestUrl.protocol}//${requestUrl.host}".`
      );
    }
  }

  /**
   * Build authorization header from auth config. The request URL is required
   * so credentials can be checked against their operator-declared host
   * binding before they are attached.
   */
  private buildAuthHeader(auth: BundleAuthConfig, requestUrl: URL): string | null {
    if (auth.type === 'basic') {
      const username = auth.username || '';
      const passwordEnv = auth.password_env || '';
      let password = '';
      if (passwordEnv) {
        // Defence in depth: addBundle/the constructor already validate, but
        // never read an unauthorised or host-unbound credential here either.
        this.assertAuthorisedAuthEnvName(passwordEnv);
        this.assertCredentialBoundToHost(passwordEnv, requestUrl);
        password = process.env[passwordEnv] || '';
      }
      const credentials = Buffer.from(`${username}:${password}`).toString('base64');
      return `Basic ${credentials}`;
    }

    if (auth.type === 'bearer') {
      const tokenEnv = auth.token_env || '';
      if (tokenEnv) {
        this.assertAuthorisedAuthEnvName(tokenEnv);
        this.assertCredentialBoundToHost(tokenEnv, requestUrl);
        const token = process.env[tokenEnv] || '';
        if (token) {
          return `Bearer ${token}`;
        }
      }
    }

    return null;
  }

  /**
   * Download a file from URL to destination
   */
  private downloadFile(
    url: string,
    dest: string,
    headers?: Record<string, string>,
    etag?: string,
    lastModified?: string,
    auth?: BundleAuthConfig,
    redirectsRemaining = 5
  ): Promise<{ notModified: boolean; etag?: string; lastModified?: string }> {
    return new Promise((resolve, reject) => {
      const parsedUrl = new URL(url);
      const isHttps = parsedUrl.protocol === 'https:';
      const httpModule = isHttps ? https : http;

      const requestHeaders: Record<string, string> = {
        'User-Agent': 'Anvil-BundleManager/1.0',
        ...headers,
      };

      // Add authentication header if configured. buildAuthHeader throws when
      // the credential env name is unauthorised or the credential is bound to
      // a different host; a throw here rejects the promise before any request
      // is made, so the credential never reaches the wire.
      if (auth) {
        const authHeader = this.buildAuthHeader(auth, parsedUrl);
        if (authHeader) {
          requestHeaders['Authorization'] = authHeader;
        }
      }

      // Add conditional request headers
      if (etag) {
        requestHeaders['If-None-Match'] = etag;
      }
      if (lastModified) {
        requestHeaders['If-Modified-Since'] = lastModified;
      }

      const options = {
        hostname: parsedUrl.hostname,
        port: parsedUrl.port || (isHttps ? 443 : 80),
        path: parsedUrl.pathname + parsedUrl.search,
        method: 'GET',
        headers: requestHeaders,
        timeout: this.timeoutMs,
      };

      const request = httpModule.request(options, (response) => {
        // Handle redirects
        if (response.statusCode === 301 || response.statusCode === 302) {
          const location = response.headers.location;
          if (!location) {
            reject(new Error('Redirect without location header'));
            return;
          }

          // Bound the redirect chain to avoid an unbounded recursion DoS from
          // a server that loops (A→B→A) or redirects indefinitely.
          if (redirectsRemaining <= 0) {
            reject(new Error('Too many redirects'));
            return;
          }

          let target: URL;
          try {
            // Resolve relative redirects against the current URL.
            target = new URL(location, url);
          } catch {
            reject(new Error(`Invalid redirect location: ${location}`));
            return;
          }

          // A redirect must not downgrade transport security: enforce HTTPS
          // on the redirect hop (allowing localhost for development/testing,
          // matching the initial-URL policy in downloadBundle).
          const targetIsLocalhost =
            target.hostname === 'localhost' || target.hostname === '127.0.0.1';
          if (target.protocol !== 'https:' && !targetIsLocalhost) {
            reject(new Error(`Refusing to follow redirect to non-HTTPS URL: ${target.href}`));
            return;
          }

          // Never leak credentials across origins. When the redirect crosses
          // origin, drop the configured auth AND all caller-supplied headers:
          // `BundleConfig.headers` is arbitrary input that may carry
          // `Authorization`, `Cookie`, `Proxy-Authorization`, `X-API-Key`,
          // etc., none of which should reach a different host. Same-origin
          // redirects keep both. (downloadFile re-adds its own `User-Agent`
          // and conditional `If-*` headers regardless.)
          const sameOrigin = target.origin === parsedUrl.origin;
          const forwardedAuth = sameOrigin ? auth : undefined;
          const forwardedHeaders = sameOrigin ? headers : undefined;

          this.downloadFile(
            target.href,
            dest,
            forwardedHeaders,
            etag,
            lastModified,
            forwardedAuth,
            redirectsRemaining - 1
          )
            .then(resolve)
            .catch(reject);
          return;
        }

        // Handle 304 Not Modified
        if (response.statusCode === 304) {
          resolve({ notModified: true });
          return;
        }

        if (response.statusCode !== 200) {
          reject(new Error(`Download failed: HTTP ${response.statusCode}`));
          return;
        }

        const file = createWriteStream(dest);

        response.pipe(file);

        file.on('finish', () => {
          file.close();
          resolve({
            notModified: false,
            etag: response.headers.etag as string | undefined,
            lastModified: response.headers['last-modified'] as string | undefined,
          });
        });

        file.on('error', (err) => {
          file.close();
          if (existsSync(dest)) {
            unlinkSync(dest);
          }
          reject(err);
        });
      });

      request.on('error', (err) => {
        if (existsSync(dest)) {
          unlinkSync(dest);
        }
        reject(err);
      });

      request.on('timeout', () => {
        request.destroy();
        if (existsSync(dest)) {
          unlinkSync(dest);
        }
        reject(new Error(`Download timeout after ${this.timeoutMs}ms`));
      });

      request.end();
    });
  }

  /**
   * Compute SHA256 checksum of a file
   */
  private async computeChecksum(filePath: string): Promise<string> {
    const content = readFileSync(filePath);
    return createHash('sha256').update(content).digest('hex');
  }

  /**
   * Verify signature of a file using public key
   */
  private async verifySignature(
    filePath: string,
    signaturePath: string,
    publicKey: string
  ): Promise<boolean> {
    try {
      if (!existsSync(signaturePath)) {
        debug(`Signature file not found: ${signaturePath}`);
        return false;
      }

      const fileContent = readFileSync(filePath);
      const signature = readFileSync(signaturePath);

      const verify = createVerify('SHA256');
      verify.update(fileContent);
      verify.end();

      return verify.verify(publicKey, signature);
    } catch (error) {
      debug('Signature verification error', error);
      return false;
    }
  }

  /**
   * Extract a tarball to destination directory
   */
  private async extractBundle(tarPath: string, destDir: string): Promise<void> {
    const { resolve, sep } = await import('node:path');
    const resolvedDest = resolve(destDir);

    await pipeline(
      createReadStream(tarPath),
      createGunzip(),
      extract({
        cwd: destDir,
        strip: 0,
        filter: (entryPath) => {
          const resolved = resolve(destDir, entryPath);
          if (resolved !== resolvedDest && !resolved.startsWith(resolvedDest + sep)) {
            debug(`Zip-slip blocked: ${entryPath} escapes ${destDir}`);
            return false;
          }
          return true;
        },
      })
    );
  }
}

// Import createReadStream for extraction
import { createReadStream } from 'node:fs';

/**
 * Create a singleton bundle manager
 */
let defaultManager: BundleManager | null = null;

export function getBundleManager(config?: BundleManagerConfig): BundleManager {
  if (!defaultManager || config) {
    defaultManager = new BundleManager(config);
  }
  return defaultManager;
}
