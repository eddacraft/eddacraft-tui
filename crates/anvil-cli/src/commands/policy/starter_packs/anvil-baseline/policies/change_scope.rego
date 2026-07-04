# Change Scope Policy — bounds the size of a single change set.
#
# Large change sets are hard to review and raise the chance a risky edit slips
# through unnoticed. This policy counts the files in the working-tree diff and
# nudges (or refuses) when a change set grows beyond the configured bounds. It
# reads only the PolicyInput v1 diff, so it is safe on the pre-write path.

package anvil.policies.change_scope

import rego.v1

# Soft and hard bounds. Both default here and may be overridden per workspace
# through the policy input `config` object.
default warn_above := 10

default max_changed_files := 25

warn_above := input.config.warn_changed_files if {
	is_number(input.config.warn_changed_files)
}

max_changed_files := input.config.max_changed_files if {
	is_number(input.config.max_changed_files)
}

changed_count := count(input.diff.changed_files)

# Hard bound: a change set past the ceiling is refused, with remediation.
violation contains msg if {
	changed_count > max_changed_files
	msg := sprintf(
		"Change set touches %d files, above the ceiling of %d. Split it into smaller, independently reviewable commits, or raise `config.max_changed_files` deliberately.",
		[changed_count, max_changed_files],
	)
}

# Soft bound: a large-but-allowed change set is flagged for a closer look.
warning contains msg if {
	changed_count > warn_above
	changed_count <= max_changed_files
	msg := sprintf(
		"Change set touches %d files (soft limit %d). Consider splitting it so reviewers can reason about each part.",
		[changed_count, warn_above],
	)
}
