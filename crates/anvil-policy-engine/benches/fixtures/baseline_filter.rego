package baseline_filter

import rego.v1

# Representative ADR-003 ("new edges only") policy (POLENG-008 parity fixture).
# Standard Rego only. Emits a finding for every new edge whose fingerprint is
# not already in the baseline cohort — exercising set comprehension over the
# baseline plus per-edge set membership.

baselined_fps contains fp if {
	some finding in input.baseline.findings
	fp := finding.fingerprint
}

findings contains f if {
	some edge in input.diff.new_edges
	fp := sprintf("%s>%s", [edge.from, edge.to])
	not fp in baselined_fps
	f := {
		"severity": "warning",
		"message": sprintf("new edge %s -> %s", [edge.from, edge.to]),
		"fingerprint": fp,
	}
}
