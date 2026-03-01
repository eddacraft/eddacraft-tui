/**
 * Policy Config - YAML-based policy configuration with org/team/local layering
 *
 * Handles .anvil/config.yml which supports:
 * - Org-level policy sources (git repos)
 * - Team-level policy definitions with metadata
 * - Local overrides
 * - Enforcement levels and graduated rollout
 */

import { existsSync, readFileSync, writeFileSync, mkdirSync, readdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import YAML from 'yaml';
import { z } from 'zod';
import { createDebugger } from '@eddacraft/anvil-core';

const log = createDebugger('service');

// ---------------------------------------------------------------------------
// Zod schemas for config.yml validation
// ---------------------------------------------------------------------------

const EnforcementLevelSchema = z.enum(['block', 'warn', 'info', 'off']);

const PolicyEntrySchema = z.object({
  name: z.string().max(200),
  reason: z.string().max(1000).optional(),
  owner: z.string().max(200).optional(),
  enforcement: EnforcementLevelSchema,
  effective: z.string().optional(),
  tags: z.array(z.string().max(100)).optional(),
});

const OrgPolicySourceSchema = z.object({
  source: z.string().max(500),
  ref: z.string().max(200).optional(),
});

const AnnouncementEntrySchema = z.object({
  message: z.string().max(2000),
  expires: z.string().optional(),
  level: z.enum(['info', 'warning']).optional(),
});

const PoliciesConfigSchema = z.object({
  org: OrgPolicySourceSchema.optional(),
  team: z.array(PolicyEntrySchema).optional(),
  local: z.array(PolicyEntrySchema).optional(),
  starter_profile: z.string().optional(),
});

const AnvilConfigSchema = z.object({
  policies: PoliciesConfigSchema.optional(),
  announcements: z.array(AnnouncementEntrySchema).optional(),
});

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Enforcement level for a policy */
export type EnforcementLevel = 'block' | 'warn' | 'info' | 'off';

/** Source layer a policy comes from */
export type PolicySource = 'org' | 'team' | 'local' | 'starter' | 'bundle';

/** A single team/local policy entry in config.yml */
export interface PolicyEntry {
  name: string;
  reason?: string;
  owner?: string;
  enforcement: EnforcementLevel;
  /** ISO date string — policy starts enforcing on this date */
  effective?: string;
  /** Tags for grouping / filtering */
  tags?: string[];
}

/** Org-level source reference */
export interface OrgPolicySource {
  source: string;
  ref?: string;
}

/** Top-level policies section in .anvil/config.yml */
export interface PoliciesConfig {
  org?: OrgPolicySource;
  team?: PolicyEntry[];
  local?: PolicyEntry[];
  /** Starter profile name applied during init */
  starter_profile?: string;
}

/** Full .anvil/config.yml schema */
export interface AnvilConfig {
  policies?: PoliciesConfig;
  /** Announcements shown on gate/validate (MOTD-style) */
  announcements?: AnnouncementEntry[];
}

/** MOTD-style announcement */
export interface AnnouncementEntry {
  message: string;
  /** ISO date after which the announcement expires */
  expires?: string;
  level?: 'info' | 'warning';
}

/** A resolved policy with all metadata, regardless of source layer */
export interface ResolvedPolicy {
  name: string;
  source: PolicySource;
  enforcement: EnforcementLevel;
  reason?: string;
  owner?: string;
  effective?: string;
  tags?: string[];
  /** Whether this policy is currently active (effective date reached, not off) */
  active: boolean;
  /** Whether this policy has a matching .rego file on disk */
  hasRegoFile: boolean;
  /** Path to the .rego file if found */
  regoPath?: string;
}

// ---------------------------------------------------------------------------
// Config file path helpers
// ---------------------------------------------------------------------------

const CONFIG_FILENAME = 'config.yml';
const CONFIG_DIR = '.anvil';

export function getConfigPath(workspaceRoot: string): string {
  return join(workspaceRoot, CONFIG_DIR, CONFIG_FILENAME);
}

// ---------------------------------------------------------------------------
// PolicyConfigManager
// ---------------------------------------------------------------------------

export class PolicyConfigManager {
  private readonly configPath: string;
  private readonly workspaceRoot: string;

  constructor(workspaceRoot: string) {
    this.workspaceRoot = workspaceRoot;
    this.configPath = getConfigPath(workspaceRoot);
  }

  /** Check whether .anvil/config.yml exists */
  exists(): boolean {
    return existsSync(this.configPath);
  }

  /** Load and parse .anvil/config.yml. Returns empty config if missing. */
  load(): AnvilConfig {
    log(`PolicyConfigManager.load: path=${this.configPath}`);
    if (!this.exists()) {
      log('PolicyConfigManager.load: config file not found, returning empty');
      return {};
    }

    try {
      const raw = readFileSync(this.configPath, 'utf-8');
      const parsed = YAML.parse(raw);
      if (!parsed || typeof parsed !== 'object') {
        return {};
      }
      return AnvilConfigSchema.parse(parsed);
    } catch (error) {
      log(`PolicyConfigManager.load: failed to parse config: ${error}`);
      return {};
    }
  }

  /** Write config back to .anvil/config.yml */
  save(config: AnvilConfig): void {
    const dir = dirname(this.configPath);
    if (!existsSync(dir)) {
      mkdirSync(dir, { recursive: true });
    }

    const yamlStr = YAML.stringify(config, { lineWidth: 100 });
    writeFileSync(this.configPath, yamlStr, 'utf-8');
  }

  /** Get the path being managed */
  getPath(): string {
    return this.configPath;
  }

  // -----------------------------------------------------------------------
  // Policy resolution
  // -----------------------------------------------------------------------

  /**
   * Resolve all policies from every layer into a flat list.
   *
   * Priority (highest wins): local > team > org > starter
   * If the same policy name appears in multiple layers, the higher-priority
   * layer's entry wins.
   */
  resolvePolicies(config?: AnvilConfig): ResolvedPolicy[] {
    log('PolicyConfigManager.resolvePolicies');
    const cfg = config ?? this.load();
    const map = new Map<string, ResolvedPolicy>();

    // Starter profile policies (lowest priority) — inferred from .rego files
    const regoFiles = this.discoverRegoFiles();
    for (const regoName of regoFiles) {
      const entry: ResolvedPolicy = {
        name: regoName.name,
        source: 'starter',
        enforcement: 'block',
        active: true,
        hasRegoFile: true,
        regoPath: regoName.path,
      };
      map.set(regoName.name, entry);
    }

    // Org policies (if declared, conceptual — actual rego files come from sync)
    // We don't resolve individual org policies here since they come from a remote source.
    // The org source is stored in config for sync purposes.

    // Team policies
    if (cfg.policies?.team) {
      for (const entry of cfg.policies.team) {
        const existing = map.get(entry.name);
        map.set(entry.name, {
          name: entry.name,
          source: 'team',
          enforcement: entry.enforcement,
          reason: entry.reason,
          owner: entry.owner,
          effective: entry.effective,
          tags: entry.tags,
          active: this.isActive(entry),
          hasRegoFile: existing?.hasRegoFile ?? this.hasRegoFile(entry.name),
          regoPath: existing?.regoPath ?? this.findRegoPath(entry.name),
        });
      }
    }

    // Local overrides (highest priority)
    if (cfg.policies?.local) {
      for (const entry of cfg.policies.local) {
        const existing = map.get(entry.name);
        map.set(entry.name, {
          name: entry.name,
          source: 'local',
          enforcement: entry.enforcement,
          reason: entry.reason,
          owner: entry.owner,
          effective: entry.effective,
          tags: entry.tags,
          active: this.isActive(entry),
          hasRegoFile: existing?.hasRegoFile ?? this.hasRegoFile(entry.name),
          regoPath: existing?.regoPath ?? this.findRegoPath(entry.name),
        });
      }
    }

    const resolved = Array.from(map.values());
    log(
      `PolicyConfigManager.resolvePolicies: ${resolved.length} policies resolved (${resolved.filter((p) => p.active).length} active)`
    );
    return resolved;
  }

  // -----------------------------------------------------------------------
  // Policy mutation helpers
  // -----------------------------------------------------------------------

  /** Disable a policy by setting enforcement to 'off' in local overrides */
  disablePolicy(policyName: string): AnvilConfig {
    log(`PolicyConfigManager.disablePolicy: ${policyName}`);
    const config = this.load();
    if (!config.policies) {
      config.policies = {};
    }
    if (!config.policies.local) {
      config.policies.local = [];
    }

    const existing = config.policies.local.find((p) => p.name === policyName);
    if (existing) {
      existing.enforcement = 'off';
    } else {
      config.policies.local.push({
        name: policyName,
        enforcement: 'off',
        reason: 'Disabled via `anvil policy disable`',
      });
    }

    this.save(config);
    return config;
  }

  /** Enable a policy by removing the local 'off' override or setting enforcement level */
  enablePolicy(policyName: string, enforcement: EnforcementLevel = 'block'): AnvilConfig {
    log(`PolicyConfigManager.enablePolicy: ${policyName} enforcement=${enforcement}`);
    const config = this.load();
    if (!config.policies) {
      config.policies = {};
    }
    if (!config.policies.local) {
      config.policies.local = [];
    }

    const idx = config.policies.local.findIndex((p) => p.name === policyName);
    if (idx >= 0) {
      if (enforcement === 'block') {
        // Remove local override entirely, let team/org take effect
        config.policies.local.splice(idx, 1);
      } else {
        config.policies.local[idx].enforcement = enforcement;
      }
    } else if (enforcement !== 'block') {
      // No existing local entry — create one to persist the requested enforcement level
      config.policies.local.push({
        name: policyName,
        enforcement,
        reason: 'Enabled via `anvil policy enable`',
      });
    }

    this.save(config);
    return config;
  }

  /** Add or update a team policy entry */
  setTeamPolicy(entry: PolicyEntry): AnvilConfig {
    const config = this.load();
    if (!config.policies) {
      config.policies = {};
    }
    if (!config.policies.team) {
      config.policies.team = [];
    }

    const idx = config.policies.team.findIndex((p) => p.name === entry.name);
    if (idx >= 0) {
      config.policies.team[idx] = entry;
    } else {
      config.policies.team.push(entry);
    }

    this.save(config);
    return config;
  }

  /** Set or update the org source */
  setOrgSource(source: OrgPolicySource): AnvilConfig {
    const config = this.load();
    if (!config.policies) {
      config.policies = {};
    }
    config.policies.org = source;
    this.save(config);
    return config;
  }

  // -----------------------------------------------------------------------
  // Scaffold helpers
  // -----------------------------------------------------------------------

  /**
   * Generate a scaffold config.yml for an org repo.
   * Takes the current team policies and writes them as the org baseline.
   */
  generateOrgScaffold(_orgName: string): string {
    const config = this.load();
    const teamPolicies = config.policies?.team ?? [];

    const orgConfig: AnvilConfig = {
      policies: {
        team: teamPolicies.length > 0 ? teamPolicies : undefined,
      },
    };

    return YAML.stringify(orgConfig, { lineWidth: 100 });
  }

  /**
   * Generate a POLICIES.md document from resolved policies.
   */
  generatePoliciesDoc(): string {
    const resolved = this.resolvePolicies();
    const config = this.load();
    const lines: string[] = [];

    lines.push('# Policy Documentation');
    lines.push('');
    lines.push('> Auto-generated by `anvil policy doc`. Do not edit manually.');
    lines.push('');

    // Org source
    if (config.policies?.org) {
      lines.push('## Org Source');
      lines.push('');
      lines.push(`- **Repository:** \`${config.policies.org.source}\``);
      if (config.policies.org.ref) {
        lines.push(`- **Version:** \`${config.policies.org.ref}\``);
      }
      lines.push('');
    }

    // Active policies table
    const active = resolved.filter((p) => p.active);
    const inactive = resolved.filter((p) => !p.active);

    if (active.length > 0) {
      lines.push('## Active Policies');
      lines.push('');
      lines.push('| Policy | Source | Enforcement | Owner | Reason |');
      lines.push('|--------|--------|-------------|-------|--------|');
      for (const p of active) {
        const owner = p.owner ?? '-';
        const reason = p.reason ?? '-';
        lines.push(`| ${p.name} | ${p.source} | ${p.enforcement} | ${owner} | ${reason} |`);
      }
      lines.push('');
    }

    if (inactive.length > 0) {
      lines.push('## Pending / Disabled Policies');
      lines.push('');
      lines.push('| Policy | Source | Enforcement | Effective | Reason |');
      lines.push('|--------|--------|-------------|-----------|--------|');
      for (const p of inactive) {
        const effective = p.effective ?? '-';
        const reason = p.reason ?? '-';
        lines.push(`| ${p.name} | ${p.source} | ${p.enforcement} | ${effective} | ${reason} |`);
      }
      lines.push('');
    }

    // Announcements
    const announcements = config.announcements?.filter(
      (a) => !a.expires || new Date(a.expires) > new Date()
    );
    if (announcements && announcements.length > 0) {
      lines.push('## Announcements');
      lines.push('');
      for (const a of announcements) {
        const prefix = a.level === 'warning' ? '**Warning:**' : 'Info:';
        lines.push(`- ${prefix} ${a.message}`);
      }
      lines.push('');
    }

    lines.push('---');
    lines.push(`*Generated on ${new Date().toISOString().split('T')[0]}*`);
    lines.push('');

    return lines.join('\n');
  }

  // -----------------------------------------------------------------------
  // Internal helpers
  // -----------------------------------------------------------------------

  private isActive(entry: PolicyEntry): boolean {
    if (entry.enforcement === 'off') {
      return false;
    }
    if (entry.effective) {
      return new Date(entry.effective) <= new Date();
    }
    return true;
  }

  private hasRegoFile(policyName: string): boolean {
    return this.findRegoPath(policyName) !== undefined;
  }

  private findRegoPath(policyName: string): string | undefined {
    const policyDir = join(this.workspaceRoot, CONFIG_DIR, 'policies');
    const candidate = join(policyDir, `${policyName}.rego`);
    return existsSync(candidate) ? candidate : undefined;
  }

  private discoverRegoFiles(): Array<{ name: string; path: string }> {
    const policyDir = join(this.workspaceRoot, CONFIG_DIR, 'policies');
    if (!existsSync(policyDir)) {
      return [];
    }

    try {
      const entries: string[] = readdirSync(policyDir);
      return entries
        .filter((f: string) => f.endsWith('.rego') && !f.endsWith('_test.rego'))
        .map((f: string) => ({
          name: f.replace(/\.rego$/, ''),
          path: join(policyDir, f),
        }));
    } catch {
      return [];
    }
  }
}

// ---------------------------------------------------------------------------
// Starter profiles — opinionated defaults based on detected project type
// ---------------------------------------------------------------------------

export type StarterProfileName =
  | 'web-frontend'
  | 'web-backend'
  | 'fullstack'
  | 'library'
  | 'monorepo'
  | 'generic';

export interface StarterProfile {
  name: StarterProfileName;
  description: string;
  policies: PolicyEntry[];
}

const STARTER_PROFILES: Record<StarterProfileName, StarterProfile> = {
  'web-frontend': {
    name: 'web-frontend',
    description: 'React, Vue, Svelte, or other frontend framework',
    policies: [
      {
        name: 'secret-scan',
        reason: 'Secrets in source control are the #1 cause of security breaches.',
        enforcement: 'block',
      },
      {
        name: 'coverage_min',
        reason: 'Maintain test coverage to catch regressions early.',
        enforcement: 'warn',
      },
      {
        name: 'change_scope',
        reason: 'Large changes are harder to review and more likely to introduce bugs.',
        enforcement: 'warn',
      },
      {
        name: 'security_baseline',
        reason: 'Security-sensitive files need additional review.',
        enforcement: 'block',
      },
    ],
  },
  'web-backend': {
    name: 'web-backend',
    description: 'Express, NestJS, or other backend framework',
    policies: [
      {
        name: 'secret-scan',
        reason: 'Secrets in source control are the #1 cause of security breaches.',
        enforcement: 'block',
      },
      {
        name: 'coverage_min',
        reason: 'Backend services need thorough test coverage.',
        enforcement: 'block',
      },
      {
        name: 'change_scope',
        reason: 'Large changes are harder to review and more likely to introduce bugs.',
        enforcement: 'warn',
      },
      {
        name: 'security_baseline',
        reason: 'Security-sensitive files need additional review.',
        enforcement: 'block',
      },
    ],
  },
  fullstack: {
    name: 'fullstack',
    description: 'Next.js or similar fullstack framework',
    policies: [
      {
        name: 'secret-scan',
        reason: 'Secrets in source control are the #1 cause of security breaches.',
        enforcement: 'block',
      },
      {
        name: 'coverage_min',
        reason: 'Maintain test coverage to catch regressions early.',
        enforcement: 'warn',
      },
      {
        name: 'change_scope',
        reason: 'Large changes are harder to review and more likely to introduce bugs.',
        enforcement: 'warn',
      },
      {
        name: 'security_baseline',
        reason: 'Security-sensitive files need additional review.',
        enforcement: 'block',
      },
    ],
  },
  library: {
    name: 'library',
    description: 'Reusable library or package',
    policies: [
      {
        name: 'secret-scan',
        reason: 'Secrets in source control are the #1 cause of security breaches.',
        enforcement: 'block',
      },
      {
        name: 'coverage_min',
        reason: 'Libraries need high test coverage for consumer confidence.',
        enforcement: 'block',
      },
      {
        name: 'change_scope',
        reason: 'Large changes are harder to review and more likely to introduce bugs.',
        enforcement: 'warn',
      },
    ],
  },
  monorepo: {
    name: 'monorepo',
    description: 'NX, Turborepo, Lerna, or workspace-based monorepo',
    policies: [
      {
        name: 'secret-scan',
        reason: 'Secrets in source control are the #1 cause of security breaches.',
        enforcement: 'block',
      },
      {
        name: 'coverage_min',
        reason: 'Maintain test coverage to catch regressions early.',
        enforcement: 'warn',
      },
      {
        name: 'change_scope',
        reason: 'Large changes spanning packages are harder to review.',
        enforcement: 'warn',
      },
      {
        name: 'security_baseline',
        reason: 'Security-sensitive files need additional review.',
        enforcement: 'block',
      },
    ],
  },
  generic: {
    name: 'generic',
    description: 'General-purpose project',
    policies: [
      {
        name: 'secret-scan',
        reason: 'Secrets in source control are the #1 cause of security breaches.',
        enforcement: 'block',
      },
      {
        name: 'coverage_min',
        reason: 'Maintain test coverage to catch regressions early.',
        enforcement: 'warn',
      },
      {
        name: 'change_scope',
        reason: 'Large changes are harder to review and more likely to introduce bugs.',
        enforcement: 'warn',
      },
    ],
  },
};

/**
 * Select a starter profile based on detected project characteristics.
 */
export function selectStarterProfile(framework: string, monorepo: string): StarterProfile {
  if (monorepo !== 'none') {
    return STARTER_PROFILES.monorepo;
  }

  switch (framework) {
    case 'nextjs':
      return STARTER_PROFILES.fullstack;
    case 'react':
    case 'vue':
    case 'svelte':
    case 'angular':
      return STARTER_PROFILES['web-frontend'];
    case 'express':
    case 'nestjs':
      return STARTER_PROFILES['web-backend'];
    case 'node':
      return STARTER_PROFILES.library;
    default:
      return STARTER_PROFILES.generic;
  }
}

export function getStarterProfile(name: StarterProfileName): StarterProfile {
  return STARTER_PROFILES[name];
}

export function getAllStarterProfiles(): StarterProfile[] {
  return Object.values(STARTER_PROFILES);
}
