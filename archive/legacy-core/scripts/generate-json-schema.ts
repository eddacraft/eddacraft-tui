#!/usr/bin/env node

/**
 * Script to generate JSON Schema from Zod schema
 * This creates the aps.schema.json file that can be used by external tools
 */

import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';
import { getJSONSchemaString } from '../src/schema/json-schema.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const outputPath = join(__dirname, '..', 'src', 'schema', 'aps.schema.json');

try {
  const jsonSchema = getJSONSchemaString(true);
  writeFileSync(outputPath, jsonSchema, 'utf-8');
  console.log(`✅ JSON Schema generated successfully at: ${outputPath}`);
  console.log(`   Schema version: ${JSON.parse(jsonSchema).version}`);
} catch (error) {
  console.error('❌ Failed to generate JSON Schema:', error);
  process.exit(1);
}
