# Tests for security_baseline policy

package anvil.policies.security_baseline_test

import data.anvil.policies.security_baseline

# Test that sensitive file without review triggers violation
test_sensitive_without_review if {
  count(security_baseline.violation) > 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_update", "path": "src/auth/login.ts"}
      ],
      "tags": []
    },
    "config": {"sensitive_patterns": ["**/auth/**"]}
  }
}

# Test that sensitive file with review passes
test_sensitive_with_review if {
  count(security_baseline.violation) == 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_update", "path": "src/auth/login.ts"}
      ],
      "tags": ["security-review"]
    },
    "config": {"sensitive_patterns": ["**/auth/**"]}
  }
}

# Test that non-sensitive file passes
test_nonsensitive_passes if {
  count(security_baseline.violation) == 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_update", "path": "src/utils/helpers.ts"}
      ],
      "tags": []
    },
    "config": {"sensitive_patterns": ["**/auth/**"]}
  }
}

# Test security-reviewed tag also works
test_security_reviewed_tag if {
  count(security_baseline.violation) == 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_update", "path": "src/auth/login.ts"}
      ],
      "tags": ["security-reviewed"]
    },
    "config": {"sensitive_patterns": ["**/auth/**"]}
  }
}

# Test credential file triggers violation
test_credential_file if {
  count(security_baseline.violation) > 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_create", "path": "config/credentials.json"}
      ],
      "tags": []
    },
    "config": {"sensitive_patterns": ["**/*credential*"]}
  }
}

# Test env file triggers violation
test_env_file if {
  count(security_baseline.violation) > 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_update", "path": ".env.production"}
      ],
      "tags": []
    },
    "config": {"sensitive_patterns": ["**/*.env*"]}
  }
}
