# Coverage Minimum Policy
# Enforces minimum test coverage thresholds

package anvil.policies.coverage_min

import rego.v1

# Default minimum coverage (configurable via input.config)
default min_coverage := 80

min_coverage := input.config.min_coverage if {
  input.config.min_coverage
}

# Violation when coverage is below threshold
violation contains msg if {
  coverage := input.context.coverage.lines
  coverage < min_coverage
  msg := sprintf("Test coverage %v%% is below minimum %v%%", [coverage, min_coverage])
}

# Info when coverage is good but could be improved
info contains msg if {
  coverage := input.context.coverage.lines
  coverage >= min_coverage
  coverage < 90
  msg := sprintf("Coverage is %v%% - consider improving to 90%%+", [coverage])
}
