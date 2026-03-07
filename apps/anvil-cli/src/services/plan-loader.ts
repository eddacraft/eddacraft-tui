/**
 * Plan loader service that supports both APS and external formats
 * @module cli/services/plan-loader
 */

import { readFile } from 'node:fs/promises';
import { extname } from 'node:path';
import YAML from 'yaml';
import { APSValidator } from '@eddacraft/anvil-core';
import type { APSPlan } from '@eddacraft/anvil-core';
import { AdapterRegistry } from '@eddacraft/anvil-adapters';
import { FormatDetectionService } from './format-detection.js';
import { debug } from '../utils/output.js';
import type {
  PlanLoaderService,
  LoadPlanOptions,
  LoadPlanResult,
  PlanLoadError,
} from '../types/services.js';

/**
 * Implementation of plan loader service
 */
export class PlanLoader implements PlanLoaderService {
  private formatDetection: FormatDetectionService;
  private validator: APSValidator;
  private registry: AdapterRegistry;

  constructor(options?: { minConfidence?: number }) {
    this.formatDetection = new FormatDetectionService(options);
    this.validator = new APSValidator();
    this.registry = AdapterRegistry.getInstance();
  }

  /**
   * Load plan from file (APS or external format)
   * @param filePath - Path to plan file
   * @param options - Loading options
   * @returns Loaded plan with metadata
   */
  async loadPlan(filePath: string, options?: LoadPlanOptions): Promise<LoadPlanResult> {
    try {
      const content = await readFile(filePath, 'utf-8');
      return this.loadPlanFromContent(content, {
        ...options,
        filePath,
      });
    } catch (error) {
      const err = error as NodeJS.ErrnoException;
      throw new PlanLoadErrorImpl(`Failed to load plan: ${err.message}`, filePath, err);
    }
  }

  /**
   * Load plan from content string
   * @param content - Plan content
   * @param options - Loading options
   * @returns Loaded plan with metadata
   */
  async loadPlanFromContent(
    content: string,
    options?: LoadPlanOptions & { filePath?: string }
  ): Promise<LoadPlanResult> {
    const filePath = options?.filePath;

    // Check if user explicitly requested native APS format
    if (options?.format === 'aps' || this.isNativeAPS(content, filePath)) {
      return this.loadAPSPlan(content, options);
    }

    // Detect format using adapter registry
    const detection = options?.format
      ? await this.detectSpecificFormat(content, options.format, filePath)
      : await this.formatDetection.detectFormat(content, filePath);

    if (!detection) {
      throw new PlanLoadErrorImpl(
        'Unable to detect plan format. Use --format to specify explicitly.',
        filePath
      );
    }

    // Parse using detected adapter
    const parseContext = {
      repositoryPath: process.cwd(),
      timestamp: new Date().toISOString(),
    };

    const parseResult = await detection.adapter.parse(
      content,
      parseContext,
      options?.adapterOptions
    );

    if (!parseResult.success || !parseResult.data) {
      const errorMessages =
        parseResult.errors?.map((e: { message: string }) => e.message).join(', ') ||
        'Unknown parse error';
      throw new PlanLoadErrorImpl(
        `Failed to parse ${detection.format} format: ${errorMessages}`,
        filePath
      );
    }

    // Validate parsed plan
    const validationResult = await this.validator.validate(parseResult.data, {
      strict: options?.strict ?? false,
      validateHash: options?.validateHash ?? false,
    });

    return {
      plan: parseResult.data,
      validation: validationResult,
      sourceFormat: {
        format: detection.format,
        adapter: detection.adapter.metadata.name,
        confidence: detection.detection.confidence,
        filePath,
      },
      warnings: parseResult.warnings,
    };
  }

  /**
   * Load native APS plan (JSON or YAML)
   */
  private async loadAPSPlan(content: string, options?: LoadPlanOptions): Promise<LoadPlanResult> {
    let data: unknown;

    try {
      // Try parsing as JSON first
      data = JSON.parse(content);
    } catch {
      try {
        // Fall back to YAML
        data = YAML.parse(content);
      } catch {
        debug('loadAPSPlan: JSON parse failed, trying YAML');
        throw new PlanLoadErrorImpl('Invalid APS format: must be valid JSON or YAML');
      }
    }

    // Validate APS schema
    const validationResult = await this.validator.validate(data as APSPlan, {
      strict: options?.strict ?? false,
      validateHash: options?.validateHash ?? false,
    });

    if (!validationResult.valid) {
      const errorMessages = validationResult.issues
        ?.map((e: { message: string }) => e.message)
        .join(', ');
      throw new PlanLoadErrorImpl(`Invalid APS plan: ${errorMessages}`);
    }

    return {
      plan: validationResult.data!,
      validation: validationResult,
    };
  }

  /**
   * Check if content is native APS format (JSON/YAML)
   */
  private isNativeAPS(content: string, filePath?: string): boolean {
    // Check file extension
    if (filePath) {
      const ext = extname(filePath).toLowerCase();
      if (ext === '.json' || ext === '.yaml' || ext === '.yml') {
        return true;
      }
    }

    // Check if content starts with JSON object/array
    const trimmed = content.trim();
    if (trimmed.startsWith('{') || trimmed.startsWith('[')) {
      try {
        JSON.parse(content);
        return true;
      } catch {
        debug('isNativeAPS: JSON parse failed, not native APS');
      }
    }

    return false;
  }

  /**
   * Detect a specific format by name
   */
  private async detectSpecificFormat(content: string, formatName: string, filePath?: string) {
    const adapter = this.getAdapterByFormat(formatName);
    if (!adapter) {
      throw new PlanLoadErrorImpl(`Unknown format: ${formatName}`, filePath);
    }

    const detection = adapter.detect(content);
    if (!detection.detected) {
      throw new PlanLoadErrorImpl(`File does not match ${formatName} format`, filePath);
    }

    return {
      format: formatName,
      adapter,
      detection,
      filePath,
    };
  }

  /**
   * Get adapter by format name
   */
  private getAdapterByFormat(formatName: string) {
    const adapters = this.registry.listAdapters();
    return adapters.find(
      (a: { metadata: { name: string } }) =>
        a.metadata.name.toLowerCase() === formatName.toLowerCase()
    );
  }
}

/**
 * Error implementation for plan loading failures
 */
class PlanLoadErrorImpl extends Error implements PlanLoadError {
  public readonly filePath?: string;
  public override readonly cause?: Error;

  constructor(message: string, filePath?: string, cause?: Error) {
    super(message);
    this.name = 'PlanLoadError';
    this.filePath = filePath;
    this.cause = cause;
    Error.captureStackTrace?.(this, PlanLoadErrorImpl);
  }
}
