/**
 * Entropy Detector - Shannon entropy-based detection for secrets
 *
 * Detects high-entropy strings that are likely to be secrets or sensitive data.
 */

import { PatternMatcher } from './secret-patterns.js';

/**
 * Secret finding from entropy detection
 */
export interface EntropyFinding {
  file: string;
  line: number;
  type: string;
  match: string;
  context: string;
  entropy: number;
  source: 'entropy';
}

/**
 * Configuration for entropy detection
 */
export interface EntropyDetectorConfig {
  /** Minimum entropy threshold for detection (default: 4.5) */
  entropy_threshold: number;
  /** Minimum string length for entropy analysis (default: 16) */
  min_entropy_length: number;
  /** Patterns to allowlist (reduce false positives) */
  allowlist: string[];
}

/**
 * Entropy detector for high-entropy strings
 */
export class EntropyDetector {
  private matcher = new PatternMatcher();

  /**
   * Calculate Shannon entropy of a string
   * Higher entropy = more randomness = likely a secret
   */
  calculateEntropy(str: string): number {
    if (!str || str.length === 0) return 0;

    const charFrequency: Record<string, number> = {};
    for (const char of str) {
      charFrequency[char] = (charFrequency[char] || 0) + 1;
    }

    let entropy = 0;
    const len = str.length;
    for (const char in charFrequency) {
      const frequency = charFrequency[char] / len;
      entropy -= frequency * Math.log2(frequency);
    }

    return entropy;
  }

  /**
   * Detect high-entropy strings that might be secrets
   */
  detectHighEntropyStrings(
    line: string,
    lineNumber: number,
    file: string,
    config: EntropyDetectorConfig
  ): EntropyFinding[] {
    const findings: EntropyFinding[] = [];
    const threshold = config.entropy_threshold;
    const minLength = config.min_entropy_length;

    // Extract potential secret strings (quoted strings, assignments)
    const stringPatterns = [
      // Quoted strings
      /['"]([^'"]{16,})['"]/,
      // Variable assignments with alphanumeric values
      /[:=]\s*['"]?([a-zA-Z0-9_/+=-]{16,})['"]?/,
    ];

    for (const pattern of stringPatterns) {
      const matches = line.match(pattern);
      if (matches && matches[1]) {
        const candidate = matches[1];

        // Skip if too short or already detected by pattern
        if (candidate.length < minLength) continue;
        if (this.matcher.isAllowlisted(candidate, config.allowlist)) continue;

        // Skip if it looks like code or common patterns
        if (this.matcher.looksLikeCode(candidate)) continue;

        const entropy = this.calculateEntropy(candidate);
        if (entropy >= threshold) {
          findings.push({
            file,
            line: lineNumber,
            type: 'High Entropy String',
            match: this.matcher.redactSecret(candidate),
            context: this.matcher.redactLine(line.trim()),
            entropy: Math.round(entropy * 100) / 100,
            source: 'entropy',
          });
        }
      }
    }

    return findings;
  }
}
