package repo

import rego.v1

# Representative repo-scan policy (POLENG-008 parity fixture). Standard Rego
# only. Heavier iteration: computes per-file fan-in over the whole edge list
# and lists hotspots — stresses comprehension/iteration on both engines.

import_count(file) := count([e |
	some e in input.repo_state.edges
	e.to == file
])

summary := {
	"file_count": count(input.repo_state.files),
	"edge_count": count(input.repo_state.edges),
	"hotspots": [f |
		some f in input.repo_state.files
		import_count(f) >= 3
	],
}
