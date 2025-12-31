export type CheckStatus = 'pending' | 'running' | 'passed' | 'failed' | 'skipped' | 'cached';

export interface CheckProgress {
  id: string;
  name: string;
  status: CheckStatus;
  progress: number;
  startTime?: Date;
  endTime?: Date;
  durationMs?: number;
  message?: string;
  cached?: boolean;
}

export interface ParallelProgressState {
  checks: CheckProgress[];
  startTime: Date;
  overallProgress: number;
  eta?: number;
}

export function calculateOverallProgress(checks: CheckProgress[]): number {
  if (checks.length === 0) return 0;

  const totalProgress = checks.reduce((sum, check) => {
    if (check.status === 'cached') return sum + 100;
    if (check.status === 'passed' || check.status === 'failed') return sum + 100;
    if (check.status === 'skipped') return sum + 100;
    return sum + check.progress;
  }, 0);

  return Math.round(totalProgress / checks.length);
}

export function calculateETA(checks: CheckProgress[]): number | undefined {
  const completed = checks.filter(
    (c) => c.status === 'passed' || c.status === 'failed' || c.status === 'cached'
  );
  const running = checks.filter((c) => c.status === 'running');

  const timedChecks = completed.filter((c) => c.durationMs !== undefined && c.durationMs > 0);
  if (timedChecks.length === 0) return undefined;

  const avgDuration =
    timedChecks.reduce((sum, c) => sum + (c.durationMs ?? 0), 0) / timedChecks.length;

  const remaining = checks.filter((c) => c.status === 'pending' || c.status === 'running');

  const runningPartial = running.reduce((sum, c) => {
    const remainingPercent = (100 - c.progress) / 100;
    return sum + avgDuration * remainingPercent;
  }, 0);

  const pendingTime = (remaining.length - running.length) * avgDuration;

  return Math.round(runningPartial + pendingTime);
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const seconds = ms / 1000;
  if (seconds < 60) return `${seconds.toFixed(1)}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = Math.round(seconds % 60);
  return `${minutes}m ${remainingSeconds}s`;
}

export function getStatusColour(status: CheckStatus): string {
  switch (status) {
    case 'passed':
      return '#64748b';
    case 'failed':
      return '#dc2626';
    case 'running':
      return '#f97316';
    case 'pending':
      return '#475569';
    case 'skipped':
      return '#94a3b8';
    case 'cached':
      return '#22c55e';
  }
}

export function getStatusIcon(status: CheckStatus): string {
  switch (status) {
    case 'passed':
      return '◆';
    case 'failed':
      return '✖';
    case 'running':
      return '●';
    case 'pending':
      return '○';
    case 'skipped':
      return '○';
    case 'cached':
      return '⚡';
  }
}
