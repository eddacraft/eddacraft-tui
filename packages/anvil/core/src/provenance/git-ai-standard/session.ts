import { createHash, randomUUID } from 'crypto';
import type { SessionHash, AgentId } from './types.js';

/**
 * Generate a session hash from tool and conversation ID
 *
 * Per Git AI Standard v3.0.0: 16-character SHA-256 prefix of {tool}:{conversation_id}
 *
 * @param tool - AI tool name (e.g., "claude-code", "cursor")
 * @param conversationId - Unique session/conversation identifier
 * @returns 16-character hex session hash
 */
export function generateSessionHash(tool: string, conversationId: string): SessionHash {
  const input = `${tool}:${conversationId}`;
  const fullHash = createHash('sha256').update(input).digest('hex');
  return fullHash.slice(0, 16) as SessionHash;
}

/**
 * Generate a session hash from an AgentId
 */
export function sessionHashFromAgentId(agentId: AgentId): SessionHash {
  return generateSessionHash(agentId.tool, agentId.id);
}

/**
 * Create an AgentId from parameters
 *
 * @param options - Agent identification options
 * @returns AgentId object
 */
export function createAgentId(options: {
  tool: string;
  conversationId?: string;
  model?: string;
}): AgentId {
  const { tool, model } = options;

  // Generate conversation ID if not provided (using crypto for robust randomness)
  const conversationId = options.conversationId ?? `${Date.now()}-${randomUUID().slice(0, 8)}`;

  return {
    tool,
    id: conversationId,
    model,
  };
}

/**
 * Known AI tool environment detection patterns
 */
const AI_TOOL_PATTERNS = [
  {
    tool: 'claude-code',
    envVars: ['CLAUDE_SESSION_ID', 'CLAUDE_CODE_SESSION'],
    modelVar: 'CLAUDE_MODEL',
  },
  {
    tool: 'cursor',
    envVars: ['CURSOR_SESSION', 'CURSOR_SESSION_ID'],
    modelVar: 'CURSOR_MODEL',
  },
  {
    tool: 'copilot',
    envVars: ['GITHUB_COPILOT_TOKEN', 'COPILOT_SESSION'],
    modelVar: undefined,
  },
  {
    tool: 'codewhisperer',
    envVars: ['AWS_CODEWHISPERER_SESSION'],
    modelVar: undefined,
  },
  {
    tool: 'tabnine',
    envVars: ['TABNINE_SESSION'],
    modelVar: undefined,
  },
] as const;

/**
 * Detect current AI tool and create AgentId from environment
 *
 * Checks environment variables for known AI tool session identifiers.
 * Returns null if no AI tool is detected.
 *
 * @returns AgentId if an AI tool is detected, null otherwise
 */
export function detectCurrentAgent(): AgentId | null {
  for (const pattern of AI_TOOL_PATTERNS) {
    for (const envVar of pattern.envVars) {
      const sessionId = process.env[envVar];
      if (sessionId) {
        return createAgentId({
          tool: pattern.tool,
          conversationId: sessionId,
          model: pattern.modelVar ? process.env[pattern.modelVar] : undefined,
        });
      }
    }
  }

  return null;
}

/**
 * Create an AgentId for manual/explicit use
 *
 * Use this when you know the exact tool and session ID.
 *
 * @param tool - AI tool name
 * @param sessionId - Session/conversation ID
 * @param model - Optional model identifier
 * @returns AgentId object
 */
export function createExplicitAgent(tool: string, sessionId: string, model?: string): AgentId {
  return {
    tool,
    id: sessionId,
    model,
  };
}

/**
 * Format an AgentId for display
 */
export function formatAgentId(agent: AgentId): string {
  const parts = [agent.tool];
  if (agent.model) {
    parts.push(`(${agent.model})`);
  }
  parts.push(`session:${agent.id.slice(0, 8)}...`);
  return parts.join(' ');
}
