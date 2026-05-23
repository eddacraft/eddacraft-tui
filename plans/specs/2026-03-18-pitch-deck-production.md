# Pitch Deck Production — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce a 13-slide investor pitch deck (.pptx) from the approved
direction spec and pipeline materials, following the eddacraft design system
(Nordic Terminal / dark brutalist).

**Architecture:** Three-phase approach — (1) sync pipeline source files with
brainstorming decisions, (2) generate the .pptx deck using the pptx skill, (3)
update investor FAQ and create leave-behind one-pager. All content comes from
two authoritative sources: `plans/pitch-deck/` (pipeline) and
`docs/archive/specs/2026-03-18-pitch-deck-direction-design.md` (direction
spec).

**Tech Stack:** pptx skill (eddacraft Design System enforced), markdown, git

---

## Source Files

| File                                                               | Role                                                                                   |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| `docs/archive/specs/2026-03-18-pitch-deck-direction-design.md` | Direction spec — slides 10-12, slide 6 update, "no AI inside" thread, investor context |
| `plans/pitch-deck/content/slide-copy.md`                           | Slide-by-slide copy (slides 1-9 complete, 10-12 to be updated)                         |
| `plans/pitch-deck/content/visual-specs.md`                         | Per-slide layout, colour, typography, visual elements                                  |
| `plans/pitch-deck/content/data-viz-specs.md`                       | Chart types, data sources, colour mapping                                              |
| `plans/pitch-deck/content/talking-points.md`                       | Presenter notes per slide                                                              |
| `plans/pitch-deck/strategy/slide-outline.md`                       | Slide structure and theme mapping                                                      |
| `plans/pitch-deck/content/investor-faq.md`                         | Investor FAQ (needs valuation question added)                                          |
| `plans/pitch-deck/deliverables/one-pager.md`                       | One-pager (needs update with direction spec decisions)                                 |

---

## Phase 1: Sync Pipeline Materials

Update the pipeline source files to reflect brainstorming decisions before deck
generation. This ensures a single source of truth.

### Task 1: Update slide-copy.md with slides 10-12 and slide 6 headline

**Files:**

- Modify: `plans/pitch-deck/content/slide-copy.md` — slides 10-12 (search for
  `[EVIDENCE NEEDED]` blocks) and slide 6 headline
- Reference: `docs/archive/specs/2026-03-18-pitch-deck-direction-design.md`

- [ ] **Step 1: Replace slide 10 copy**

Replace the `[EVIDENCE NEEDED]` block for slide 10 (find
`## Slide 10: Traction`) with the traction copy from the direction spec:

- Headline: "Built what others are pitching"
- Three-column structure: Built Today / Launch Trajectory (targets) / Ecosystem
- Data callout: "Competitors in this category are raising on decks. Anvil is
  raising on a working product. Not vibe-coded — precision-engineered in a
  domain where AI fails."
- Presenter notes from direction spec

- [ ] **Step 2: Replace slide 11 copy**

Replace the `[EVIDENCE NEEDED]` block for slide 11 (find `## Slide 11: Team`)
with the team copy from the direction spec:

- Headline: "25 years building what enterprises buy"
- Founder block: Joshua Boys credentials
- Advisory bench text block
- "Solo founder = strength" framing
- Closing beat: "Built to make you trust your AI more — not by asking you to
  trust ours."

- [ ] **Step 3: Replace slide 12 copy**

Replace the `[EVIDENCE NEEDED]` block for slide 12 (find `## Slide 12: The ask`)
with the ask copy from the direction spec:

- Headline: "Own the category before the window closes"
- The Raise: £3-5M, £15-25M pre-money
- Use of Funds: Engineering 40%, GTM 30%, Strategic Acquisition 20%, Ops 10%
- Milestones: profitability, 5,000+ waitlist, 50+ paying teams, phase 2 ready
- Data callout: EU AI Act August 2026

- [ ] **Step 4: Update slide 6 headline**

Change slide 6 headline from "Built in Rust. Runs in your terminal." to "Built
in Rust. No AI inside. 50ms per check." Update presenter notes to include the
deterministic/precision messaging.

- [ ] **Step 5: Commit**

```bash
git add plans/pitch-deck/content/slide-copy.md
git commit -m "docs(pitch): sync slide copy with direction spec decisions"
```

---

### Task 2: Update slide-outline.md with completed slide metadata

**Files:**

- Modify: `plans/pitch-deck/strategy/slide-outline.md` — slides 10-12 (search
  for `[EVIDENCE NEEDED]`) and slide 6 headline

- [ ] **Step 1: Update slides 10-12 in outline**

Replace the `[EVIDENCE NEEDED]` entries for slides 10, 11, and 12 with the
finalised headlines, purposes, primary data points, and win theme mappings from
the direction spec.

- [ ] **Step 2: Update slide 6 headline in outline**

Change from "Built in Rust. Runs in your terminal." to "Built in Rust. No AI
inside. 50ms per check."

- [ ] **Step 3: Commit**

```bash
git add plans/pitch-deck/strategy/slide-outline.md
git commit -m "docs(pitch): update slide outline with finalised metadata"
```

---

### Task 3: Update visual-specs.md with slides 10-12 layouts

**Token note:** The existing visual-specs.md uses shorthand tokens (`--anvil`,
`--edda`). Maintain consistency with existing file conventions — the pptx skill
will map these to canonical tokens (`--anvil-ember`, `--edda-growth`) at
generation time (see Task 5 token aliasing note).

**Files:**

- Modify: `plans/pitch-deck/content/visual-specs.md` — slides 10-12 (search for
  `[To be designed`)

- [ ] **Step 1: Write slide 10 visual spec**

Replace the placeholder for slide 10 (find `## Slide 10: Traction`) with:

- Layout: Three-column metric dashboard (Built Today / Launch Trajectory /
  Ecosystem)
- Metric cards with large numbers in `--anvil`
- Column headers in JetBrains Mono, `--text-muted`
- "Built Today" items use `--edda` checkmark prefix
- "Launch Trajectory" items use `--anvil` arrow prefix
- Clear visual distinction between confirmed metrics and targets

- [ ] **Step 2: Write slide 11 visual spec**

Replace the placeholder for slide 11 (find `## Slide 11: Team`) with:

- Layout: Left 40% founder portrait, right 60% credentials
- Founder name: JetBrains Mono, `--text-primary`
- Credentials: Inter, `--text-primary`
- Advisory bench: text block below, Inter, `--text-muted`
- "Solo founder = strength" section: metric cards showing capital efficiency
- No silhouette placeholders — named advisors or text block only

- [ ] **Step 3: Write slide 12 visual spec**

Replace the placeholder for slide 12 (find `## Slide 12: The ask`). Note: the
existing spec at lines 326-332 already has the correct horizontal stacked bar
layout — only the category labels need updating from "Compliance certification"
to the four-way split (Engineering/GTM/Strategic Acquisition/Ops):

- Layout: Centred. Large funding number top, use-of-funds bar middle, milestones
  timeline bottom
- Funding amount: JetBrains Mono, 96px, `--anvil-ember`
- Use of funds: horizontal stacked bar (Engineering `--anvil-ember`, GTM
  `--edda-growth`, Acquisition `--text-primary`, Ops `--text-muted`)
- Milestones timeline: same style as slide 3, with August 2026 EU AI Act marked
- Phase 2 vision: single line at bottom, Inter, `--text-muted`

- [ ] **Step 4: Commit**

```bash
git add plans/pitch-deck/content/visual-specs.md
git commit -m "docs(pitch): add visual specs for slides 10-12"
```

---

### Task 4: Add valuation rationale to investor FAQ

**Files:**

- Modify: `plans/pitch-deck/content/investor-faq.md`

- [ ] **Step 1: Read current investor FAQ**

Read the file to understand existing questions and format.

- [ ] **Step 2: Add valuation rationale question**

Add a new FAQ entry:

**Q: Why £15-25M pre-money for a pre-revenue company?**

A: The valuation reflects four factors: (1) category heat — AI governance is the
hottest new category in developer tooling, with record pre-seed rounds
establishing valuation precedent; (2) product maturity — Anvil is a
production-grade Rust system with a working policy engine, semantic graph, and
authorship attribution, while most competitors are pre-product; (3) regulatory
forcing function — the EU AI Act creates mandatory spend with a known deadline
(August 2026), meaning this is not speculative demand; (4) capital efficiency —
£0 raised to date with a production product built, demonstrating exceptional
capital-to-output ratio.

- [ ] **Step 3: Commit**

```bash
git add plans/pitch-deck/content/investor-faq.md
git commit -m "docs(pitch): add valuation rationale to investor FAQ"
```

---

## Phase 2: Generate the Deck

### Task 5: Generate .pptx pitch deck

**Files:**

- Create: `plans/pitch-deck/deliverables/eddacraft-anvil-pitch-deck.pptx`

Use the `pptx` skill to generate the deck. The pptx skill enforces the eddacraft
Design System (dark-only, 5-colour palette, monospace headers, no decoration).

**Token aliasing note:** The visual-specs.md and direction spec use shorthand
tokens (`--anvil`, `--edda`, `--text-muted`). The pptx skill's canonical tokens
are `--anvil-ember`, `--edda-growth`, `--text-primary`, `--structure`. Map as
follows when generating:

- `--anvil` → `--anvil-ember` (#cc5500)
- `--edda` → `--edda-growth` (#2e8b57)
- `--text-muted` → `--text-muted` (#85858a) (same name)
- `--text-primary` → `--text-primary` (#ebebeb) (same name)
- `--surface` → `--surface` (#141416) (same name)
- `--void` → `--void` (#0d0d0f) (same name)
- `--structure` → `--structure` (#2a2a2e) (same name)

- [ ] **Step 1: Invoke pptx skill**

Generate a 13-slide .pptx deck using:

- Slide copy from `plans/pitch-deck/content/slide-copy.md` (now updated)
- Visual specs from `plans/pitch-deck/content/visual-specs.md` (now updated)
- Data viz specs from `plans/pitch-deck/content/data-viz-specs.md`
- eddacraft design system palette and typography

Slides to generate:

1. Title — brand moment, Anvil logo, "AI governance for developers"
2. Problem — "AI writes half the code. Nobody governs it." + bar chart
3. Why Now — "The compliance clock is ticking" + timeline
4. Solution — "Deterministic governance at file save" + capability list
5. How It Works — pipeline flow diagram + PASS/WARN/BLOCK outputs
6. Product — "Built in Rust. No AI inside. 50ms per check." + TUI mockup
7. Market — nested TAM/SAM/SOM rectangles
8. Competitive — 2x2 positioning matrix
9. Business Model — land-and-expand funnel + tier cards
10. Traction — three-column metric dashboard
11. Team — founder portrait layout + credentials
12. The Ask — funding amount + use-of-funds bar + milestones timeline
13. Appendix — table of contents for deep-dive materials

- [ ] **Step 2: Content QA**

Run content extraction to verify all text landed correctly:

```bash
pip install markitdown && python -m markitdown plans/pitch-deck/deliverables/eddacraft-anvil-pitch-deck.pptx
```

Verify:

- All 13 slides present with correct headlines
- Presenter notes populated for all slides
- No `[EVIDENCE NEEDED]` placeholders remain
- Slide copy matches `slide-copy.md` content

- [ ] **Step 3: Visual QA**

Convert to images for visual inspection:

```bash
# Generate thumbnail images of each slide
python .claude/skills/pptx/thumbnail.py plans/pitch-deck/deliverables/eddacraft-anvil-pitch-deck.pptx
# Or via LibreOffice if thumbnail.py unavailable:
soffice --headless --convert-to pdf plans/pitch-deck/deliverables/eddacraft-anvil-pitch-deck.pptx
pdftoppm -png -r 150 eddacraft-anvil-pitch-deck.pdf slide
```

Verify visually:

- eddacraft palette applied (void background #0d0d0f, anvil-ember accent
  #cc5500)
- Typography correct (JetBrains Mono headlines, Inter body)
- No rounded corners, no gradients, no shadows
- Data callouts visible and correctly coloured
- Charts/diagrams render correctly (2x2 matrix, pipeline flow, TAM circles)

**Do not declare success until at least one fix-and-verify cycle is complete.**
If issues are found, fix and re-generate, then re-run visual QA.

- [ ] **Step 4: Commit**

```bash
git add plans/pitch-deck/deliverables/eddacraft-anvil-pitch-deck.pptx
git commit -m "feat(pitch): generate 13-slide investor pitch deck"
```

---

## Phase 3: Supporting Materials

### Task 6: Update one-pager with direction spec decisions

**Files:**

- Modify: `plans/pitch-deck/deliverables/one-pager.md`

- [ ] **Step 1: Read current one-pager**

Read the file to understand existing content and format.

- [ ] **Step 2: Update with brainstorming decisions**

Ensure the one-pager reflects:

- "No AI inside" positioning
- Traction framing (built what others are pitching)
- Team summary (Joshua Boys, 25+ years, Microsoft Azure, Arkahna)
- Ask summary (£3-5M, use of funds, milestones)
- Phase 2 vision mention (knowledge worker governance)

- [ ] **Step 3: Commit**

```bash
git add plans/pitch-deck/deliverables/one-pager.md
git commit -m "docs(pitch): update one-pager with direction spec decisions"
```

---

### Task 7: Update pipeline status tracker

**Files:**

- Modify: `plans/pitch-deck/status.md`

- [ ] **Step 1: Mark all phases complete**

Update status.md to reflect:

- Phase 1 Research: Complete
- Phase 2 Strategy: Complete
- Phase 3 Content: Complete (slides 10-12 now filled)
- Phase 4 Synthesis: Complete
- Phase 5 Deck Production: Complete
- Note: slides 10-12 traction numbers are launch targets, will be updated with
  actuals ahead of raise

- [ ] **Step 2: Commit**

```bash
git add plans/pitch-deck/status.md
git commit -m "docs(pitch): mark all pipeline phases complete"
```

---

### Task 8: Push all changes

- [ ] **Step 1: Push to remote**

```bash
git push origin pitch
```

- [ ] **Step 2: Verify**

Confirm all commits pushed successfully to the pitch branch.
