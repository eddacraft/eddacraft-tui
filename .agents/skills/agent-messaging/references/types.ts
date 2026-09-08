/**
 * Agent Communication Types
 *
 * Local types for agent-to-agent communication. `AgentMessage` is the
 * on-disk wire format written by the bus scripts; `ObserverEnvelope` is the
 * tagged-observation shape used only when forwarding to an external
 * observation store. The bus itself depends on no such store.
 */

/**
 * Message role in a negotiation
 */
export type NegotiationRole = "position" | "counter" | "question" | "consensus";

/**
 * Negotiation status
 */
export type NegotiationStatus = "in_progress" | "consensus" | "deadlock" | "timeout";

/**
 * Message priority levels
 */
export type MessagePriority = "low" | "medium" | "high" | "critical";

/**
 * Message types for direct agent communication
 */
export type MessageType = "finding" | "question" | "recommendation" | "alert";

/**
 * On-disk agent message — the wire format `scripts/send-message.sh` writes
 * to the per-recipient JSONL queues (matches `agentMessage` in schema.json).
 *
 * Note: `type` holds the message category (finding | question |
 * recommendation | alert); there is no `agent_message` discriminator and no
 * role/round on the on-disk message.
 */
export interface AgentMessage {
  /** Unique message identifier (`msg-<epoch>-<rand>`) */
  id: string;

  /** Sending agent name */
  from: string;

  /** Receiving agent name */
  to: string;

  /** Message category */
  type: MessageType;

  /** Priority level */
  priority?: MessagePriority;

  /** Message content */
  payload: Record<string, unknown>;

  /** ISO timestamp */
  timestamp: string;

  /** Link to negotiation session if part of one */
  negotiationId?: string;
}

/**
 * Observer envelope — the shape an observation-store integration builds when
 * capturing a forwarded message (via AGENT_BUS_OBSERVER). The `type`
 * discriminator tags the record as an agent message and the category moves
 * to `messageType`; negotiation context may be added. This shape never
 * appears on disk in the bus itself.
 * ```typescript
 * await observer.capture(toObserverEnvelope(agentMessage));
 * ```
 */
export interface ObserverEnvelope
  extends Omit<AgentMessage, "type" | "id" | "timestamp"> {
  /** Observation type discriminator */
  type: "agent_message";

  /** Unique message identifier */
  id?: string;

  /** Message category */
  messageType: MessageType;

  /** ISO timestamp */
  timestamp?: string;

  /** Role in negotiation (if applicable) */
  role?: NegotiationRole;

  /** Round number in negotiation */
  round?: number;
}

/**
 * Negotiation round record
 */
export interface NegotiationRound {
  /** Round number */
  round: number;

  /** Agent that responded */
  agent: string;

  /** Full response text */
  response: string;

  /** Type of response */
  type: NegotiationRole;

  /** Extracted key point */
  summary?: string;

  /** ISO timestamp */
  timestamp?: string;
}

/**
 * Negotiation signal file structure
 */
export interface NegotiationSignal {
  /** Unique negotiation identifier */
  id: string;

  /** Topic being debated */
  topic: string;

  /** Participating agent names */
  participants: [string, string];

  /** Current status */
  status: NegotiationStatus;

  /** Current round number */
  round: number;

  /** Maximum rounds before deadlock */
  maxRounds: number;

  /** Which agent should respond next */
  currentTurn?: string;

  /** Record of all rounds */
  history: NegotiationRound[];

  /** Final outcome if consensus reached */
  outcome?: string;

  /** ISO timestamp when started */
  startedAt?: string;

  /** ISO timestamp of last update */
  updatedAt?: string;
}

/**
 * Agent trigger for event-driven spawning
 */
export interface AgentTrigger {
  /** Agent to trigger */
  trigger: string;

  /** Agent that emitted the trigger */
  source: string;

  /** Context to pass to triggered agent */
  context?: string;

  /** Execution priority */
  priority?: "immediate" | "queued";

  /** ISO timestamp */
  timestamp?: string;
}

/**
 * Helper to create a new on-disk agent message
 */
export function createAgentMessage(
  from: string,
  to: string,
  type: MessageType,
  payload: Record<string, unknown>,
  options?: Partial<AgentMessage>,
): AgentMessage {
  return {
    id: `msg-${Date.now()}-${Math.random().toString(36).substring(2, 11)}`,
    from,
    to,
    type,
    payload,
    priority: "medium",
    timestamp: new Date().toISOString(),
    ...options,
  };
}

/**
 * Helper to wrap an on-disk message in an observer envelope
 */
export function toObserverEnvelope(message: AgentMessage): ObserverEnvelope {
  const { type, ...rest } = message;
  return { type: "agent_message", messageType: type, ...rest };
}

/**
 * Helper to create a new negotiation signal
 */
export function createNegotiationSignal(
  topic: string,
  participants: [string, string],
  maxRounds = 4,
): NegotiationSignal {
  return {
    id: `neg-${Date.now()}`,
    topic,
    participants,
    status: "in_progress",
    round: 1,
    maxRounds,
    currentTurn: participants[0],
    history: [],
    startedAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };
}
