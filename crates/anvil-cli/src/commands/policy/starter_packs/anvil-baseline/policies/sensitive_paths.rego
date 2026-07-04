# Sensitive Paths Policy — advises on changes to secrets and CI configuration.
#
# Editing CI workflows, credential files, or environment files can change how
# the project builds, deploys, or authenticates. This policy flags such changes
# in the working-tree diff so a reviewer can look before they land. It reads
# only PolicyInput v1 changed paths, so it is safe on the pre-write path.
#
# Slice 1 is advisory by design: every finding is `warning`-tier and never
# fails the gate. High-confidence path shapes get strong wording; lower-
# confidence substring heuristics get softer wording so a false positive can
# never block a change on its own. Blocking behaviour, when it arrives, comes
# from Anvil's posture-driven enforcement routing, not from Rego severity.

package anvil.policies.sensitive_paths

import rego.v1

# Precise, high-confidence sensitive shapes: CI configuration and environment
# files, matched by an exact path prefix or suffix.
precise_sensitive(path) if startswith(path, ".github/workflows/")

precise_sensitive(path) if startswith(path, ".github/actions/")

precise_sensitive(path) if endswith(path, ".env")

precise_sensitive(path) if contains(path, ".env.")

# Lower-confidence substring heuristics: a name that merely looks secret-ish.
heuristic_sensitive(path) if contains(lower(path), "secret")

heuristic_sensitive(path) if contains(lower(path), "credential")

heuristic_sensitive(path) if endswith(lower(path), "id_rsa")

heuristic_sensitive(path) if contains(lower(path), "token")

heuristic_sensitive(path) if contains(lower(path), "password")

heuristic_sensitive(path) if contains(lower(path), "apikey")

# Strong advisory for a precise match.
warning contains msg if {
	some path in input.diff.changed_files
	precise_sensitive(path)
	msg := sprintf(
		"`%s` changes CI configuration or an environment file. Have a second reviewer confirm the change before it lands; once verdict-aware exceptions are available, `anvil exception grant sensitive-paths` can record the review.",
		[path],
	)
}

# Softer advisory for a heuristic match that is not already flagged precisely.
warning contains msg if {
	some path in input.diff.changed_files
	not precise_sensitive(path)
	heuristic_sensitive(path)
	msg := sprintf(
		"`%s` looks like it may hold a secret. Confirm no credential is committed; if the file is safe, no action is needed.",
		[path],
	)
}
