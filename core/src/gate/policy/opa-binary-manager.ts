/**
 * OPA Binary Manager - Download, cache, and manage OPA binary
 */

import { existsSync, mkdirSync, chmodSync, createWriteStream, unlinkSync } from 'fs';
import { join } from 'path';
import { homedir, platform, arch } from 'os';
import { execSync, exec } from 'child_process';
import { promisify } from 'util';
import https from 'https';
import http from 'http';

const execAsync = promisify(exec);

/**
 * Default OPA version to download
 */
const DEFAULT_OPA_VERSION = '0.60.0';

/**
 * Binary cache directory
 */
const DEFAULT_CACHE_DIR = join(homedir(), '.anvil', 'bin');

/**
 * Platform and architecture mapping for OPA downloads
 */
const PLATFORM_MAP: Record<string, string> = {
  darwin: 'darwin',
  linux: 'linux',
  win32: 'windows',
};

const ARCH_MAP: Record<string, string> = {
  x64: 'amd64',
  arm64: 'arm64',
};

export interface OPABinaryConfig {
  /** OPA version to use */
  version?: string;
  /** Directory to cache OPA binary */
  cacheDir?: string;
  /** Whether to auto-download if missing */
  autoDownload?: boolean;
}

export interface BinaryInfo {
  /** Path to the OPA binary */
  path: string;
  /** OPA version */
  version: string;
  /** Platform (darwin, linux, windows) */
  platform: string;
  /** Architecture (amd64, arm64) */
  arch: string;
}

/**
 * Manages OPA binary download, caching, and version verification
 */
export class OPABinaryManager {
  private readonly version: string;
  private readonly cacheDir: string;
  private readonly autoDownload: boolean;
  private cachedBinaryPath: string | null = null;

  constructor(config: OPABinaryConfig = {}) {
    this.version = process.env.ANVIL_OPA_VERSION || config.version || DEFAULT_OPA_VERSION;
    this.cacheDir = config.cacheDir || DEFAULT_CACHE_DIR;
    this.autoDownload = config.autoDownload ?? true;
  }

  /**
   * Ensure OPA binary is available, downloading if necessary
   */
  async ensureBinary(): Promise<string> {
    // Check environment override first
    const envPath = process.env.ANVIL_OPA_PATH;
    if (envPath) {
      if (existsSync(envPath)) {
        await this.verifyVersion(envPath);
        return envPath;
      }
      throw new Error(`ANVIL_OPA_PATH specified but file not found: ${envPath}`);
    }

    // Check if already cached
    if (this.cachedBinaryPath && existsSync(this.cachedBinaryPath)) {
      return this.cachedBinaryPath;
    }

    // Check cache directory
    const binaryPath = this.getBinaryPath();
    if (existsSync(binaryPath)) {
      const isValid = await this.verifyVersion(binaryPath);
      if (isValid) {
        this.cachedBinaryPath = binaryPath;
        return binaryPath;
      }
      // Invalid version, remove and re-download
      unlinkSync(binaryPath);
    }

    // Check if OPA is in PATH
    const systemOpa = this.findInPath();
    if (systemOpa) {
      const isValid = await this.verifyVersion(systemOpa);
      if (isValid) {
        this.cachedBinaryPath = systemOpa;
        return systemOpa;
      }
    }

    // Auto-download if enabled
    if (this.autoDownload) {
      await this.downloadBinary();
      this.cachedBinaryPath = binaryPath;
      return binaryPath;
    }

    throw new Error(
      `OPA binary not found. Install OPA v${this.version} or set ANVIL_OPA_PATH environment variable.`
    );
  }

  /**
   * Get information about the current OPA binary
   */
  async getBinaryInfo(): Promise<BinaryInfo | null> {
    try {
      const binaryPath = await this.ensureBinary();
      return {
        path: binaryPath,
        version: this.version,
        platform: this.getPlatform(),
        arch: this.getArch(),
      };
    } catch {
      return null;
    }
  }

  /**
   * Force re-download of OPA binary
   */
  async forceDownload(): Promise<string> {
    const binaryPath = this.getBinaryPath();
    if (existsSync(binaryPath)) {
      unlinkSync(binaryPath);
    }
    await this.downloadBinary();
    this.cachedBinaryPath = binaryPath;
    return binaryPath;
  }

  /**
   * Get the expected binary path for current platform
   */
  private getBinaryPath(): string {
    const binaryName = this.getBinaryName();
    return join(this.cacheDir, binaryName);
  }

  /**
   * Get the binary filename for current platform
   */
  private getBinaryName(): string {
    const plat = this.getPlatform();
    const architecture = this.getArch();
    const ext = plat === 'windows' ? '.exe' : '';
    return `opa-${this.version}-${plat}-${architecture}${ext}`;
  }

  /**
   * Get normalised platform name
   */
  private getPlatform(): string {
    const plat = platform();
    const mapped = PLATFORM_MAP[plat];
    if (!mapped) {
      throw new Error(`Unsupported platform: ${plat}`);
    }
    return mapped;
  }

  /**
   * Get normalised architecture name
   */
  private getArch(): string {
    const architecture = arch();
    const mapped = ARCH_MAP[architecture];
    if (!mapped) {
      throw new Error(`Unsupported architecture: ${architecture}`);
    }
    return mapped;
  }

  /**
   * Find OPA in system PATH
   */
  private findInPath(): string | null {
    try {
      const cmd = platform() === 'win32' ? 'where opa' : 'which opa';
      const result = execSync(cmd, { encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'] });
      const path = result.trim().split('\n')[0];
      return path && existsSync(path) ? path : null;
    } catch {
      return null;
    }
  }

  /**
   * Verify OPA binary version matches expected version
   */
  private async verifyVersion(binaryPath: string): Promise<boolean> {
    try {
      const { stdout } = await execAsync(`"${binaryPath}" version`);
      // OPA version output format: "Version: 0.60.0"
      const match = stdout.match(/Version:\s*(\d+\.\d+\.\d+)/);
      if (!match) {
        return false;
      }
      const installedVersion = match[1];
      // Allow minor version differences (0.60.x matches 0.60.0)
      const [major, minor] = this.version.split('.');
      const [instMajor, instMinor] = installedVersion.split('.');
      return major === instMajor && minor === instMinor;
    } catch {
      return false;
    }
  }

  /**
   * Download OPA binary from official releases
   */
  private async downloadBinary(): Promise<void> {
    const url = this.getDownloadUrl();
    const binaryPath = this.getBinaryPath();

    // Ensure cache directory exists
    if (!existsSync(this.cacheDir)) {
      mkdirSync(this.cacheDir, { recursive: true });
    }

    console.warn(`Downloading OPA v${this.version}...`);
    console.warn(`  URL: ${url}`);
    console.warn(`  Destination: ${binaryPath}`);

    await this.downloadFile(url, binaryPath);

    // Make executable on Unix systems
    if (platform() !== 'win32') {
      chmodSync(binaryPath, 0o755);
    }

    // Verify download
    const isValid = await this.verifyVersion(binaryPath);
    if (!isValid) {
      unlinkSync(binaryPath);
      throw new Error('Downloaded OPA binary failed verification');
    }

    console.warn(`OPA v${this.version} downloaded successfully`);
  }

  /**
   * Get download URL for current platform
   */
  private getDownloadUrl(): string {
    const plat = this.getPlatform();
    const architecture = this.getArch();
    const ext = plat === 'windows' ? '.exe' : '';

    // OPA release URL format
    // https://openpolicyagent.org/downloads/v0.60.0/opa_linux_amd64
    return `https://openpolicyagent.org/downloads/v${this.version}/opa_${plat}_${architecture}${ext}`;
  }

  /**
   * Download a file from URL to destination
   */
  private downloadFile(url: string, dest: string): Promise<void> {
    return new Promise((resolve, reject) => {
      const file = createWriteStream(dest);
      const protocol = url.startsWith('https') ? https : http;

      const request = protocol.get(url, (response) => {
        // Handle redirects
        if (response.statusCode === 301 || response.statusCode === 302) {
          const redirectUrl = response.headers.location;
          if (!redirectUrl) {
            reject(new Error('Redirect without location header'));
            return;
          }
          file.close();
          unlinkSync(dest);
          this.downloadFile(redirectUrl, dest).then(resolve).catch(reject);
          return;
        }

        if (response.statusCode !== 200) {
          reject(new Error(`Download failed: HTTP ${response.statusCode}`));
          return;
        }

        response.pipe(file);

        file.on('finish', () => {
          file.close();
          resolve();
        });
      });

      request.on('error', (err) => {
        file.close();
        unlinkSync(dest);
        reject(err);
      });

      file.on('error', (err) => {
        file.close();
        unlinkSync(dest);
        reject(err);
      });
    });
  }
}

/**
 * Create a singleton OPA binary manager
 */
let defaultManager: OPABinaryManager | null = null;

export function getOPABinaryManager(config?: OPABinaryConfig): OPABinaryManager {
  if (!defaultManager || config) {
    defaultManager = new OPABinaryManager(config);
  }
  return defaultManager;
}
