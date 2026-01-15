# Tests for coverage_min policy

package anvil.policies.coverage_min_test

import future.keywords.if
import data.anvil.policies.coverage_min

# Test that low coverage triggers violation
test_low_coverage_fails if {
  count(coverage_min.violation) > 0 with input as {
    "context": {"coverage": {"lines": 50}},
    "config": {"min_coverage": 80}
  }
}

# Test that sufficient coverage passes
test_sufficient_coverage_passes if {
  count(coverage_min.violation) == 0 with input as {
    "context": {"coverage": {"lines": 85}},
    "config": {"min_coverage": 80}
  }
}

# Test custom threshold
test_custom_threshold if {
  count(coverage_min.violation) > 0 with input as {
    "context": {"coverage": {"lines": 85}},
    "config": {"min_coverage": 90}
  }
}
