# Tests for change_scope policy

package anvil.policies.change_scope_test

import future.keywords.if
import data.anvil.policies.change_scope

# Test that too many files triggers violation
test_too_many_files if {
  count(change_scope.violation) > 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_create", "path": "f1.ts", "directory": "src"},
        {"type": "file_create", "path": "f2.ts", "directory": "src"},
        {"type": "file_create", "path": "f3.ts", "directory": "src"}
      ]
    },
    "config": {"max_files": 2}
  }
}

# Test that acceptable file count passes
test_acceptable_files if {
  count(change_scope.violation) == 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_create", "path": "f1.ts", "directory": "src"}
      ]
    },
    "config": {"max_files": 20}
  }
}

# Test too many directories
test_too_many_directories if {
  count(change_scope.violation) > 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_create", "path": "a/f1.ts", "directory": "a"},
        {"type": "file_create", "path": "b/f2.ts", "directory": "b"},
        {"type": "file_create", "path": "c/f3.ts", "directory": "c"}
      ]
    },
    "config": {"max_directories": 2}
  }
}

# Test warning for moderate changes
test_warning_for_moderate_changes if {
  count(change_scope.warning) > 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_create", "path": "f1.ts", "directory": "src"},
        {"type": "file_create", "path": "f2.ts", "directory": "src"},
        {"type": "file_create", "path": "f3.ts", "directory": "src"},
        {"type": "file_create", "path": "f4.ts", "directory": "src"},
        {"type": "file_create", "path": "f5.ts", "directory": "src"},
        {"type": "file_create", "path": "f6.ts", "directory": "src"},
        {"type": "file_create", "path": "f7.ts", "directory": "src"},
        {"type": "file_create", "path": "f8.ts", "directory": "src"},
        {"type": "file_create", "path": "f9.ts", "directory": "src"},
        {"type": "file_create", "path": "f10.ts", "directory": "src"},
        {"type": "file_create", "path": "f11.ts", "directory": "src"}
      ]
    },
    "config": {"max_files": 20}
  }
}
