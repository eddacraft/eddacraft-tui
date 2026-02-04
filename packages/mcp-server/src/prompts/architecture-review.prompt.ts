import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';

/**
 * Registers the `architecture-review` prompt template on the given MCP server.
 *
 * Produces a structured prompt for reviewing a file against the project's
 * architecture rules, including layer assignments and import boundaries.
 */
export function registerArchitectureReviewPrompt(server: McpServer): void {
  server.registerPrompt(
    'architecture-review',
    {
      title: 'Architecture Review',
      description: 'Prompt template for reviewing a file against Anvil architecture rules',
      argsSchema: {
        filePath: z.string().describe('File to review (relative to workspace root)'),
        workspaceRoot: z
          .string()
          .optional()
          .describe('Workspace root directory (defaults to current working directory)'),
      },
    },
    ({ filePath, workspaceRoot }) => {
      const root = workspaceRoot ?? '.';

      return {
        messages: [
          {
            role: 'user' as const,
            content: {
              type: 'text' as const,
              text: `Review ${filePath} against the project's architecture rules.

## Review checklist:

### 1. Layer assignment
- Determine which architecture layer this file belongs to
- Verify the file is in the correct directory for its layer
- Check that the file's responsibilities match its layer's purpose

### 2. Import boundary verification
- List all imports in the file
- For each cross-layer import, use \`anvil_query_boundary\` to verify it is allowed:
  \`\`\`
  anvil_query_boundary({
    sourceFile: "${filePath}",
    targetFile: "<imported-file>",
    workspaceRoot: "${root}"
  })
  \`\`\`
- Flag any imports that violate architecture boundaries

### 3. Dependency direction
- Verify dependencies flow in the correct direction (typically: UI -> Application -> Domain -> Infrastructure)
- Check for circular dependencies between layers
- Ensure domain/core layers do not depend on outer layers

### 4. Run architecture checks
- Execute \`anvil_check\` with architecture-only checks:
  \`\`\`
  anvil_check({
    files: ["${filePath}"],
    workspaceRoot: "${root}",
    checks: ["architecture"]
  })
  \`\`\`
- Review any warnings and provide remediation guidance

### 5. Report findings
- Summarize which layer the file belongs to
- List any boundary violations found
- Provide specific recommendations for fixing violations
- Note any architectural concerns even if no violations were detected`,
            },
          },
        ],
      };
    }
  );
}
