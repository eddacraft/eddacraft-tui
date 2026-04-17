import pc from 'picocolors';

export interface OutputOptions {
  json?: boolean;
  quiet?: boolean;
  colour?: boolean;
}

export type Row = Record<string, unknown>;

export interface Column {
  key: string;
  header?: string;
  format?: (value: unknown, row: Row) => string;
}

function normaliseCell(value: unknown): string {
  if (value === null || value === undefined) return '';
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  if (value instanceof Date) return value.toISOString();
  return JSON.stringify(value);
}

// Collapse control characters (newlines, tabs, etc.) to a single space so
// user-controlled fields (notes, JSON blobs) cannot corrupt the table layout.
function sanitiseCell(value: string): string {
  // eslint-disable-next-line no-control-regex
  return value.replace(/[\x00-\x1F\x7F]+/g, ' ').trim();
}

export function renderTable(rows: Row[], columns: Column[]): string {
  if (rows.length === 0) return '';

  const cells = rows.map((row) =>
    columns.map((col) => {
      const raw = row[col.key];
      const formatted = col.format ? col.format(raw, row) : normaliseCell(raw);
      return sanitiseCell(formatted);
    })
  );

  const widths = columns.map((col, idx) => {
    const header = col.header ?? col.key;
    const headerWidth = header.length;
    const cellWidth = cells.reduce((max, row) => Math.max(max, row[idx]!.length), 0);
    return Math.max(headerWidth, cellWidth);
  });

  const header = columns.map((col, idx) => (col.header ?? col.key).padEnd(widths[idx]!)).join('  ');
  const divider = widths.map((w) => '-'.repeat(w)).join('  ');
  const body = cells
    .map((row) => row.map((cell, idx) => cell.padEnd(widths[idx]!)).join('  '))
    .join('\n');

  return [header, divider, body].join('\n');
}

export function formatJson(data: unknown): string {
  return JSON.stringify(data, null, 2);
}

export function shouldUseColour(
  options: OutputOptions = {},
  tty: boolean = !!process.stdout.isTTY
): boolean {
  if (options.colour === false) return false;
  if (options.json) return false;
  if (options.quiet) return false;
  if (process.env.NO_COLOR !== undefined) return false;
  return tty;
}

export function formatError(message: string, options: OutputOptions = {}): string {
  if (shouldUseColour(options, !!process.stderr.isTTY)) {
    return `${pc.red('error:')} ${message}`;
  }
  return `error: ${message}`;
}

export function formatSuccess(message: string, options: OutputOptions = {}): string {
  if (shouldUseColour(options)) {
    return `${pc.green('✓')} ${message}`;
  }
  return message;
}
