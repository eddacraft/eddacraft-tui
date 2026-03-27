export type CheckResultStatus = 'passed' | 'failed' | 'skipped' | 'warning';

export interface CheckResult {
  id: string;
  name: string;
  status: CheckResultStatus;
  score: number;
  message: string;
  details?: string[];
  duration?: number;
  category?: string;
}

export interface GateResult {
  planId: string;
  planPath?: string;
  overall: boolean;
  score: number;
  checks: CheckResult[];
  duration: number;
  timestamp: Date;
}

export type FilterStatus = 'all' | 'passed' | 'failed' | 'skipped';

export interface GateExplorerState {
  selectedIndex: number;
  expandedChecks: Set<string>;
  filterStatus: FilterStatus;
  searchTerm: string;
}

export function getFilteredChecks(
  checks: CheckResult[],
  filter: FilterStatus,
  searchTerm: string
): CheckResult[] {
  let filtered = checks;

  if (filter !== 'all') {
    filtered = filtered.filter((c) => {
      if (filter === 'failed') return c.status === 'failed' || c.status === 'warning';
      return c.status === filter;
    });
  }

  if (searchTerm) {
    const lower = searchTerm.toLowerCase();
    filtered = filtered.filter(
      (c) =>
        c.name.toLowerCase().includes(lower) ||
        c.message.toLowerCase().includes(lower) ||
        c.category?.toLowerCase().includes(lower)
    );
  }

  return filtered;
}

export function getFailedCheckIndices(checks: CheckResult[]): number[] {
  return checks
    .map((c, idx) => ({ check: c, idx }))
    .filter(({ check }) => check.status === 'failed' || check.status === 'warning')
    .map(({ idx }) => idx);
}

export function getStatusIcon(status: CheckResultStatus): string {
  switch (status) {
    case 'passed':
      return '◆';
    case 'failed':
      return '✖';
    case 'skipped':
      return '○';
    case 'warning':
      return '◈';
  }
}

export function getStatusColour(status: CheckResultStatus): string {
  switch (status) {
    case 'passed':
      return '#64748b';
    case 'failed':
      return '#dc2626';
    case 'skipped':
      return '#94a3b8';
    case 'warning':
      return '#fbbf24';
  }
}

export function formatScore(score: number): string {
  return `${Math.round(score)}%`;
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}
