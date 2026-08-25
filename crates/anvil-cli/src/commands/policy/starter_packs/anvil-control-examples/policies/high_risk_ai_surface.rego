# Agent, model, and prompt path review — advisory on those prefixes.
#
# Warning-tier; never vetoes. Complements ai-decision-logging (which asks for
# an audit path) by flagging each matching file for human review. Engineering
# template, not an AI-system risk classification.
#
# Reads only PolicyInput v1 `diff.changed_files`.

package anvil.policies.high_risk_ai_surface

import rego.v1

ai_surface(path) if startswith(lower(path), "agents/")

ai_surface(path) if contains(lower(path), "/agents/")

ai_surface(path) if startswith(lower(path), "models/")

ai_surface(path) if contains(lower(path), "/models/")

ai_surface(path) if startswith(lower(path), "prompts/")

ai_surface(path) if contains(lower(path), "/prompts/")

warning contains msg if {
	some path in input.diff.changed_files
	ai_surface(path)
	msg := sprintf(
		"`%s` is under an agent, model, or prompt path. Have a human review the change before it lands.",
		[path],
	)
}
