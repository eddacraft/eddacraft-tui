import {
  architectureYamlExists,
  parseArchitectureDefinition,
  mergeWithTemplate,
} from './yaml-parser.js';
import {
  needsRegeneration,
  writeDCConfig,
  dcConfigExists,
  getDCConfigPath,
} from './dc-generator.js';
import {
  needsRegoRegeneration,
  writeRegoPolicy,
  regoExists,
  getRegoPath,
} from './rego-generator.js';
import type { ArchitectureDefinition } from './definition-schema.js';

export interface CompileResult {
  dcConfig: { path: string; regenerated: boolean };
  regoPolicy: { path: string; regenerated: boolean };
  definition: ArchitectureDefinition;
}

export interface CompileOptions {
  force?: boolean;
  skipDC?: boolean;
  skipRego?: boolean;
}

export async function compileArchitecture(
  workspaceRoot: string,
  options: CompileOptions = {}
): Promise<CompileResult> {
  if (!architectureYamlExists(workspaceRoot)) {
    throw new Error('No architecture.yaml found. Run: anvil architecture init');
  }

  const definition = await parseArchitectureDefinition(workspaceRoot);
  const merged = mergeWithTemplate(definition);

  const result: CompileResult = {
    dcConfig: { path: getDCConfigPath(workspaceRoot), regenerated: false },
    regoPolicy: { path: getRegoPath(workspaceRoot), regenerated: false },
    definition: merged,
  };

  if (!options.skipDC) {
    const needsDC =
      options.force ||
      !dcConfigExists(workspaceRoot) ||
      (await needsRegeneration(workspaceRoot, merged));
    if (needsDC) {
      result.dcConfig.path = await writeDCConfig(workspaceRoot, merged);
      result.dcConfig.regenerated = true;
    }
  }

  if (!options.skipRego) {
    const needsRego =
      options.force ||
      !regoExists(workspaceRoot) ||
      (await needsRegoRegeneration(workspaceRoot, merged));
    if (needsRego) {
      result.regoPolicy.path = await writeRegoPolicy(workspaceRoot, merged);
      result.regoPolicy.regenerated = true;
    }
  }

  return result;
}

export async function needsCompilation(workspaceRoot: string): Promise<{
  dc: boolean;
  rego: boolean;
  any: boolean;
}> {
  if (!architectureYamlExists(workspaceRoot)) {
    return { dc: false, rego: false, any: false };
  }

  const definition = await parseArchitectureDefinition(workspaceRoot);
  const merged = mergeWithTemplate(definition);

  const dc = !dcConfigExists(workspaceRoot) || (await needsRegeneration(workspaceRoot, merged));
  const rego = !regoExists(workspaceRoot) || (await needsRegoRegeneration(workspaceRoot, merged));

  return { dc, rego, any: dc || rego };
}
