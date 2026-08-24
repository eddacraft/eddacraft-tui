package anvil.policies.eval.anvil_baseline_sensitive_paths_test

import rego.v1

import data.anvil.policies.eval.anvil_baseline_sensitive_paths as p

test_workflow_change_warns if {
	count(p.findings) == 1 with input as {"diff": {"changed_files": [".github/workflows/ci.yml"]}}
}

test_ordinary_change_is_clean if {
	count(p.findings) == 0 with input as {"diff": {"changed_files": ["src/app.rs"]}}
}

test_env_file_warns if {
	count(p.findings) == 1 with input as {"diff": {"changed_files": [".env"]}}
}

test_heuristic_token_warns if {
	count(p.findings) == 1 with input as {"diff": {"changed_files": ["src/api-token.rs"]}}
}

test_precise_match_is_not_double_counted if {
	count(p.findings) == 1 with input as {"diff": {"changed_files": [".github/workflows/secret.yml"]}}
}
