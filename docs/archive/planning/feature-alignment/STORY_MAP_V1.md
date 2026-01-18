# STORY_MAP_V1 — Canonical User Stories

## Primary persona: Individual developer using AI

1. Boundary safety in-flow  
   As a developer, I want Anvil to warn me on file save when I introduce a new
   cross-context dependency, so I can fix it before it becomes drift.

2. AI escape-hatch prevention  
   As a developer, I want Anvil to flag high-confidence AI anti-patterns (eslint
   disables, new `any`), so I don’t ship “technically valid but wrong” code.

3. Intentional exceptions  
   As a developer, I want to suppress a warning with a required note stored in
   code + provenance, so future readers understand why.

## Secondary personas

- Tech lead/architect: drift reports and NEW vs existing visibility
- Manager: AI throughput increases without more incidents

## AI tooling (later)

- AI consumes exported constraints to avoid generating forbidden patterns
