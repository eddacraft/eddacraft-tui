import type { APSPlan, ValidationResult } from '@eddacraft/anvil-core';

export interface SpecContext {
  repositoryPath?: string;
  branch?: string;
  commit?: string;
  author?: string;
  timestamp?: string;
  additionalContext?: Record<string, unknown>;
}

export interface ExternalSpec {
  format: string;
  version: string;
  content: unknown;
  metadata?: Record<string, unknown>;
}

export interface ConversionResult<T = unknown> {
  success: boolean;
  data?: T;
  errors?: ConversionError[];
  warnings?: ConversionWarning[];
}

export interface ConversionError {
  code: string;
  message: string;
  path?: string;
  line?: number;
  column?: number;
  details?: unknown;
}

export interface ConversionWarning {
  code: string;
  message: string;
  path?: string;
  line?: number;
  column?: number;
  details?: unknown;
}

export interface SpecToolAdapter {
  readonly name: string;
  readonly version: string;
  readonly supportedFormats: readonly string[];

  generateSpec(intent: string, context: SpecContext): Promise<APSPlan>;
  validateSpec(spec: APSPlan): Promise<ValidationResult>;
  convertToAPS(spec: ExternalSpec): Promise<ConversionResult<APSPlan>>;
  convertFromAPS(spec: APSPlan): Promise<ConversionResult<ExternalSpec>>;

  canImport(format: string): boolean;
  canExport(format: string): boolean;
}

export interface AdapterConfig {
  preserveComments?: boolean;
  preserveMetadata?: boolean;
  strictMode?: boolean;
  formatOptions?: Record<string, unknown>;
}

export abstract class BaseAdapter implements SpecToolAdapter {
  abstract readonly name: string;
  abstract readonly version: string;
  abstract readonly supportedFormats: readonly string[];

  constructor(protected config: AdapterConfig = {}) {}

  abstract generateSpec(intent: string, context: SpecContext): Promise<APSPlan>;
  abstract validateSpec(spec: APSPlan): Promise<ValidationResult>;
  abstract convertToAPS(spec: ExternalSpec): Promise<ConversionResult<APSPlan>>;
  abstract convertFromAPS(spec: APSPlan): Promise<ConversionResult<ExternalSpec>>;

  canImport(format: string): boolean {
    return this.supportedFormats.includes(format);
  }

  canExport(format: string): boolean {
    return this.supportedFormats.includes(format);
  }
}
