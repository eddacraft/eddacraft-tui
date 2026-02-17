import type {
  CommandRule,
  CommandSafetyConfig,
  ResolvedCommandSafetyConfig,
  WorkingDirectoryConfig,
  CommandSafetyOutputConfig,
} from '../rules/types.js';
import { DEFAULT_GIT_RULES, DEFAULT_FILESYSTEM_RULES } from '../rules/index.js';
import { createDebugger } from '@eddacraft/anvil-core';

const debug = createDebugger('gate');

const DEFAULT_WORKING_DIRECTORY: Required<WorkingDirectoryConfig> = {
  allowDeleteInCwd: false,
  tempDirPatterns: ['/tmp', '/var/tmp'],
};

const DEFAULT_OUTPUT: Required<CommandSafetyOutputConfig> = {
  verbose: true,
  showSuggestions: true,
  showReferences: true,
};

export function loadCommandSafetyRules(config: CommandSafetyConfig): CommandRule[] {
  debug('loadCommandSafetyRules: loading default rules');
  let rules: CommandRule[] = [...DEFAULT_GIT_RULES, ...DEFAULT_FILESYSTEM_RULES];

  if (config.rules?.disabled && config.rules.disabled.length > 0) {
    debug('loadCommandSafetyRules: disabling %d rules', config.rules.disabled.length);
    const disabledSet = new Set(config.rules.disabled);
    rules = rules.filter((r) => !disabledSet.has(r.id));
  }

  if (config.rules?.overrides && config.rules.overrides.length > 0) {
    debug('loadCommandSafetyRules: applying %d overrides', config.rules.overrides.length);
    for (const override of config.rules.overrides) {
      const ruleIndex = rules.findIndex((r) => r.id === override.id);
      if (ruleIndex !== -1) {
        if (override.action === 'disable') {
          rules.splice(ruleIndex, 1);
        } else {
          rules[ruleIndex] = {
            ...rules[ruleIndex],
            ...(override.action && { action: override.action }),
            ...(override.severity && { severity: override.severity }),
          };
        }
      }
    }
  }

  if (config.rules?.custom && config.rules.custom.length > 0) {
    debug('loadCommandSafetyRules: adding %d custom rules', config.rules.custom.length);
    rules.push(...config.rules.custom);
  }

  debug('loadCommandSafetyRules: total rules=%d', rules.length);
  return rules;
}

export function resolveCommandSafetyConfig(
  userConfig?: CommandSafetyConfig
): ResolvedCommandSafetyConfig {
  const config = userConfig ?? {};
  debug(
    'resolveCommandSafetyConfig: enabled=%s strict=%s',
    config.enabled ?? true,
    config.strict ?? false
  );

  return {
    enabled: config.enabled ?? true,
    strict: config.strict ?? false,
    rules: loadCommandSafetyRules(config),
    workingDirectory: {
      ...DEFAULT_WORKING_DIRECTORY,
      ...config.workingDirectory,
    },
    output: {
      ...DEFAULT_OUTPUT,
      ...config.output,
    },
  };
}

export const DEFAULT_COMMAND_SAFETY_CONFIG: ResolvedCommandSafetyConfig = {
  enabled: true,
  strict: false,
  rules: [...DEFAULT_GIT_RULES, ...DEFAULT_FILESYSTEM_RULES],
  workingDirectory: DEFAULT_WORKING_DIRECTORY,
  output: DEFAULT_OUTPUT,
};
