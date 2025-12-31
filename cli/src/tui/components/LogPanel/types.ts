export type LogLevel = 'error' | 'warn' | 'info' | 'debug';

export interface LogEntry {
  id: string;
  timestamp: Date;
  level: LogLevel;
  message: string;
  source?: string;
}

export interface LogFilter {
  levels: Set<LogLevel>;
  search: string;
}

export const DEFAULT_LOG_FILTER: LogFilter = {
  levels: new Set(['error', 'warn', 'info', 'debug']),
  search: '',
};

export const LOG_LEVEL_PRIORITY: Record<LogLevel, number> = {
  error: 0,
  warn: 1,
  info: 2,
  debug: 3,
};

export function createLogEntry(level: LogLevel, message: string, source?: string): LogEntry {
  return {
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`,
    timestamp: new Date(),
    level,
    message,
    source,
  };
}

export function filterEntries(entries: LogEntry[], filter: LogFilter): LogEntry[] {
  return entries.filter((entry) => {
    if (!filter.levels.has(entry.level)) {
      return false;
    }

    if (filter.search) {
      const searchLower = filter.search.toLowerCase();
      const messageMatch = entry.message.toLowerCase().includes(searchLower);
      const sourceMatch = entry.source?.toLowerCase().includes(searchLower) ?? false;
      if (!messageMatch && !sourceMatch) {
        return false;
      }
    }

    return true;
  });
}

export function formatTimestamp(date: Date): string {
  const hours = date.getHours().toString().padStart(2, '0');
  const minutes = date.getMinutes().toString().padStart(2, '0');
  const seconds = date.getSeconds().toString().padStart(2, '0');
  return `${hours}:${minutes}:${seconds}`;
}
