# Tests for the personal_data_paths policy.

package anvil.policies.personal_data_paths_test

import rego.v1

import data.anvil.policies.personal_data_paths

test_personal_data_dir_warns if {
	count(personal_data_paths.warning) > 0 with input as {"diff": {"changed_files": ["personal_data/export.csv"]}}
}

test_hyphenated_personal_data_warns if {
	count(personal_data_paths.warning) > 0 with input as {"diff": {"changed_files": ["data/personal-data/store.json"]}}
}

test_users_prefix_warns if {
	count(personal_data_paths.warning) > 0 with input as {"diff": {"changed_files": ["users/alice.json"]}}
}

test_nested_users_warns if {
	count(personal_data_paths.warning) > 0 with input as {"diff": {"changed_files": ["app/users/alice.json"]}}
}

test_profiles_warns if {
	count(personal_data_paths.warning) > 0 with input as {"diff": {"changed_files": ["src/profiles/settings.rs"]}}
}

test_exports_warns if {
	count(personal_data_paths.warning) > 0 with input as {"diff": {"changed_files": ["reports/exports/today.csv"]}}
}

test_pii_prefix_warns if {
	count(personal_data_paths.warning) > 0 with input as {"diff": {"changed_files": ["pii/names.txt"]}}
}

test_ordinary_source_is_clean if {
	count(personal_data_paths.warning) == 0 with input as {"diff": {"changed_files": ["src/app.rs"]}}
}
