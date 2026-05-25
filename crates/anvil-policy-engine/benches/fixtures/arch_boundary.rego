package arch

import rego.v1

# Representative architecture-boundary policy (POLENG-008 parity fixture).
# Standard Rego only — no anvil.* builtins — so it runs on both regorus and
# the Go OPA reference. Flags new dependency edges where an outer-layer file
# (mod10/mod11) imports a core-layer file (mod0/mod1): a layering violation.

outer(p) if startswith(p, "src/mod10/")
outer(p) if startswith(p, "src/mod11/")

core(p) if startswith(p, "src/mod0/")
core(p) if startswith(p, "src/mod1/")

findings contains f if {
	some edge in input.diff.new_edges
	outer(edge.from)
	core(edge.to)
	f := {
		"severity": "warning",
		"message": sprintf("layering violation: %s -> %s", [edge.from, edge.to]),
		"from": edge.from,
		"to": edge.to,
	}
}
