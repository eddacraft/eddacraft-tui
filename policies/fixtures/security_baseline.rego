# Security Baseline Policy
# Requires security review for sensitive changes

package anvil.policies.security_baseline

import rego.v1

# Sensitive path patterns (configurable via input.config)
default sensitive_patterns := [
  "**/auth/**",
  "**/security/**",
  "**/*credential*",
  "**/*secret*",
  "**/*.env*",
  "**/config/keys/**"
]

sensitive_patterns := input.config.sensitive_patterns if {
  input.config.sensitive_patterns
}

# Check if a path matches any sensitive pattern
is_sensitive(path) if {
  pattern := sensitive_patterns[_]
  glob.match(pattern, ["/"], path)
}

# Violation when changing sensitive files without security-review tag
violation contains msg if {
  change := input.plan.proposed_changes[_]
  is_sensitive(change.path)
  not has_security_review
  msg := sprintf("Changes to '%s' require security-review tag", [change.path])
}

# Check for security-review tag
has_security_review if {
  "security-review" in input.plan.tags
}

has_security_review if {
  "security-reviewed" in input.plan.tags
}

# Warning for files that look like they might contain secrets
warning contains msg if {
  change := input.plan.proposed_changes[_]
  looks_like_secret_file(change.path)
  not is_sensitive(change.path)
  msg := sprintf("File '%s' may contain secrets - consider adding to sensitive_patterns", [change.path])
}

# Heuristic check for secret-like files
looks_like_secret_file(path) if {
  contains(lower(path), "key")
}

looks_like_secret_file(path) if {
  contains(lower(path), "token")
}

looks_like_secret_file(path) if {
  contains(lower(path), "password")
}
