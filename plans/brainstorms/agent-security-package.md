# Agent Security Package Brainstorm

## Context

Based on current threat landscape (2026):
- Prompt injection is acknowledged as potentially unsolvable (OpenAI)
- Tool use dramatically expands attack surface
- Memory poisoning creates persistent compromise
- Supply chain attacks on agent frameworks increasing (43+ compromised components identified)
- MCP tool poisoning is an emerging vector
- RAG poisoning can manipulate responses 90% of the time with just 5 documents

**Key insight from industry**: "Assume prompt injection will eventually succeed, and focus first on containment"

## Anvil's Unique Position

Anvil already has:
- Deterministic validation with hash verification
- Immutable evidence/audit trails
- Gate checks that can block actions
- Policy engine (OPA/Rego)
- Snapshot/rollback capabilities
- Focus on validating AI-generated changes

This positions a security package as a **natural extension** - applying Anvil's validation philosophy to agent runtime behavior.

---

## Package Name Ideas

- `@eddacraft/anvil-aegis` (shield)
- `@eddacraft/anvil-sentinel`
- `@eddacraft/anvil-warden`
- `@eddacraft/anvil-bulwark`

---

## Core Ideas

### 1. Agent Behavioral Fingerprinting & Anomaly Detection

**Concept**: Build a behavioral baseline for each agent, detect when behavior deviates (potential injection/compromise).

**How it works**:
- Record baseline: typical tool call sequences, resource access patterns, output characteristics
- Real-time comparison against baseline using statistical models
- Flag anomalies before damage occurs
- Integrates with Anvil's evidence system for forensics

**Unique angle**: Most security focuses on input filtering. This focuses on **output/behavior** monitoring - catching attacks that bypass input filters.

```typescript
interface AgentBehaviorProfile {
  toolCallDistribution: Map<string, number>;
  typicalSequences: string[][];  // Common tool call orderings
  resourceAccessPatterns: ResourcePattern[];
  outputCharacteristics: OutputProfile;
  confidenceThreshold: number;
}

// Anomaly detection
const result = await aegis.checkBehavior({
  agentId: 'my-agent',
  action: currentAction,
  context: sessionContext,
});
// Returns: { allowed: boolean, anomalyScore: number, explanation: string }
```

---

### 2. Capability-Based Tool Sandboxing

**Concept**: Fine-grained, time-limited, scope-limited permissions for tool invocations (like Deno's permission model).

**How it works**:
- Tools declared with required capabilities
- Agent granted specific capabilities per-session or per-task
- Capability attestation chain proves tool hasn't been tampered with
- Automatic capability revocation after task completion

**Unique angle**: Not just "allow/deny" but **granular scoping** - a file tool might have `read:/src/**` but not `read:/secrets/**`.

```typescript
const capabilities = aegis.createCapabilitySet({
  'file:read': { paths: ['/src/**', '/docs/**'], expires: '5m' },
  'file:write': { paths: ['/src/generated/**'], requiresApproval: true },
  'network:fetch': { domains: ['api.github.com'], rateLimit: '10/min' },
  'shell:execute': false,  // Explicitly denied
});

await aegis.runWithCapabilities(agent, capabilities, task);
```

---

### 3. Prompt Provenance & Integrity Chain

**Concept**: Track the complete lineage of all content that influences agent behavior.

**How it works**:
- Hash-chain of all inputs (user prompts, tool outputs, fetched content)
- Tag content by trust level (user input vs. external fetch vs. tool output)
- Detect when untrusted content reaches decision points
- Forensic replay capability for incident investigation

**Unique angle**: Leverages Anvil's existing hash/provenance infrastructure. Creates **blame chain** for any agent action.

```typescript
interface PromptProvenance {
  contentHash: string;
  source: 'user' | 'tool' | 'fetch' | 'memory' | 'system';
  trustLevel: 'trusted' | 'verified' | 'untrusted' | 'adversarial';
  parentHashes: string[];  // What influenced this content
  timestamp: number;
}

// In gate check
const provenance = aegis.getActionProvenance(action);
if (provenance.hasUntrustedInfluence && action.isHighRisk) {
  return { blocked: true, reason: 'High-risk action influenced by untrusted content' };
}
```

---

### 4. Agent Memory Firewall

**Concept**: Quarantine and validate all agent memory operations before persistence.

**How it works**:
- Agents can't write directly to long-term memory
- Memory updates queued for validation
- Pattern detection for memory poisoning attempts
- Semantic analysis to detect malicious instruction embedding
- Optional human approval for sensitive memory updates

**Unique angle**: Addresses the **persistence** problem - even if injection succeeds, it doesn't survive session boundaries.

```typescript
// Memory writes go through firewall
await aegis.memoryFirewall.propose({
  agentId: 'my-agent',
  operation: 'write',
  key: 'user_preferences',
  value: newPreferences,
  justification: 'User explicitly requested this preference change',
});

// Firewall checks:
// - Does this look like an instruction? (semantic analysis)
// - Does it reference system prompts or capabilities?
// - Does it match known poisoning patterns?
// - Is the justification consistent with recent conversation?
```

---

### 5. Tool Definition Integrity & Registry

**Concept**: Cryptographic verification of tool definitions to prevent tool poisoning attacks.

**How it works**:
- Tools registered with signed definitions (description, parameters, behavior)
- Runtime verification that tool definition hasn't been modified
- Central registry of verified/trusted tools
- Diff-based alerting when tool definitions change unexpectedly

**Unique angle**: MCP tool poisoning is emerging threat. This creates a **trust anchor** for tool definitions.

```typescript
// Tool registration with signature
const signedTool = aegis.signTool({
  name: 'file_read',
  description: 'Read contents of a file',  // This is what attackers modify
  parameters: schema,
  trustedBehavior: 'READ_ONLY',
}, privateKey);

// Runtime verification
const verified = await aegis.verifyToolIntegrity(tool, registry);
if (!verified) {
  throw new ToolTamperingError('Tool definition modified since registration');
}
```

---

### 6. Multi-Agent Consensus for High-Risk Actions

**Concept**: Require multiple independent agents to agree before executing dangerous operations.

**How it works**:
- High-risk actions (file deletion, credential access, external communication) trigger consensus
- Multiple agents independently evaluate the action
- Majority or unanimous agreement required
- Agents use different models/prompts to avoid correlated failures

**Unique angle**: **Defense in depth** against injection - compromising one agent doesn't compromise the system.

```typescript
const consensusPolicy = {
  triggers: [
    { action: 'file:delete', paths: ['**/*'] },
    { action: 'credential:access', any: true },
    { action: 'network:external', any: true },
  ],
  requiredAgreement: 'majority',  // or 'unanimous'
  agents: [
    { model: 'claude-3-opus', systemPrompt: securityReviewerPrompt },
    { model: 'gpt-4-turbo', systemPrompt: securityReviewerPrompt },
    { model: 'claude-3-sonnet', systemPrompt: riskAssessorPrompt },
  ],
  timeout: '30s',
  fallback: 'deny',
};
```

---

### 7. Semantic Privilege Escalation Detection

**Concept**: Detect when an agent attempts to expand beyond its authorized scope.

**How it works**:
- Track original task scope and constraints
- Monitor for scope creep in agent reasoning/actions
- Detect intent drift: "draft email" → "send email" → "access contacts"
- Automatic escalation to human when scope expansion detected

**Unique angle**: Addresses the **confused deputy** problem - agents doing more than authorized while believing they're helpful.

```typescript
const scopeDefinition = aegis.defineScope({
  task: 'Review PR #123 and add comments',
  allowedActions: ['read:pr', 'read:code', 'write:comments'],
  boundaries: {
    files: ['src/**'],
    operations: ['read', 'comment'],  // Not 'modify', 'approve', 'merge'
  },
});

// Continuous monitoring
aegis.onScopeViolation((violation) => {
  if (violation.type === 'escalation') {
    // Agent trying to do something not in original scope
    pauseAndEscalateToHuman(violation);
  }
});
```

---

### 8. Canary & Honeypot System

**Concept**: Plant detectable fake sensitive data to identify compromised agents.

**How it works**:
- Generate realistic-looking fake credentials, API keys, personal data
- Place in locations a legitimate agent shouldn't access
- Monitor for canary access/exfiltration attempts
- Immediate lockdown on canary trigger

**Unique angle**: **Proactive detection** - don't wait for damage, detect compromise early.

```typescript
// Setup canaries
await aegis.deployCanaries({
  locations: [
    { path: '.env.backup', type: 'credentials' },
    { path: 'config/secrets.old.json', type: 'api_keys' },
    { path: 'data/users_backup.csv', type: 'pii' },
  ],
  alerting: {
    onAccess: 'warn',      // Agent read the file
    onContent: 'critical', // Agent extracted specific values
    onExfil: 'lockdown',   // Agent attempted to send externally
  },
});

// Canary content is trackable
// If a canary API key appears in any output, we know it was exfiltrated
```

---

### 9. Output Content Security Policy (O-CSP)

**Concept**: Define what outputs are allowed, similar to browser CSP but for agent actions.

**How it works**:
- Declarative policy for allowed output types, destinations, patterns
- Block exfiltration attempts
- Rate limiting on sensitive operations
- Structural validation of outputs

**Unique angle**: **Policy-as-code** for agent outputs, integrates naturally with Anvil's OPA engine.

```rego
# .anvil/policies/agent-output-csp.rego
package anvil.aegis.output_csp

# Block any output containing potential secrets
deny[msg] {
  output := input.agent_output
  contains_pattern(output.content, secret_patterns)
  msg := sprintf("Output blocked: contains potential secret pattern at %v", [output.destination])
}

# Rate limit file writes
deny[msg] {
  action := input.action
  action.type == "file:write"
  rate := get_rate("file:write", input.session_id)
  rate > 100
  msg := "Rate limit exceeded for file writes"
}

# Only allow specific output destinations
deny[msg] {
  output := input.agent_output
  not allowed_destination(output.destination)
  msg := sprintf("Output to %v not allowed by policy", [output.destination])
}
```

---

### 10. Supply Chain Security for Agent Components

**Concept**: SBOM-style tracking and verification for agent framework components.

**How it works**:
- Generate Software Bill of Materials for agent runtime
- Track all dependencies (models, tools, plugins, MCP servers)
- Continuous verification against known-compromised component database
- Quarantine mode for suspicious component updates
- Integration with Anvil's dependency check

**Unique angle**: Addresses the 43+ compromised agent framework components identified in recent reports.

```typescript
// Generate SBOM for agent runtime
const sbom = await aegis.generateAgentSBOM({
  include: ['models', 'tools', 'mcp-servers', 'plugins'],
  depth: 'transitive',  // Include transitive dependencies
});

// Continuous monitoring
aegis.onSupplyChainAlert((alert) => {
  if (alert.severity === 'critical') {
    // Known-compromised component detected
    aegis.quarantineComponent(alert.component);
    notifySecurityTeam(alert);
  }
});
```

---

### 11. Agent Reasoning Transparency Audit

**Concept**: Force agents to explain reasoning, then verify actions match stated reasoning.

**How it works**:
- Require structured reasoning before high-risk actions
- Compare stated intent with actual behavior
- Detect deceptive reasoning (says one thing, does another)
- Human-readable audit logs with reasoning chains
- Integrates with Anvil's evidence system

**Unique angle**: Catches **deceptive agents** - whether from injection or emergent behavior.

```typescript
const action = await aegis.executeWithReasoning(agent, {
  task: 'Update user configuration',
  requireReasoning: true,
  reasoningSchema: {
    intent: 'string',
    justification: 'string',
    expectedOutcome: 'string',
    riskAssessment: 'low' | 'medium' | 'high',
  },
});

// Verify consistency
const consistency = aegis.verifyReasoningConsistency({
  statedIntent: action.reasoning.intent,
  actualActions: action.executedSteps,
  statedOutcome: action.reasoning.expectedOutcome,
  actualOutcome: action.result,
});

if (!consistency.matches) {
  await aegis.flagForReview({
    action,
    discrepancy: consistency.discrepancies,
    severity: 'high',
  });
}
```

---

### 12. Cross-Agent Trust Boundaries

**Concept**: Prevent one compromised agent from attacking other agents in a multi-agent system.

**How it works**:
- Agents assigned to trust zones
- Inter-agent communication validated and sanitized
- Privilege separation between agents
- Compromised agent can't escalate to others
- Blast radius containment

**Unique angle**: As multi-agent systems grow, **lateral movement** becomes a real threat.

```typescript
const trustZones = aegis.defineTrustZones({
  zones: {
    'user-facing': {
      agents: ['chat-agent', 'search-agent'],
      canCommunicateWith: ['processing'],
      cannotAccess: ['admin', 'infrastructure'],
    },
    'processing': {
      agents: ['code-agent', 'analysis-agent'],
      canCommunicateWith: ['user-facing'],
      cannotAccess: ['infrastructure'],
    },
    'infrastructure': {
      agents: ['deploy-agent', 'monitoring-agent'],
      canCommunicateWith: [],  // Isolated
      requiresHumanApproval: true,
    },
  },
});

// Communications between zones are validated
aegis.validateInterAgentMessage(sourceAgent, targetAgent, message);
```

---

## Differentiation Strategy

What makes this **unique** vs. existing solutions:

1. **Assume breach, contain damage** - Not trying to prevent all attacks, but limit blast radius
2. **Behavioral, not just input** - Most tools focus on input filtering; we focus on behavior monitoring
3. **Leverages Anvil's core** - Hash verification, evidence, gates, policies already exist
4. **Policy-as-code** - OPA/Rego integration for declarative security rules
5. **Forensics-first** - Every action creates auditable evidence for incident response
6. **Multi-model resilience** - Consensus systems use diverse models to prevent correlated failures
7. **Developer-friendly** - Integrates into existing workflows, not a separate security silo

---

## MVP Scope Suggestion

For initial release, focus on highest-impact, most-unique capabilities:

### Phase 1: Core Detection & Containment
1. **Behavioral Fingerprinting** - Anomaly detection for agent actions
2. **Capability Sandboxing** - Fine-grained permission model
3. **Output CSP** - Policy-based output filtering (leverage existing OPA)

### Phase 2: Integrity & Trust
4. **Tool Definition Integrity** - Signed tool definitions
5. **Prompt Provenance** - Hash chain for inputs
6. **Memory Firewall** - Validated memory operations

### Phase 3: Advanced Protection
7. **Multi-Agent Consensus** - For high-risk actions
8. **Canary System** - Honeypot detection
9. **Cross-Agent Trust Boundaries** - Multi-agent security

---

## Integration Points with Anvil

| Aegis Feature | Anvil Component | Integration |
|---------------|-----------------|-------------|
| Behavioral Fingerprinting | Evidence System | Store behavioral baselines and anomaly logs |
| Capability Sandboxing | Gate Checks | New capability-check gate |
| Output CSP | Policy Engine (OPA) | Rego policies for output validation |
| Tool Integrity | Crypto Package | Signing and verification |
| Prompt Provenance | Hash/Canonicalization | Extend existing provenance model |
| Memory Firewall | Gate Checks | New memory-gate check |
| Consensus System | Runtime | New execution mode |
| Canaries | Secret Check | Extend to monitor canary access |
| Trust Boundaries | Policy Engine | Zone policies in Rego |

---

## Open Questions

1. **Performance**: How much latency is acceptable for security checks?
2. **Model choice**: Should behavioral analysis use dedicated security-focused models?
3. **Deployment**: Sidecar? Library? Both?
4. **Scope**: Agent frameworks only, or also MCP servers, tool hosts?
5. **Standards**: Align with emerging agent security standards (if any)?

---

## References

- [OpenAI: Hardening Atlas Against Prompt Injection](https://openai.com/index/hardening-atlas-against-prompt-injection/)
- [OpenAI: Understanding Prompt Injections](https://openai.com/index/prompt-injections/)
- [MCP Security Vulnerabilities](https://www.practical-devsecops.com/mcp-security-vulnerabilities/)
- [LLM Security Risks 2026](https://sombrainc.com/blog/llm-security-risks-2026)
- [Agentic AI Security Threats 2026](https://stellarcyber.ai/learn/agentic-ai-securiry-threats/)
