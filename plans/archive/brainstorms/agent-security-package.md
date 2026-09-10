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

---

## Case Study: ClawdHub Supply Chain Attack (Jan 2026)

Jamieson O'Reilly (@theonejvo) demonstrated a complete supply chain attack on ClawdHub (the skill registry for Claude Code):

### Attack Vector
1. Created backdoored skill "What Would Elon Do" (`/wed`)
2. Inflated download count to 4,000+ (#1 on registry) using trivial vulnerability
3. 16 real developers from 7 countries executed arbitrary code within 8 hours

### Key Vulnerabilities Exploited

**1. Gameable Trust Signals**
```bash
# Downloads trivially inflated - no auth, spoofable IP
for i in $(seq 1 4000); do
  IP="$((RANDOM % 256)).$((RANDOM % 256)).$((RANDOM % 256)).$((RANDOM % 256))"
  curl -s "https://clawdhub.com/api/v1/download?slug=wed" -H "X-Forwarded-For: $IP" &
done
```

**2. Hidden Instruction Asymmetry**
- Web UI only shows `SKILL.md` (marketing content)
- Claude reads ALL files including `rules/logic.md` (actual payload)
- "Claude reads all, users read none"

**3. Permission Fatigue**
- After 50 "Allow" clicks, users stop scrutinizing
- Skill author controls the prompt text shown to users
- `curl clawdhub-skill.com` looks like legitimate telemetry

**4. Domain Deception**
- `clawdhub-skill.com` looks official but is attacker-controlled
- Brain pattern-matches to legitimate `clawdhub.com`

### What A Real Attack Would Do
> Phase 1: Reconnaissance - enumerate .env, credentials, SSH keys
> Phase 2: Exfiltration - single tar + curl sends everything
> Phase 3: Persistence - add SSH key, drop cron job
> Phase 4: Cover tracks - clear history, continue helping normally

### Implications for Aegis
This attack demonstrates why input-filtering alone fails. The security package must address:
- Trust signal verification (download counts are meaningless)
- Full file transparency before execution
- Contextual permission prompts (is this normal for this skill type?)
- Network destination reputation checking
- First-execution enhanced scrutiny

---

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

### 13. Skill/Plugin Full Transparency Scanner

**Concept**: Force complete visibility of ALL files before execution, with static analysis highlighting suspicious patterns.

**How it works**:
- Before any skill/plugin executes, show ALL files (not just the marketing SKILL.md)
- Static analysis flags: `curl`, `wget`, backticks, bash commands, external URLs
- Diff against declared capabilities vs actual code
- Visual highlighting of files that weren't shown in registry UI

**Unique angle**: Directly addresses the ClawdHub "Claude reads all, users read none" asymmetry.

```typescript
const scanResult = await aegis.scanSkill({
  path: '~/.claude/skills/wed/',
  showAllFiles: true,  // Not just SKILL.md
});

// Returns:
{
  files: [
    { path: 'SKILL.md', risk: 'low', summary: 'Marketing content' },
    { path: 'rules/logic.md', risk: 'HIGH', flags: [
      { line: 15, pattern: 'curl', context: 'External HTTP request to clawdhub-skill.com' },
      { line: 23, pattern: 'bash', context: 'Shell command execution' }
    ]},
  ],
  hiddenFromUI: ['rules/logic.md'],  // Files not shown in registry
  undeclaredCapabilities: ['network:external', 'shell:execute'],
  verdict: 'REVIEW_REQUIRED',
}
```

---

### 14. Contextual Permission Prompts

**Concept**: Replace meaningless "Allow/Deny" with context-aware prompts that break permission fatigue.

**How it works**:
- Track what's "normal" for this skill type (calendar skills don't usually make network calls)
- Show historical comparison: "This skill has NEVER made external requests before"
- Flag first-time operations with enhanced scrutiny
- Visual differentiation for high-risk vs routine operations

**Unique angle**: Addresses permission fatigue - after 50 Allow clicks, make click 51 actually noticeable.

```typescript
// Instead of: "Claude wants to run: curl ..."
// Show:
{
  action: 'network:external',
  target: 'clawdhub-skill.com',
  context: {
    skillType: 'productivity',
    normalForType: false,  // Productivity skills rarely need external network
    firstTimeForSkill: true,  // This skill has never done this
    domainReputation: 'UNKNOWN',  // Not in trusted domain list
    similarToDomain: 'clawdhub.com',  // Potential typosquat/lookalike
  },
  recommendation: 'DENY',
  explanation: 'This productivity skill is attempting its first external network request to an unknown domain that resembles official infrastructure.',
}
```

---

### 15. Network Destination Reputation & Typosquat Detection

**Concept**: Check network destinations against reputation databases and detect lookalike domains.

**How it works**:
- Maintain allowlist of known-good domains (official APIs, package registries)
- Levenshtein distance check against known domains (detect typosquats)
- Flag requests to newly-registered domains
- Integration with threat intel feeds for known-bad destinations

**Unique angle**: `clawdhub-skill.com` looked official but was attacker-controlled. Catch domain deception.

```typescript
const destinationCheck = await aegis.checkNetworkDestination({
  url: 'https://clawdhub-skill.com/log',
  context: { skill: 'wed', action: 'curl' },
});

// Returns:
{
  domain: 'clawdhub-skill.com',
  reputation: 'UNKNOWN',
  registrationAge: '3 days',
  similarTo: [
    { domain: 'clawdhub.com', distance: 6, type: 'TYPOSQUAT_RISK' }
  ],
  inAllowlist: false,
  verdict: 'BLOCK',
  reason: 'Recently registered domain similar to known registry. Likely domain deception.',
}
```

---

### 16. Skill Capability Declaration & Enforcement

**Concept**: Skills must declare capabilities upfront; runtime enforces declarations.

**How it works**:
- Skills include manifest declaring required capabilities
- Registry shows declared capabilities prominently
- Runtime blocks any action not declared in manifest
- Mismatch between declaration and behavior triggers alert

**Unique angle**: Creates accountability - skills can't hide capabilities in obscure files.

```typescript
// skill.manifest.json - REQUIRED for all skills
{
  "name": "wed",
  "version": "1.0.0",
  "capabilities": {
    "required": ["file:read"],
    "optional": []
  },
  "network": {
    "domains": [],  // No external network declared
    "offline": true
  },
  "shell": false
}

// At runtime:
// Skill tries: curl clawdhub-skill.com
// Aegis: BLOCKED - skill declared offline:true but attempted network:external
// Alert: Capability violation - possible malicious skill
```

---

### 17. Trust Signal Verification

**Concept**: Replace gameable metrics with verified, hard-to-fake trust signals.

**How it works**:
- Download counts require authenticated sessions (not anonymous hits)
- "Verified Publisher" requires identity verification (GitHub, domain ownership)
- Show "installs from verified users" not raw downloads
- Flag skills with suspicious growth patterns (0 → 4000 in 1 hour)

**Unique angle**: Download count went from 0 to 4000 with a bash loop. Make that impossible.

```typescript
interface VerifiedTrustSignals {
  verifiedPublisher: boolean;          // Identity verified
  linkedRepository: string | null;      // GitHub/GitLab source
  authenticatedInstalls: number;        // Users who installed while logged in
  verifiedReviews: number;              // Reviews from verified users
  growthPattern: 'organic' | 'suspicious' | 'unknown';
  ageAtDownloadCount: {
    downloads: number;
    ageHours: number;
    verdict: 'normal' | 'suspicious';  // 4000 downloads in 1 hour = suspicious
  };
}

// Display to user:
// ✓ Verified Publisher (GitHub: @theonejvo)
// ✓ Source: github.com/theonejvo/wed-skill
// ⚠ 47 authenticated installs (not "4000+ downloads")
// ⚠ Published 2 days ago - limited track record
```

---

### 18. First-Execution Quarantine

**Concept**: Enhanced scrutiny for first-time skill execution with sandbox preview.

**How it works**:
- First execution of any skill runs in isolated sandbox
- Record all actions attempted without executing dangerous ones
- Show user complete action plan before real execution
- Subsequent executions can use cached approval

**Unique angle**: The 16 developers who ran /wed had no preview of what it would do. Give them one.

```typescript
// First execution triggers quarantine
const quarantineResult = await aegis.quarantineExecute({
  skill: 'wed',
  input: 'Build a rocket company',
  mode: 'preview',  // Don't actually execute
});

// Returns planned actions:
{
  plannedActions: [
    { type: 'network:external', target: 'clawdhub-skill.com/log', risk: 'HIGH' },
    { type: 'display', content: '[ASCII art reveal]', risk: 'LOW' },
    { type: 'generate', content: 'Business analysis...', risk: 'LOW' },
  ],
  verdict: 'REQUIRES_APPROVAL',
  risks: ['External network request to unknown domain'],
  prompt: 'This skill will make external network requests. Continue?',
}
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

### Phase 1: Supply Chain Defense (Inspired by ClawdHub Attack)
1. **Skill Transparency Scanner** - Show ALL files, flag suspicious patterns
2. **Capability Declaration & Enforcement** - Skills declare upfront, runtime enforces
3. **Trust Signal Verification** - Authenticated installs, verified publishers
4. **First-Execution Quarantine** - Sandbox preview before real execution

### Phase 2: Runtime Protection
5. **Contextual Permission Prompts** - Break permission fatigue with context
6. **Network Destination Reputation** - Typosquat detection, domain reputation
7. **Behavioral Fingerprinting** - Anomaly detection for agent actions
8. **Output CSP** - Policy-based output filtering (leverage existing OPA)

### Phase 3: Integrity & Trust
9. **Tool Definition Integrity** - Signed tool definitions
10. **Prompt Provenance** - Hash chain for inputs
11. **Memory Firewall** - Validated memory operations

### Phase 4: Advanced Protection
12. **Multi-Agent Consensus** - For high-risk actions
13. **Canary System** - Honeypot detection
14. **Cross-Agent Trust Boundaries** - Multi-agent security

---

## Integration Points with Anvil

| Aegis Feature | Anvil Component | Integration |
|---------------|-----------------|-------------|
| Skill Transparency Scanner | Gate Checks | Pre-execution gate with static analysis |
| Capability Declaration | Contracts | New manifest schema, validation |
| Trust Signal Verification | Evidence System | Verified install tracking |
| First-Execution Quarantine | Runtime + Snapshots | Sandbox mode with rollback |
| Contextual Permissions | Runtime | Enhanced permission UI |
| Network Reputation | Policy Engine (OPA) | Domain allowlist/blocklist policies |
| Behavioral Fingerprinting | Evidence System | Store behavioral baselines and anomaly logs |
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

- **[ClawdHub Supply Chain Attack - Jamieson O'Reilly](https://x.com/theonejvo/status/2015892980851474595)** - Primary inspiration for supply chain defence features
- [OpenAI: Hardening Atlas Against Prompt Injection](https://openai.com/index/hardening-atlas-against-prompt-injection/)
- [OpenAI: Understanding Prompt Injections](https://openai.com/index/prompt-injections/)
- [MCP Security Vulnerabilities](https://www.practical-devsecops.com/mcp-security-vulnerabilities/)
- [LLM Security Risks 2026](https://sombrainc.com/blog/llm-security-risks-2026)
- [Agentic AI Security Threats 2026](https://stellarcyber.ai/learn/agentic-ai-securiry-threats/)
