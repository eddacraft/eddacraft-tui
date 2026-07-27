use serde_json::Value;

use super::{
    affected_tests, apply_patch, check, find_callers, find_dependents, fix, gate, impact_of_change,
    query_boundary, search_symbols, status, suppress, symbol_context, validate_write,
};

pub struct ToolDefinition {
    pub name: &'static str,
    pub requires_auth: bool,
    /// CIB-091d / MCP26-006: this tool projects identity-only graph data (the
    /// GCTX read surface), so its successful payload is charged against the
    /// process-local `graph://` egress byte ceiling — the same credit
    /// `resources/read` spends — closing the `tools/call` reassembly back door
    /// past the resource cap.
    pub charges_graph_egress: bool,
    descriptor: fn() -> Value,
    call: fn(&Value) -> Value,
}

impl ToolDefinition {
    pub fn descriptor(&self) -> Value {
        (self.descriptor)()
    }

    pub fn call(&self, arguments: &Value) -> Value {
        (self.call)(arguments)
    }
}

static TOOLS: &[ToolDefinition] = &[
    ToolDefinition {
        name: validate_write::TOOL_NAME,
        requires_auth: true,
        charges_graph_egress: false,
        descriptor: validate_write::descriptor,
        call: validate_write::call,
    },
    ToolDefinition {
        name: apply_patch::TOOL_NAME,
        requires_auth: true,
        charges_graph_egress: false,
        descriptor: apply_patch::descriptor,
        call: apply_patch::call,
    },
    ToolDefinition {
        name: status::TOOL_NAME,
        requires_auth: false,
        charges_graph_egress: false,
        descriptor: status::descriptor,
        call: status::call,
    },
    ToolDefinition {
        name: check::TOOL_NAME,
        requires_auth: false,
        charges_graph_egress: false,
        descriptor: check::descriptor,
        call: check::call,
    },
    // CIB-144: `anvil_gate` triggers an antipattern scan (planless mode) or a
    // full `anvil gate` subprocess (full mode), so it authenticates by default,
    // matching the mutating write tools. An unauthenticated `tools/call` gets
    // the shared auth-required envelope and the scan never runs.
    ToolDefinition {
        name: gate::TOOL_NAME,
        requires_auth: true,
        charges_graph_egress: false,
        descriptor: gate::descriptor,
        call: gate::call,
    },
    ToolDefinition {
        name: query_boundary::TOOL_NAME,
        requires_auth: false,
        charges_graph_egress: false,
        descriptor: query_boundary::descriptor,
        call: query_boundary::call,
    },
    // CIB-144: `anvil_suppress` and `anvil_fix` mutate workspace files, so they
    // authenticate by default — the same contract as the pre-write tools
    // (`anvil_validate_write` / `anvil_apply_patch`). An unauthenticated
    // `tools/call` gets the shared auth-required envelope and no write happens;
    // a local dev session (`ANVIL_DEV=1`) short-circuits the auth check and is
    // unaffected. They keep the same path-containment, redaction, and
    // embedded-fallback contract as `anvil_check`.
    ToolDefinition {
        name: suppress::TOOL_NAME,
        requires_auth: true,
        charges_graph_egress: false,
        descriptor: suppress::descriptor,
        call: suppress::call,
    },
    ToolDefinition {
        name: fix::TOOL_NAME,
        requires_auth: true,
        charges_graph_egress: false,
        descriptor: fix::descriptor,
        call: fix::call,
    },
    // GCTX-010 / ADR-084: read-only identity-only symbol search. `requires_auth`
    // is `false` for parity with the other read-only context tools (status /
    // check / query_boundary); the real authority gate is the daemon-side
    // workspace-root admission (C3 / CE-8), not the MCP auth cache.
    ToolDefinition {
        name: search_symbols::TOOL_NAME,
        requires_auth: false,
        charges_graph_egress: true,
        descriptor: search_symbols::descriptor,
        call: search_symbols::call,
    },
    // GCTX-011 / ADR-084: read-only file-keyed dependents traversal. Same
    // read-only posture as `search_symbols`; the authority gate is the
    // daemon-side workspace-root admission (C3 / CE-8), not the MCP auth cache.
    ToolDefinition {
        name: find_dependents::TOOL_NAME,
        requires_auth: false,
        charges_graph_egress: true,
        descriptor: find_dependents::descriptor,
        call: find_dependents::call,
    },
    // GCTX-014 / ADR-084 / GCALL-007: read-only symbol-keyed caller traversal.
    // Same read-only posture and daemon-side admission gate as the sibling tools.
    ToolDefinition {
        name: find_callers::TOOL_NAME,
        requires_auth: false,
        charges_graph_egress: true,
        descriptor: find_callers::descriptor,
        call: find_callers::call,
    },
    // GCTX-012 / ADR-084: read-only change-impact report. Same read-only posture
    // and daemon-side admission gate as the sibling GCTX tools.
    ToolDefinition {
        name: impact_of_change::TOOL_NAME,
        requires_auth: false,
        charges_graph_egress: true,
        descriptor: impact_of_change::descriptor,
        call: impact_of_change::call,
    },
    // GCTX-013 / ADR-084: read-only affected-tests report (likely tests +
    // coverage gaps). Same read-only posture and daemon-side admission gate.
    ToolDefinition {
        name: affected_tests::TOOL_NAME,
        requires_auth: false,
        charges_graph_egress: true,
        descriptor: affected_tests::descriptor,
        call: affected_tests::call,
    },
    // GCTX-023 / ADR-084: bounded symbol-context slice (search + impact + snippets).
    ToolDefinition {
        name: symbol_context::TOOL_NAME,
        requires_auth: false,
        charges_graph_egress: true,
        descriptor: symbol_context::descriptor,
        call: symbol_context::call,
    },
];

pub fn all() -> &'static [ToolDefinition] {
    TOOLS
}

pub fn find(name: &str) -> Option<&'static ToolDefinition> {
    all().iter().find(|tool| tool.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_lists_registered_tools() {
        let tools = all();

        assert_eq!(tools.len(), 14);
        let names: Vec<&str> = tools.iter().map(|t| t.name).collect();
        assert_eq!(
            names,
            vec![
                validate_write::TOOL_NAME,
                apply_patch::TOOL_NAME,
                status::TOOL_NAME,
                check::TOOL_NAME,
                gate::TOOL_NAME,
                query_boundary::TOOL_NAME,
                suppress::TOOL_NAME,
                fix::TOOL_NAME,
                search_symbols::TOOL_NAME,
                find_dependents::TOOL_NAME,
                find_callers::TOOL_NAME,
                impact_of_change::TOOL_NAME,
                affected_tests::TOOL_NAME,
                symbol_context::TOOL_NAME,
            ],
        );
        for tool in tools {
            assert_eq!(tool.descriptor()["name"], tool.name);
        }
    }

    #[test]
    fn mutating_and_execution_tools_require_auth() {
        // CIB-144: the file-mutating tools (`anvil_fix`, `anvil_suppress`) and
        // the execution-triggering tool (`anvil_gate`) authenticate by default,
        // matching the pre-write tools (`anvil_validate_write` /
        // `anvil_apply_patch`). An unauthenticated `tools/call` for any of them
        // must hit the auth-required branch in `tools_call_response` instead of
        // running the side effect. Read-only context tools stay open.
        for name in [fix::TOOL_NAME, suppress::TOOL_NAME, gate::TOOL_NAME] {
            let tool = find(name).unwrap_or_else(|| panic!("{name} is registered"));
            assert!(
                tool.requires_auth,
                "{name} must require auth (CIB-144: mutating/execution tools are authenticated by default)"
            );
        }
    }

    #[test]
    fn read_only_context_tools_stay_open() {
        // Guard the other side of the CIB-144 contract: the read-only status /
        // check / query tools stay unauthenticated so `requires_auth` was not
        // flipped wholesale.
        for name in [
            status::TOOL_NAME,
            check::TOOL_NAME,
            query_boundary::TOOL_NAME,
            search_symbols::TOOL_NAME,
        ] {
            let tool = find(name).unwrap_or_else(|| panic!("{name} is registered"));
            assert!(!tool.requires_auth, "{name} stays read-only/open");
        }
    }

    #[test]
    fn registry_finds_known_tools_and_rejects_unknown() {
        assert!(find(validate_write::TOOL_NAME).is_some());
        assert!(find(apply_patch::TOOL_NAME).is_some());
        assert!(find(status::TOOL_NAME).is_some());
        assert!(find(check::TOOL_NAME).is_some());
        assert!(find(gate::TOOL_NAME).is_some());
        assert!(find(query_boundary::TOOL_NAME).is_some());
        assert!(find(suppress::TOOL_NAME).is_some());
        assert!(find(fix::TOOL_NAME).is_some());
        assert!(find(search_symbols::TOOL_NAME).is_some());
        assert!(find(find_dependents::TOOL_NAME).is_some());
        assert!(find(find_callers::TOOL_NAME).is_some());
        assert!(find(impact_of_change::TOOL_NAME).is_some());
        assert!(find(affected_tests::TOOL_NAME).is_some());
        assert!(find(symbol_context::TOOL_NAME).is_some());
        assert!(find("anvil_does_not_exist").is_none());
    }
}
