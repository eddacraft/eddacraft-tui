# Domain, Command, Event, and Projection Model

## Purpose

This document proposes the app-layer domain model and interaction contracts for a converged APS and anvil product.

It is deliberately framework-neutral. Dioxus, Tauri, React, Ratatui, browser clients, MCP tools, and CLI commands should all sit outside this model.

---

## 1. Modelling principles

1. **Intent is not execution.** APS work describes what is authorised; workspaces and runs describe attempts to satisfy it.
2. **A workspace is not a task.** It is an environment used by one or more attempts.
3. **A session is not a tab.** A session is durable activity; a tab is one visual rendering of it.
4. **Evidence is not merely output.** It must be attributable and related to a claim or decision.
5. **Commands request change. Events record accepted facts. Projections describe current views.**
6. **The daemon owns durable runtime behaviour.** Clients may cache and render but do not become independent authorities.
7. **Repository documents may remain authoritative.** The daemon must reconcile rather than silently replace them.
8. **Modules own their aggregates.** Cross-module effects happen through commands, events, and stable ports.

---

## 2. Bounded contexts

### 2.1 Identity and capability

Owns:

- actors;
- humans;
- agents;
- service identities;
- roles;
- capabilities;
- credentials and leases;
- actor provenance.

### 2.2 Project and repository

Owns:

- logical projects;
- repository registrations;
- physical checkouts;
- repository identity grouping;
- environment associations;
- source-control provider references.

### 2.3 APS planning

Owns:

- plan/index;
- module;
- work item;
- decision;
- issue or question;
- action plan;
- dependencies;
- status;
- validation requirement;
- learning;
- planning warnings;
- planning document mutations.

### 2.4 Workspace

Owns:

- workspace;
- repository attachment;
- worktree or checkout;
- environment;
- layout reference;
- archive state;
- links to plan items and runs.

### 2.5 Run and session

Owns:

- run;
- session;
- provider runtime instance;
- process state;
- input and output stream;
- tool calls;
- run timeline;
- terminal attachment.

### 2.6 Governance and policy

Owns:

- policy evaluation;
- approval request;
- approval response;
- permission decision;
- gate decision;
- exception;
- enforcement mode.

### 2.7 Evidence and verification

Owns:

- evidence record;
- evidence bundle;
- claim;
- verification attempt;
- verification result;
- provenance chain;
- artefact references.

### 2.8 Attention and notification

Owns projections and preferences for:

- attention item;
- unread state;
- settlement;
- notification delivery;
- dismissal or snooze state.

It must not own the source domain state that caused attention.

### 2.9 UI workspace state

Owns durable user presentation preferences such as:

- open tabs;
- pinned tabs;
- tab order;
- split-pane layout;
- selected project scope;
- saved filters;
- theme and density.

It does not own runs, sessions, planning status, or process lifecycle.

---

## 3. Core entities and aggregates

## 3.1 Actor

```rust
pub struct ActorRef {
    pub actor_id: ActorId,
    pub actor_type: ActorType,
    pub display_name: String,
    pub organisation_id: Option<OrganisationId>,
    pub agent_id: Option<AgentId>,
}
```

Actor types may include:

```text
human
agent
service
system
external
```

An actor reference must be included in every mutating command and resulting auditable event.

---

## 3.2 Logical project

A logical project groups the planning, repositories, workspaces, and evidence for one product or delivery context.

```rust
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub repositories: Vec<RepositoryRef>,
    pub planning_roots: Vec<PlanningRootRef>,
    pub status: ProjectStatus,
    pub version: AggregateVersion,
}
```

A physical checkout is not a project. Multiple checkouts of the same repository may appear beneath one logical project.

---

## 3.3 APS plan

```rust
pub struct Plan {
    pub id: PlanId,
    pub project_id: ProjectId,
    pub title: String,
    pub problem: String,
    pub success_criteria: Vec<Criterion>,
    pub modules: Vec<ModuleRef>,
    pub risks: Vec<RiskRef>,
    pub source: PlanningSource,
    pub version: AggregateVersion,
}
```

The plan should preserve a source reference to its Markdown document and content version or hash.

---

## 3.4 APS module

```rust
pub struct Module {
    pub id: ModuleId,
    pub plan_id: PlanId,
    pub title: String,
    pub purpose: String,
    pub scope: ScopeDefinition,
    pub constraints: Vec<Constraint>,
    pub work_items: Vec<WorkItemRef>,
    pub status: ModuleStatus,
    pub source: PlanningSource,
    pub version: AggregateVersion,
}
```

---

## 3.5 APS work item

```rust
pub struct WorkItem {
    pub id: WorkItemId,
    pub module_id: ModuleId,
    pub title: String,
    pub intent: String,
    pub expected_outcomes: Vec<Outcome>,
    pub validation: Vec<ValidationRequirement>,
    pub dependencies: Vec<WorkItemRef>,
    pub non_scope: Vec<String>,
    pub status: WorkItemStatus,
    pub authority: AuthorityState,
    pub source: PlanningSource,
    pub version: AggregateVersion,
}
```

Suggested statuses:

```text
Draft
Shaping
DecisionNeeded
AwaitingApproval
Ready
Running
Verification
Blocked
Complete
Cancelled
```

The exact stored APS vocabulary may differ. The projection layer may map several source statuses into common application lanes.

### Invariants

- A work item cannot become `Ready` while required dependencies are incomplete.
- A work item cannot become `Running` without execution authority.
- A work item cannot become `Complete` without satisfying the configured completion policy.
- A UI drag is not itself authority; only the accepted transition command changes status.

---

## 3.6 Decision

```rust
pub struct Decision {
    pub id: DecisionId,
    pub project_id: ProjectId,
    pub plan_id: Option<PlanId>,
    pub title: String,
    pub question: String,
    pub options: Vec<DecisionOption>,
    pub outcome: Option<DecisionOutcome>,
    pub status: DecisionStatus,
    pub decided_by: Option<ActorRef>,
    pub decided_at: Option<Timestamp>,
    pub source: PlanningSource,
    pub version: AggregateVersion,
}
```

A durable architectural decision is different from a runtime approval request.

---

## 3.7 Workspace

```rust
pub struct Workspace {
    pub id: WorkspaceId,
    pub project_id: ProjectId,
    pub purpose: WorkspacePurpose,
    pub work_item_id: Option<WorkItemId>,
    pub repositories: Vec<WorkspaceRepository>,
    pub isolation: IsolationMode,
    pub status: WorkspaceStatus,
    pub active_run_ids: Vec<RunId>,
    pub archived_at: Option<Timestamp>,
    pub version: AggregateVersion,
}
```

Isolation modes:

```text
SharedCheckout
GitWorktree
RemoteEnvironment
ExternallyManaged
EphemeralSandbox
```

### Invariants

- Destructive resource removal must not erase durable history.
- A workspace may be archived while its runs, evidence, and provenance remain inspectable.
- Shared checkout execution may require stronger approval or exclusive locking.

---

## 3.8 Run

```rust
pub struct Run {
    pub id: RunId,
    pub project_id: ProjectId,
    pub workspace_id: Option<WorkspaceId>,
    pub work_item_id: Option<WorkItemId>,
    pub run_type: RunType,
    pub status: RunStatus,
    pub actor: ActorRef,
    pub policy_context: PolicyContextRef,
    pub session_ids: Vec<SessionId>,
    pub evidence_bundle_id: EvidenceBundleId,
    pub started_at: Timestamp,
    pub finished_at: Option<Timestamp>,
    pub version: AggregateVersion,
}
```

Run types may include:

```text
Implementation
Review
Verification
Scan
Audit
Repair
Research
AdHoc
```

### Invariants

- Every run has an attributable initiating actor.
- A governed run records the policy context used at admission.
- A run may succeed operationally while failing verification.
- A run can be planless.

---

## 3.9 Session

```rust
pub struct Session {
    pub id: SessionId,
    pub run_id: RunId,
    pub workspace_id: Option<WorkspaceId>,
    pub session_type: SessionType,
    pub provider: ProviderRef,
    pub durable_handle: Option<String>,
    pub status: SessionStatus,
    pub started_at: Timestamp,
    pub last_activity_at: Timestamp,
    pub finished_at: Option<Timestamp>,
    pub version: AggregateVersion,
}
```

Session types may include:

```text
Agent
Terminal
HumanReview
ExternalJob
Tool
CI
```

A provider process may be replaced or reconnected without changing the durable session identity where continuity is possible.

---

## 3.10 Evidence record

```rust
pub struct EvidenceRecord {
    pub id: EvidenceId,
    pub bundle_id: EvidenceBundleId,
    pub evidence_type: EvidenceType,
    pub source: EvidenceSource,
    pub actor: ActorRef,
    pub claim_refs: Vec<ClaimRef>,
    pub content_ref: EvidenceContentRef,
    pub integrity: IntegrityMetadata,
    pub correlation_id: CorrelationId,
    pub recorded_at: Timestamp,
}
```

Evidence content should be referenced or stored according to size, sensitivity, and retention policy rather than embedded indiscriminately in events.

---

## 3.11 Evidence bundle

```rust
pub struct EvidenceBundle {
    pub id: EvidenceBundleId,
    pub project_id: ProjectId,
    pub work_item_id: Option<WorkItemId>,
    pub run_id: Option<RunId>,
    pub evidence_ids: Vec<EvidenceId>,
    pub claims: Vec<Claim>,
    pub verification_status: VerificationStatus,
    pub version: AggregateVersion,
}
```

---

## 3.12 Approval request

```rust
pub struct ApprovalRequest {
    pub id: ApprovalRequestId,
    pub run_id: RunId,
    pub requested_by: ActorRef,
    pub action: ProposedAction,
    pub scope: CapabilityScope,
    pub policy_basis: Vec<PolicyRef>,
    pub expires_at: Option<Timestamp>,
    pub status: ApprovalStatus,
    pub version: AggregateVersion,
}
```

An approval response must be bound to the exact request version and action digest.

---

## 3.13 Gate decision

```rust
pub struct GateDecision {
    pub id: GateDecisionId,
    pub gate_type: GateType,
    pub target: GateTarget,
    pub evidence_refs: Vec<EvidenceId>,
    pub policy_refs: Vec<PolicyRef>,
    pub outcome: GateOutcome,
    pub conditions: Vec<DecisionCondition>,
    pub decided_by: DecisionActor,
    pub decided_at: Timestamp,
}
```

---

## 3.14 Attention item

Attention is preferably a projection rather than a primary aggregate.

```rust
pub struct AttentionItemProjection {
    pub id: AttentionItemId,
    pub project_id: ProjectId,
    pub target: AttentionTarget,
    pub reason: AttentionReason,
    pub severity: AttentionSeverity,
    pub title: String,
    pub unread: bool,
    pub settled: bool,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
```

A small durable preference record may track read, snoozed, dismissed, or settled presentation state.

---

## 3.15 Tab and layout state

```rust
pub struct TabState {
    pub id: TabId,
    pub owner_actor_id: ActorId,
    pub window_id: WindowId,
    pub title: String,
    pub kind: TabKind,
    pub target: TabTarget,
    pub pinned: bool,
    pub order: u32,
}

pub struct LayoutState {
    pub owner_actor_id: ActorId,
    pub window_id: WindowId,
    pub layout: LayoutNode,
    pub version: AggregateVersion,
}
```

A tab target might point to a workspace overview, plan, session renderer, diff, evidence bundle, or system view.

Closing a tab never implies stopping its target.

---

## 4. Command model

## 4.1 Command envelope

```rust
pub struct CommandEnvelope<T> {
    pub command_id: CommandId,
    pub command_type: String,
    pub command_version: u16,
    pub actor: ActorRef,
    pub target: CommandTarget,
    pub payload: T,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CausationId>,
    pub expected_version: Option<AggregateVersion>,
    pub idempotency_key: Option<IdempotencyKey>,
    pub source_surface: SourceSurface,
    pub requested_at: Timestamp,
}
```

Source surfaces may include:

```text
Native
Web
Tray
CLI
TUI
MCP
Automation
System
ExternalIntegration
```

## 4.2 Command result

```rust
pub enum CommandResult<T> {
    Completed(T),
    Accepted { operation_id: OperationId },
    Rejected(CommandRejection),
    Conflict(CommandConflict),
    Unavailable(CommandUnavailable),
    Failed(StructuredError),
}
```

A rejected command is not an exceptional transport failure. It should carry user-actionable reasons.

## 4.3 Planning command catalogue

```text
planning.register_project
planning.refresh
planning.create_plan
planning.update_plan
planning.create_module
planning.update_module
planning.create_work_item
planning.update_work_item
planning.transition_work_item
planning.authorise_work_item
planning.start_work_item
planning.complete_work_item
planning.record_learning
planning.create_decision
planning.resolve_decision
planning.record_issue
planning.export
planning.audit
```

## 4.4 Workspace command catalogue

```text
workspace.create
workspace.attach_repository
workspace.create_worktree
workspace.attach_environment
workspace.open
workspace.pause
workspace.resume
workspace.archive
workspace.abandon
workspace.restore
```

## 4.5 Run and session command catalogue

```text
run.create
run.admit
run.start
run.pause
run.resume
run.complete
run.fail
run.abandon
session.start
session.send_input
session.interrupt
session.stop
session.restart
session.detach
session.reattach
```

## 4.6 Governance command catalogue

```text
policy.evaluate
approval.request
approval.respond
gate.evaluate
gate.record_decision
exception.request
exception.revoke
```

## 4.7 Evidence command catalogue

```text
evidence.record
evidence.attach
evidence.create_bundle
evidence.submit_bundle
verification.start
verification.record_result
```

## 4.8 Presentation command catalogue

Presentation commands may persist user preferences but must remain clearly separated:

```text
ui.open_tab
ui.close_tab
ui.pin_tab
ui.reorder_tabs
ui.update_layout
ui.set_filter
ui.mark_attention_read
ui.settle_attention
```

---

## 5. Event model

## 5.1 Event envelope

```rust
pub struct EventEnvelope<T> {
    pub event_id: EventId,
    pub event_type: String,
    pub event_version: u16,
    pub aggregate_id: AggregateId,
    pub aggregate_version: AggregateVersion,
    pub actor: ActorRef,
    pub payload: T,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CausationId>,
    pub occurred_at: Timestamp,
}
```

## 5.2 Example events

### Planning

```text
planning.project_registered
planning.documents_refreshed
planning.work_item_created
planning.work_item_authorised
planning.work_item_status_changed
planning.learning_recorded
planning.decision_resolved
planning.conflict_detected
```

### Workspace

```text
workspace.created
workspace.repository_attached
workspace.worktree_created
workspace.status_changed
workspace.archived
workspace.resource_cleanup_failed
```

### Run and session

```text
run.created
run.admitted
run.started
run.paused
run.completed
run.failed
session.started
session.connected
session.output_appended
session.waiting_for_input
session.completed
session.disconnected
```

### Governance and evidence

```text
approval.requested
approval.responded
policy.evaluated
evidence.recorded
evidence.bundle_submitted
verification.completed
gate.decision_recorded
exception.granted
```

### System

```text
system.daemon_started
system.daemon_degraded
system.module_failed
system.migration_completed
system.adapter_disconnected
```

---

## 6. Query and projection model

Queries must be read-only and purpose-specific.

## 6.1 Project catalogue projection

```rust
pub struct ProjectCatalogueProjection {
    pub projects: Vec<ProjectSummary>,
    pub selected_project_id: Option<ProjectId>,
    pub total_attention_count: u32,
}
```

## 6.2 Operational inbox projection

```rust
pub struct OperationalInboxProjection {
    pub scope: InboxScope,
    pub active_items: Vec<OperationalRow>,
    pub settled_items: Vec<OperationalRow>,
    pub filters: AvailableFilters,
    pub unread_count: u32,
}
```

Rows may project workspaces, runs, sessions, approvals, or verification states into one operational form.

## 6.3 Planning board projection

```rust
pub struct PlanningBoardProjection {
    pub project_id: ProjectId,
    pub lanes: Vec<PlanningLane>,
    pub transition_rules: Vec<TransitionRuleSummary>,
    pub warnings: Vec<PlanningWarning>,
}
```

The projection should include permissible transitions or enough information for the UI to preview them, while the daemon remains authoritative.

## 6.4 Plan explorer projection

```rust
pub struct PlanExplorerProjection {
    pub plans: Vec<PlanTreeNode>,
    pub selected: Option<PlanningTarget>,
    pub dependency_summary: DependencySummary,
}
```

## 6.5 Workspace projection

```rust
pub struct WorkspaceProjection {
    pub workspace: WorkspaceSummary,
    pub repositories: Vec<WorkspaceRepositorySummary>,
    pub runs: Vec<RunSummary>,
    pub sessions: Vec<SessionSummary>,
    pub change_summary: ChangeSummary,
    pub evidence_summary: EvidenceBundleSummary,
    pub available_actions: Vec<AvailableAction>,
}
```

## 6.6 Run timeline projection

```rust
pub struct RunTimelineProjection {
    pub run: RunSummary,
    pub entries: Vec<RunTimelineEntry>,
    pub current_attention: Vec<AttentionReason>,
}
```

## 6.7 Evidence and gate projection

```rust
pub struct EvidenceReviewProjection {
    pub target: EvidenceTarget,
    pub claims: Vec<ClaimProjection>,
    pub evidence: Vec<EvidenceSummary>,
    pub gate_decisions: Vec<GateDecisionSummary>,
    pub gaps: Vec<EvidenceGap>,
}
```

## 6.8 System projection

```rust
pub struct SystemStatusProjection {
    pub daemon: DaemonStatus,
    pub modules: Vec<ModuleStatus>,
    pub adapters: Vec<AdapterStatus>,
    pub migrations: MigrationStatus,
    pub active_jobs: Vec<JobSummary>,
    pub recent_errors: Vec<ErrorSummary>,
}
```

---

## 7. Repository authority and file mutation

APS may remain Markdown-authoritative. The app layer should therefore use a deliberate write model.

### Read path

```text
APS files
  → bounded loader
    → canonical parser
      → domain model
        → indexed projection
```

### Write path

```text
transition or edit command
  → validate actor and expected version
    → apply domain rule
      → generate deterministic document patch
        → preview where required
          → atomic file write
            → emit event
              → refresh projections
```

### Required safeguards

- content hash or file version check;
- atomic replace;
- symlink and containment policy;
- backup where appropriate;
- no rewriting unrelated prose without need;
- structured conflict response;
- post-write parse and validation;
- actor and command provenance.

---

## 8. Cross-context interaction examples

## 8.1 Work item becomes Ready

```text
planning.transition_work_item
  → APS validates dependencies and document version
  → planning.work_item_status_changed
  → planning board projection refreshes
  → eligibility projection may add the item
```

No workspace is created automatically unless a policy or explicit automation subscribes to the event.

## 8.2 Workspace created from a work item

```text
workspace.create(work_item_id)
  → planning context queried
  → workspace aggregate created
  → repository adapter creates worktree if requested
  → workspace.created
  → work item projection gains workspace link
  → operational inbox gains active row
```

## 8.3 Agent asks for a restricted capability

```text
session tool request
  → policy.evaluate
  → approval.requested
  → run pauses at the action boundary
  → attention item appears
  → approval.respond
  → exact action digest validated
  → run resumes or action is denied
```

## 8.4 Completion and verification

```text
run completes
  → evidence bundle submitted
  → verification run starts
  → verification result recorded
  → gate decision recorded
  → planning.complete_work_item may be accepted or rejected
```

Operational completion and APS completion are related but not identical.

---

## 9. Idempotency, concurrency, and retries

### 9.1 Expected versions

Commands that update aggregates or repository documents should include an expected version or content hash.

### 9.2 Idempotency

Commands retried after transport interruption must use idempotency keys. The daemon should return the original result when the command was already accepted.

### 9.3 Long-running commands

Commands such as workspace creation, agent start, export, audit, or verification may return an operation identifier and emit progress events.

### 9.4 Conflict handling

The command layer must distinguish:

- domain rule rejection;
- policy rejection;
- aggregate version conflict;
- repository file conflict;
- resource collision;
- unavailable provider;
- transport failure.

Clients should not display every failure as a generic error toast.

---

## 10. Capability model

A capability is a durable named permission or feature contract.

```rust
pub struct CapabilityGrant {
    pub capability: CapabilityId,
    pub scope: CapabilityScope,
    pub actor_id: ActorId,
    pub source: CapabilitySource,
    pub constraints: Vec<CapabilityConstraint>,
    pub expires_at: Option<Timestamp>,
}
```

Capabilities may be granted by:

- local installation mode;
- licence or entitlement;
- organisation policy;
- project role;
- temporary approval;
- daemon configuration;
- runtime availability.

The UI may use capabilities to compose available actions, but the daemon must enforce them.

---

## 11. Public and private dependency boundaries

Suggested direction:

```text
public shared primitives
  → APS domain and application
    → standalone APS CLI/TUI
    → anvil integration adapters
      → proprietary governance/runtime/desktop modules
```

APS may depend on public shared command, event, identity, repository, and projection primitives.

APS must not depend on:

- proprietary policy modules;
- licensing;
- enterprise control-plane code;
- hosted-only storage;
- desktop framework packages;
- private integrations.

anvil may depend on APS through public interfaces and events.

---

## 12. Recommended first contracts to stabilise

Before major UI work, stabilise:

1. identifier newtypes;
2. actor and source-surface types;
3. command envelope and structured result;
4. event envelope;
5. capability identifiers and scope;
6. project, work-item, workspace, run, and session references;
7. planning board projection;
8. operational inbox projection;
9. workspace projection;
10. system status projection;
11. structured errors;
12. API contract version negotiation.

These are sufficient to build the first vertical slice without pretending the entire domain is already final.
