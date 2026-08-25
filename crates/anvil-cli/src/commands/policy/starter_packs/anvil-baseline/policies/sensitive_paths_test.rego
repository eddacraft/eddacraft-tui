# Tests for the sensitive_paths policy.
#
# Every case uses the real production input shape (PolicyInput v1:
# `input.diff.changed_files`). There is no injected `config` key — the policy
# reads none. The policy is advisory-only: it defines no `violation` rule, so
# these cases assert on `warning` alone.

package anvil.policies.sensitive_paths_test

import rego.v1

import data.anvil.policies.sensitive_paths

# Positive (precise): a workflow change raises an advisory.
test_workflow_change_warns if {
	count(sensitive_paths.warning) > 0 with input as {"diff": {"changed_files": [".github/workflows/ci.yml"]}}
}

# Positive (precise): an environment file raises an advisory.
test_env_file_warns if {
	count(sensitive_paths.warning) > 0 with input as {"diff": {"changed_files": ["config/.env.production"]}}
}

# One case per matcher (CPACKS-009). Before these, six of the rule's ten
# matchers could be deleted with the suite still green — a mutation run proved
# it. Named individually rather than table-driven in one case so `opa test
# --verbose` reports *which* matcher stopped firing.

# Precise: a composite action definition.
test_action_definition_warns if {
	count(sensitive_paths.warning) > 0 with input as {"diff": {"changed_files": [".github/actions/setup/action.yml"]}}
}

# Precise: a bare `.env` suffix, distinct from the `.env.` infix above.
test_env_suffix_warns if {
	count(sensitive_paths.warning) > 0 with input as {"diff": {"changed_files": ["config/production.env"]}}
}

# Heuristic: `secret`.
test_secret_name_warns if {
	count(sensitive_paths.warning) > 0 with input as {"diff": {"changed_files": ["src/secret_loader.rs"]}}
}

# Heuristic: `credential`.
test_credential_name_warns if {
	count(sensitive_paths.warning) > 0 with input as {"diff": {"changed_files": ["infra/credential_store.ts"]}}
}

# Heuristic: `id_rsa` suffix.
test_id_rsa_warns if {
	count(sensitive_paths.warning) > 0 with input as {"diff": {"changed_files": ["keys/id_rsa"]}}
}

# Heuristic: `token`.
test_token_file_warns if {
	count(sensitive_paths.warning) > 0 with input as {"diff": {"changed_files": ["src/token_store.rs"]}}
}

# Heuristic: `password`.
test_password_name_warns if {
	count(sensitive_paths.warning) > 0 with input as {"diff": {"changed_files": ["src/password_field.tsx"]}}
}

# Heuristic: `apikey`.
test_apikey_name_warns if {
	count(sensitive_paths.warning) > 0 with input as {"diff": {"changed_files": ["src/apikey_util.go"]}}
}

# The heuristics are case-insensitive (`lower(path)`); a capitalised name must
# still fire, or the `lower` call could be dropped unnoticed.
test_heuristic_is_case_insensitive if {
	count(sensitive_paths.warning) > 0 with input as {"diff": {"changed_files": ["src/Password_Field.tsx"]}}
}

# Negative: an ordinary source change raises no advisory.
test_ordinary_change_is_clean if {
	count(sensitive_paths.warning) == 0 with input as {"diff": {"changed_files": ["src/app.rs"]}}
}

# A precise match is not double-counted as a heuristic match.
test_precise_match_counts_once if {
	count(sensitive_paths.warning) == 1 with input as {"diff": {"changed_files": [".github/workflows/ci.yml"]}}
}
