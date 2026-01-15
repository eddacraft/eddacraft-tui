/**
 * Circular Detector - Detect and categorize circular dependencies
 *
 * Handles detection and categorization of various violation types from dependency-cruiser.
 */

import type { CruiserViolation } from './dependency-analyzer.js';
import type { ArchitectureBaseline } from '@anvil/core/architecture';

/**
 * Categorized violation types
 */
export type ViolationType = 'circular' | 'orphan' | 'layer' | 'other';

/**
 * Circular detector for analyzing violations
 */
export class CircularDetector {
  /**
   * Categorize a violation by type
   */
  categoriseViolation(violation: CruiserViolation): ViolationType {
    if (violation.cycle && violation.cycle.length > 0) {
      return 'circular';
    }
    if (violation.rule.name.includes('orphan')) {
      return 'orphan';
    }
    if (violation.rule.name.includes('layer') || violation.rule.name.includes('boundary')) {
      return 'layer';
    }
    return 'other';
  }

  /**
   * Check if a violation is new compared to baseline
   */
  isNewViolation(violation: CruiserViolation, baseline: ArchitectureBaseline): boolean {
    const baselineViolations = baseline.baseline_snapshot.violations;
    return !baselineViolations.some((bv) => {
      const filesMatch = bv.from_file === violation.from && bv.to_file === violation.to;
      if (!filesMatch) return false;
      if (bv.rule) {
        return bv.rule === violation.rule.name;
      }
      return true;
    });
  }

  /**
   * Count violations by type
   */
  countViolationsByType(violations: CruiserViolation[]): Record<string, number> {
    const violationsByType: Record<string, number> = {
      circular: 0,
      orphan: 0,
      layer: 0,
      other: 0,
    };

    for (const v of violations) {
      const type = this.categoriseViolation(v);
      violationsByType[type]++;
    }

    return violationsByType;
  }

  /**
   * Filter violations to only new ones based on baseline
   */
  filterNewViolations(
    violations: CruiserViolation[],
    baseline: ArchitectureBaseline | null
  ): CruiserViolation[] {
    if (!baseline) {
      return violations;
    }

    return violations.filter((v) => this.isNewViolation(v, baseline));
  }
}
