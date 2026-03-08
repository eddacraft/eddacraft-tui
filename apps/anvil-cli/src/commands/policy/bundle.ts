import { existsSync, readFileSync } from 'node:fs';
import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { success, error, info, warning, print, blank, debug } from '../../utils/output.js';
import { coerceNonNegativeInt } from '../../utils/option-coerce.js';
import {
  BundleManager,
  GateConfigManager,
  getBundleManager,
  type BundleAuthConfig,
  type BundleConfig,
  type PolicyBundleConfig,
  type PolicyVerificationConfig,
} from '@eddacraft/anvil-runtime';

function formatRelativeTime(timestamp: number): string {
  const diff = Date.now() - timestamp;
  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) return `${days}d ago`;
  if (hours > 0) return `${hours}h ago`;
  if (minutes > 0) return `${minutes}m ago`;
  return 'just now';
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}

function getBundleStatus(entry: { expires_at: number; signature_verified: boolean } | null): {
  text: string;
  color: typeof chalk.green;
} {
  if (!entry) {
    return { text: 'not synced', color: chalk.yellow };
  }

  const isExpired = Date.now() > entry.expires_at;

  if (isExpired) {
    return { text: 'expired', color: chalk.yellow };
  }

  if (entry.signature_verified) {
    return { text: 'verified', color: chalk.green };
  }

  return { text: 'synced', color: chalk.cyan };
}

function deriveBundleName(url: string): string {
  try {
    const parsed = new URL(url);
    const pathParts = parsed.pathname.split('/').filter(Boolean);
    const lastPart = pathParts[pathParts.length - 1] || '';

    let name = lastPart
      .replace(/\.tar\.gz$/, '')
      .replace(/\.tgz$/, '')
      .replace(/\.bundle$/, '')
      .replace(/\.opa$/, '');

    if (!name) {
      name = parsed.hostname.replace(/\./g, '-');
    }

    return name;
  } catch {
    debug('policy: URL parsing failed for bundle name, using timestamp fallback');
    return `bundle-${Date.now().toString(36)}`;
  }
}

function resolveBundleName(bundle: PolicyBundleConfig): string {
  return bundle.name ?? deriveBundleName(bundle.url);
}

function toBundleAuthConfig(
  auth: PolicyBundleConfig['auth'] | undefined
): BundleAuthConfig | undefined {
  if (!auth) {
    return undefined;
  }

  if (auth.type === 'basic') {
    const basicAuth: BundleAuthConfig = {
      type: 'basic',
    };

    if (auth.username) {
      basicAuth.username = auth.username;
    }

    if (auth.password) {
      basicAuth.password_env = auth.password;
    }

    return basicAuth;
  }

  if (auth.type === 'bearer') {
    const bearerAuth: BundleAuthConfig = {
      type: 'bearer',
    };

    if (auth.token) {
      bearerAuth.token_env = auth.token;
    }

    return bearerAuth;
  }

  return undefined;
}

function toBundleConfig(
  bundle: PolicyBundleConfig,
  verification: PolicyVerificationConfig | undefined
): BundleConfig {
  const name = resolveBundleName(bundle);
  const config: BundleConfig = {
    name,
    url: bundle.url,
  };

  if (bundle.polling_interval !== undefined) {
    config.refresh_interval_ms = bundle.polling_interval;
  }

  const auth = toBundleAuthConfig(bundle.auth);
  if (auth) {
    config.auth = auth;
  }

  const signatureKey = verification?.keys?.[name];
  if (signatureKey) {
    config.signature_key = signatureKey;
  }

  return config;
}

function createPolicyBundleListCommand(): Command {
  return new Command('list').description('List configured policy bundles').action(async () => {
    try {
      const workspaceRoot = getWorkspaceRoot();
      const configManager = new GateConfigManager(workspaceRoot);
      const config = configManager.loadConfig();

      const bundles = config.policy?.bundles || [];

      if (bundles.length === 0) {
        info('No policy bundles configured');
        print(chalk.dim('\nRun `anvil policy bundle add <url>` to add a bundle'));
        return;
      }

      const bundleManager = getBundleManager();

      print(chalk.bold('\nConfigured Policy Bundles:\n'));

      print(
        chalk.dim('  ') +
          chalk.bold('Name'.padEnd(20)) +
          chalk.bold('URL'.padEnd(40)) +
          chalk.bold('Last Sync'.padEnd(15)) +
          chalk.bold('Status')
      );
      print(chalk.dim('  ' + '-'.repeat(85)));

      for (const bundle of bundles) {
        const bundleName = resolveBundleName(bundle);
        const entry = await bundleManager.getBundleEntry(bundleName);
        const lastSync = entry ? formatRelativeTime(entry.downloaded_at) : '-';
        const status = getBundleStatus(entry);
        const enabledIndicator = bundle.enabled === false ? chalk.dim('[disabled] ') : '';

        const maxUrlLen = 38;
        const displayUrl =
          bundle.url.length > maxUrlLen ? bundle.url.slice(0, maxUrlLen - 2) + '..' : bundle.url;

        print(
          '  ' +
            enabledIndicator +
            chalk.cyan(bundleName.padEnd(20 - enabledIndicator.length)) +
            chalk.dim(displayUrl.padEnd(40)) +
            chalk.dim(lastSync.padEnd(15)) +
            status.color(status.text)
        );

        if (entry) {
          print(chalk.dim(`      Size: ${formatSize(entry.size_bytes)}`));
        }
      }

      blank();
      success(`${bundles.length} bundle(s) configured`);
    } catch (err) {
      if (err instanceof CliError || err instanceof CliExit) throw err;
      error(`Failed to list bundles: ${err instanceof Error ? err.message : 'Unknown error'}`);
      throw new CliError('Failed to list bundles');
    }
  });
}

function createPolicyBundleAddCommand(): Command {
  return new Command('add')
    .description('Add a remote policy bundle')
    .argument('<url>', 'URL of the bundle to add')
    .option('-n, --name <name>', 'Name for the bundle (defaults to URL basename)')
    .option('-r, --refresh <ms>', 'Refresh interval in milliseconds', '300000')
    .option('-k, --key <path>', 'Path to public key for signature verification')
    .option('--auth-user <username>', 'Username for basic authentication')
    .option('--auth-pass-env <envvar>', 'Environment variable containing password for basic auth')
    .option('--auth-token-env <envvar>', 'Environment variable containing bearer token')
    .option('--no-sync', 'Do not download the bundle immediately')
    .action(
      async (
        url: string,
        options: {
          name?: string;
          refresh?: string;
          key?: string;
          authUser?: string;
          authPassEnv?: string;
          authTokenEnv?: string;
          sync?: boolean;
        }
      ) => {
        const spinner = ora('Adding bundle configuration...').start();

        try {
          const workspaceRoot = getWorkspaceRoot();
          const configManager = new GateConfigManager(workspaceRoot);
          const config = configManager.loadConfig();

          if (!config.policy) {
            config.policy = {};
          }
          if (!config.policy.bundles) {
            config.policy.bundles = [];
          }

          const bundleName = options.name || deriveBundleName(url);

          const existingIndex = config.policy.bundles.findIndex((b) => b.name === bundleName);
          if (existingIndex >= 0) {
            spinner.fail(`Bundle '${bundleName}' already exists`);
            print(chalk.dim('\nUse a different --name or remove the existing bundle first'));
            throw new CliError(`Bundle '${bundleName}' already exists`);
          }

          const bundleConfig: PolicyBundleConfig = {
            name: bundleName,
            url,
            polling_interval: (() => {
              return coerceNonNegativeInt(options.refresh || '300000', '--refresh');
            })(),
            enabled: true,
          };

          if (options.key) {
            if (!existsSync(options.key)) {
              spinner.fail(`Key file not found: ${options.key}`);
              throw new CliError(`Bundle signature key file not found: ${options.key}`);
            }
            const signatureKey = readFileSync(options.key, 'utf-8').trim();

            if (!config.policy.verification) {
              config.policy.verification = {};
            }
            if (!config.policy.verification.keys) {
              config.policy.verification.keys = {};
            }
            config.policy.verification.keys[bundleName] = signatureKey;
            config.policy.verification.require_signatures = true;
          }

          if (options.authUser || options.authPassEnv || options.authTokenEnv) {
            const auth: PolicyBundleConfig['auth'] = {
              type: options.authTokenEnv ? 'bearer' : 'basic',
            };

            if (options.authUser) {
              auth.username = options.authUser;
            }
            if (options.authPassEnv) {
              auth.password = options.authPassEnv;
            }
            if (options.authTokenEnv) {
              auth.token = options.authTokenEnv;
            }

            bundleConfig.auth = auth;
          }

          config.policy.bundles.push(bundleConfig);
          configManager.saveConfig(config);

          spinner.succeed(`Added bundle '${bundleName}'`);

          if (options.sync !== false) {
            const syncSpinner = ora('Downloading bundle...').start();

            try {
              const bundleManagerConfig = toBundleConfig(bundleConfig, config.policy?.verification);
              const bundleManager = new BundleManager({
                bundles: [bundleManagerConfig],
              });

              const result = await bundleManager.downloadBundle(bundleManagerConfig.name);

              if (result.success) {
                syncSpinner.succeed(`Bundle downloaded to ${result.path}`);
              } else {
                syncSpinner.warn(`Download failed: ${result.error}`);
                print(chalk.dim('\nRun `anvil policy bundle sync` to retry'));
              }
            } catch (syncErr) {
              syncSpinner.warn(
                `Download failed: ${syncErr instanceof Error ? syncErr.message : 'Unknown error'}`
              );
              print(chalk.dim('\nRun `anvil policy bundle sync` to retry'));
            }
          }

          blank();
          success('Bundle configuration saved to .anvilrc');
        } catch (err) {
          if (err instanceof CliError || err instanceof CliExit) throw err;
          spinner.fail('Failed to add bundle');
          error(err instanceof Error ? err.message : 'Unknown error');
          throw new CliError('Failed to add bundle');
        }
      }
    );
}

function createPolicyBundleRemoveCommand(): Command {
  return new Command('remove')
    .description('Remove a policy bundle')
    .argument('<name>', 'Name of the bundle to remove')
    .option('--keep-cache', 'Keep cached bundle files')
    .action(async (name: string, options: { keepCache?: boolean }) => {
      const spinner = ora(`Removing bundle '${name}'...`).start();

      try {
        const workspaceRoot = getWorkspaceRoot();
        const configManager = new GateConfigManager(workspaceRoot);
        const config = configManager.loadConfig();

        const bundles = config.policy?.bundles || [];
        const bundleIndex = bundles.findIndex((b) => resolveBundleName(b) === name);

        if (bundleIndex < 0) {
          spinner.fail(`Bundle '${name}' not found`);
          print(chalk.dim('\nUse `anvil policy bundle list` to see available bundles'));
          throw new CliError(`Bundle '${name}' not found for removal`);
        }

        const bundleName = resolveBundleName(bundles[bundleIndex]);

        bundles.splice(bundleIndex, 1);
        if (config.policy) {
          config.policy.bundles = bundles;
        }
        configManager.saveConfig(config);

        if (!options.keepCache) {
          const bundleManager = getBundleManager();
          await bundleManager.invalidateBundle(bundleName);
          spinner.succeed(`Removed bundle '${bundleName}' and cleared cache`);
        } else {
          spinner.succeed(`Removed bundle '${bundleName}' (cache preserved)`);
        }

        success('Bundle configuration updated');
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        spinner.fail('Failed to remove bundle');
        error(err instanceof Error ? err.message : 'Unknown error');
        throw new CliError('Failed to remove bundle');
      }
    });
}

function createPolicyBundleSyncCommand(): Command {
  return new Command('sync')
    .description('Download or update policy bundles')
    .option('-f, --force', 'Force re-download even if cached')
    .option('-n, --name <name>', 'Sync only a specific bundle')
    .action(async (options: { force?: boolean; name?: string }) => {
      const spinner = ora('Syncing policy bundles...').start();

      try {
        const workspaceRoot = getWorkspaceRoot();
        const configManager = new GateConfigManager(workspaceRoot);
        const config = configManager.loadConfig();

        let bundles = config.policy?.bundles || [];
        const verification = config.policy?.verification;

        if (bundles.length === 0) {
          spinner.warn('No bundles configured');
          print(chalk.dim('\nRun `anvil policy bundle add <url>` to add a bundle'));
          return;
        }

        if (options.name) {
          bundles = bundles.filter((b) => resolveBundleName(b) === options.name);
          if (bundles.length === 0) {
            spinner.fail(`Bundle '${options.name}' not found`);
            throw new CliError(`Bundle '${options.name}' not found for sync`);
          }
        }

        const enabledBundles = bundles.filter((b) => b.enabled !== false);

        if (enabledBundles.length === 0) {
          spinner.warn('All bundles are disabled');
          return;
        }

        const bundleConfigs = enabledBundles.map((bundle) => toBundleConfig(bundle, verification));

        if (options.force) {
          const bundleManager = getBundleManager();
          for (const bundle of bundleConfigs) {
            await bundleManager.invalidateBundle(bundle.name);
          }
        }

        const bundleManager = new BundleManager({
          bundles: bundleConfigs,
        });

        spinner.text = `Syncing ${bundleConfigs.length} bundle(s)...`;

        const results = await bundleManager.syncAll();

        spinner.stop();

        print(chalk.bold('\nBundle Sync Results:\n'));

        let successCount = 0;
        let failCount = 0;

        for (const result of results) {
          if (result.success) {
            successCount++;
            const updateStatus = result.updated ? chalk.green('updated') : chalk.dim('unchanged');
            print(`  ${chalk.green('✓')} ${result.name}: ${updateStatus}`);
            if (result.path) {
              print(chalk.dim(`      Path: ${result.path}`));
            }
          } else {
            failCount++;
            print(`  ${chalk.red('✗')} ${result.name}: ${chalk.red(result.error || 'Failed')}`);
          }
        }

        blank();

        if (failCount === 0) {
          success(`All ${successCount} bundle(s) synced successfully`);
        } else if (successCount > 0) {
          warning(`${successCount} succeeded, ${failCount} failed`);
        } else {
          error(`All ${failCount} bundle(s) failed to sync`);
          throw new CliError('All bundles failed to sync');
        }
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        spinner.fail('Bundle sync failed');
        error(err instanceof Error ? err.message : 'Unknown error');
        throw new CliError('Bundle sync failed');
      }
    });
}

export function createPolicyBundleCommand(): Command {
  return new Command('bundle')
    .description('Manage remote policy bundles')
    .addCommand(createPolicyBundleListCommand())
    .addCommand(createPolicyBundleAddCommand())
    .addCommand(createPolicyBundleRemoveCommand())
    .addCommand(createPolicyBundleSyncCommand());
}
