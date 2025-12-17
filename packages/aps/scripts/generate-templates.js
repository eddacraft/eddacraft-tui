#!/usr/bin/env node
/**
 * Generate APS planning document templates
 *
 * Usage: node generate-templates.js [output-dir]
 */

import { writeFileSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { generateAllTemplates } from '../dist/templates/generator.js';

const OUTPUT_DIR = process.argv[2] || './templates';

try {
  // Create output directory
  mkdirSync(OUTPUT_DIR, { recursive: true });

  // Generate all templates
  const templates = generateAllTemplates();

  // Write each template to a file
  writeFileSync(join(OUTPUT_DIR, 'index-template.md'), templates.index, 'utf8');
  writeFileSync(join(OUTPUT_DIR, 'leaf-template.md'), templates.leaf, 'utf8');
  writeFileSync(join(OUTPUT_DIR, 'simple-template.md'), templates.simple, 'utf8');

  console.log(`✓ Generated templates in ${OUTPUT_DIR}/`);
  console.log('  - index-template.md');
  console.log('  - leaf-template.md');
  console.log('  - simple-template.md');
} catch (error) {
  console.error('✗ Failed to generate templates:', error);
  process.exit(1);
}
