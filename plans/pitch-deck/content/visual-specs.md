# Visual Specifications: Anvil Pitch Deck

## Global Design Standards

### Palette (from EddaTheme)
| Token | Hex | Usage |
|-------|-----|-------|
| `--void` | `#0d0d0f` | Slide background (all slides) |
| `--surface` | `#141416` | Card/container backgrounds |
| `--structure` | `#2a2a2e` | Borders, dividers, grid lines |
| `--text-primary` | `#ebebeb` | Headlines, primary text |
| `--text-muted` | `#85858a` | Secondary text, labels, captions |
| `--anvil` / EMBER | `#cc5500` | Primary accent -- data emphasis, CTAs, key metrics |
| `--edda` / GROWTH | `#2e8b57` | Positive metrics, growth indicators |
| ERROR | `#c94a4a` | Negative data, warnings (sparingly) |
| WARNING | `#d08c38` | Caution states |

### Typography
| Element | Font | Size | Colour |
|---------|------|------|--------|
| Headline | JetBrains Mono | 36-44px | `--text-primary` |
| Subhead | Inter | 20-24px | `--text-muted` |
| Body | Inter | 16-18px | `--text-primary` |
| Data callout | JetBrains Mono | 64-96px | `--anvil` |
| Labels | JetBrains Mono | 12-14px | `--text-muted` |
| Code/terminal | JetBrains Mono | 14-16px | `--text-primary` on `--surface` |

### Layout Rules
- All slides: 16:9 aspect ratio
- Background: `--void` (#0d0d0f) -- always
- Borders: 1px solid `--structure` -- sharp corners (0px radius)
- No drop shadows, no gradients, no rounded corners
- Generous whitespace (void-space) -- minimum 60px margins
- Content grid: 12-column where applicable
- EddaCraft brandmark in footer of every slide (small, `--text-muted`)

---

## Slide 1: Title

### Layout
Full-width centred. Three vertical zones: upper (logo), middle (headline), lower (tagline + brandmark).

### Visual Hierarchy
1. Anvil brandmark (EMBER, centred, large)
2. "AI governance for developers" (headline, `--text-primary`)
3. EddaCraft identity + version (footer, `--text-muted`)

### Visual Elements
- Anvil macro logo rendered in `--anvil` (#cc5500):
  ```
  ████     ████
  ██         ██
  ██  █████  ██
  ██         ██   a n v i l
  ██  █████  ██
  ██         ██
  ████     ████
  ```
- Subtle `--structure` border framing the content area
- No imagery -- pure brand typography moment

### Typography
- Headline: JetBrains Mono, 44px, `--text-primary`
- Tagline: Inter, 24px, `--text-muted`
- Footer: JetBrains Mono, 12px, `--text-muted`

---

## Slide 2: The problem

### Layout
Split: left 40% (text), right 60% (data visualisation).

### Visual Hierarchy
1. Data callout: "1.7x" in large `--anvil` type (right side, dominant)
2. Headline: "AI writes half the code. Nobody governs it." (left)
3. Supporting metrics as horizontal bar pairs (AI vs human)

### Visual Elements
- **Horizontal bar chart** comparing AI vs human code metrics:
  - Defects per PR: AI (10.83, `--anvil`) vs Human (6.45, `--text-muted`)
  - Critical issues: AI bar (1.4x, `--anvil`) vs Human baseline (`--structure`)
  - Security findings: AI bar (1.57x, `--anvil`) vs Human baseline
- Bar chart background: `--surface` with `--structure` grid lines
- Labels: JetBrains Mono, 14px, `--text-muted`

### Colour Usage
- All AI data bars: `--anvil` (#cc5500)
- All human baseline bars: `--text-muted` (#85858a)
- Grid lines: `--structure` (#2a2a2e)

---

## Slide 3: Why now

### Layout
Timeline layout. Horizontal timeline across bottom 40% of slide. Key metrics above.

### Visual Hierarchy
1. Data callout: "August 2026" in `--anvil`, large
2. Timeline showing regulatory milestones
3. Gartner spend forecast as rising bar

### Visual Elements
- **Horizontal timeline** (left to right):
  - 2025 Q1: EU prohibitions (dot, `--text-muted`)
  - 2025 Q3: GPAI obligations (dot, `--text-muted`)
  - **2026 Aug**: High-risk enforcement (large dot, `--anvil`, pulsing if animated)
  - 2027: Full enforcement (dot, `--structure`)
  - 2030: >$1B governance (dot, `--edda`)
- **Small bar chart** (upper right): AI governance spend trajectory
  - 2026: $492M (`--anvil`)
  - 2030: >$1B (`--edda`)
- Penalty callout: "7% of global turnover" in `--anvil` text

### Colour Usage
- Timeline track: `--structure`
- Active milestone: `--anvil`
- Future positive: `--edda`
- Past milestones: `--text-muted`

---

## Slide 4: The solution

### Layout
Full-width with centred content block. Feature list with icon-style micro-prefixes.

### Visual Hierarchy
1. Headline: "Deterministic governance at file save" (centred, large)
2. Five capability lines with Anvil micro-prefix `[ = ]`
3. Subtle `--structure` border box framing the capability list

### Visual Elements
- Capability list using micro-prefix `[ = ]` in `--anvil`:
  ```
  [ = ]  Policy enforcement at file save
  [ = ]  Deterministic analysis -- not AI reviewing AI
  [ = ]  Line-level authorship: human / AI / mixed / unknown
  [ = ]  Architecture drift detection
  [ = ]  Policy-as-code (OPA/Rego)
  ```
- Background: `--void` with single `--surface` card containing the list
- Border: 1px `--structure`, sharp corners

### Typography
- Headline: JetBrains Mono, 40px, `--text-primary`
- Capability text: Inter, 18px, `--text-primary`
- Micro-prefix: JetBrains Mono, 18px, `--anvil`

---

## Slide 5: How it works

### Layout
Full-width flow diagram. Horizontal pipeline with 5 stages.

### Visual Hierarchy
1. Flow diagram (dominant, centred)
2. Headline above
3. Stage labels below each node

### Visual Elements
- **Horizontal pipeline diagram**:
  ```
  [File Save] --> [Parse] --> [Attribute] --> [Evaluate] --> [Govern]
  ```
  - Nodes: `--surface` background, `--structure` border, sharp corners
  - Active node highlight: `--anvil` border on left edge of each box
  - Arrows: `--text-muted` thin lines with `--anvil` arrowheads
  - Stage descriptions below each node in `--text-muted`
- Below the pipeline, three example outputs:
  ```
  PASS   Policy met. Architecture stable.        (--edda)
  WARN   Boundary stress increasing. Review.     (--warning / #d08c38)
  BLOCK  Trust invariant violated. Fix required.  (--error / #c94a4a)
  ```

### Typography
- Node labels: JetBrains Mono, 16px, `--text-primary`
- Descriptions: Inter, 14px, `--text-muted`
- Output examples: JetBrains Mono, 14px, respective state colour

---

## Slide 6: Product

### Layout
Full-width product screenshot. Thin headline bar above, thin caption below.

### Visual Hierarchy
1. TUI screenshot (dominant, 80% of slide area)
2. Headline: minimal, above screenshot
3. Caption: "Built in Rust. Ships as a single binary." below

### Visual Elements
- **Full TUI screenshot** showing the three-zone layout:
  - Header: Macro anvil logo in EMBER
  - Left pane: Active policy display (`[ ≡ ] ACTIVE_POLICY`)
  - Right pane: Signal interceptor with live governance events, EMBER border
  - Footer: System logs + EddaCraft watermark
- Screenshot framed with `--structure` border, sharp corners
- If no actual screenshot available: **generate a faithful mockup** using the TUI spec layout

### Image Direction
- Dark terminal, monospace font, `--anvil` highlights on active elements
- Sharp edges, `--void` background visible around the terminal frame
- No browser chrome -- pure terminal presentation
- Aspect ratio: 16:9, content-dense but not cluttered

---

## Slide 7: Market opportunity

### Layout
Three-column for TAM/SAM/SOM. Stacked bar or nested circles.

### Visual Hierarchy
1. TAM figure (large, top): "USD 21.5B"
2. SAM figure: "USD 1.5-2.0B"
3. SOM figure: "USD 50-100M"
4. Gartner callout: "USD 492M AI governance (2026)"

### Visual Elements
- **Three nested rectangles** (not circles -- sharp corners):
  - Outer (TAM): `--structure` border, `--surface` fill
  - Middle (SAM): `--text-muted` border, slightly lighter
  - Inner (SOM): `--anvil` border, `--anvil` fill at 15% opacity
- Labels inside each rectangle: JetBrains Mono for numbers, Inter for description
- **Gartner source callout** bottom-right in `--text-muted`
- Growth arrow: `--edda` showing trajectory from 2026 to 2030

### Typography
- TAM number: JetBrains Mono, 48px, `--text-primary`
- SAM number: JetBrains Mono, 36px, `--text-primary`
- SOM number: JetBrains Mono, 28px, `--anvil`
- Source attribution: Inter, 12px, `--text-muted`

---

## Slide 8: Competitive landscape

### Layout
2x2 matrix, full-width. Anvil highlighted in its quadrant.

### Visual Hierarchy
1. Matrix (dominant)
2. Anvil logo in top-left quadrant (highlighted)
3. Category labels in other quadrants

### Visual Elements
- **2x2 grid**:
  - X-axis: Pre-commit | Post-commit
  - Y-axis: Deterministic | Probabilistic
  - Top-left (Anvil): `--anvil` background at 15% opacity, `--anvil` border, Anvil logo
  - Top-right (Static Analysis): `--surface` background, category label in `--text-muted`
  - Bottom-left (Empty): `--void` background, no content
  - Bottom-right (AI Review): `--surface` background, category label in `--text-muted`
- Grid lines: `--structure`
- Axis labels: JetBrains Mono, 14px, `--text-muted`
- Quadrant content: Inter, 16px, respective colours

### Colour Usage
- Anvil quadrant: `--anvil` accent
- Other quadrants: `--text-muted` text on `--surface` background
- Empty quadrant: pure `--void`

---

## Slide 9: Business model

### Layout
Two zones: left (adoption funnel), right (pricing tiers).

### Visual Hierarchy
1. Funnel diagram showing land-and-expand (left, dominant)
2. Tier comparison (right)
3. Headline above

### Visual Elements
- **Horizontal funnel** (not vertical):
  ```
  [CLI Install] --> [Team Adoption] --> [Enterprise Policy] --> [Compliance Packs]
  ```
  - Each stage wider (in revenue, not narrower)
  - Stages coloured from `--text-muted` (left) through `--anvil` to `--edda` (right)
- **Tier cards** (right side):
  - Community (open source): `--structure` border
  - Team: `--anvil` border
  - Enterprise: `--edda` border
- Sharp corners on all cards

---

## Slide 10: Traction

### Layout
[To be designed when traction data is available]

### Visual Elements
- Metric cards with large numbers in `--anvil`
- Growth chart if time-series data available
- Logo grid if design partner logos are available

---

## Slide 11: Team

### Layout
[To be designed when team information is provided]

### Visual Elements
- Headshots in `--surface` frames with `--structure` borders (sharp corners)
- Name: JetBrains Mono, `--text-primary`
- Role: Inter, `--text-muted`
- No background imagery

---

## Slide 12: The ask

### Layout
Centred, minimal. Large number (funding amount) with use-of-funds breakdown.

### Visual Elements
- **Funding amount**: JetBrains Mono, 96px, `--anvil`
- **Use of funds**: Horizontal stacked bar
  - Engineering: `--anvil`
  - Go-to-market: `--edda`
  - Compliance certification: `--text-muted`
- **Milestones**: Timeline below the bar (same style as Slide 3)

---

## Slide 13: Appendix

### Layout
Table-of-contents style with section references.

### Visual Elements
- Section list with `--structure` dividers between topics
- Each section marked with micro-prefix:
  - `[ = ]` Technical Architecture
  - `[ ≡ ]` Competitive Detail
  - `[ ■ ]` Regulatory Timeline
- Minimal design -- reference material, not presentation material
