import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';

/**
 * Registers the `suppress-violation` prompt template on the given MCP server.
 *
 * Guides the AI through properly suppressing a warning with a time-boxed
 * comment, including when suppression is appropriate versus fixing.
 */
export function registerSuppressViolationPrompt(server: McpServer): void {
  server.registerPrompt(
    'suppress-violation',
    {
      title: 'Suppress Violation',
      description: 'Guided prompt for adding a time-boxed suppression to an Anvil warning',
      argsSchema: {
        warningId: z.string().describe('The warning ID (e.g., AP-003)'),
        filePath: z.string().describe('File containing the violation'),
        line: z.coerce.number().describe('Line number of the violation (1-based)'),
        reason: z.string().optional().describe('Reason for suppressing instead of fixing'),
      },
    },
    ({ warningId, filePath, line, reason }) => ({
      messages: [
        {
          role: 'user' as const,
          content: {
            type: 'text' as const,
            text: `Suppress Anvil warning ${warningId} in ${filePath} at line ${line}.

${reason ? `Reason provided: ${reason}\n\n` : ''}## When suppression is appropriate:

- The violation is in generated code that will be overwritten
- A fix requires a larger refactor that is planned but not yet scheduled
- The warning is a false positive for this specific context
- The code is being deprecated and will be removed soon

## When you should fix instead of suppress:

- The fix is straightforward (e.g., replacing \`: any\` with a proper type)
- The violation indicates a real architectural problem
- The suppression would hide a genuine bug

## Suppression format:

Use the \`anvil_suppress\` tool with these parameters:
- \`filePath\`: "${filePath}"
- \`warningId\`: "${warningId}"
- \`line\`: ${line}
- \`reason\`: A clear, specific explanation of why suppression is needed
- \`expiryDays\`: Number of days before the suppression expires (default: 30)

This will insert a comment above the target line:
\`\`\`
// @anvil-ignore ${warningId}: <reason> [expires: YYYY-MM-DD]
\`\`\`

## Important reminders:

1. Always provide a meaningful reason -- "TODO" or "fix later" is not sufficient
2. Suppressions expire automatically after the specified number of days
3. Expired suppressions will show up as new warnings during \`anvil_check\`
4. Review suppressed warnings regularly to determine if they can be fixed
5. After suppressing, run \`anvil_check\` to confirm the warning is suppressed`,
          },
        },
      ],
    })
  );
}
