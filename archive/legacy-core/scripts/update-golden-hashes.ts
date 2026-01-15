/**
 * Script to update golden file hashes
 * Run with: npx tsx core/scripts/update-golden-hashes.ts
 */

import { readFile, writeFile } from 'fs/promises';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { generateHash } from '../src/crypto/hash.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const FIXTURES_DIR = join(__dirname, '..', 'src', '__fixtures__', 'golden-plans');

async function updateGoldenHashes() {
  const files = ['simple-plan.json', 'complex-plan.json', 'minimal-plan.json'];

  for (const file of files) {
    const path = join(FIXTURES_DIR, file);
    const content = await readFile(path, 'utf8');
    const plan = JSON.parse(content);

    // Remove existing hash
    const { hash: _, ...planWithoutHash } = plan;

    // Generate new hash
    const newHash = generateHash(planWithoutHash);

    // Update plan with new hash
    const updatedPlan = {
      ...plan,
      hash: newHash,
    };

    // Write back
    await writeFile(path, JSON.stringify(updatedPlan, null, 2) + '\n');
    console.log(`Updated ${file} with hash: ${newHash}`);
  }
}

updateGoldenHashes().catch(console.error);
