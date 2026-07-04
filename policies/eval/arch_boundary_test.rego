# Tests for the arch-boundary eval policy (EVALCI-005).
#
# Positive / negative / threshold coverage per the repo's minimum-tests
# convention (mirrors policies/fixtures/*_test.rego).

package anvil.policies.arch_boundary_test

import rego.v1

import data.anvil.policies.arch_boundary

# Positive: a new edge crossing from an outer layer into a core layer is flagged.
test_crossing_new_edge_is_flagged if {
	count(arch_boundary.findings) == 1 with input as {
		"diff": {"new_edges": [{"from": "src/ui/panel.rs", "to": "src/db/pool.rs"}]},
	}
}

# Negative: an edge that stays within a single layer is not a boundary crossing.
test_intra_layer_edge_is_ignored if {
	count(arch_boundary.findings) == 0 with input as {
		"diff": {"new_edges": [{"from": "src/ui/panel.rs", "to": "src/ui/theme.rs"}]},
	}
}

# Negative: a pre-existing (baseline) crossing edge is not re-flagged — new
# edges only (ADR-003). Only `input.diff.new_edges` drives findings.
test_baseline_edge_is_not_reflagged if {
	count(arch_boundary.findings) == 0 with input as {
		"repo_state": {"edges": [{"from": "src/ui/panel.rs", "to": "src/db/pool.rs"}]},
		"diff": {"new_edges": []},
	}
}

# Threshold: every crossing new edge yields its own finding.
test_multiple_crossing_edges_all_flagged if {
	count(arch_boundary.findings) == 2 with input as {
		"diff": {"new_edges": [
			{"from": "src/ui/panel.rs", "to": "src/db/pool.rs"},
			{"from": "src/app/main.rs", "to": "src/store/kv.rs"},
		]},
	}
}
