# Talking Points: Anvil Pitch Deck

Per-slide talking track for the presenter. 2-3 sentences each -- what to say,
not what is on the slide.

---

## Slide 1: Title

Open with the category, not the product. "AI governance for developers is a new
category. No tool enforces governance at the point of code generation. Anvil
does."

## Slide 2: The problem

Lead with the number. "Forty-six per cent of production code is now
AI-generated. That code produces 1.7 times more defects, and fewer than half of
developers review it before committing. The governance gap is not theoretical --
it is measured."

## Slide 3: Why now

Create urgency with the deadline. "The EU AI Act high-risk requirements become
enforceable in August -- five months from now. Penalties reach 7% of global
turnover. Gartner forecasts nearly half a billion dollars in AI governance spend
this year. The market is shifting from optional to mandatory."

## Slide 4: The solution

State the differentiator plainly. "Anvil enforces policy at file save, not after
commit. It uses deterministic analysis -- policy-as-code, not AI reviewing AI.
Every line is attributed: human, AI, mixed, or unknown. This is architecturally
different from everything else in the market."

## Slide 5: How it works

Walk the flow. "Save a file. Anvil parses it in milliseconds using Rust and
tree-sitter. It classifies authorship at the line level. It evaluates your
OPA/Rego policies. It updates the architecture graph. It emits a governance
event: pass, warn, or block. The entire loop is synchronous with development."

## Slide 6: Product

Let the product speak. "This is Anvil running in the terminal. Real-time
governance events. Policy enforcement live. Built in Rust, ships as a single
binary. No cloud account required. It runs where your code runs."

## Slide 7: Market opportunity

Anchor to the Gartner number. "AI governance platforms are a 492-million-dollar
market this year, growing past a billion by 2030. That is just the governance
segment. The broader market -- AI code tools plus application security -- is
21.5 billion. And regulatory pressure means this spend is mandatory, not
discretionary."

## Slide 8: Competitive landscape

Point to the empty quadrant. "Every static analysis tool operates after commit.
Every AI review tool is probabilistic. The top-left quadrant -- deterministic
and pre-commit -- is empty. That is where Anvil sits. Moving into this position
requires re-architecting an entire product, not adding a feature."

## Slide 9: Business model

Show the expansion path. "Developers adopt Anvil bottom-up -- CLI install,
immediate value. The expansion trigger is compliance: a SOC 2 audit, the EU AI
Act deadline, an enterprise customer asking about AI governance. Policy packs
and enterprise features create the upsell."

## Slide 10: Traction

[EVIDENCE NEEDED -- populate when traction data is available]

## Slide 11: Team

[EVIDENCE NEEDED -- populate when team bios are provided]

## Slide 12: The ask

[EVIDENCE NEEDED -- populate when funding details are confirmed]

## Slide 13: Appendix

"These slides back every claim in the deck with detailed evidence. Technical
architecture, competitive feature matrix, regulatory timeline, financial model.
Use them for Q&A."
