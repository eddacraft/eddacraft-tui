import chalk from 'chalk';
import { describe, expect, it } from 'vitest';

import { colourConfidence, colourStatus } from './utils.js';

describe('colourConfidence', () => {
  it('returns green for confidence above 0.7', () => {
    expect(colourConfidence(0.85)).toBe(chalk.green('0.85'));
  });

  it('returns yellow for confidence between 0.4 and 0.7', () => {
    expect(colourConfidence(0.5)).toBe(chalk.yellow('0.50'));
  });

  it('returns red for confidence below 0.4', () => {
    expect(colourConfidence(0.2)).toBe(chalk.red('0.20'));
  });

  it('returns yellow at exactly 0.4 (lower boundary)', () => {
    expect(colourConfidence(0.4)).toBe(chalk.yellow('0.40'));
  });

  it('returns yellow at exactly 0.7 (upper boundary)', () => {
    expect(colourConfidence(0.7)).toBe(chalk.yellow('0.70'));
  });

  it('returns green just above 0.7', () => {
    expect(colourConfidence(0.71)).toBe(chalk.green('0.71'));
  });

  it('returns red just below 0.4', () => {
    expect(colourConfidence(0.39)).toBe(chalk.red('0.39'));
  });
});

describe('colourStatus', () => {
  it('returns cyan for active', () => {
    expect(colourStatus('active')).toBe(chalk.cyan('active'));
  });

  it('returns green for promoted', () => {
    expect(colourStatus('promoted')).toBe(chalk.green('promoted'));
  });

  it('returns yellow for dismissed', () => {
    expect(colourStatus('dismissed')).toBe(chalk.yellow('dismissed'));
  });

  it('returns red for expired', () => {
    expect(colourStatus('expired')).toBe(chalk.red('expired'));
  });

  it('returns uncoloured string for unknown status', () => {
    expect(colourStatus('unknown')).toBe('unknown');
  });
});
