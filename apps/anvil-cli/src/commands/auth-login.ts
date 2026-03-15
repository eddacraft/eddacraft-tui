/**
 * Auth Login Command (BAUTH-014)
 *
 * Interactive CLI authentication via device code flow or email OTP.
 *
 * Usage:
 *   anvil auth login          Device code flow (default)
 *   anvil auth login --otp    Email OTP flow
 */

import { Command } from 'commander';
import * as readline from 'node:readline';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as os from 'node:os';
import chalk from 'chalk';
import { success, error, info, print } from '../utils/output.js';
import { CliError } from '../utils/cli-error.js';
import { saveAuth } from '../services/auth-store.js';
import { saveLicence } from '../services/licence-store.js';

interface AuthLoginOptions {
  otp?: boolean;
  apiUrl?: string;
}

interface DeviceStartResponse {
  pollToken: string;
  userCode: string;
  verificationUrl: string;
  expiresIn: number;
}

interface DevicePollResponse {
  status: 'pending' | 'confirmed' | 'expired';
  license?: string;
  refreshToken?: string;
  expiresAt?: string;
}

interface OtpRequestResponse {
  ok: boolean;
}

interface OtpVerifyResponse {
  license: string;
  refreshToken: string;
  expiresAt: string;
}

interface StoredCredentials {
  license: string;
  refreshToken: string;
  expiresAt: string;
  email: string;
}

function getApiUrl(opts: AuthLoginOptions): string {
  return opts.apiUrl ?? process.env['ANVIL_API_URL'] ?? 'https://api.eddacraft.ai/api/v1';
}

function getCredentialsPath(): string {
  const configDir = process.env['XDG_CONFIG_HOME'] ?? path.join(os.homedir(), '.config');
  return path.join(configDir, 'anvil', 'credentials.json');
}

function saveCredentials(data: StoredCredentials): void {
  const credPath = getCredentialsPath();
  fs.mkdirSync(path.dirname(credPath), { recursive: true });
  fs.writeFileSync(credPath, JSON.stringify(data, null, 2), { mode: 0o600 });
  fs.chmodSync(credPath, 0o600);

  // Write to existing auth stores so protected commands work
  saveLicence(data.license);
  saveAuth({
    token: data.license,
    user: { email: data.email },
    scopes: ['beta'],
    expiresAt: data.expiresAt,
    verifiedAt: new Date().toISOString(),
  });
}

function prompt(question: string): Promise<string> {
  const rl = readline.createInterface({ input: process.stdin, output: process.stderr });
  return new Promise((resolve) => {
    rl.question(question, (answer) => {
      rl.close();
      resolve(answer.trim());
    });
  });
}

async function apiPost<T>(baseUrl: string, endpoint: string, body: unknown): Promise<T> {
  const url = `${baseUrl.replace(/\/+$/, '')}${endpoint}`;

  let res: Response;
  try {
    res = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(30_000),
    });
  } catch (err) {
    if (err instanceof DOMException && err.name === 'TimeoutError') {
      throw new Error('Request timed out. Check your internet connection and try again.');
    }
    throw new Error(
      `Could not connect to ${baseUrl}. Check your internet connection and try again.`
    );
  }

  if (!res.ok) {
    const text = await res.text();
    const truncated = text.length > 200 ? text.slice(0, 200) + '...' : text;
    throw new Error(`API error ${res.status}: ${truncated}`);
  }

  return (await res.json()) as T;
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function deviceCodeFlow(apiUrl: string): Promise<void> {
  const email = await prompt('Email: ');
  if (!email) {
    error('Email is required');
    throw new CliError('Email is required');
  }

  info('Starting device code flow...');

  const startResult = await apiPost<DeviceStartResponse>(apiUrl, '/auth/device/start', { email });

  print('');
  print(`To authenticate, open this URL:`);
  print(`  ${chalk.bold.cyan(startResult.verificationUrl)}`);
  print('');
  print(`And enter code: ${chalk.bold.yellow(startResult.userCode)}`);
  print('');
  info('Waiting for confirmation...');

  const pollIntervalMs = 5_000;
  const maxAttempts = Math.ceil((startResult.expiresIn || 300) / (pollIntervalMs / 1000));

  for (let attempt = 0; attempt < maxAttempts; attempt++) {
    await sleep(pollIntervalMs);

    const pollResult = await apiPost<DevicePollResponse>(apiUrl, '/auth/device/poll', {
      pollToken: startResult.pollToken,
    });

    if (pollResult.status === 'confirmed') {
      if (!pollResult.license || !pollResult.refreshToken || !pollResult.expiresAt) {
        error('Authentication succeeded but the server response was incomplete.');
        throw new CliError('Incomplete auth response');
      }

      saveCredentials({
        license: pollResult.license,
        refreshToken: pollResult.refreshToken,
        expiresAt: pollResult.expiresAt,
        email,
      });

      print('');
      success(`Authenticated as ${chalk.bold(email)}`);
      info(`Credentials saved to ${getCredentialsPath()}`);
      return;
    }

    if (pollResult.status === 'expired') {
      error('The device code has expired. Please try again.');
      throw new CliError('Device code expired');
    }

    // status === 'pending' — continue polling
    process.stderr.write('.');
  }

  error('Timed out waiting for confirmation. Please try again.');
  throw new CliError('Device code flow timed out');
}

async function otpFlow(apiUrl: string): Promise<void> {
  const email = await prompt('Email: ');
  if (!email) {
    error('Email is required');
    throw new CliError('Email is required');
  }

  info('Requesting verification code...');

  await apiPost<OtpRequestResponse>(apiUrl, '/auth/otp/request', { email });

  print('');
  info('A verification code has been sent to your email.');

  const maxAttempts = 3;

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    const code = await prompt('Enter code: ');
    if (!code) {
      error('Code is required');
      if (attempt < maxAttempts) {
        info(`${maxAttempts - attempt} attempt(s) remaining`);
        continue;
      }
      throw new CliError('No code provided');
    }

    try {
      const result = await apiPost<OtpVerifyResponse>(apiUrl, '/auth/otp/verify', { email, code });

      saveCredentials({
        license: result.license,
        refreshToken: result.refreshToken,
        expiresAt: result.expiresAt,
        email,
      });

      print('');
      success(`Authenticated as ${chalk.bold(email)}`);
      info(`Credentials saved to ${getCredentialsPath()}`);
      return;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Verification failed';
      error(message);

      if (attempt < maxAttempts) {
        info(`${maxAttempts - attempt} attempt(s) remaining`);
      }
    }
  }

  error('Maximum attempts reached. Please try again.');
  throw new CliError('OTP verification failed');
}

export function createAuthLoginCommand(): Command {
  const auth = new Command('auth').description('Authentication commands');

  auth
    .command('login')
    .description('Authenticate with Anvil via device code or email OTP')
    .option('--otp', 'Use email OTP instead of device code')
    .option('--api-url <url>', 'API base URL')
    .action(async (opts: AuthLoginOptions) => {
      const apiUrl = getApiUrl(opts);

      try {
        if (opts.otp) {
          await otpFlow(apiUrl);
        } else {
          await deviceCodeFlow(apiUrl);
        }
      } catch (err) {
        if (err instanceof CliError) throw err;
        error(err instanceof Error ? err.message : 'Authentication failed');
        throw new CliError('Authentication failed');
      }
    });

  return auth;
}
