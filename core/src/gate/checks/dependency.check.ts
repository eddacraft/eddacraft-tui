import { BaseCheck } from '../check.interface.js';
import { CheckContext, GateResult } from '../../types/gate.types.js';
import { exec } from 'child_process';
import { promisify } from 'util';
import { existsSync } from 'fs';
import { join } from 'path';

const execAsync = promisify(exec);

interface AuditAdvisory {
  id: number;
  title: string;
  severity: 'info' | 'low' | 'moderate' | 'high' | 'critical';
  url: string;
  cves: string[];
  module_name: string;
  vulnerable_versions: string;
  patched_versions: string;
  recommendation: string;
  findings: Array<{
    version: string;
    paths: string[];
  }>;
}

interface VulnerabilityCount {
  info: number;
  low: number;
  moderate: number;
  high: number;
  critical: number;
  total: number;
}

interface NormalisedAuditResult {
  advisories: Record<string, AuditAdvisory>;
  metadata: {
    vulnerabilities: VulnerabilityCount;
  };
}

interface NpmV7Vulnerability {
  name: string;
  severity: 'info' | 'low' | 'moderate' | 'high' | 'critical';
  isDirect: boolean;
  via: Array<string | { title?: string; url?: string; severity?: string; cwe?: string[] }>;
  effects: string[];
  range: string;
  nodes: string[];
  fixAvailable: boolean | { name: string; version: string; isSemVerMajor: boolean };
}

interface NpmV7AuditResult {
  auditReportVersion: number;
  vulnerabilities: Record<string, NpmV7Vulnerability>;
  metadata: {
    vulnerabilities: VulnerabilityCount;
    dependencies: {
      prod: number;
      dev: number;
      optional: number;
      peer: number;
      peerOptional: number;
      total: number;
    };
  };
}

interface YarnClassicAdvisory {
  type: 'auditAdvisory';
  data: {
    advisory: {
      id: number;
      title: string;
      module_name: string;
      severity: 'info' | 'low' | 'moderate' | 'high' | 'critical';
      url: string;
      cves: string[];
      vulnerable_versions: string;
      patched_versions: string;
      recommendation: string;
      findings: Array<{ version: string; paths: string[] }>;
    };
  };
}

interface YarnClassicSummary {
  type: 'auditSummary';
  data: {
    vulnerabilities: VulnerabilityCount;
  };
}

const SEVERITY_SCORES = {
  critical: 100,
  high: 75,
  moderate: 50,
  low: 25,
  info: 10,
};

const SEVERITY_ORDER = ['critical', 'high', 'moderate', 'low', 'info'] as const;

type PackageManager = 'npm' | 'yarn' | 'pnpm';

export class DependencyCheck extends BaseCheck {
  name = 'dependency';
  description = 'Scan for dependency vulnerabilities using npm/yarn/pnpm audit';

  async run(context: CheckContext): Promise<GateResult> {
    try {
      // Check if package.json exists
      const packageJsonPath = join(context.workspace_root, 'package.json');
      if (!existsSync(packageJsonPath)) {
        return this.createSuccess('No package.json found, skipping dependency check', 100);
      }

      // Detect package manager
      const packageManager = this.detectPackageManager(context.workspace_root);
      if (!packageManager) {
        return this.createSuccess('No lock file found, skipping dependency check', 100);
      }

      // Determine severity threshold from config (default: moderate)
      const minSeverity = (context.check_config.min_severity as string) || 'moderate';
      const severityIndex = SEVERITY_ORDER.indexOf(minSeverity as (typeof SEVERITY_ORDER)[number]);

      // Run audit with detected package manager
      const auditResult = await this.runAudit(context.workspace_root, packageManager);

      if (!auditResult) {
        return this.createSuccess('No vulnerabilities found', 100);
      }

      const advisories = Object.values(auditResult.advisories);
      const metadata = auditResult.metadata.vulnerabilities;

      // Filter vulnerabilities by severity threshold
      const relevantVulns = advisories.filter((advisory) => {
        const vulnSeverityIndex = SEVERITY_ORDER.indexOf(advisory.severity);
        return vulnSeverityIndex <= severityIndex;
      });

      // Sort by severity (critical first)
      relevantVulns.sort((a, b) => {
        const aScore = SEVERITY_SCORES[a.severity];
        const bScore = SEVERITY_SCORES[b.severity];
        return bScore - aScore;
      });

      // Calculate score based on vulnerabilities
      const criticalCount = metadata.critical;
      const highCount = metadata.high;
      const moderateCount = metadata.moderate;
      const lowCount = metadata.low;

      const score = this.calculateScore(criticalCount, highCount, moderateCount, lowCount);

      // Determine if check passed
      const failOnCritical = context.check_config.fail_on_critical !== false;
      const failOnHigh = context.check_config.fail_on_high !== false;
      const failOnModerate = context.check_config.fail_on_moderate === true;

      const hasCritical = criticalCount > 0;
      const hasHigh = highCount > 0;
      const hasModerate = moderateCount > 0;

      const passed =
        !(failOnCritical && hasCritical) &&
        !(failOnHigh && hasHigh) &&
        !(failOnModerate && hasModerate);

      const totalRelevant = relevantVulns.length;
      const message = passed
        ? `Dependency check passed: ${metadata.total} vulnerabilities found (${criticalCount} critical, ${highCount} high, ${moderateCount} moderate, ${lowCount} low)`
        : `Dependency check failed: ${totalRelevant} vulnerabilities exceed threshold (${criticalCount} critical, ${highCount} high, ${moderateCount} moderate)`;

      // Generate fix suggestions
      const fixSuggestions = this.generateFixSuggestions(relevantVulns, packageManager);

      return this.createResult(passed, message, score, {
        total: metadata.total,
        critical: criticalCount,
        high: highCount,
        moderate: moderateCount,
        low: lowCount,
        info: metadata.info,
        packageManager,
        vulnerabilities: relevantVulns.map((adv) => ({
          id: adv.id,
          title: adv.title,
          severity: adv.severity,
          module: adv.module_name,
          cves: adv.cves,
          url: adv.url,
          vulnerableVersions: adv.vulnerable_versions,
          patchedVersions: adv.patched_versions,
          recommendation: adv.recommendation,
        })),
        fixSuggestions,
      });
    } catch (error) {
      // If audit command fails, it might be due to no vulnerabilities or command error
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';

      // pnpm audit exits with code 1 if vulnerabilities are found
      // Check if it's a legitimate error or just vulnerabilities found
      if (errorMessage.includes('EAUDITNOLOCK')) {
        return this.createSuccess('No lock file found, skipping dependency check', 100);
      }

      return this.createFailure('Dependency check failed', errorMessage);
    }
  }

  private detectPackageManager(workspaceRoot: string): PackageManager | null {
    if (existsSync(join(workspaceRoot, 'pnpm-lock.yaml'))) {
      return 'pnpm';
    }
    if (existsSync(join(workspaceRoot, 'yarn.lock'))) {
      return 'yarn';
    }
    if (existsSync(join(workspaceRoot, 'package-lock.json'))) {
      return 'npm';
    }
    return null;
  }

  private async runAudit(
    workspaceRoot: string,
    packageManager: PackageManager
  ): Promise<NormalisedAuditResult | null> {
    const auditCommand = `${packageManager} audit --json`;

    try {
      const { stdout } = await execAsync(auditCommand, {
        cwd: workspaceRoot,
        maxBuffer: 10 * 1024 * 1024,
      });

      return this.normaliseAuditOutput(stdout, packageManager);
    } catch (error) {
      if (error && typeof error === 'object' && 'stdout' in error) {
        const stdout = (error as { stdout: string }).stdout;
        if (stdout) {
          try {
            return this.normaliseAuditOutput(stdout, packageManager);
          } catch {
            return null;
          }
        }
      }

      if (
        error &&
        typeof error === 'object' &&
        'stderr' in error &&
        typeof (error as { stderr: string }).stderr === 'string'
      ) {
        const stderr = (error as { stderr: string }).stderr;
        if (
          stderr.includes('No known vulnerabilities found') ||
          stderr.includes('0 vulnerabilities') ||
          stderr.includes('found 0 vulnerabilities')
        ) {
          return null;
        }
      }

      throw error;
    }
  }

  private normaliseAuditOutput(
    stdout: string,
    packageManager: PackageManager
  ): NormalisedAuditResult | null {
    if (packageManager === 'yarn') {
      return this.normaliseYarnAudit(stdout);
    }

    const parsed = JSON.parse(stdout);

    if (packageManager === 'npm' && this.isNpmV7Format(parsed)) {
      return this.normaliseNpmV7Audit(parsed as NpmV7AuditResult);
    }

    return this.normalisePnpmAudit(parsed);
  }

  private isNpmV7Format(parsed: unknown): boolean {
    return (
      typeof parsed === 'object' &&
      parsed !== null &&
      'auditReportVersion' in parsed &&
      (parsed as { auditReportVersion: number }).auditReportVersion >= 2
    );
  }

  private normalisePnpmAudit(parsed: {
    advisories?: Record<string, AuditAdvisory>;
    metadata?: { vulnerabilities?: VulnerabilityCount };
  }): NormalisedAuditResult | null {
    if (!parsed.advisories || !parsed.metadata?.vulnerabilities) {
      return null;
    }

    return {
      advisories: parsed.advisories,
      metadata: { vulnerabilities: parsed.metadata.vulnerabilities },
    };
  }

  private normaliseNpmV7Audit(parsed: NpmV7AuditResult): NormalisedAuditResult | null {
    if (!parsed.vulnerabilities || !parsed.metadata?.vulnerabilities) {
      return null;
    }

    const advisories: Record<string, AuditAdvisory> = {};
    let idCounter = 1;

    for (const [moduleName, vuln] of Object.entries(parsed.vulnerabilities)) {
      const viaDetails = vuln.via.find(
        (v): v is { title?: string; url?: string; severity?: string; cwe?: string[] } =>
          typeof v === 'object' && v !== null
      );

      advisories[String(idCounter)] = {
        id: idCounter,
        title: viaDetails?.title || `Vulnerability in ${moduleName}`,
        severity: vuln.severity,
        url: viaDetails?.url || '',
        cves: viaDetails?.cwe || [],
        module_name: moduleName,
        vulnerable_versions: vuln.range,
        patched_versions:
          typeof vuln.fixAvailable === 'object' ? `>=${vuln.fixAvailable.version}` : '',
        recommendation:
          typeof vuln.fixAvailable === 'object'
            ? `Update to ${vuln.fixAvailable.name}@${vuln.fixAvailable.version}`
            : vuln.fixAvailable
              ? 'Run npm audit fix'
              : 'No fix available',
        findings: vuln.nodes.map((node) => ({ version: vuln.range, paths: [node] })),
      };
      idCounter++;
    }

    return {
      advisories,
      metadata: { vulnerabilities: parsed.metadata.vulnerabilities },
    };
  }

  private normaliseYarnAudit(stdout: string): NormalisedAuditResult | null {
    const lines = stdout.trim().split('\n');
    const advisories: Record<string, AuditAdvisory> = {};
    let metadata: VulnerabilityCount = {
      info: 0,
      low: 0,
      moderate: 0,
      high: 0,
      critical: 0,
      total: 0,
    };

    for (const line of lines) {
      if (!line.trim()) continue;

      try {
        const parsed = JSON.parse(line) as YarnClassicAdvisory | YarnClassicSummary;

        if (parsed.type === 'auditAdvisory') {
          const advisory = parsed.data.advisory;
          advisories[String(advisory.id)] = advisory;
        } else if (parsed.type === 'auditSummary') {
          metadata = parsed.data.vulnerabilities;
        }
      } catch {
        continue;
      }
    }

    if (Object.keys(advisories).length === 0 && metadata.total === 0) {
      return null;
    }

    return { advisories, metadata: { vulnerabilities: metadata } };
  }

  private calculateScore(critical: number, high: number, moderate: number, low: number): number {
    // Start with perfect score
    let score = 100;

    // Deduct points based on severity
    score -= critical * 20; // Critical: -20 points each
    score -= high * 10; // High: -10 points each
    score -= moderate * 5; // Moderate: -5 points each
    score -= low * 2; // Low: -2 points each

    return Math.max(0, score);
  }

  private generateFixSuggestions(
    vulnerabilities: AuditAdvisory[],
    packageManager: PackageManager
  ): string[] {
    const suggestions: string[] = [];

    // Group by module
    const moduleMap = new Map<string, AuditAdvisory[]>();
    for (const vuln of vulnerabilities) {
      const existing = moduleMap.get(vuln.module_name) || [];
      existing.push(vuln);
      moduleMap.set(vuln.module_name, existing);
    }

    // Generate suggestions for each module
    for (const [module, vulns] of Array.from(moduleMap.entries())) {
      const highestSeverity = vulns[0].severity; // Already sorted by severity
      const patchedVersions = vulns[0].patched_versions;

      if (patchedVersions && patchedVersions !== '<0.0.0') {
        suggestions.push(
          `Update ${module} to ${patchedVersions} (fixes ${vulns.length} ${highestSeverity} vulnerability(ies))`
        );
      } else {
        suggestions.push(
          `Review ${module}: No patch available for ${vulns.length} ${highestSeverity} vulnerability(ies)`
        );
      }
    }

    // Add package manager-specific fix command
    if (suggestions.length > 0) {
      const fixCommand =
        packageManager === 'yarn'
          ? `${packageManager} audit fix` // yarn uses 'audit fix'
          : `${packageManager} audit fix`; // npm and pnpm both use 'audit fix'
      suggestions.push(`Run \`${fixCommand}\` to automatically apply available patches`);
    }

    return suggestions;
  }
}
