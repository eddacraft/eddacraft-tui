/**
 * Edda Stack
 *
 * The Kindling · Ember · Edda memory architecture for Anvil.
 *
 * This package provides:
 * - Shared contracts for all three layers
 * - Type definitions and schemas
 * - Port interfaces for layer abstractions
 * - Utilities for cross-layer operations
 *
 * @module @eddacraft/anvil-edda-stack
 */

import { readFileSync } from 'node:fs';

// Re-export all contracts
export * from './contracts/index.js';

// Re-export stack configuration
export * from './config.js';

// Re-export Ember service layer
export * from './ember/index.js';

// Re-export Edda service layer
export * from './edda/index.js';

// Package metadata, read from package.json so the exported version never drifts
// from the manifest the release tooling bumps. Resolves relative to this module,
// which works identically from `dist/` (built) and `src/` (tests) since both sit
// one level under the package root.
const packageManifest = JSON.parse(
  readFileSync(new URL('../package.json', import.meta.url), 'utf8')
) as { name: string; version: string };

export const PACKAGE_VERSION: string = packageManifest.version;
export const PACKAGE_NAME: string = packageManifest.name;
