# Personal-data path review — advisory on paths that look like they hold
# personal data. Path heuristic only: it does not inspect file contents and
# does not claim data-protection coverage.
#
# Slice findings are `warning`-tier and never veto, even under interrupt.
# Reads only PolicyInput v1 `diff.changed_files`.

package anvil.policies.personal_data_paths

import rego.v1

personal_data_path(path) if contains(lower(path), "personal_data")

personal_data_path(path) if contains(lower(path), "personal-data")

personal_data_path(path) if startswith(lower(path), "users/")

personal_data_path(path) if contains(lower(path), "/users/")

personal_data_path(path) if contains(lower(path), "profiles/")

personal_data_path(path) if contains(lower(path), "exports/")

personal_data_path(path) if startswith(lower(path), "pii/")

personal_data_path(path) if contains(lower(path), "/pii/")

warning contains msg if {
	some path in input.diff.changed_files
	personal_data_path(path)
	msg := sprintf(
		"`%s` looks like a personal-data path. Confirm the change is intentional and limited; this is a review prompt, not a legal assessment.",
		[path],
	)
}
