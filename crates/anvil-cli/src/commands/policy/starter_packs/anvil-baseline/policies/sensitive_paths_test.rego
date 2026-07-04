# Tests for the sensitive_paths policy.

package anvil.policies.sensitive_paths_test

import rego.v1

import data.anvil.policies.sensitive_paths

# A workflow change without acknowledgement is a violation.
test_workflow_change_violates if {
	count(sensitive_paths.violation) > 0 with input as {"diff": {"changed_files": [".github/workflows/ci.yml"]}}
}

# The same change with an explicit acknowledgement passes.
test_acknowledged_change_passes if {
	count(sensitive_paths.violation) == 0 with input as {
		"diff": {"changed_files": [".github/workflows/ci.yml"]},
		"config": {"review_acknowledged": true},
	}
}

# An ordinary source change is neither a violation nor a warning.
test_ordinary_change_is_clean if {
	count(sensitive_paths.violation) == 0 with input as {"diff": {"changed_files": ["src/app.rs"]}}
	count(sensitive_paths.warning) == 0 with input as {"diff": {"changed_files": ["src/app.rs"]}}
}

# An environment file is treated as sensitive.
test_env_file_violates if {
	count(sensitive_paths.violation) > 0 with input as {"diff": {"changed_files": ["config/.env.production"]}}
}

# A secret-adjacent name that is not on the sensitive list warns only.
test_token_file_warns_not_violates if {
	count(sensitive_paths.warning) > 0 with input as {"diff": {"changed_files": ["src/token_store.rs"]}}
	count(sensitive_paths.violation) == 0 with input as {"diff": {"changed_files": ["src/token_store.rs"]}}
}
