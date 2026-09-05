import { describe, expect, it } from 'vitest';

import { featureFlagManifest, productCatalogue } from '../src/index.js';
import { listedImpliesOnViolations } from '../src/listed.js';

describe('FLAGCAT-019 listed implies on', () => {
  const catalogue = productCatalogue();
  const flags = featureFlagManifest().flags;

  it('defaults omitted listed to true', () => {
    const check = catalogue.deliverySurfaces.find((surface) => surface.key === 'cli.check');
    expect(check?.listed).toBe(true);
  });

  it('hides CLI-only default-off invocation surfaces', () => {
    expect(
      catalogue.deliverySurfaces
        .filter((surface) => surface.listed === false)
        .map((surface) => surface.key)
        .sort()
    ).toEqual(['cli.dashboard-web', 'cli.ember', 'cli.impact', 'cli.plan-dashboard']);
  });

  it('rejects listed CLI surfaces whose invocation is default-off', () => {
    expect(listedImpliesOnViolations(catalogue, flags)).toEqual([]);
  });

  it('keeps MCP impact-of-change listed', () => {
    const mcp = catalogue.deliverySurfaces.find(
      (surface) => surface.key === 'mcp-tool.impact-of-change'
    );
    expect(mcp?.listed).toBe(true);
  });
});
