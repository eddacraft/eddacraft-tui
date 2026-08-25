# Agent/model/prompt change without an audit or log path — advisory nudge.
#
# Fires when the change set touches an agent, model, or prompt path and does
# not also touch an audit or log path. Warning-tier; never vetoes.
# Engineering template, not an AI-system logging requirement.
#
# Reads only PolicyInput v1 `diff.changed_files`.

package anvil.policies.ai_decision_logging

import rego.v1

ai_surface(path) if startswith(lower(path), "agents/")

ai_surface(path) if contains(lower(path), "/agents/")

ai_surface(path) if startswith(lower(path), "models/")

ai_surface(path) if contains(lower(path), "/models/")

ai_surface(path) if startswith(lower(path), "prompts/")

ai_surface(path) if contains(lower(path), "/prompts/")

audit_path(path) if contains(lower(path), "audit")

audit_path(path) if startswith(lower(path), "logs/")

audit_path(path) if contains(lower(path), "/logs/")

audit_path(path) if contains(lower(path), "/log/")

touches_ai if {
	some path in input.diff.changed_files
	ai_surface(path)
}

touches_audit if {
	some path in input.diff.changed_files
	audit_path(path)
}

warning contains msg if {
	touches_ai
	not touches_audit
	msg := "This change touches an agent, model, or prompt path without also touching an audit or log path. Consider recording the decision trail in the same change set."
}
