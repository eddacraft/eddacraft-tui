# Architecture-boundary eval suite (EVALCI-005).
#
# A first-wave trust-regression policy over the frozen PolicyInput v1 shape. It
# flags a *new* dependency edge (`input.diff.new_edges`) that crosses from an
# outer layer into a core layer — a layering violation. The pre-write path
# explicitly defers this edge-based semantics to the gate/CI, so the eval
# harness is where it belongs.
#
# ADR-003 (new edges only): the pre-existing import graph in
# `input.repo_state.edges` is never re-flagged, so a historical crossing does
# not block every run.
# ADR-002 (warnings over blocks): a crossing edge is advisory (`warning`), so a
# clean gate stays exit 0 and only a deliberate escalation regresses.

package anvil.policies.arch_boundary

import rego.v1

# Outer (higher-level) layer path prefixes. A defaulted rule so a hermetic
# input document need not restate them.
default outer_prefixes := ["src/ui/", "src/app/"]

# Core (lower-level) layer path prefixes an outer layer must not depend on
# directly.
default core_prefixes := ["src/db/", "src/store/"]

outer(path) if {
	some prefix in outer_prefixes
	startswith(path, prefix)
}

core(path) if {
	some prefix in core_prefixes
	startswith(path, prefix)
}

# One finding per new edge whose importer is an outer-layer file and whose
# imported target is a core-layer file. Emitted in the frozen v1 finding shape
# (`severity`/`message`/`from`/`to`) the eval-output contract consumes.
findings contains finding if {
	some edge in input.diff.new_edges
	outer(edge.from)
	core(edge.to)
	finding := {
		"severity": "warning",
		"message": sprintf("architecture boundary crossed: %s imports %s", [edge.from, edge.to]),
		"from": edge.from,
		"to": edge.to,
	}
}
