# Tests for the change_scope policy.
#
# Every case uses the real production input shape (PolicyInput v1:
# `input.diff.changed_files`). There is no injected `config` key — the policy
# reads none.

package anvil.policies.change_scope_test

import rego.v1

import data.anvil.policies.change_scope

# Negative: a small change set raises no advisory.
test_small_change_set_is_clean if {
	count(change_scope.warning) == 0 with input as {"diff": {"changed_files": ["src/a.rs", "src/b.rs"]}}
}

# Threshold: exactly at the soft limit is still clean (the check is strict `>`).
test_soft_boundary_is_clean if {
	files := [sprintf("f%d.rs", [n]) | some n in numbers.range(1, 10)]
	count(files) == 10
	count(change_scope.warning) == 0 with input as {"diff": {"changed_files": files}}
}

# Positive: crossing the soft threshold raises one advisory.
test_soft_threshold_warns if {
	files := [sprintf("f%d.rs", [n]) | some n in numbers.range(1, 12)]
	count(change_scope.warning) > 0 with input as {"diff": {"changed_files": files}}
}

# Positive: past the hard threshold raises an advisory too.
test_hard_threshold_warns if {
	files := [sprintf("f%d.rs", [n]) | some n in numbers.range(1, 30)]
	count(change_scope.warning) > 0 with input as {"diff": {"changed_files": files}}
}
