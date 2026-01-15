import { z } from 'zod';

/**
 * AI Tool detection - what generated this code?
 */
export const AIToolSchema = z.object({
  name: z
    .enum([
      'cursor',
      'copilot',
      'claude-code',
      'chatgpt',
      'codewhisperer',
      'tabnine',
      'unknown',
      'manual',
    ])
    .describe('Detected AI coding tool'),
  version: z.string().optional().describe('Version of the AI tool if detectable'),
  confidence: z
    .enum(['high', 'medium', 'low', 'inferred'])
    .describe('Confidence level of detection'),
  indicators: z.array(z.string()).optional().describe('What indicated this tool was used'),
});

/**
 * Environment context - where was this check run?
 */
export const EnvironmentSchema = z.object({
  os: z.string().describe('Operating system'),
  node_version: z.string().describe('Node.js version'),
  anvil_version: z.string().describe('Anvil CLI version'),
  cwd: z.string().describe('Current working directory'),
  ci: z.boolean().describe('Whether running in CI environment'),
  ci_provider: z.string().optional().describe('CI provider name if detected'),
});

/**
 * Git context - what was the repo state?
 */
export const GitContextSchema = z.object({
  repository: z.string().optional().describe('Repository URL or name'),
  branch: z.string().optional().describe('Current branch'),
  commit: z.string().optional().describe('Current commit SHA'),
  commit_message: z.string().optional().describe('Current commit message'),
  author: z.string().optional().describe('Commit author'),
  dirty: z.boolean().describe('Whether there are uncommitted changes'),
  staged_files: z.array(z.string()).optional().describe('Files staged for commit'),
  modified_files: z.array(z.string()).optional().describe('Modified files'),
});

/**
 * Check result summary for provenance
 */
export const CheckSummarySchema = z.object({
  name: z.string().describe('Check name'),
  passed: z.boolean().describe('Whether the check passed'),
  score: z.number().optional().describe('Score if applicable'),
  issues_count: z.number().optional().describe('Number of issues found'),
  duration_ms: z.number().optional().describe('How long the check took'),
});

/**
 * Full provenance record for a check run
 */
export const ProvenanceRecordSchema = z.object({
  // Identification
  id: z.string().describe('Unique provenance record ID'),
  timestamp: z.string().datetime().describe('When the check was run'),

  // What was checked
  scope: z.enum(['directory', 'staged', 'files', 'plan']).describe('What scope was checked'),
  files_checked: z.array(z.string()).describe('List of files that were checked'),
  files_count: z.number().describe('Total number of files checked'),

  // Results
  overall_passed: z.boolean().describe('Whether all checks passed'),
  overall_score: z.number().describe('Overall score (0-100)'),
  checks: z.array(CheckSummarySchema).describe('Summary of each check'),

  // Context
  environment: EnvironmentSchema.describe('Environment information'),
  git: GitContextSchema.optional().describe('Git repository context'),
  ai_tool: AIToolSchema.optional().describe('Detected AI tool'),

  // Linking
  plan_id: z.string().optional().describe('Associated plan ID if applicable'),
  parent_id: z.string().optional().describe('Previous provenance record ID'),

  // Metadata
  trigger: z
    .enum(['manual', 'pre-commit', 'ci', 'watch', 'api'])
    .describe('What triggered this check'),
  duration_ms: z.number().describe('Total duration of the check run'),
  user: z.string().optional().describe('User who ran the check'),
});

/**
 * Provenance index for quick lookups
 */
export const ProvenanceIndexSchema = z.object({
  version: z.number().describe('Index format version'),
  last_updated: z.string().datetime().describe('When the index was last updated'),
  records: z
    .array(
      z.object({
        id: z.string(),
        timestamp: z.string(),
        passed: z.boolean(),
        scope: z.string(),
        files_count: z.number(),
        commit: z.string().optional(),
      })
    )
    .describe('Summary of all provenance records'),
  statistics: z
    .object({
      total_checks: z.number(),
      total_passed: z.number(),
      total_failed: z.number(),
      last_pass: z.string().optional(),
      last_fail: z.string().optional(),
    })
    .describe('Aggregate statistics'),
});

// Type exports
export type AITool = z.infer<typeof AIToolSchema>;
export type Environment = z.infer<typeof EnvironmentSchema>;
export type GitContext = z.infer<typeof GitContextSchema>;
export type CheckSummary = z.infer<typeof CheckSummarySchema>;
export type ProvenanceRecord = z.infer<typeof ProvenanceRecordSchema>;
export type ProvenanceIndex = z.infer<typeof ProvenanceIndexSchema>;
