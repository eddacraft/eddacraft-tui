# Sensitive Paths Policy — guards changes to secrets and CI configuration.
#
# Editing CI workflows, credential files, or environment files can change how
# the project builds, deploys, or authenticates. This policy flags such changes
# in the working-tree diff and asks for an explicit review acknowledgement
# before they are treated as safe. It reads only PolicyInput v1 changed paths,
# so it is safe on the pre-write path.

package anvil.policies.sensitive_paths

import rego.v1

# A changed file is sensitive when its path matches a known high-risk shape.
sensitive(path) if startswith(path, ".github/workflows/")

sensitive(path) if startswith(path, ".github/actions/")

sensitive(path) if endswith(path, ".env")

sensitive(path) if contains(path, ".env.")

sensitive(path) if contains(lower(path), "secret")

sensitive(path) if contains(lower(path), "credential")

sensitive(path) if endswith(lower(path), "id_rsa")

# The change carries an explicit review acknowledgement.
review_acknowledged if input.config.review_acknowledged == true

# Sensitive change without acknowledgement: refuse, with remediation.
violation contains msg if {
	some path in input.diff.changed_files
	sensitive(path)
	not review_acknowledged
	msg := sprintf(
		"`%s` touches secrets or CI configuration but the change carries no review acknowledgement. Have a second reviewer confirm the change, then set `config.review_acknowledged: true`.",
		[path],
	)
}

# A path that merely looks secret-adjacent gets an advisory nudge, never a
# refusal, so a false positive cannot block a change on its own.
warning contains msg if {
	some path in input.diff.changed_files
	not sensitive(path)
	secret_adjacent(path)
	msg := sprintf(
		"`%s` looks like it may hold a secret. Confirm no credential is committed; add it to the sensitive set if it should require review.",
		[path],
	)
}

secret_adjacent(path) if contains(lower(path), "token")

secret_adjacent(path) if contains(lower(path), "password")

secret_adjacent(path) if contains(lower(path), "apikey")
