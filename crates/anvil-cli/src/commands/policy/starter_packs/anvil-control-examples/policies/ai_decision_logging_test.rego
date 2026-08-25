# Tests for the ai_decision_logging policy.

package anvil.policies.ai_decision_logging_test

import rego.v1

import data.anvil.policies.ai_decision_logging

test_agent_change_without_audit_warns if {
	count(ai_decision_logging.warning) > 0 with input as {"diff": {"changed_files": ["agents/router.rs"]}}
}

test_model_change_without_audit_warns if {
	count(ai_decision_logging.warning) > 0 with input as {"diff": {"changed_files": ["models/ranker.py"]}}
}

test_prompt_change_without_audit_warns if {
	count(ai_decision_logging.warning) > 0 with input as {"diff": {"changed_files": ["prompts/system.md"]}}
}

test_nested_agent_without_audit_warns if {
	count(ai_decision_logging.warning) > 0 with input as {"diff": {"changed_files": ["src/agents/tool.rs"]}}
}

test_agent_with_audit_is_clean if {
	count(ai_decision_logging.warning) == 0 with input as {"diff": {"changed_files": ["agents/router.rs", "audit/decisions.log"]}}
}

test_agent_with_logs_dir_is_clean if {
	count(ai_decision_logging.warning) == 0 with input as {"diff": {"changed_files": ["agents/router.rs", "logs/run.jsonl"]}}
}

test_ordinary_source_is_clean if {
	count(ai_decision_logging.warning) == 0 with input as {"diff": {"changed_files": ["src/app.rs"]}}
}
