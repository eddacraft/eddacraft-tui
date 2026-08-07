# Reference API and Data Models

## 1. Purpose and status

This document provides illustrative Rust and wire-format sketches to make the architecture concrete.

These examples are **not frozen public API**. They should inform spikes and discussion, then be replaced by validated designs.

## 2. Core identifiers

```rust
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EntityId(pub uuid::Uuid);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(pub uuid::Uuid);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CommandRunId(pub uuid::Uuid);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NodeId(pub std::sync::Arc<str>);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ProjectionId(pub uuid::Uuid);
```

IDs must not encode tree position or renderer geometry.

## 3. Runtime and entities

```rust
pub struct Runtime {
    // Private arenas, scheduler, action registry, sessions and observers.
}

pub struct Entity<T> {
    id: EntityId,
    _marker: std::marker::PhantomData<T>,
}

pub struct WeakEntity<T> {
    id: EntityId,
    _marker: std::marker::PhantomData<T>,
}

pub trait EntityState: Send + Sync + 'static {
    fn semantics(&self, cx: &SemanticContext<'_>) -> SemanticNode;
}

impl Runtime {
    pub fn create<T: EntityState>(&self, state: T) -> Entity<T>;

    pub fn read<T, R>(
        &self,
        entity: &Entity<T>,
        f: impl FnOnce(&T) -> R,
    ) -> Result<R, EntityError>;

    pub fn update<T, R>(
        &self,
        entity: &Entity<T>,
        cause: MutationCause,
        f: impl FnOnce(&mut T, &mut EntityContext<T>) -> R,
    ) -> Result<R, EntityError>;
}
```

Alternative ownership models should be tested before adoption.

## 4. Mutation attribution

```rust
pub enum MutationCause {
    Action(ActionInvocationId),
    CommandEvent(CommandRunId),
    Resource(ResourceId),
    System(SystemCause),
    Replay(ReplayEventId),
}

pub struct MutationRecord {
    pub transaction_id: uuid::Uuid,
    pub cause: MutationCause,
    pub changed_entities: Vec<EntityId>,
    pub invalidations: Vec<Invalidation>,
    pub diagnostics: Vec<Diagnostic>,
}
```

## 5. Typed actions

```rust
pub trait Action: serde::Serialize + Send + Sync + 'static {
    const NAME: &'static str;

    fn metadata() -> ActionMetadata;
}

pub struct ActionMetadata {
    pub label: &'static str,
    pub description: &'static str,
    pub risk: Risk,
    pub required_permissions: &'static [Permission],
    pub default_bindings: &'static [BindingHint],
}

#[derive(Clone, Copy, Debug)]
pub enum Risk {
    ReadOnly,
    LocalMutation,
    ExternalMutation,
    Destructive,
    Privileged,
}

pub struct ActionEnvelope<A: Action> {
    pub invocation_id: uuid::Uuid,
    pub session_id: SessionId,
    pub actor: Actor,
    pub target: EntityId,
    pub action: A,
    pub source: InvocationSource,
}
```

Example:

```rust
#[derive(serde::Serialize)]
pub struct PromoteToWorkspace {
    pub node: NodeId,
    pub preferred_region: Option<RegionIntent>,
}

impl Action for PromoteToWorkspace {
    const NAME: &'static str = "workspace.promote";

    fn metadata() -> ActionMetadata {
        ActionMetadata {
            label: "Open in workspace",
            description: "Promote this item into a detailed workspace",
            risk: Risk::ReadOnly,
            required_permissions: &[],
            default_bindings: &[BindingHint::Key("enter")],
        }
    }
}
```

## 6. Commands

```rust
pub trait Command: Send + Sync + 'static {
    type Input: CommandInput;
    type Output: CommandOutput;
    type Event: CommandEvent;
    type Error: std::error::Error + Send + Sync + 'static;

    const NAME: &'static str;
    const VERSION: u32;

    fn metadata() -> CommandMetadata;

    fn validate(
        input: &Self::Input,
        cx: &ValidationContext<'_>,
    ) -> Result<(), Diagnostics>;

    fn preview(
        input: &Self::Input,
        cx: &CommandContext<'_>,
    ) -> impl std::future::Future<Output = Result<Option<Preview>, Self::Error>> + Send;

    fn execute(
        input: Self::Input,
        cx: CommandContext<'_>,
        events: EventSink<Self::Event>,
    ) -> impl std::future::Future<Output = Result<Self::Output, Self::Error>> + Send;
}
```

A macro or builder may generate parts of this contract after ergonomic testing.

## 7. Command metadata

```rust
pub struct CommandMetadata {
    pub summary: &'static str,
    pub description: &'static str,
    pub risk: Risk,
    pub permissions: &'static [Permission],
    pub supports_preview: bool,
    pub supports_cancel: bool,
    pub retry: RetryPolicy,
    pub compensation: CompensationPolicy,
}

pub enum RetryPolicy {
    Never,
    UserInitiated,
    Automatic {
        attempts: u32,
        backoff: Backoff,
    },
}
```

## 8. Input provenance

```rust
pub struct ResolvedInput<T> {
    pub value: T,
    pub source: InputSource,
    pub explicitly_supplied: bool,
}

pub enum InputSource {
    CliArgument,
    Pipeline,
    Environment { name: String },
    Configuration { path: String, key: String },
    StoredPreference,
    InteractivePrompt,
    Agent,
    RemoteApi,
    Default,
}
```

## 9. Command events

```rust
pub enum StandardCommandEvent {
    Progress(ProgressEvent),
    Log(LogEvent),
    Diagnostic(Diagnostic),
    Prompt(PromptRequest),
    Diff(DiffArtifact),
    Artifact(ArtifactRef),
    ApprovalRequired(ApprovalRequest),
    StatusChanged(CommandStatus),
    Completed(CommandCompletion),
}

pub struct ProgressEvent {
    pub operation_id: String,
    pub label: String,
    pub completed: Option<u64>,
    pub total: Option<u64>,
    pub unit: Option<String>,
    pub importance: EventImportance,
}
```

## 10. Command run

```rust
pub struct CommandRun {
    pub id: CommandRunId,
    pub command_name: String,
    pub command_version: u32,
    pub actor: Actor,
    pub source: InvocationSource,
    pub status: CommandStatus,
    pub started_at: Option<Timestamp>,
    pub finished_at: Option<Timestamp>,
    pub events: FlowDocumentId,
    pub tasks: TaskTreeId,
    pub approvals: Vec<ApprovalId>,
    pub artifacts: Vec<ArtifactRef>,
}

pub enum CommandStatus {
    Created,
    Validating,
    AwaitingInput,
    AwaitingApproval,
    Queued,
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
    Compensating,
    Compensated,
}
```

## 11. Structured tasks

```rust
pub struct TaskSpec {
    pub name: String,
    pub owner: TaskOwner,
    pub cancellation: CancellationPolicy,
    pub persistence: TaskPersistence,
}

pub enum TaskOwner {
    Entity(EntityId),
    Command(CommandRunId),
    Session(SessionId),
    Runtime,
}

pub enum TaskPersistence {
    Scoped,
    SurviveProjection,
    SurviveRendererDisconnect,
    Durable,
}
```

## 12. Resources

```rust
pub enum ResourceState<T, E> {
    Idle,
    Loading,
    Ready(T),
    Stale(T),
    Refreshing(T),
    Failed(E),
    Cancelled,
}

pub struct Resource<T, E> {
    pub id: ResourceId,
    pub version: u64,
    pub state: ResourceState<T, E>,
}
```

## 13. Semantic nodes

```rust
pub struct SemanticNode {
    pub id: NodeId,
    pub role: SemanticRole,
    pub name: Option<String>,
    pub description: Option<String>,
    pub value: Option<SemanticValue>,
    pub state: SemanticState,
    pub relationships: Vec<SemanticRelationship>,
    pub actions: Vec<ActionDescriptor>,
    pub children: Vec<NodeId>,
    pub content: SemanticContent,
    pub presentation: PresentationHints,
}

pub enum SemanticRole {
    Application,
    Document,
    Section,
    Heading,
    Paragraph,
    Code,
    Diff,
    List,
    ListItem,
    Table,
    Tree,
    Button,
    TextField,
    Progress,
    Status,
    Diagnostic,
    Finding,
    Evidence,
    Approval,
    Image,
    Diagram,
    Link,
    Custom(String),
}
```

## 14. Flow model

```rust
pub struct FlowDocument {
    pub id: FlowDocumentId,
    pub title: Option<String>,
    pub nodes: Vec<NodeId>,
    pub revision: u64,
}

pub enum FlowContent {
    RichText(PreparedRichText),
    Code(PreparedCode),
    Diff(PreparedDiff),
    Log(PreparedLog),
    Diagnostic(Diagnostic),
    Progress(ProgressModel),
    Finding(FindingSummary),
    Evidence(EvidenceSummary),
    Approval(ApprovalRequest),
    Artifact(ArtifactRef),
    Media(MediaReference),
    Container(FlowContainer),
}
```

## 15. Rich Flow fragments

```rust
pub enum FlowFragment {
    Text(TextRun),
    SoftBreak,
    HardBreak,
    InlineCode(TextRun),
    Link(LinkFragment),
    Chip(ChipFragment),
    InlineAction(ActionRef),
    EmbeddedNode(NodeId),
    MediaAnchor(MediaPlacementId),
}

pub struct TextRun {
    pub text: std::sync::Arc<str>,
    pub role: TextRole,
    pub source_range: Option<SourceRange>,
}
```

## 16. Prepared text and layout cursors

```rust
pub struct PreparedRichText {
    pub fragments: Vec<PreparedFragment>,
    pub source_index: SourceIndex,
    pub revision: u64,
}

pub struct FlowCursor {
    pub node: NodeId,
    pub fragment_index: usize,
    pub grapheme_offset: usize,
}

pub struct FlowRange {
    pub start: FlowCursor,
    pub end: FlowCursor,
}

pub trait FlowLayouter {
    fn layout_range(
        &mut self,
        prepared: &PreparedRichText,
        constraints: &FlowConstraints,
        start: &FlowCursor,
        viewport: ViewportExtent,
    ) -> FlowLayoutPage;
}
```

## 17. Flow constraints

```rust
pub struct FlowConstraints {
    pub inline_extent: LogicalLength,
    pub block_extent: Option<LogicalLength>,
    pub regions: Vec<FlowRegion>,
    pub typography: TypographyMetrics,
    pub capability: FlowCapability,
}

pub struct FlowRegion {
    pub block_start: LogicalLength,
    pub block_end: LogicalLength,
    pub available_segments: Vec<InlineSegment>,
}
```

The terminal implementation maps logical lengths to cells. Other renderers use their own units.

## 18. Scene model

```rust
pub struct Scene {
    pub id: WorkspaceId,
    pub regions: Vec<SceneRegion>,
    pub projections: Vec<Projection>,
    pub focus: FocusState,
    pub overlays: Vec<ProjectionId>,
}

pub struct Projection {
    pub id: ProjectionId,
    pub node: NodeId,
    pub region: RegionIntent,
    pub mode: ProjectionMode,
    pub shared_state: ProjectionSharedState,
    pub renderer_state_key: Option<RendererStateKey>,
}

pub enum RegionIntent {
    Primary,
    Secondary,
    Inspector,
    Navigation,
    BottomPanel,
    Overlay,
    Modal,
    Custom(String),
}
```

## 19. Promotion API

```rust
pub struct PromotionRequest {
    pub node: NodeId,
    pub preferred_region: Option<RegionIntent>,
    pub mode: ProjectionMode,
    pub retain_flow_summary: bool,
}

impl SceneCoordinator {
    pub fn promote(
        &mut self,
        request: PromotionRequest,
        cx: &mut RuntimeContext<'_>,
    ) -> Result<ProjectionId, Diagnostic>;

    pub fn collapse(
        &mut self,
        projection: ProjectionId,
        cx: &mut RuntimeContext<'_>,
    ) -> Result<(), Diagnostic>;
}
```

## 20. Raw specification

Illustrative JSON:

```json
{
  "specId": "assessment-view",
  "version": "1.0",
  "revision": 12,
  "root": "page",
  "nodes": {
    "page": {
      "type": "Stack",
      "props": { "gap": "large" },
      "children": ["summary", "findings"]
    },
    "summary": {
      "type": "RunSummary",
      "props": { "run": { "$resource": "run.current" } },
      "children": []
    },
    "findings": {
      "type": "FindingList",
      "props": { "items": { "$resource": "run.findings" } },
      "actions": ["finding.open"],
      "children": []
    }
  }
}
```

## 21. Catalogue contract

```rust
pub trait ComponentDefinition: Send + Sync + 'static {
    type Prepared: Send + Sync + 'static;

    fn descriptor(&self) -> ComponentDescriptor;

    fn prepare(
        &self,
        raw: &RawNode,
        cx: &mut PrepareContext<'_>,
    ) -> Result<Self::Prepared, Diagnostics>;

    fn semantics(
        &self,
        prepared: &Self::Prepared,
        cx: &SemanticContext<'_>,
    ) -> SemanticNode;
}

pub struct ComponentDescriptor {
    pub name: String,
    pub version: u32,
    pub prop_schema: Schema,
    pub allowed_actions: Vec<ActionName>,
    pub required_permissions: Vec<Permission>,
    pub media_policy: MediaPolicy,
    pub fallbacks: Vec<FallbackDescriptor>,
}
```

Renderer implementations are registered separately from semantic component definitions.

## 22. Renderer component contract

```rust
pub trait RendererComponent<R: Renderer>: Send + Sync {
    fn capabilities(&self) -> CapabilityRequirements;

    fn prepare_renderer_state(
        &self,
        node: &PreparedNode,
        cx: &mut R::PrepareContext<'_>,
    ) -> Result<R::NodeState, Diagnostics>;

    fn layout(
        &self,
        node: &PreparedNode,
        state: &mut R::NodeState,
        cx: &mut R::LayoutContext<'_>,
    ) -> R::LayoutNode;

    fn paint(
        &self,
        node: &PreparedNode,
        state: &R::NodeState,
        layout: &R::LayoutNode,
        cx: &mut R::PaintContext<'_>,
    );
}
```

Type erasure may be required for dynamic catalogues but should remain inside registry implementation.

## 23. Specification patches

```rust
pub struct SpecPatchEnvelope {
    pub spec_id: String,
    pub base_revision: u64,
    pub new_revision: u64,
    pub transaction_id: uuid::Uuid,
    pub source: PatchSource,
    pub trust: TrustClass,
    pub operations: Vec<SpecPatch>,
    pub provenance: Option<ProvenanceRef>,
}

pub enum SpecPatch {
    AddNode { node: RawNode },
    RemoveNode { id: NodeId },
    ReplaceNode { id: NodeId, node: RawNode },
    SetProp { id: NodeId, path: PropPath, value: RawValue },
    RemoveProp { id: NodeId, path: PropPath },
    InsertChild { parent: NodeId, index: usize, child: NodeId },
    RemoveChild { parent: NodeId, child: NodeId },
    MoveNode { id: NodeId, parent: NodeId, index: usize },
    SetVisibility { id: NodeId, expression: Option<Expression> },
    AttachAction { id: NodeId, action: ActionName },
    DetachAction { id: NodeId, action: ActionName },
}
```

## 24. Terminal profile

```rust
pub struct TerminalProfile {
    pub identity: TerminalIdentity,
    pub tty: TtyKind,
    pub cells: CellSize,
    pub pixels: Option<PixelSize>,
    pub cell_pixels: Option<PixelSize>,
    pub colour: TerminalColourProfile,
    pub appearance: Appearance,
    pub keyboard: KeyboardCapabilities,
    pub mouse: MouseCapabilities,
    pub hyperlinks: CapabilityState,
    pub synchronised_output: CapabilityState,
    pub graphics: GraphicsCapabilities,
    pub multiplexer: Option<MultiplexerProfile>,
    pub latency: LatencyProfile,
    pub quirks: Vec<TerminalQuirk>,
}
```

## 25. Colour model

```rust
pub struct OklchColour {
    pub lightness: f32,
    pub chroma: f32,
    pub hue_degrees: f32,
    pub alpha: f32,
}

pub enum ColourIntent {
    Fixed(OklchColour),
    Token(ColourToken),
    Derived {
        source: ColourToken,
        lightness_delta: f32,
        chroma_scale: f32,
    },
}

pub struct ThemeDefinition {
    pub name: String,
    pub dark: ThemeVariant,
    pub light: Option<ThemeVariant>,
    pub contrast_targets: ContrastTargets,
    pub no_colour: NoColourTheme,
}

pub struct ResolvedTerminalTheme {
    pub depth: ColourDepth,
    pub colours: std::collections::HashMap<ColourToken, ResolvedColour>,
    pub attributes: std::collections::HashMap<ColourToken, TextAttributes>,
    pub diagnostics: Vec<ThemeDiagnostic>,
}
```

## 26. Media model

```rust
pub struct MediaAsset {
    pub id: MediaAssetId,
    pub source: MediaSource,
    pub kind: MediaKind,
    pub intrinsic_size: Option<PixelSize>,
    pub colour_space: Option<ColourSpace>,
    pub content_hash: Option<ContentHash>,
    pub accessibility: MediaAccessibility,
    pub trust: TrustClass,
}

pub struct MediaPlacement {
    pub id: MediaPlacementId,
    pub asset: MediaAssetId,
    pub fit: MediaFit,
    pub focal_point: Option<NormalisedPoint>,
    pub crop: Option<NormalisedRect>,
    pub opacity: f32,
    pub z_index: i32,
    pub role: MediaRole,
    pub fallback: FallbackPolicy,
}

pub enum MediaSource {
    Embedded { bytes: std::sync::Arc<[u8]> },
    ApprovedLocalFile { capability: FileCapability, path: std::path::PathBuf },
    ContentAddressed { hash: ContentHash },
    CommandArtifact { artifact: ArtifactRef },
    ApprovedRemote { capability: NetworkCapability, url: String },
    GeneratedDiagram { source: DiagramSource },
}
```

## 27. Media accessibility

```rust
pub struct MediaAccessibility {
    pub purpose: MediaPurpose,
    pub alt_text: Option<String>,
    pub long_description: Option<String>,
    pub structured_alternative: Option<SemanticDocumentRef>,
}

pub enum MediaPurpose {
    Meaningful,
    Decorative,
    Evidence,
    Diagram,
    Chart,
    Identity,
}
```

## 28. Diagnostic model

```rust
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub summary: String,
    pub detail: Option<String>,
    pub source: Option<DiagnosticSource>,
    pub labels: Vec<DiagnosticLabel>,
    pub related: Vec<DiagnosticRef>,
    pub help: Option<String>,
    pub actions: Vec<ActionDescriptor>,
    pub security_class: SecurityClass,
}
```

## 29. Headless renderer

```rust
pub trait HeadlessRenderer {
    fn render_semantics(&self, root: &SemanticNode) -> SemanticSnapshot;
    fn render_text(&self, root: &SemanticNode, options: TextRenderOptions) -> String;
    fn render_json(&self, root: &SemanticNode) -> serde_json::Value;
}
```

The headless renderer is not merely a test helper. It is a required expression for automation, accessibility and agents.

## 30. Terminal renderer mode selection

```rust
pub enum TerminalMode {
    Plain,
    Inline,
    Workspace,
    Remote,
}

pub struct ModePolicy {
    pub requested: Option<TerminalMode>,
    pub allow_workspace: bool,
    pub prefer_inline: bool,
    pub non_interactive: NonInteractivePolicy,
}

pub fn choose_mode(
    profile: &TerminalProfile,
    policy: &ModePolicy,
    command: &CommandMetadata,
) -> TerminalMode;
```

## 31. Ratatui compatibility sketch

```rust
#[cfg(feature = "ratatui-compat")]
pub struct RatatuiHost<State> {
    pub id: NodeId,
    pub state: State,
    pub semantics: fn(&State) -> SemanticNode,
    pub render: fn(&mut State, &mut ratatui::Frame<'_>, ratatui::layout::Rect),
    pub action: fn(&mut State, &ActionEnvelopeErased) -> ActionResult,
}
```

The host lives in a terminal compatibility crate. Core APIs never mention it.

## 32. Example north-star command

```rust
#[derive(CommandInput)]
pub struct AssessInput {
    #[source(cli, prompt, config, default = ".")]
    pub path: std::path::PathBuf,

    #[source(cli, env = "POLICY_SET", config, prompt)]
    pub policy: PolicySetId,
}

pub struct Assess;

impl Command for Assess {
    type Input = AssessInput;
    type Output = AssessmentResult;
    type Event = AssessmentEvent;
    type Error = AssessmentError;

    const NAME: &'static str = "assess";
    const VERSION: u32 = 1;

    fn metadata() -> CommandMetadata {
        CommandMetadata {
            summary: "Assess a repository against a policy set",
            description: "Runs checks, gathers evidence and produces findings",
            risk: Risk::ReadOnly,
            permissions: &[Permission::ReadWorkspace],
            supports_preview: false,
            supports_cancel: true,
            retry: RetryPolicy::UserInitiated,
            compensation: CompensationPolicy::None,
        }
    }

    // validate, preview and execute omitted
}
```

## 33. Example Flow projection

```rust
fn assessment_flow(run: Entity<CommandRun>, cx: &SemanticContext<'_>) -> FlowDocument {
    FlowDocument::builder()
        .node(run_summary(run.clone(), cx))
        .node(progress_block(run.clone(), cx))
        .nodes(finding_blocks(run, cx))
        .build()
}
```

## 34. Example promotion

```rust
runtime.dispatch(
    finding_entity,
    PromoteToWorkspace {
        node: finding_node_id,
        preferred_region: Some(RegionIntent::Primary),
    },
)?;
```

The same action may be invoked by:

- Enter in the terminal;
- clicking a web card;
- a native command palette;
- an agent with permission;
- an automated test.

## 35. Example generated specification patch

```json
{
  "specId": "assessment-view",
  "baseRevision": 12,
  "newRevision": 13,
  "transactionId": "52ec5b50-fbca-4aa5-81dc-45386a14d650",
  "source": {
    "kind": "agent",
    "id": "analysis-agent"
  },
  "trust": "agent-generated",
  "operations": [
    {
      "op": "addNode",
      "node": {
        "id": "finding-F-214",
        "type": "FindingCard",
        "props": {
          "finding": { "$resource": "findings.F-214" }
        },
        "actions": ["finding.open", "evidence.open"]
      }
    },
    {
      "op": "insertChild",
      "parent": "findings",
      "index": 0,
      "child": "finding-F-214"
    }
  ]
}
```

The runtime validates that the source may create `FindingCard` and attach only the declared actions.

## 36. Semantic snapshot example

```json
{
  "id": "finding-F-214",
  "role": "finding",
  "name": "Unreviewed model-generated migration",
  "state": {
    "severity": "high",
    "expanded": false
  },
  "value": {
    "evidenceCount": 4,
    "remediationAvailable": true
  },
  "actions": [
    "finding.open",
    "evidence.open",
    "remediation.request"
  ]
}
```

Semantic snapshots should be stable across terminal, web and native renderers.

## 37. API design guidelines

When converting these sketches into real APIs:

1. Prefer explicit typed state over generic property bags.
2. Keep renderer types at renderer boundaries.
3. Make lifecycle and ownership visible in method names and types.
4. Avoid requiring application authors to understand internal type erasure.
5. Preserve good compiler errors.
6. Use derive macros only where they remove repetition without hiding semantics.
7. Support dynamic catalogue entries without making static Rust components pay unnecessary runtime cost.
8. Separate stable IDs from display labels.
9. Make security-relevant operations explicit.
10. Provide low-level escape hatches in adapter crates rather than weakening the core.

## 38. API validation checklist

Before stabilising an API, verify:

- Is it used by the greenfield reference app?
- Is it used by the terminal renderer?
- Is it used or validated by the sibling web renderer?
- Does it work headlessly?
- Does it preserve stable identity?
- Is task ownership obvious?
- Can it be inspected and replayed?
- Does it have a trust and permission story?
- Can current Anvil/`eddacraft-tui` integrate through an adapter?
- Is the error message understandable when used incorrectly?
