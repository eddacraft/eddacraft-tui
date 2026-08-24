package anvil.policies.eval.anvil_baseline_change_scope_test

import rego.v1

import data.anvil.policies.eval.anvil_baseline_change_scope as p

test_small_change_set_is_clean if {
	count(p.findings) == 0 with input as {"diff": {"changed_files": ["src/a.rs", "src/b.rs"]}}
}

test_soft_threshold_warns if {
	files := [sprintf("f%d.rs", [n]) | some n in numbers.range(1, 12)]
	count(p.findings) == 1 with input as {"diff": {"changed_files": files}}
}

test_soft_boundary_is_clean if {
	files := [sprintf("f%d.rs", [n]) | some n in numbers.range(1, 10)]
	count(p.findings) == 0 with input as {"diff": {"changed_files": files}}
}

test_hard_threshold_warns if {
	files := [sprintf("f%d.rs", [n]) | some n in numbers.range(1, 26)]
	count(p.findings) == 1 with input as {"diff": {"changed_files": files}}
}
