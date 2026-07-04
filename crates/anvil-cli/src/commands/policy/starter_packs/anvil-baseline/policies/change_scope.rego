# Change Scope Policy — advises when a change set grows large.
#
# Large change sets are hard to review and raise the chance a risky edit slips
# through unnoticed. This policy counts the files in the working-tree diff and
# raises an advisory when the change set crosses a soft or hard threshold. It
# reads only the PolicyInput v1 diff, so it is safe on the pre-write path.
#
# Slice 1 is advisory by design: every finding is `warning`-tier and never
# fails the gate. Blocking behaviour, when it arrives, comes from Anvil's
# posture-driven enforcement routing, not from the severity a Rego rule
# declares.

package anvil.policies.change_scope

import rego.v1

# Advisory thresholds. These are fixed in-rego defaults for slice 1; there is
# no per-workspace override on the current PolicyInput contract.
soft_limit := 10

hard_limit := 25

changed_count := count(input.diff.changed_files)

# Soft advisory: a change set worth a closer look.
warning contains msg if {
	changed_count > soft_limit
	changed_count <= hard_limit
	msg := sprintf(
		"Change set touches %d files (soft advisory threshold %d). Consider splitting it so reviewers can reason about each part.",
		[changed_count, soft_limit],
	)
}

# Strong advisory: a change set past the hard threshold.
warning contains msg if {
	changed_count > hard_limit
	msg := sprintf(
		"Change set touches %d files, past the advisory ceiling of %d. Strongly consider splitting it into smaller, independently reviewable commits.",
		[changed_count, hard_limit],
	)
}
