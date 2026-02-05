/**
 * Agent Manager
 *
 * Handles agent identification, registration, and heartbeat management
 * for multi-agent coordination in Anvil.
 */

import { promises as fs } from 'node:fs';
import { dirname, join } from 'node:path';
import {
  AgentRegistrySchema,
  AgentInfoSchema,
  type AgentInfo,
  type AgentRegistry,
  type AgentRegistration,
  type AgentType,
  type ConcurrencyConfig,
  getDefaultConcurrencyConfig,
} from './types.js';
import { atomicWriteJson, readJsonSafe } from './atomic.js';
import { createDebugger } from '@eddacraft/anvil-core';

const debug = createDebugger('agent');

// ============================================================================
// Agent ID Detection
// ============================================================================

/**
 * Environment variables for agent identification
 */
const AGENT_ENV_VARS = {
  // Explicit Anvil agent ID
  ANVIL_AGENT_ID: 'ANVIL_AGENT_ID',
  ANVIL_AGENT_TYPE: 'ANVIL_AGENT_TYPE',
  ANVIL_AGENT_NAME: 'ANVIL_AGENT_NAME',
  ANVIL_SESSION_ID: 'ANVIL_SESSION_ID',

  // Claude Code specific
  CLAUDE_SESSION_ID: 'CLAUDE_SESSION_ID',
  CLAUDE_CODE_SESSION: 'CLAUDE_CODE_SESSION',

  // Cursor specific
  CURSOR_SESSION_ID: 'CURSOR_SESSION_ID',

  // General AI tool indicators
  AI_TOOL: 'AI_TOOL',
  EDITOR_PID: 'EDITOR_PID',

  // CI indicators
  CI: 'CI',
  GITHUB_ACTIONS: 'GITHUB_ACTIONS',
  GITLAB_CI: 'GITLAB_CI',
  CIRCLECI: 'CIRCLECI',
  JENKINS_URL: 'JENKINS_URL',
};

/**
 * Detect agent type from environment
 */
export function detectAgentType(): AgentType {
  const env = process.env;

  // Explicit type setting
  if (env[AGENT_ENV_VARS.ANVIL_AGENT_TYPE]) {
    const type = env[AGENT_ENV_VARS.ANVIL_AGENT_TYPE]?.toLowerCase();
    if (
      type &&
      ['claude', 'cursor', 'copilot', 'aider', 'continue', 'codeium', 'human', 'ci'].includes(type)
    ) {
      return type as AgentType;
    }
  }

  // CI detection
  if (
    env[AGENT_ENV_VARS.CI] ||
    env[AGENT_ENV_VARS.GITHUB_ACTIONS] ||
    env[AGENT_ENV_VARS.GITLAB_CI] ||
    env[AGENT_ENV_VARS.CIRCLECI] ||
    env[AGENT_ENV_VARS.JENKINS_URL]
  ) {
    return 'ci';
  }

  // Claude Code detection
  if (env[AGENT_ENV_VARS.CLAUDE_SESSION_ID] || env[AGENT_ENV_VARS.CLAUDE_CODE_SESSION]) {
    return 'claude';
  }

  // Cursor detection
  if (env[AGENT_ENV_VARS.CURSOR_SESSION_ID]) {
    return 'cursor';
  }

  // Explicit AI tool setting
  if (env[AGENT_ENV_VARS.AI_TOOL]) {
    const tool = env[AGENT_ENV_VARS.AI_TOOL]?.toLowerCase();
    if (tool === 'aider') return 'aider';
    if (tool === 'continue') return 'continue';
    if (tool === 'codeium') return 'codeium';
    if (tool === 'copilot') return 'copilot';
  }

  // Check if running interactively (likely human)
  if (process.stdin.isTTY && !env[AGENT_ENV_VARS.AI_TOOL]) {
    return 'human';
  }

  return 'unknown';
}

/**
 * Get or generate agent ID
 */
export function getAgentId(): string {
  // Check explicit agent ID
  const explicitId = process.env[AGENT_ENV_VARS.ANVIL_AGENT_ID];
  if (explicitId) {
    return explicitId;
  }

  // Use session IDs if available
  if (process.env[AGENT_ENV_VARS.ANVIL_SESSION_ID]) {
    return `session-${process.env[AGENT_ENV_VARS.ANVIL_SESSION_ID]}`;
  }
  if (process.env[AGENT_ENV_VARS.CLAUDE_SESSION_ID]) {
    return `claude-${process.env[AGENT_ENV_VARS.CLAUDE_SESSION_ID]}`;
  }
  if (process.env[AGENT_ENV_VARS.CURSOR_SESSION_ID]) {
    return `cursor-${process.env[AGENT_ENV_VARS.CURSOR_SESSION_ID]}`;
  }

  // CI-specific IDs
  if (process.env['GITHUB_RUN_ID']) {
    return `gh-${process.env['GITHUB_RUN_ID']}-${process.env['GITHUB_RUN_ATTEMPT'] || '1'}`;
  }
  if (process.env['CI_JOB_ID']) {
    return `ci-${process.env['CI_JOB_ID']}`;
  }

  // Generate based on process
  return `proc-${process.pid}-${Date.now().toString(36)}`;
}

/**
 * Get session ID if available
 */
export function getSessionId(): string | undefined {
  return (
    process.env[AGENT_ENV_VARS.ANVIL_SESSION_ID] ||
    process.env[AGENT_ENV_VARS.CLAUDE_SESSION_ID] ||
    process.env[AGENT_ENV_VARS.CURSOR_SESSION_ID] ||
    undefined
  );
}

/**
 * Get agent name from environment or generate one
 */
export function getAgentName(): string {
  const explicitName = process.env[AGENT_ENV_VARS.ANVIL_AGENT_NAME];
  if (explicitName) {
    return explicitName;
  }

  const type = detectAgentType();
  const pid = process.pid;

  switch (type) {
    case 'claude':
      return `Claude Code (${pid})`;
    case 'cursor':
      return `Cursor AI (${pid})`;
    case 'copilot':
      return `GitHub Copilot (${pid})`;
    case 'aider':
      return `Aider (${pid})`;
    case 'continue':
      return `Continue (${pid})`;
    case 'codeium':
      return `Codeium (${pid})`;
    case 'human':
      return `Human Developer (${pid})`;
    case 'ci':
      return `CI Runner (${pid})`;
    default:
      return `Agent (${pid})`;
  }
}

/**
 * Create agent info from environment
 */
export function createAgentInfo(overrides?: Partial<AgentInfo>): AgentInfo {
  return AgentInfoSchema.parse({
    id: getAgentId(),
    type: detectAgentType(),
    pid: process.pid,
    name: getAgentName(),
    sessionId: getSessionId(),
    ...overrides,
  });
}

// ============================================================================
// Agent Manager
// ============================================================================

/**
 * Options for AgentManager
 */
export interface AgentManagerOptions {
  /** Workspace root directory */
  workspaceRoot: string;

  /** Concurrency configuration */
  config?: Partial<ConcurrencyConfig>;

  /** Agent info (auto-detected if not provided) */
  agentInfo?: AgentInfo;
}

/**
 * Agent Manager
 *
 * Manages agent registration, heartbeat, and lifecycle in a multi-agent environment.
 */
export class AgentManager {
  private readonly workspaceRoot: string;
  private readonly config: ConcurrencyConfig;
  private readonly agent: AgentInfo;
  private readonly registryPath: string;

  private heartbeatTimer: NodeJS.Timeout | null = null;
  private isRegistered = false;

  constructor(options: AgentManagerOptions) {
    this.workspaceRoot = options.workspaceRoot;
    this.config = {
      ...getDefaultConcurrencyConfig(),
      ...options.config,
    };
    this.agent = options.agentInfo ?? createAgentInfo();
    this.registryPath = join(this.workspaceRoot, this.config.registryPath);
  }

  /**
   * Check if agent is registered
   */
  get registered(): boolean {
    return this.isRegistered;
  }

  /**
   * Get the current agent info
   */
  getAgent(): AgentInfo {
    return { ...this.agent };
  }

  /**
   * Get the agent ID
   */
  getAgentId(): string {
    return this.agent.id;
  }

  /**
   * Register this agent
   */
  async register(operation?: string): Promise<void> {
    await this.ensureDir();

    const registry = await this.loadRegistry();
    const now = new Date().toISOString();

    const registration: AgentRegistration = {
      agent: this.agent,
      registeredAt: now,
      lastHeartbeat: now,
      heartbeatCount: 0,
      state: 'active',
      currentOperation: operation,
      workspaceRoot: this.workspaceRoot,
    };

    registry.agents[this.agent.id] = registration;
    registry.updatedAt = now;

    await this.saveRegistry(registry);
    this.isRegistered = true;

    debug(`Agent registered: ${this.agent.id} (${this.agent.type})`);
  }

  /**
   * Start heartbeat timer
   */
  startHeartbeat(): void {
    if (this.heartbeatTimer) {
      return;
    }

    this.heartbeatTimer = setInterval(async () => {
      try {
        await this.heartbeat();
      } catch (error) {
        debug('Heartbeat failed:', error);
      }
    }, this.config.heartbeatIntervalMs);

    // Don't block process exit for heartbeat
    this.heartbeatTimer.unref();

    debug(`Heartbeat started: interval=${this.config.heartbeatIntervalMs}ms`);
  }

  /**
   * Stop heartbeat timer
   */
  stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
      debug('Heartbeat stopped');
    }
  }

  /**
   * Send a heartbeat
   */
  async heartbeat(operation?: string): Promise<void> {
    const registry = await this.loadRegistry();
    const registration = registry.agents[this.agent.id];

    if (!registration) {
      // Re-register if not in registry
      await this.register(operation);
      return;
    }

    const now = new Date().toISOString();

    registration.lastHeartbeat = now;
    registration.heartbeatCount++;
    registration.state = 'active';
    if (operation !== undefined) {
      registration.currentOperation = operation;
    }

    registry.updatedAt = now;
    await this.saveRegistry(registry);

    debug(`Heartbeat sent: ${this.agent.id} (count=${registration.heartbeatCount})`);
  }

  /**
   * Update current operation
   */
  async setOperation(operation: string | undefined): Promise<void> {
    const registry = await this.loadRegistry();
    const registration = registry.agents[this.agent.id];

    if (registration) {
      registration.currentOperation = operation;
      registration.lastHeartbeat = new Date().toISOString();
      registry.updatedAt = new Date().toISOString();
      await this.saveRegistry(registry);
    }
  }

  /**
   * Unregister this agent
   */
  async unregister(): Promise<void> {
    this.stopHeartbeat();

    const registry = await this.loadRegistry();

    if (registry.agents[this.agent.id]) {
      registry.agents[this.agent.id].state = 'terminated';
      registry.updatedAt = new Date().toISOString();
      await this.saveRegistry(registry);
    }

    this.isRegistered = false;
    debug(`Agent unregistered: ${this.agent.id}`);
  }

  /**
   * Get all registered agents
   */
  async getAllAgents(): Promise<AgentRegistration[]> {
    const registry = await this.loadRegistry();
    return Object.values(registry.agents);
  }

  /**
   * Get active agents
   */
  async getActiveAgents(): Promise<AgentRegistration[]> {
    const registry = await this.loadRegistry();
    const now = Date.now();
    const staleThreshold = this.config.staleThresholdMs;

    return Object.values(registry.agents).filter((reg) => {
      const lastHeartbeat = new Date(reg.lastHeartbeat).getTime();
      const isStale = now - lastHeartbeat > staleThreshold;
      return reg.state === 'active' && !isStale;
    });
  }

  /**
   * Cleanup stale agents
   */
  async cleanupStaleAgents(): Promise<string[]> {
    const registry = await this.loadRegistry();
    const now = Date.now();
    const staleThreshold = this.config.staleThresholdMs;
    const staleAgents: string[] = [];

    for (const [id, reg] of Object.entries(registry.agents)) {
      const lastHeartbeat = new Date(reg.lastHeartbeat).getTime();
      const isStale = now - lastHeartbeat > staleThreshold;

      if (isStale && reg.state === 'active') {
        reg.state = 'stale';
        staleAgents.push(id);
        debug(`Agent marked stale: ${id}`);
      }
    }

    if (staleAgents.length > 0) {
      registry.updatedAt = new Date().toISOString();
      await this.saveRegistry(registry);
    }

    return staleAgents;
  }

  /**
   * Check if an agent is stale
   */
  async isAgentStale(agentId: string): Promise<boolean> {
    const registry = await this.loadRegistry();
    const registration = registry.agents[agentId];

    if (!registration) {
      return true;
    }

    if (registration.state === 'stale' || registration.state === 'terminated') {
      return true;
    }

    const now = Date.now();
    const lastHeartbeat = new Date(registration.lastHeartbeat).getTime();
    return now - lastHeartbeat > this.config.staleThresholdMs;
  }

  /**
   * Check if a process is still running
   */
  isProcessRunning(pid: number): boolean {
    try {
      // Sending signal 0 doesn't actually send a signal but checks if process exists
      process.kill(pid, 0);
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Get git user for commit trailers
   */
  getGitAgentTrailer(): string {
    return `Anvil-Agent: ${this.agent.id} (${this.agent.type})`;
  }

  /**
   * Load registry from file
   */
  private async loadRegistry(): Promise<AgentRegistry> {
    const data = await readJsonSafe(this.registryPath);

    if (data) {
      const result = AgentRegistrySchema.safeParse(data);
      if (result.success) {
        return result.data;
      }
      debug('Invalid registry schema, creating new:', result.error);
    }

    return {
      version: '1.0.0',
      updatedAt: new Date().toISOString(),
      agents: {},
    };
  }

  /**
   * Save registry to file
   */
  private async saveRegistry(registry: AgentRegistry): Promise<void> {
    await atomicWriteJson(this.registryPath, registry);
  }

  /**
   * Ensure registry directory exists
   */
  private async ensureDir(): Promise<void> {
    const dir = dirname(this.registryPath);
    await fs.mkdir(dir, { recursive: true });
  }
}

/**
 * Create an agent manager
 */
export function createAgentManager(options: AgentManagerOptions): AgentManager {
  return new AgentManager(options);
}

/**
 * Global agent manager singleton (optional usage pattern)
 */
let globalAgentManager: AgentManager | null = null;

/**
 * Initialize global agent manager
 */
export function initializeGlobalAgent(options: AgentManagerOptions): AgentManager {
  globalAgentManager = createAgentManager(options);
  return globalAgentManager;
}

/**
 * Get global agent manager
 */
export function getGlobalAgent(): AgentManager | null {
  return globalAgentManager;
}
