# Experience and Design Specification

## 1. Purpose

This document defines how applications built on the runtime should behave and feel. It focuses on the qualities that create satisfaction, trust and wonder rather than only enumerating widgets.

The intended experience is contemporary but recognisably terminal-native. It should not imitate a browser inside character cells. It should make the shell, command and workspace feel like parts of one coherent environment.

## 2. Experience promise

> The interface composes itself around the work, reveals complexity only when useful, preserves the user’s place, and always leaves behind a durable, understandable record.

## 3. Experience principles

### 3.1 Calm over spectacle

The interface may be visually rich, but it must never become noisy merely to demonstrate terminal capabilities.

- Motion explains causality.
- Colour explains hierarchy and state.
- Images add information.
- Borders and chrome create structure only where needed.
- Idle surfaces remain still.

### 3.2 Progressive disclosure

A simple command should remain a simple command.

The experience may progressively develop:

```text
plain result
  ↓
inline live block
  ↓
expanded Flow details
  ↓
promoted workspace panel
  ↓
full multi-region Scene
```

The user must never be forced into a dashboard merely because the framework can render one.

### 3.3 Spatial memory

The system should protect the user’s mental map.

- Focus remains on the same semantic object through refresh.
- Selection survives sorting and streaming when the selected object still exists.
- Scroll anchors are semantic rather than raw row offsets.
- Promoted content retains its relationship to the originating Flow node.
- Resize changes arrangement without making the user rediscover their place.

### 3.4 Durable output

Interactive experiences should leave useful terminal history.

After an inline or full-screen operation, the user should retain:

- the final outcome;
- important diagnostics;
- artefact locations;
- a stable run identifier;
- a resume or reopen command;
- a clean terminal state.

### 3.5 Discoverability without clutter

Every action should be discoverable, but the interface should not show every possible shortcut at once.

Use:

- contextual help;
- a command palette;
- short active-action hints;
- searchable action descriptions;
- consistent semantic verbs;
- progressive detail.

### 3.6 Graceful degradation

The interface must remain understandable when:

- colour is unavailable;
- images are unavailable;
- the terminal is narrow;
- the session is remote;
- the user uses only a keyboard;
- the output is redirected;
- a component is unsupported;
- data is missing;
- an operation fails.

### 3.7 Trust through transparency

The interface should make clear:

- what is happening;
- what initiated it;
- what authority it has;
- what evidence supports a result;
- what will change if approved;
- how to cancel, undo or inspect;
- whether content was generated, remote or untrusted.

## 4. Application modes

## 4.1 Plain mode

Plain mode is the canonical non-interactive representation.

It must:

- work in CI, pipes and redirected output;
- avoid cursor movement and raw mode;
- provide stable human-readable output;
- provide explicit structured output when requested;
- never emit decorative animation frames;
- include durable identifiers and actionable next steps.

Example:

```text
Assessment complete: 14 passed, 2 warnings, 1 failure

FAIL F-214  Unreviewed model-generated migration
     Evidence: 4 entries
     Review:   anvil finding open F-214

Run: 01K2Q7…
```

## 4.2 Inline mode

Inline mode creates live interaction while respecting terminal scrollback.

It should:

- begin quickly without clearing the terminal;
- reserve only the region currently needed;
- append durable Flow nodes as they settle;
- avoid redrawing historical output unnecessarily;
- support compact prompts, progress and approvals;
- allow promotion into workspace mode;
- collapse cleanly back to ordinary output.

Inline mode should be the default interactive experience for commands that do not need a full workspace immediately.

## 4.3 Workspace mode

Workspace mode provides the full Scene experience.

It should:

- enter only when the user asks or the command genuinely requires it;
- preserve the current Flow and command state;
- provide focus-visible multi-region interaction;
- include a command palette and contextual actions;
- support code, diff, evidence, media and inspector surfaces;
- leave a clean terminal and durable summary on exit.

## 4.4 Remote mode

Remote mode adapts to latency and capability uncertainty.

It should:

- reduce unnecessary frames;
- coalesce progress updates;
- prefer stable text and conservative media protocols;
- preserve keyboard-first operation;
- explain disabled visual capabilities;
- remain fully operable over SSH and common multiplexers.

## 5. The command blooms into an application

The signature interaction is a command that grows only as necessary.

### Stage 1: invocation

```text
$ tool assess .
```

The shell responds immediately with identity and intent.

```text
Assessing repository…
Policy set       engineering-standard
Run              01K2Q7…
```

### Stage 2: live Flow

Structured progress appears as a live block. Logs and low-priority detail remain folded unless requested.

```text
◐ Assessing  9/17 checks
  secrets       passed
  dependencies  passed
  intent        running
```

### Stage 3: meaningful event

A finding appears as a semantic block rather than an arbitrary line.

```text
▲ High  F-214
Unreviewed model-generated migration
4 evidence entries · proposed remediation available
```

### Stage 4: promotion

The user opens or pins the finding. The application promotes it into Scene.

- The Flow node remains visible as its origin or compact summary.
- The detailed panel uses the same finding ID.
- Running work continues without restarting.
- Focus moves predictably.

### Stage 5: deep interaction

The workspace may show:

- finding summary;
- evidence timeline;
- related code;
- proposed diff;
- image or diagram evidence;
- policy explanation;
- approval actions.

### Stage 6: collapse and durable result

When the user exits or collapses the workspace:

- the run continues or finishes according to command policy;
- the Flow contains a settled summary;
- the shell remains clean;
- the user receives resume and artefact references.

## 6. Flow experience

### 6.1 Flow is not a chat transcript by default

Flow may contain conversation, but its primary model is a durable work document.

Nodes should communicate their type through structure, labels and actions rather than relying on chat bubbles for everything.

### 6.2 Node identity

Every significant node should expose a stable reference when useful:

```text
F-214        finding
R-01K2Q7     command run
E-88A        evidence entry
A-023        artefact
```

References should be copyable and usable in CLI or links.

### 6.3 Streaming behaviour

When content streams:

- existing visible words should not jitter;
- the viewport should remain anchored unless following the tail;
- partially generated structures should show a clear pending state;
- completed content should settle rather than disappear;
- the user should be able to pause automatic following;
- very fast low-value updates should be coalesced.

### 6.4 Expand and collapse

Collapsed nodes should communicate:

- identity;
- outcome or state;
- severity;
- quantity of hidden detail;
- key available actions.

Example:

```text
▸ Tool calls  12 completed · 1 warning
```

Expansion should preserve the user’s viewport anchor.

### 6.5 Mixed content

Flow may contain prose, code, chips, links, actions and media in one semantic sequence.

The renderer should avoid creating a bordered panel for every fragment. Use visual containers only when they clarify ownership, state or interaction.

### 6.6 Search and navigation

Flow must support:

- text search;
- semantic filtering by node type, status or severity;
- jump to next diagnostic/finding/action;
- deep-link navigation;
- return to previous anchor;
- follow-tail toggle.

### 6.7 Copy and export

A user should be able to copy or export:

- visible text;
- a complete node;
- Markdown;
- JSON;
- a command invocation;
- an evidence bundle;
- an artefact reference.

The chosen representation should preserve meaning rather than copy raw box-drawing characters.

## 7. Scene experience

### 7.1 Workspace composition

The default workspace should be sparse. Regions appear because content needs them.

A typical hierarchy:

```text
primary work region
secondary detail or evidence region
navigation/outline region when useful
bottom panel for logs or command activity when requested
overlay layer for palette, prompts and approvals
```

### 7.2 Focus

Focus must always be visible.

It should use multiple cues where needed:

- border emphasis;
- cursor/marker;
- title state;
- selection styling;
- semantic announcement.

Focus should never rely on subtle colour alone.

### 7.3 Navigation

Navigation conventions should be consistent but contextual:

- arrow keys always work;
- optional Vim-style bindings may be enabled;
- Tab or explicit focus actions move between regions;
- Escape returns through a predictable hierarchy;
- command palette searches all available actions;
- text-entry fields suspend character-based navigation bindings.

### 7.4 Promotion

Promotion should feel like the content is expanding into workspace, not like an unrelated screen replacement.

Recommended behaviour:

- preserve a visual or semantic origin marker;
- retain the node’s title and identity;
- focus the most useful detailed control;
- keep the originating command or Flow visible when space permits;
- record the transition for back navigation and replay.

### 7.5 Collapse

Collapse should:

- retain relevant local state;
- return focus to the originating node or nearest sensible location;
- update the Flow summary if the state changed;
- avoid losing running work or evidence.

### 7.6 Overlays and approvals

Overlays should be used for temporary decisions, not as the primary navigation model.

A high-risk approval should show:

- exact proposed operation;
- actor and source;
- affected resources;
- preview or diff;
- evidence and policy reason;
- approve, reject and revise actions;
- whether undo is available.

## 8. Colour experience

### 8.1 Semantic purpose

Colour should communicate:

- hierarchy;
- state;
- severity;
- focus;
- grouping;
- provenance or trust;
- diff meaning.

Colour should not be used merely to fill empty space.

### 8.2 Adaptive themes

The application should adapt to:

- dark and light terminal backgrounds;
- truecolour, ANSI 256 and ANSI 16;
- user-supplied palettes;
- no-colour preferences;
- high-contrast preferences.

The experience should be recognisably the same product without assuming exact RGB fidelity.

### 8.3 State redundancy

Every status must combine colour with another signal.

Examples:

```text
✓ Passed
▲ Warning
✕ Failed
◐ Running
○ Queued
! Approval required
```

### 8.4 Focus and selection

Focus, hover/pointer targeting and selection are distinct states and must not collapse into the same colour treatment.

### 8.5 Diff colour

Diffs should support:

- added, removed and modified semantic tokens;
- line and character-level emphasis;
- no-colour prefixes and patterns;
- contrast-safe backgrounds;
- optional syntax colouring that does not obscure diff meaning.

### 8.6 Generated series colours

Charts, agents and workstreams may require generated colours. The runtime should select perceptually distinct colours and add labels or patterns where the palette cannot preserve distinction.

## 9. Images and media experience

### 9.1 Use media when it adds information

Appropriate uses include:

- screenshots;
- architecture diagrams;
- charts whose shape matters;
- image evidence;
- generated artefacts;
- avatars or identity marks where useful;
- visual comparison.

Do not rasterise labels, code or ordinary interface text.

### 9.2 Progressive media resolution

Media should appear in stages without layout instability:

1. A reserved placeholder with name, purpose and dimensions.
2. A lightweight preview or cell representation.
3. The best native protocol representation.
4. An interactive promoted viewer when requested.

### 9.3 Flow media

In a wide Flow, prose may wrap beside a meaningful image or live diagram.

In a narrow Flow, media should stack at a deliberate point in the document.

The surrounding content should not jump when the media representation upgrades.

### 9.4 Media controls

Available actions may include:

- open/promote;
- zoom;
- fit/fill;
- inspect metadata;
- copy path or reference;
- save/export where authorised;
- view structured alternative;
- compare with another asset;
- reveal evidence source.

### 9.5 Media fallback

Fallback order should favour meaning rather than protocol novelty.

For a diagram:

```text
native graphic → cell preview → structured outline/tree → alt text
```

For a screenshot:

```text
native graphic → cell preview → metadata + file reference + purpose description
```

### 9.6 Animation

Animated media should:

- be paused by default when not essential;
- honour reduced motion;
- avoid consuming frames when off-screen;
- expose pause/replay controls;
- degrade to a representative still image.

## 10. Motion and transition experience

### 10.1 Motion communicates cause

Useful motion examples:

- a promoted node visually establishes its relationship to a new panel;
- a completed operation settles into a final state;
- an inserted block expands from its anchor;
- a panel resize interpolates when local and fast enough;
- a failure calls attention once.

### 10.2 Motion must be interruptible

User input, resize or a new state transition should be able to interrupt an animation without leaving invalid geometry.

### 10.3 Slow and remote terminals

The runtime may replace motion with discrete state changes under constrained latency. Semantic history must remain identical.

### 10.4 Reduced motion

Reduced-motion mode should eliminate non-essential interpolation and repeated animation while retaining progress and state changes.

## 11. Error, empty and degraded states

### 11.1 Errors remain in context

An error should appear where the operation or content belongs, with:

- concise summary;
- structured details;
- source location where applicable;
- cause and impact;
- available recovery actions;
- stable diagnostic code.

### 11.2 Unsupported content

Unsupported components should not vanish.

Example:

```text
[HeatMap unavailable in this renderer]
Data points: 348
Open as table · Export JSON
```

### 11.3 Missing data

Missing data should distinguish:

- not yet loaded;
- unavailable;
- permission denied;
- not applicable;
- failed to resolve;
- intentionally redacted.

A generic em dash may be used only when the distinction is not important.

### 11.4 Empty states

An empty state should explain:

- what the region normally contains;
- why it is empty if known;
- the most relevant action.

### 11.5 Disconnection

A disconnected renderer or data source should show:

- last known state;
- connection status;
- whether work continues;
- retry/reconnect actions;
- potential staleness.

## 12. Responsive experience

### 12.1 Semantic priority

Components should define priority tiers.

Example finding:

```text
Wide:
severity + title + evidence preview + owner + actions

Medium:
severity + title + evidence count + primary action

Narrow:
severity + title + compact status
```

### 12.2 Narrow terminals

Narrow mode should:

- stack rather than squeeze critical content;
- prioritise identity and state;
- collapse secondary metadata;
- keep actions reachable through palette or menu;
- avoid horizontal scrolling except for code, diff and data where unavoidable.

### 12.3 Height constraints

Low-height terminals should prioritise:

- active content;
- command state;
- focus and actions;
- temporary hiding of secondary chrome.

### 12.4 Ultrawide terminals

Wide terminals should not stretch prose to unreadable line lengths. Use:

- maximum reading widths;
- additional context panels;
- media or evidence alongside prose;
- multi-column data where semantically useful.

## 13. Keyboard, mouse and touchpad experience

### 13.1 Keyboard first

Every core operation must be available by keyboard.

### 13.2 Mouse support

Mouse and touchpad should improve:

- selection;
- scrolling;
- resizing;
- opening nodes;
- context menus;
- media interaction.

Mouse support must not create hidden actions unavailable elsewhere.

### 13.3 Keybinding philosophy

- Bind semantic actions, not component internals.
- Keep common navigation predictable.
- Allow application and user overrides.
- Display the active contextual binding.
- Never interpret text-entry characters as commands.

### 13.4 Chords

Multi-key chords may be supported, but the framework should provide timeout, discoverability and conflict diagnostics.

## 14. Command palette and help

The command palette is a universal action surface.

It should search:

- available actions;
- commands;
- nodes and entities;
- recent runs;
- settings;
- help topics.

Each result should show:

- action label;
- context;
- shortcut;
- risk/approval marker where relevant;
- why it is available or disabled.

Contextual help should derive from the same action registry rather than manually maintained footer text.

## 15. Accessibility experience

### 15.1 Semantic equivalence

The system must provide equivalent meaning without relying on:

- colour;
- image;
- spatial position alone;
- animation;
- mouse input.

### 15.2 Focus announcements

When focus changes, an accessibility representation should be able to communicate:

- role;
- name;
- state;
- position;
- relevant actions;
- important validation information.

### 15.3 Live updates

Do not announce every streamed token. Announce meaningful state transitions such as:

- command started;
- approval required;
- finding discovered;
- operation failed;
- operation completed.

### 15.4 Images and diagrams

Meaningful images must have:

- concise purpose-oriented alternative text;
- structured detail for complex diagrams or charts;
- access to underlying data where appropriate.

### 15.5 Sequential mode

A user should be able to switch to or export a sequential semantic representation of the active application.

## 16. Agent experience

### 16.1 Agents use capabilities, not pixels

An agent sees:

- semantic nodes;
- typed state;
- available actions;
- action schemas;
- permissions;
- diagnostics;
- structured media alternatives.

It does not need to press keys or infer meaning from colour.

### 16.2 Visible agency

Agent actions should appear in session history with:

- agent identity;
- requested action;
- inputs;
- authorisation result;
- produced changes;
- evidence and outcome.

### 16.3 Governed generative UI

An agent may compose catalogue components, but the interface should indicate generated or untrusted origin where relevant.

Agent-generated interfaces must not look authoritative solely because they use product styling.

### 16.4 Human override

A human should be able to:

- inspect why an agent action is available;
- reject or revise requests;
- pause streaming or automated progression;
- revoke permissions;
- return to a stable prior state where supported.

## 17. Cross-renderer experience consistency

Consistency means preserving:

- command and action names;
- entity identity;
- state and severity;
- permissions and approvals;
- Flow ordering and history;
- relationships;
- media purpose and alternatives;
- user preferences where portable.

Consistency does not require preserving:

- exact panel geometry;
- terminal keybindings in a browser;
- browser routes in a terminal;
- identical animations;
- the same density at every size;
- identical use of modals, tabs or windows.

## 18. Signature user journeys

### 18.1 Long-running assessment

1. User starts command in shell.
2. Inline Flow displays stable run identity and progress.
3. User continues working or opens workspace.
4. Findings stream without resetting focus.
5. User promotes one finding.
6. Evidence, code and diff appear in Scene.
7. User approves or rejects a typed remediation.
8. Workspace collapses to a durable summary.
9. Run can be reopened in terminal or web.

### 18.2 Generated dashboard

1. Agent or server proposes a specification patch.
2. Runtime validates source, catalogue and permissions.
3. New semantic nodes appear incrementally.
4. Unsupported media or components use deliberate fallbacks.
5. User inspects specification provenance in devtools or product UI.
6. Actions execute through the shared command runtime.

### 18.3 Image evidence

1. A finding references a screenshot.
2. Flow reserves stable media geometry.
3. Text alternative and metadata are immediately available.
4. Terminal resolves the best supported image protocol.
5. User promotes the screenshot into a comparison Scene.
6. Image identity and evidence relationship remain intact.
7. Text-only export includes purpose, source and artefact reference.

### 18.4 Renderer handoff

1. User starts a local command in terminal.
2. Run creates durable session ID.
3. User opens the same run in a browser.
4. Browser shows the same findings, evidence and approvals with web-native layout.
5. User performs an authorised action.
6. Terminal receives the state update without losing its current Flow anchor.

## 19. Anti-patterns

Applications built on the framework should avoid:

- clearing the screen for trivial commands;
- putting every item in a bordered card;
- global fixed key handling inside text fields;
- resetting selection after refresh;
- showing spinners with no operation identity;
- replacing structured diagnostics with coloured strings;
- using red/green as the only distinction;
- emitting raw logs as the primary data model;
- hiding unsupported components;
- loading arbitrary image paths from generated specs;
- recreating a node when promoting it;
- forcing web layouts into terminal cells;
- forcing terminal geometry into web/native renderers;
- continuous frame polling while idle;
- animation that delays user input;
- generic confirmation prompts that omit the actual impact.

## 20. “Wonderful” acceptance checklist

A reference experience should be judged successful when:

- It begins as quickly and simply as a good CLI.
- The user always knows what is running and how to stop it.
- The interface can grow into a workspace without restarting work.
- Focus, selection and scroll remain stable under streaming updates.
- Every action is discoverable and semantically named.
- A node can move between Flow and Scene while retaining identity.
- Colour is polished in truecolour and still clear in monochrome.
- Images feel integrated rather than pasted into a rectangle.
- A media placeholder can upgrade without a layout jump.
- Narrow and remote terminals remain first-class.
- Errors degrade visibly and helpfully.
- Exiting leaves useful scrollback and no terminal damage.
- A browser or native renderer can present the same run without imitating terminal layout.
- An agent can operate through typed actions without screen scraping.
- The experience is impressive because it is coherent, not because it is busy.
