const DEFAULT_NEXT = '/anvil/overview';

export function validateNext(next: string | null | undefined): string {
  if (!next) return DEFAULT_NEXT;
  if (!next.startsWith('/')) return DEFAULT_NEXT;
  if (next.startsWith('//')) return DEFAULT_NEXT;
  try {
    const resolved = new URL(next, 'https://placeholder.invalid').pathname;
    if (resolved !== '/anvil' && !resolved.startsWith('/anvil/')) return DEFAULT_NEXT;
    return resolved;
  } catch {
    return DEFAULT_NEXT;
  }
}
