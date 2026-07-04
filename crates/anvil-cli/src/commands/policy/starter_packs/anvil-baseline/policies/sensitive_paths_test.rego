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

# Positive (heuristic): a secret-adjacent name raises an advisory.
test_token_file_warns if {
	count(sensitive_paths.warning) > 0 with input as {"diff": {"changed_files": ["src/token_store.rs"]}}
}

# Negative: an ordinary source change raises no advisory.
test_ordinary_change_is_clean if {
	count(sensitive_paths.warning) == 0 with input as {"diff": {"changed_files": ["src/app.rs"]}}
}

# A precise match is not double-counted as a heuristic match.
test_precise_match_counts_once if {
	count(sensitive_paths.warning) == 1 with input as {"diff": {"changed_files": [".github/workflows/ci.yml"]}}
}
