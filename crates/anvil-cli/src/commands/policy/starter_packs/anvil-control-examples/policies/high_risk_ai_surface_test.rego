# Tests for the high_risk_ai_surface policy.

package anvil.policies.high_risk_ai_surface_test

import rego.v1

import data.anvil.policies.high_risk_ai_surface

test_agents_prefix_warns if {
	count(high_risk_ai_surface.warning) > 0 with input as {"diff": {"changed_files": ["agents/router.rs"]}}
}

test_nested_agents_warns if {
	count(high_risk_ai_surface.warning) > 0 with input as {"diff": {"changed_files": ["src/agents/tool.rs"]}}
}

test_models_prefix_warns if {
	count(high_risk_ai_surface.warning) > 0 with input as {"diff": {"changed_files": ["models/ranker.py"]}}
}

test_prompts_prefix_warns if {
	count(high_risk_ai_surface.warning) > 0 with input as {"diff": {"changed_files": ["prompts/system.md"]}}
}

test_ordinary_source_is_clean if {
	count(high_risk_ai_surface.warning) == 0 with input as {"diff": {"changed_files": ["src/app.rs"]}}
}

test_each_matching_path_is_flagged if {
	count(high_risk_ai_surface.warning) == 2 with input as {"diff": {"changed_files": ["agents/a.rs", "prompts/b.md"]}}
}
