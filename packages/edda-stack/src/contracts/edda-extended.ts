/**
 * Extended Edda Contracts for Full System Implementation
 *
 * This extends the base EDDA-001 schema with additional contracts
 * for promotion, authority, enforcement, and lifecycle management.
 *
 * @module @anvil/edda-stack/contracts/edda-extended
 */

import type {
  MemoryId,
  MemoryType,
  MemoryStatus,
  EddaConfidenceLevel,
  ProposalId,
  Timestamp,
  ProvenanceChain,
} from './index.js';

// ============================================================================
// CORE EXTENSIONS
// ============================================================================

/**
 * Principal - represents an actor in the system
 */
export interface Principal {
  type: 'human' | 'agent' | 'team' | 'system';
  identifier: string; // user:alice, agent:anvil, team:platform, system:edda
}

/**
 * Scope specifier - where a memory applies
 */
export interface ScopeSpecifier {
  type: 'global' | 'workspace' | 'team' | 'project' | 'domain';
  identifier?: string; // team:platform, project:anvil
  exclusions?: string[]; // Explicit out-of-scope areas
}

// ============================================================================
// MEMORY OBJECT EXTENSIONS
// ============================================================================

/**
 * Authority metadata - who owns and can modify a memory
 */
export interface AuthorityMetadata {
  owner: Principal;
  reviewers: Principal[];
  visibility: 'public' | 'team' | 'private';
}

/**
 * Enforcement policy - how to enforce a memory
 */
export interface EnforcementPolicy {
  mode: 'advisory' | 'warning' | 'blocking' | 'audit_only';
  hooks: EnforcementHookType[];
  override_requires?: AuthorityLevel[];
}

export type EnforcementHookType =
  | 'pre_execution'
  | 'validation'
  | 'guidance'
  | 'post_execution'
  | 'approval_required';

export type AuthorityLevel =
  | 'system'
  | 'org_admin'
  | 'team_lead'
  | 'contributor'
  | 'agent'
  | 'readonly';

/**
 * Review policy - when to re-validate a memory
 */
export interface ReviewPolicy {
  strategy: 'none' | 'time_based' | 'event_triggered' | 'usage_based';
  interval_days?: number;
  trigger_events?: ReviewTrigger[];
  usage_threshold?: number;
  last_reviewed_at?: Timestamp;
}

export type ReviewTrigger =
  | 'supersession_proposed'
  | 'violation_threshold'
  | 'contradiction_detected'
  | 'staleness_detected';

/**
 * Extended memory object with full governance features
 */
export interface MemoryObjectExtended {
  // Core from EDDA-001
  id: MemoryId;
  type: MemoryType;
  status: MemoryStatus;
  statement: string;
  context: {
    when: string;
    why: string;
    conditions: string[];
    scope?: string; // Deprecated: use top-level scope
    tags: string[];
  };
  confidence: EddaConfidenceLevel;
  confidence_rationale?: string;
  provenance: ProvenanceChain;
  attribution: {
    promoted_by: string;
    promoted_at: Timestamp;
    reason: string;
  };
  evolution: {
    supersedes: MemoryId[];
    superseded_by?: MemoryId;
    retired_at?: Timestamp;
    retired_by?: string;
    retired_reason?: string;
  };
  created_at: Timestamp;
  updated_at: Timestamp;
  metadata: Record<string, unknown>;

  // Extensions
  scope: ScopeSpecifier;
  authority: AuthorityMetadata;
  enforcement: EnforcementPolicy;
  review_policy: ReviewPolicy;
}

// ============================================================================
// TYPE-SPECIFIC METADATA
// ============================================================================

export interface DecisionMetadata {
  alternatives_considered: string[];
  consequences: {
    expected: string[];
    observed?: string[];
  };
  decision_maker: Principal;
  irreversible: boolean;
}

export interface PatternMetadata {
  applies_to: string[];
  examples: CodeReference[];
  anti_patterns?: string[];
}

export interface CodeReference {
  file_path: string;
  line_range?: { start: number; end: number };
  commit?: string;
  description: string;
}

export interface WarningMetadata {
  severity: 'low' | 'medium' | 'high' | 'critical';
  incident_references?: string[];
  mitigation?: string;
}

export interface ConstraintMetadata {
  constraint_type: 'technical' | 'policy' | 'resource' | 'regulatory';
  workaround?: string;
  expiry_condition?: string;
}

export interface DoctrineMetadata {
  principle_category: 'engineering' | 'security' | 'operations' | 'cultural';
  ratified_by?: string;
  ratified_at?: Timestamp;
}

export interface LessonMetadata {
  incident_id?: string;
  cost?: {
    time?: string;
    money?: string;
    reputation?: string;
  };
  preventable: boolean;
}

// ============================================================================
// PROMOTION PIPELINE
// ============================================================================

export type PromotionRequestId = `EDDA-PR-${string}`;

export type PromotionStatus =
  | 'awaiting_review'
  | 'under_review'
  | 'approved'
  | 'rejected'
  | 'needs_revision';

export interface PromotionRequest {
  id: PromotionRequestId;
  proposal_id: ProposalId;
  status: PromotionStatus;

  // Transformation
  proposed_memory: MemoryObjectExtended;
  transformation_notes?: string;

  // Review
  reviewer?: Principal;
  review_started_at?: Timestamp;
  review_completed_at?: Timestamp;
  decision_rationale?: string;

  // Metadata
  requested_by: Principal;
  requested_at: Timestamp;
  priority: 'low' | 'normal' | 'high';
}

export interface PromotionReview {
  decision: 'approve' | 'reject' | 'revise';
  rationale: string;

  // Modifications
  modifications?: MemoryObjectPatch;

  // Context
  reviewer: Principal;
  reviewed_at: Timestamp;
  consulted_with?: Principal[];
  related_memories?: MemoryId[];
}

export interface MemoryObjectPatch {
  statement?: string;
  context?: {
    when?: string;
    why?: string;
    conditions?: string[];
    tags?: string[];
  };
  confidence?: EddaConfidenceLevel;
  confidence_rationale?: string;
  authority?: Partial<AuthorityMetadata>;
  scope?: Partial<ScopeSpecifier>;
  enforcement?: Partial<EnforcementPolicy>;
  review_policy?: Partial<ReviewPolicy>;
}

export interface PromotionDiff {
  proposal: {
    id: ProposalId;
    type: string;
    confidence: number;
    summary: string;
  };
  memory: MemoryObjectExtended;

  transformations: {
    type_mapping: string;
    confidence_mapping: string;
    scope_inference: string;
    enforcement_recommendation: string;
  };

  conflicts: ConflictDetection[];
  provenance_summary: string;
}

export interface ConflictDetection {
  memory_id: MemoryId;
  conflict_type: 'contradiction' | 'duplication' | 'supersession';
  severity: 'low' | 'medium' | 'high';
  explanation: string;
}

// ============================================================================
// REJECTION TRACKING
// ============================================================================

export type RejectionId = `EDDA-REJ-${string}`;

export type RejectionCategory =
  | 'insufficient_evidence'
  | 'incorrect_interpretation'
  | 'duplicate'
  | 'out_of_scope'
  | 'not_valuable'
  | 'conflicts_with_existing'
  | 'needs_more_observation';

export interface RejectionRecord {
  rejection_id: RejectionId;
  proposal_id: ProposalId;
  rejected_by: Principal;
  rejected_at: Timestamp;

  reason_category: RejectionCategory;
  explanation: string;

  // Learning signals
  false_positive: boolean;
  insufficient_evidence: boolean;
  duplicate_of?: MemoryId;
  policy_violation?: string;

  // Feedback loop
  ember_adjustment?: EmberFeedback;
}

export interface EmberFeedback {
  adjust_confidence_by: number; // -0.2 to +0.2
  adjust_factors: Array<{
    factor: string;
    new_weight: number;
  }>;
  rationale: string;
}

// ============================================================================
// VERSIONING
// ============================================================================

export interface MemoryVersion {
  version: number;
  memory_id: MemoryId;
  snapshot: MemoryObjectExtended;
  change_type: 'created' | 'updated' | 'superseded' | 'retired';
  changed_by: Principal;
  changed_at: Timestamp;
  change_reason: string;
  diff?: MemoryObjectPatch;
}

export interface EvolutionChain {
  root_memory_id: MemoryId;
  versions: MemoryVersion[];
  current_version: number;
  supersession_tree?: SupersessionNode[];
}

export interface SupersessionNode {
  memory_id: MemoryId;
  supersedes: MemoryId[];
  superseded_by?: MemoryId;
  active: boolean;
}

// ============================================================================
// AUTHORITY & RBAC
// ============================================================================

export type Permission =
  | 'read_public'
  | 'read_team'
  | 'read_all'
  | 'propose_memory'
  | 'review_promotions'
  | 'create_memory_direct'
  | 'update_memory'
  | 'retire_memory'
  | 'configure_enforcement'
  | 'manage_authority';

export interface Role {
  role_id: string;
  name: string;
  authority_level: AuthorityLevel;
  permissions: Permission[];
  scope_restriction?: ScopeSpecifier;
  principals: Principal[];
}

export interface AuthorityPolicy {
  level: AuthorityLevel;
  permissions: Permission[];
  constraints?: AuthorityConstraint[];
}

export interface AuthorityConstraint {
  type: 'scope_limited' | 'type_limited' | 'quota_limited' | 'approval_required';
  details: Record<string, unknown>;
}

export interface AgentTrustProfile {
  agent_id: string;
  trust_score: number; // 0.0 - 1.0

  // Performance
  proposals_submitted: number;
  proposals_approved: number;
  proposals_rejected: number;
  approval_rate: number;

  // Trust factors
  factors: TrustFactor[];

  // Permissions
  can_propose: boolean;
  confidence_adjustment: number;
  requires_human_review: boolean;

  last_updated: Timestamp;
}

export interface TrustFactor {
  factor: 'historical_accuracy' | 'source_quality' | 'reasoning_quality' | 'domain_expertise';
  weight: number;
  rationale: string;
}

// ============================================================================
// AUDIT TRAIL
// ============================================================================

export type AuditId = `EDDA-AUDIT-${string}`;

export type AuditOperation =
  | 'memory_created'
  | 'memory_updated'
  | 'memory_retired'
  | 'promotion_approved'
  | 'promotion_rejected'
  | 'authority_granted'
  | 'authority_revoked'
  | 'enforcement_configured'
  | 'memory_queried';

export interface AuditEntry {
  audit_id: AuditId;
  timestamp: Timestamp;
  principal: Principal;
  authority_level: AuthorityLevel;
  operation: AuditOperation;
  target_type: 'memory' | 'promotion' | 'authority' | 'config';
  target_id: string;
  changes?: Record<string, unknown>;
  rationale?: string;
  session_id?: string;
  ip_address?: string;
}

// ============================================================================
// QUERY & RETRIEVAL
// ============================================================================

export interface EddaQuery {
  // Filters
  types?: MemoryType[];
  statuses?: MemoryStatus[];
  scope?: ScopeSpecifier;
  tags?: string[];
  min_confidence?: EddaConfidenceLevel;
  owner?: Principal;
  visibility?: Array<'public' | 'team' | 'private'>;

  // Temporal
  created_after?: Timestamp;
  created_before?: Timestamp;
  updated_after?: Timestamp;

  // Text
  search_text?: string;

  // Pagination
  limit?: number;
  offset?: number;
  sort_by?: 'created_at' | 'updated_at' | 'confidence' | 'relevance';
  sort_order?: 'asc' | 'desc';
}

export interface EddaQueryResult {
  memories: MemoryObjectExtended[];
  total_count: number;
  page_info: PageInfo;

  // Aggregations
  facets?: {
    by_type?: Record<MemoryType, number>;
    by_status?: Record<MemoryStatus, number>;
    by_confidence?: Record<EddaConfidenceLevel, number>;
  };
}

export interface PageInfo {
  has_next_page: boolean;
  has_previous_page: boolean;
  start_cursor?: string;
  end_cursor?: string;
}

export interface SemanticQuery {
  query: string;
  scope?: ScopeSpecifier;
  limit?: number;
  filters?: Partial<EddaQuery>;
}

export interface SemanticResult extends Omit<EddaQueryResult, 'memories'> {
  memories: MemoryObjectWithRelevance[];
}

export interface MemoryObjectWithRelevance {
  memory: MemoryObjectExtended;
  relevance_score: number;
  match_explanation: string;
}

export interface ConflictQuery {
  memory_id?: MemoryId;
  statement?: string;
  scope?: ScopeSpecifier;
  conflict_types?: Array<'contradiction' | 'duplication' | 'supersession'>;
}

export interface ConflictResult {
  conflicts: ConflictDetection[];
  confidence: number;
}

export interface TemporalQuery {
  as_of: Timestamp;
  memory_id?: MemoryId;
  query?: EddaQuery;
}

export interface TemporalResult {
  memories: MemoryObjectExtended[];
  snapshot_info: {
    requested_time: Timestamp;
    actual_time: Timestamp;
    version_numbers: Record<MemoryId, number>;
  };
}

export interface ProvenanceQuery {
  memory_id: MemoryId;
  include_kindling?: boolean;
  include_versions?: boolean;
}

export interface ProvenanceResult {
  memory: MemoryObjectExtended;
  chain: ProvenanceChain;
  ember_proposal?: unknown; // CandidateProposal from ember.port
  kindling_observations?: unknown[]; // Observation[] from kindling.port
  versions?: MemoryVersion[];
  supersession_chain?: MemoryObjectExtended[];
  graph?: ProvenanceGraph;
}

export interface ProvenanceGraph {
  nodes: ProvenanceNode[];
  edges: ProvenanceEdge[];
}

export interface ProvenanceNode {
  id: string;
  type: 'observation' | 'proposal' | 'memory' | 'version';
  label: string;
  metadata: Record<string, unknown>;
}

export interface ProvenanceEdge {
  from: string;
  to: string;
  relationship: 'observed' | 'proposed' | 'promoted' | 'superseded' | 'versioned';
}

// ============================================================================
// ENFORCEMENT HOOKS
// ============================================================================

export type HookId = `EDDA-HOOK-${string}`;

export type HookEvent =
  | 'plan_created'
  | 'action_about_to_execute'
  | 'file_about_to_change'
  | 'command_about_to_run'
  | 'gate_evaluated'
  | 'human_approval_requested';

export interface EnforcementHook {
  hook_id: HookId;
  type: EnforcementHookType;
  name: string;
  description: string;
  trigger: HookTrigger;
  applicable_memories: MemoryMatcher;
  action: HookAction;
  enabled: boolean;
  priority: number;
}

export interface HookTrigger {
  event: HookEvent;
  conditions?: TriggerCondition[];
}

export interface TriggerCondition {
  field: string;
  operator: '==' | '!=' | 'contains' | 'matches';
  value: unknown;
}

export interface MemoryMatcher {
  types?: MemoryType[];
  tags?: string[];
  scope?: ScopeSpecifier;
  enforcement_modes?: Array<'advisory' | 'warning' | 'blocking' | 'audit_only'>;
}

export interface HookAction {
  mode: 'block' | 'warn' | 'suggest' | 'log' | 'require_approval';
  message_template: string;
  alternatives?: string[];
  approval_required_from?: AuthorityLevel[];
}

export interface PreExecutionCheck {
  action: ActionContext;
  plan?: PlanContext;
  check_type: 'policy' | 'constraint' | 'warning';
  result: CheckResult;
}

export interface ActionContext {
  action_type: string;
  action_details: Record<string, unknown>;
  scope: ScopeSpecifier;
  principal?: Principal;
}

export interface PlanContext {
  plan_id: string;
  intent: string;
  technologies?: string[];
}

export interface CheckResult {
  allowed: boolean;
  violations: Violation[];
  warnings: Warning[];
  suggestions: Suggestion[];
}

export interface Violation {
  memory_id: MemoryId;
  memory: MemoryObjectExtended;
  violation_type: 'hard_constraint' | 'policy_violation' | 'blocked_pattern';
  message: string;
  can_override: boolean;
  override_requires?: AuthorityLevel[];
}

export interface Warning {
  memory_id: MemoryId;
  memory: MemoryObjectExtended;
  severity: 'low' | 'medium' | 'high';
  message: string;
  recommendation?: string;
}

export interface Suggestion {
  memory_id: MemoryId;
  memory: MemoryObjectExtended;
  suggestion_type: 'alternative_approach' | 'best_practice' | 'reference';
  message: string;
}

export interface GuidanceRequest {
  context: PlanningContext;
  limit?: number;
}

export interface PlanningContext {
  intent: string;
  scope: ScopeSpecifier;
  technologies?: string[];
  current_phase?: 'planning' | 'implementing' | 'testing';
}

export interface GuidanceResponse {
  relevant_memories: RelevantMemory[];
  patterns_to_consider: MemoryObjectExtended[];
  warnings_to_avoid: MemoryObjectExtended[];
  lessons_learned: MemoryObjectExtended[];
}

export interface RelevantMemory {
  memory: MemoryObjectExtended;
  relevance_score: number;
  why_relevant: string;
  when_to_apply: string;
}

// ============================================================================
// LIFECYCLE MANAGEMENT
// ============================================================================

export type DeprecationReason = 'superseded' | 'obsolete' | 'incorrect' | 'consolidated';

export interface DeprecationRequest {
  memory_id: MemoryId;
  reason: DeprecationReason;
  proposed_by: Principal;
  superseded_by?: MemoryId;
  migration_guide?: string;
  deprecation_date: Timestamp;
  retirement_date: Timestamp;
  estimated_impact: ImpactAssessment;
}

export interface ImpactAssessment {
  affected_systems: string[];
  affected_teams: string[];
  dependent_memories: MemoryId[];
  enforcement_hooks_count: number;
  estimated_effort: 'low' | 'medium' | 'high';
}

export interface ReviewSchedule {
  memory_id: MemoryId;
  review_policy: ReviewPolicy;
  next_review_due: Timestamp;
  review_history: ReviewEvent[];
  staleness_score: number;
  staleness_factors: StalenessFactor[];
}

export interface ReviewEvent {
  reviewed_at: Timestamp;
  reviewed_by: Principal;
  outcome: ReviewOutcome;
  notes: string;
}

export type ReviewOutcome = 'reaffirmed' | 'updated' | 'extended_review' | 'deprecated';

export interface StalenessFactor {
  factor: 'time_since_creation' | 'time_since_last_use' | 'contradicted_by_new_data' | 'unused';
  weight: number;
  contribution: number;
}

export interface SupersessionRequest {
  old_memory_id: MemoryId;
  new_memory: Partial<MemoryObjectExtended>;
  supersession_type: 'replacement' | 'refinement' | 'consolidation';
  relationship: string;
  transition_plan?: string;
  backward_compatibility?: boolean;
  cutover_date?: Timestamp;
}

export interface SupersessionResult {
  old_memory: MemoryObjectExtended;
  new_memory: MemoryObjectExtended;
  evolution_link: {
    supersedes: MemoryId[];
    superseded_by?: MemoryId;
  };
  updated_references: ReferenceUpdate[];
  enforcement_migrations: EnforcementMigration[];
}

export interface ReferenceUpdate {
  referencing_memory_id: MemoryId;
  field: string;
  old_value: MemoryId;
  new_value: MemoryId;
}

export interface EnforcementMigration {
  hook_id: HookId;
  old_memory_id: MemoryId;
  new_memory_id: MemoryId;
  requires_reconfiguration: boolean;
}

// ============================================================================
// AGENT INTERACTION
// ============================================================================

export interface AgentCapabilities {
  agent_id: string;
  can_read_public: boolean;
  can_read_team_scoped: boolean;
  can_read_private: boolean;
  can_propose_memory: boolean;
  can_annotate: boolean;
  can_ratify: boolean;
  can_delete: boolean;
  scope_access: ScopeSpecifier[];
  trust_profile: AgentTrustProfile;
}

export interface AgentProposal {
  agent_id: string;
  agent_session_id: string;
  proposed_memory: Partial<MemoryObjectExtended>;
  evidence: Evidence[];
  reasoning: string;
  agent_confidence: number;
  confidence_factors: ConfidenceFactor[];
}

export interface Evidence {
  source_type: 'observation' | 'pattern_match' | 'semantic_similarity' | 'human_feedback';
  source_id: string;
  weight: number;
  summary: string;
}

export interface ConfidenceFactor {
  factor: string;
  value: number;
  weight: number;
}

export interface AgentAction {
  action_id: string;
  agent_id: string;
  action_type: string;
  action_details: Record<string, unknown>;
  cited_memories: MemoryCitation[];
  reasoning: string;
}

export interface MemoryCitation {
  memory_id: MemoryId;
  memory: MemoryObjectExtended;
  influence_type: 'constraint' | 'guidance' | 'pattern' | 'requirement';
  application: string;
}

export interface RejectionFeedback {
  proposal_id: ProposalId;
  rejection: RejectionRecord;
  what_was_wrong: string;
  how_to_improve: string[];
  similar_accepted_examples?: MemoryId[];
  trust_impact: number;
  new_trust_score: number;
}

// ============================================================================
// META-CAPABILITIES
// ============================================================================

export interface Contradiction {
  memory_a: MemoryObjectExtended;
  memory_b: MemoryObjectExtended;
  contradiction_type: 'direct' | 'implicit' | 'conditional';
  severity: 'low' | 'medium' | 'high' | 'critical';
  explanation: string;
  resolution_suggestions: ResolutionStrategy[];
}

export type ResolutionStrategy =
  | { type: 'supersede'; supersede_id: MemoryId; keep_id: MemoryId }
  | { type: 'scope_restriction'; narrow_scope_of: MemoryId }
  | { type: 'add_condition'; add_to: MemoryId; condition: string }
  | { type: 'merge'; into_new: Partial<MemoryObjectExtended> };

export interface KnowledgeGraph {
  nodes: KnowledgeNode[];
  edges: KnowledgeEdge[];
  clusters: KnowledgeCluster[];
  critical_paths: CriticalPath[];
}

export interface KnowledgeNode {
  id: MemoryId;
  memory: MemoryObjectExtended;
  in_degree: number;
  out_degree: number;
  centrality: number;
  cluster_id?: string;
  tags: string[];
}

export interface KnowledgeEdge {
  from: MemoryId;
  to: MemoryId;
  relationship: EdgeRelationship;
  weight: number;
}

export type EdgeRelationship =
  | 'supersedes'
  | 'depends_on'
  | 'contradicts'
  | 'supports'
  | 'related_to';

export interface KnowledgeCluster {
  cluster_id: string;
  name: string;
  memories: MemoryId[];
  centroid_tags: string[];
  size: number;
}

export interface CriticalPath {
  path: MemoryId[];
  path_type: 'decision_chain' | 'evolution_chain' | 'dependency_chain';
  importance: number;
}

export interface ImpactAnalysis {
  memory: MemoryObjectExtended;
  proposed_change: MemoryObjectPatch;
  affected_memories: MemoryId[];
  affected_hooks: EnforcementHook[];
  affected_agents: string[];
  propagation: {
    direct_dependencies: MemoryId[];
    indirect_dependencies: MemoryId[];
    total_affected: number;
  };
  risk_level: 'low' | 'medium' | 'high' | 'critical';
  risk_factors: RiskFactor[];
  recommended_actions: string[];
}

export interface RiskFactor {
  factor: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  explanation: string;
}
