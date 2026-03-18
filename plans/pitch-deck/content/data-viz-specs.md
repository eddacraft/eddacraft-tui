# Data Visualisation Specifications: Anvil Pitch Deck

## Global Chart Standards

### Colour Mapping
| Data Category | Colour | Token |
|---------------|--------|-------|
| Anvil / primary data | `#cc5500` | `--anvil` |
| Positive / growth | `#2e8b57` | `--edda` |
| Baseline / human / neutral | `#85858a` | `--text-muted` |
| Negative / risk | `#c94a4a` | ERROR |
| Warning / caution | `#d08c38` | WARNING |
| Grid lines | `#2a2a2e` | `--structure` |
| Chart background | `#141416` | `--surface` |
| Axis labels | `#85858a` | `--text-muted` |

### Chart Rules
- No pie charts or donut charts -- use horizontal bars instead
- No 3D effects
- No gradients in chart fills -- solid colours only
- Axis lines: 1px `--structure`
- Data labels: JetBrains Mono, 12-14px
- Sharp corners on all bar chart elements (0px radius)
- Generous spacing between bars (min 40% of bar width)

---

## Slide 2: AI vs Human Code Quality (Horizontal Bar Chart)

### Chart Type
Grouped horizontal bar chart -- AI vs human metrics side-by-side.

### Data

| Metric | Human | AI | Multiplier |
|--------|-------|----|-----------|
| Issues per PR | 6.45 | 10.83 | 1.7x |
| Critical issues | 1.0x (baseline) | 1.4x | 1.4x |
| Major issues | 1.0x (baseline) | 1.7x | 1.7x |
| Security findings | 1.0x (baseline) | 1.57x | 1.57x |
| Maintainability | 1.0x (baseline) | 1.64x | 1.64x |

### Colour
- Human bars: `--text-muted` (#85858a)
- AI bars: `--anvil` (#cc5500)
- Multiplier labels: `--anvil`, JetBrains Mono, 16px, right-aligned

### Source
CodeRabbit, "State of AI vs Human Code Generation Report" (2025)

### Layout
- Chart occupies right 60% of slide
- 5 rows, grouped bars
- Y-axis labels on left
- Data labels on right end of each bar

---

## Slide 3: Regulatory Timeline (Horizontal Timeline)

### Chart Type
Horizontal timeline with milestone markers and spend forecast overlay.

### Data

| Date | Event | Status |
|------|-------|--------|
| Feb 2025 | EU AI Act prohibitions active | Past |
| Aug 2025 | GPAI obligations active | Past |
| Mar 2026 | NOW | Current |
| Aug 2026 | High-risk requirements enforceable | Upcoming (5 months) |
| 2027 | Full enforcement begins | Future |
| 2030 | >75% of economies have AI regulation | Future |

### Overlay: Governance Spend
| Year | Spend |
|------|-------|
| 2026 | USD 492M |
| 2030 | >USD 1B |

### Colour
- Timeline track: `--structure` (#2a2a2e), 2px
- Past milestones: `--text-muted` (#85858a) dots, 8px
- Current marker (NOW): `--anvil` (#cc5500) dot, 16px, with vertical line
- Upcoming deadline (Aug 2026): `--anvil` (#cc5500) dot, 12px, pulsing border
- Future milestones: `--edda` (#2e8b57) dots, 8px
- Spend bars: `--anvil` and `--edda` respectively

### Source
EU AI Act, Gartner (Feb 2026)

---

## Slide 3: Gartner Spend Forecast (Small Bar Chart)

### Chart Type
Vertical bar chart -- two bars (2026 and 2030).

### Data

| Year | AI Governance Platform Spend |
|------|----------------------------|
| 2026 | USD 492M |
| 2030 | >USD 1B |

### Colour
- 2026 bar: `--anvil` (#cc5500)
- 2030 bar: `--edda` (#2e8b57)
- Growth arrow between bars: `--text-muted`

### Layout
- Small chart, upper-right of Slide 3
- Approximately 20% of slide area
- "Gartner, Feb 2026" attribution below in `--text-muted`, 12px

---

## Slide 5: Governance Pipeline (Flow Diagram)

### Chart Type
Horizontal flow diagram with 5 nodes and directional arrows.

### Data

| Stage | Label | Description |
|-------|-------|-------------|
| 1 | File Save | Developer saves file |
| 2 | Parse | Tree-sitter incremental parse (Rust) |
| 3 | Attribute | Line-level authorship classification |
| 4 | Evaluate | OPA/Rego policy check |
| 5 | Govern | Emit governance event |

### Output States

| State | Label | Colour |
|-------|-------|--------|
| Pass | Policy met | `--edda` (#2e8b57) |
| Warn | Review recommended | WARNING (#d08c38) |
| Block | Fix required | ERROR (#c94a4a) |

### Colour
- Node backgrounds: `--surface` (#141416)
- Node borders: `--structure` (#2a2a2e)
- Node left accent: `--anvil` (#cc5500), 3px vertical bar
- Arrows: `--text-muted` line, `--anvil` arrowhead
- Stage labels: JetBrains Mono, 16px, `--text-primary`
- Descriptions: Inter, 13px, `--text-muted`

### Layout
- Full-width horizontal, centred vertically
- Equal spacing between nodes
- Output states displayed below the pipeline as a legend

---

## Slide 7: TAM/SAM/SOM (Nested Rectangles)

### Chart Type
Three nested rectangles (not circles -- maintaining sharp-corner brand rule).

### Data

| Level | Market | Value | Growth |
|-------|--------|-------|--------|
| TAM | AI code tools + AppSec + governance | USD 21.5B (2025) | 20%+ CAGR |
| SAM | Governance/quality for AI-assisted dev | USD 1.5-2.0B (2026) | Growing |
| SOM | Early adopter segment (Year 3) | USD 50-100M | Target |

### Colour
- TAM rectangle: `--structure` border, `--surface` fill
- SAM rectangle: `--text-muted` border, `--surface` fill (slightly inset)
- SOM rectangle: `--anvil` border, `--anvil` fill at 15% opacity
- Labels: JetBrains Mono for numbers, Inter for descriptions

### Layout
- Centred on slide
- TAM occupies approximately 70% of slide width
- SAM inset by 15% on each side
- SOM inset further, positioned bottom-right of SAM for visual weight
- Source attribution: bottom-right, Inter 12px, `--text-muted`

### Sources
Mordor Intelligence, Gartner (Feb 2026), derived analysis

---

## Slide 8: 2x2 Positioning Matrix

### Chart Type
2x2 quadrant grid with labelled axes and quadrant contents.

### Data

| Quadrant | X-Axis | Y-Axis | Content |
|----------|--------|--------|---------|
| Top-left | Pre-commit | Deterministic | **ANVIL** (highlighted) |
| Top-right | Post-commit | Deterministic | Static Analysis category |
| Bottom-left | Pre-commit | Probabilistic | [Empty] |
| Bottom-right | Post-commit | Probabilistic | AI Code Review category |

### Colour
- Grid lines: `--structure` (#2a2a2e), 2px
- Axis labels: JetBrains Mono, 14px, `--text-muted`
- Anvil quadrant: `--anvil` border (3px), `--anvil` background at 10% opacity
- Anvil logo/name: `--anvil`, JetBrains Mono, 24px
- Other quadrant labels: Inter, 16px, `--text-muted`
- Empty quadrant: pure `--void`, no content

### Layout
- Grid centred, occupying 70% of slide area
- Axis labels outside the grid
- Anvil quadrant slightly larger or visually weighted (thicker border)
- Category examples listed in smaller text under category names (e.g., "SAST, Code Quality" under Static Analysis)

---

## Slide 9: Land-and-Expand Funnel

### Chart Type
Horizontal expansion funnel (reverse funnel -- grows wider, not narrower).

### Data

| Stage | Label | Revenue Signal |
|-------|-------|---------------|
| 1 | CLI Install | Free / community |
| 2 | Team Adoption | Per-seat subscription |
| 3 | Enterprise Policy | Enterprise tier |
| 4 | Compliance Packs | Add-on revenue |

### Colour
- Stage 1: `--text-muted` (#85858a)
- Stage 2: `--anvil` at 50% opacity
- Stage 3: `--anvil` (#cc5500)
- Stage 4: `--edda` (#2e8b57)
- Connecting arrows: `--structure`

### Layout
- Horizontal, left to right
- Each stage is a rectangle (sharp corners) that grows taller
- Labels centred within each stage
- Revenue signal below each stage in `--text-muted`

---

## AI Image Prompt Templates

### For Slide 1 (Brand/Atmospheric)
```
Abstract forge scene with a single glowing anvil form, ember orange (#cc5500)
light source, dark void background (#0d0d0f), sharp geometric shadows,
Scandinavian minimal influence, no people, moody industrial atmosphere,
hyper-clean composition, 16:9 aspect ratio, photorealistic
```

### For Slide 6 (Product Context)
```
Dark terminal interface displaying code governance output, JetBrains Mono
monospace font, amber (#cc5500) highlights on flagged code lines, three-pane
layout (left: policy rules, right: live events, bottom: system log), dark
background (#0d0d0f), sharp edges, no rounded corners, Nordic minimal
aesthetic, professional developer tooling, 16:9 aspect ratio
```

### For Slide 8 (Competitive Visual)
```
Minimal 2x2 grid on dark background (#0d0d0f), one quadrant illuminated
in amber (#cc5500), other quadrants in muted grey (#2a2a2e), sharp geometric
lines, no text, abstract representation of market positioning, Nordic
industrial aesthetic, 16:9 aspect ratio
```
