import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';

/**
 * Registers the `pre-generation` prompt template on the given MCP server.
 *
 * Produces a constraints prompt to prepend before code generation, ensuring
 * the AI respects architecture boundaries and avoids known antipatterns.
 * This is the most impactful prompt for preventing violations during generation.
 */
export function registerPreGenerationPrompt(server: McpServer): void {
  server.registerPrompt(
    'pre-generation',
    {
      title: 'Pre-Generation Constraints',
      description:
        'Constraints prompt to prepend before code generation to prevent architecture violations',
      argsSchema: {
        workspaceRoot: z.string().describe('Workspace root directory'),
        targetFile: z
          .string()
          .optional()
          .describe('Target file path for generation (used to determine layer constraints)'),
      },
    },
    ({ workspaceRoot, targetFile }) => {
      // Sanitize inputs to prevent injection into prompt template
      const safeRoot = String(workspaceRoot)
        .replace(/[\r\n`]/g, '')
        .slice(0, 500);
      const safeTarget = targetFile
        ? String(targetFile)
            .replace(/[\r\n`]/g, '')
            .slice(0, 500)
        : undefined;

      return {
        messages: [
          {
            role: 'user' as const,
            content: {
              type: 'text' as const,
              text: `[ARCHITECTURE CONSTRAINTS] Follow these rules when generating code${safeTarget ? ` for ${safeTarget}` : ''}.

## Layer definitions and boundaries:

This project uses layered architecture enforced by Anvil. Common layers include:
- **domain/core**: Pure business logic, no external dependencies. Cannot import from application, infrastructure, or UI layers.
- **application**: Use cases and orchestration. Can import from domain. Cannot import from infrastructure or UI.
- **infrastructure**: External integrations (DB, APIs, file system). Can import from domain and application.
- **ui/presentation**: User interface components. Can import from application and domain. Should not import directly from infrastructure.
- **shared/common**: Utilities shared across layers. Should have no layer-specific imports.

Dependencies must flow inward: UI -> Application -> Domain. Never the reverse.

${
  safeTarget
    ? `## Target file constraints:

The file \`${safeTarget}\` must respect the layer it belongs to. Before writing any cross-layer import, use \`anvil_query_boundary\` to verify it is allowed:
\`\`\`
anvil_query_boundary({
  sourceFile: "${safeTarget}",
  targetFile: "<file-to-import>",
  workspaceRoot: "${safeRoot}"
})
\`\`\`

`
    : ''
}## Antipatterns to avoid:

1. **Broad ESLint disables** (AP-001): Never use \`/* eslint-disable */\`. Use targeted \`// eslint-disable-next-line <specific-rule>\` instead.
2. **Console statements** (AP-002): Use a proper logger instead of \`console.log\` in production code.
3. **Untyped any** (AP-003): Never use \`: any\`. Use specific types, generics, or \`: unknown\` with type narrowing.
4. **@ts-ignore** (AP-004): Use \`@ts-expect-error\` with a description instead of \`@ts-ignore\`.
5. **Magic numbers** (AP-005): Extract numeric literals into named constants.
6. **Barrel file re-exports from parent layers**: Do not create index.ts files that re-export from higher layers.

## Before writing imports:

- Use \`anvil_query_boundary\` to check if a cross-layer import is allowed BEFORE writing it
- If the import is not allowed, restructure the code to respect boundaries
- Consider dependency injection or events for cross-layer communication

## After generating code:

- Run \`anvil_check\` against the generated file to verify no violations were introduced:
  \`\`\`
  anvil_check({
    files: ["${safeTarget ?? '<generated-file>'}"],
    workspaceRoot: "${safeRoot}"
  })
  \`\`\``,
            },
          },
        ],
      };
    }
  );
}
