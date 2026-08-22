# CPACKS-006 eval-regression projection of the shipped anvil-baseline
# `change_scope` pack member. Thresholds and copy must stay in lockstep with
# crates/anvil-cli/src/commands/policy/starter_packs/anvil-baseline/policies/change_scope.rego.
# Not a pack member — the gate still consumes the pack's `warning` rule family.
#
# `anvil policy eval --json` only treats object arrays as findings, so this
# file emits Finding objects for the report-only harness.

package anvil.policies.eval.anvil_baseline_change_scope

import rego.v1

soft_limit := 10

hard_limit := 25

changed_count := count(input.diff.changed_files)

findings contains finding if {
	changed_count > soft_limit
	changed_count <= hard_limit
	finding := {
		"severity": "warning",
		"message": sprintf(
			"Change set touches %d files (soft advisory threshold %d). Consider splitting it so reviewers can reason about each part.",
			[changed_count, soft_limit],
		),
	}
}

findings contains finding if {
	changed_count > hard_limit
	finding := {
		"severity": "warning",
		"message": sprintf(
			"Change set touches %d files, past the advisory ceiling of %d. Strongly consider splitting it into smaller, independently reviewable commits.",
			[changed_count, hard_limit],
		),
	}
}
