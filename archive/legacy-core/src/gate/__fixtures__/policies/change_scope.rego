# Change Scope Policy
# Limits the scope of changes per plan

package anvil.policies.change_scope

import future.keywords.if
import future.keywords.in

# Default limits (configurable via input.config)
default max_files := 20
default max_directories := 5

max_files := input.config.max_files if {
  input.config.max_files
}

max_directories := input.config.max_directories if {
  input.config.max_directories
}

# Violation when too many files changed
violation[msg] {
  file_count := count(input.plan.proposed_changes)
  file_count > max_files
  msg := sprintf("Plan touches %v files, maximum is %v", [file_count, max_files])
}

# Violation when too many directories affected
violation[msg] {
  directories := {dir | change := input.plan.proposed_changes[_]; dir := change.directory; dir != ""}
  dir_count := count(directories)
  dir_count > max_directories
  msg := sprintf("Plan touches %v directories, maximum is %v", [dir_count, max_directories])
}

# Warning for large but acceptable changes
warning[msg] {
  file_count := count(input.plan.proposed_changes)
  file_count > 10
  file_count <= max_files
  msg := sprintf("Plan touches %v files - consider splitting into smaller changes", [file_count])
}
