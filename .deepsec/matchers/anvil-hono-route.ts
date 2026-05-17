import type { CandidateMatch, MatcherPlugin } from 'deepsec/config';

const ROUTE_RE = /\b\w+\s*\.\s*(?:get|post|put|delete|patch|route|use)\s*\(\s*['"/]/;
const HONO_RE = /\bnew\s+Hono\s*\(/;

export const anvilHonoRoute: MatcherPlugin = {
  slug: 'anvil-hono-route',
  description: 'Anvil Hono API route or router mount entry point',
  noiseTier: 'noisy',
  filePatterns: ['apps/anvil-api/src/index.ts', 'apps/anvil-api/src/routes/**/*.ts'],
  match(content, filePath): CandidateMatch[] {
    if (/\b__tests__\b|\.(test|spec)\.ts$/.test(filePath)) return [];

    const lines = content.split('\n');
    const matches: CandidateMatch[] = [];

    for (let i = 0; i < lines.length; i++) {
      if (!HONO_RE.test(lines[i]) && !ROUTE_RE.test(lines[i])) continue;
      const start = Math.max(0, i - 1);
      const end = Math.min(lines.length, i + 5);
      matches.push({
        vulnSlug: 'anvil-hono-route',
        lineNumbers: [i + 1],
        snippet: lines.slice(start, end).join('\n'),
        matchedPattern: 'Hono route, middleware, or router mount',
      });
    }

    return matches;
  },
};
