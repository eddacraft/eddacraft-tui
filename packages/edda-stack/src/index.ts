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

// Re-export all contracts
export * from './contracts/index.js';

// Re-export stack configuration
export * from './config.js';

// Package metadata
export const PACKAGE_VERSION = '0.1.0';
export const PACKAGE_NAME = '@eddacraft/anvil-edda-stack';
