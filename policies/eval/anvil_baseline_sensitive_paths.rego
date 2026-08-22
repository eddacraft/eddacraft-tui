# CPACKS-006 eval-regression projection of the shipped anvil-baseline
# `sensitive_paths` pack member. Matchers and copy must stay in lockstep with
# crates/anvil-cli/src/commands/policy/starter_packs/anvil-baseline/policies/sensitive_paths.rego.
# Not a pack member — the gate still consumes the pack's `warning` rule family.

package anvil.policies.eval.anvil_baseline_sensitive_paths

import rego.v1

precise_sensitive(path) if startswith(path, ".github/workflows/")

precise_sensitive(path) if startswith(path, ".github/actions/")

precise_sensitive(path) if endswith(path, ".env")

precise_sensitive(path) if contains(path, ".env.")

heuristic_sensitive(path) if contains(lower(path), "secret")

heuristic_sensitive(path) if contains(lower(path), "credential")

heuristic_sensitive(path) if endswith(lower(path), "id_rsa")

heuristic_sensitive(path) if contains(lower(path), "token")

heuristic_sensitive(path) if contains(lower(path), "password")

heuristic_sensitive(path) if contains(lower(path), "apikey")

findings contains finding if {
	some path in input.diff.changed_files
	precise_sensitive(path)
	finding := {
		"severity": "warning",
		"message": sprintf(
			"`%s` changes CI configuration or an environment file. Have a second reviewer confirm the change before it lands; once verdict-aware exceptions are available, `anvil exception grant sensitive-paths` can record the review.",
			[path],
		),
	}
}

findings contains finding if {
	some path in input.diff.changed_files
	not precise_sensitive(path)
	heuristic_sensitive(path)
	finding := {
		"severity": "warning",
		"message": sprintf(
			"`%s` looks like it may hold a secret. Confirm no credential is committed; if the file is safe, no action is needed.",
			[path],
		),
	}
}
