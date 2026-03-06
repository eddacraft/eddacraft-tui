import type { MemoryObject } from '../../contracts/index.js';
import { MEMORY_SCHEMA_VERSION, MemoryObjectSchema } from '../../contracts/index.js';

export interface MigrationStep {
  fromVersion: number;
  toVersion: number;
  migrate(data: unknown): unknown;
  description: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function migrateV0ToV1(data: unknown): unknown {
  if (!isRecord(data)) {
    throw new Error('Migration from v0 to v1 requires an object payload.');
  }

  const migrated: Record<string, unknown> = {
    ...data,
    schema_version: 1,
  };

  if (!('status' in migrated)) {
    migrated.status = 'active';
  }

  if (!('evolution' in migrated)) {
    migrated.evolution = { supersedes: [] };
  }

  if (!('confidence_rationale' in migrated)) {
    migrated.confidence_rationale = undefined;
  }

  return migrated;
}

export const migrationRegistry: ReadonlyArray<MigrationStep> = [
  {
    fromVersion: 0,
    toVersion: 1,
    migrate: migrateV0ToV1,
    description: 'Add schema version and v1 default memory fields.',
  },
];

function getStepForVersion(fromVersion: number, targetVersion: number): MigrationStep | undefined {
  const candidates = migrationRegistry
    .filter((step) => step.fromVersion === fromVersion && step.toVersion <= targetVersion)
    .sort((a, b) => a.toVersion - b.toVersion);

  return candidates[0];
}

export function detectMemorySchemaVersion(data: unknown): number {
  if (!isRecord(data)) {
    return 0;
  }

  const { schema_version: schemaVersion } = data;

  if (typeof schemaVersion === 'number' && Number.isInteger(schemaVersion) && schemaVersion >= 0) {
    return schemaVersion;
  }

  return 0;
}

export function getCurrentSchemaVersion(): number {
  return MEMORY_SCHEMA_VERSION;
}

export function getMigrationChain(from: number, to: number): MigrationStep[] {
  if (!Number.isInteger(from) || from < 0) {
    throw new Error(`Invalid source schema version: ${from}.`);
  }

  if (!Number.isInteger(to) || to < 0) {
    throw new Error(`Invalid target schema version: ${to}.`);
  }

  if (from > to) {
    throw new Error(`Schema downgrades are not supported: ${from} -> ${to}.`);
  }

  if (from === to) {
    return [];
  }

  const chain: MigrationStep[] = [];
  let currentVersion = from;

  while (currentVersion < to) {
    const step = getStepForVersion(currentVersion, to);

    if (!step) {
      throw new Error(`Missing migration step from schema version ${currentVersion} to ${to}.`);
    }

    chain.push(step);
    currentVersion = step.toVersion;
  }

  return chain;
}

export function migrateMemory(
  data: unknown,
  fromVersion: number,
  targetVersion: number
): MemoryObject {
  const chain = getMigrationChain(fromVersion, targetVersion);

  const migrated = chain.reduce<unknown>((current, step) => step.migrate(current), data);

  const validationResult = MemoryObjectSchema.safeParse(migrated);

  if (!validationResult.success) {
    throw new Error(`Migrated memory failed schema validation: ${validationResult.error.message}`);
  }

  return validationResult.data;
}
