# Planning Council: Direction Validate

Use before Draft or Proposed planning becomes execution authority.

## Inputs

- candidate APS item or module
- related specs, ADRs, and authority documents
- known implementation constraints
- expected validation and release impact

## Checks

- Intent matches product and architecture scope.
- Expected Outcome is testable and not implementation prose.
- Validation is deterministic or has a durable manual evidence path.
- Dependencies and coordination callouts are current.
- The work is the right size for a single branch/PR or has explicit waves.

## Decision

Return `proceed` when the item can become Ready. Return `amend`, `split`, or
`replan` when the direction is valid but the plan needs adjustment. Return
`block` when the direction conflicts with scope, ADRs, or safety boundaries.
