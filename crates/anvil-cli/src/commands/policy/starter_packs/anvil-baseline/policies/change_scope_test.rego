# Tests for the change_scope policy.

package anvil.policies.change_scope_test

import rego.v1

import data.anvil.policies.change_scope

# A small change set trips neither the soft nor the hard bound.
test_small_change_set_is_clean if {
	count(change_scope.violation) == 0 with input as {"diff": {"changed_files": ["src/a.rs", "src/b.rs"]}}
	count(change_scope.warning) == 0 with input as {"diff": {"changed_files": ["src/a.rs", "src/b.rs"]}}
}

# A change set above the soft bound but below the ceiling warns, not refuses.
test_moderate_change_set_warns if {
	files := ["f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10", "f11", "f12"]
	count(change_scope.warning) > 0 with input as {"diff": {"changed_files": files}}
	count(change_scope.violation) == 0 with input as {"diff": {"changed_files": files}}
}

# A change set past the ceiling is a violation.
test_oversized_change_set_violates if {
	files := [sprintf("f%d.rs", [n]) | some n in numbers.range(1, 30)]
	count(files) == 30
	count(change_scope.violation) > 0 with input as {"diff": {"changed_files": files}}
}

# The ceiling is configurable through the input config object.
test_ceiling_is_configurable if {
	files := ["a", "b", "c", "d"]
	count(change_scope.violation) > 0 with input as {"diff": {"changed_files": files}, "config": {"max_changed_files": 3}}
}
