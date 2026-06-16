use serde_json::Value;

use super::{
    apply_patch, check, find_dependents, fix, gate, query_boundary, search_symbols, status,
    suppress, validate_write,
};

pub struct ToolDefinition {
    pub name: &'static str,
    pub requires_auth: bool,
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
        descriptor: validate_write::descriptor,
        call: validate_write::call,
    },
    ToolDefinition {
        name: apply_patch::TOOL_NAME,
        requires_auth: true,
        descriptor: apply_patch::descriptor,
        call: apply_patch::call,
    },
    ToolDefinition {
        name: status::TOOL_NAME,
        requires_auth: false,
        descriptor: status::descriptor,
        call: status::call,
    },
    ToolDefinition {
        name: check::TOOL_NAME,
        requires_auth: false,
        descriptor: check::descriptor,
        call: check::call,
    },
    ToolDefinition {
        name: gate::TOOL_NAME,
        requires_auth: false,
        descriptor: gate::descriptor,
        call: gate::call,
    },
    ToolDefinition {
        name: query_boundary::TOOL_NAME,
        requires_auth: false,
        descriptor: query_boundary::descriptor,
        call: query_boundary::call,
    },
    // `anvil_suppress` and `anvil_fix` mutate workspace files. They keep the
    // same path-containment, redaction, and embedded-fallback contract as
    // `anvil_check`. `requires_auth` stays `false` for parity with the
    // archived TS server until RMCPF-011 reviewers ratify a stricter
    // authority surface (e.g. forcing daemon-RPC `suppression.apply`).
    ToolDefinition {
        name: suppress::TOOL_NAME,
        requires_auth: false,
        descriptor: suppress::descriptor,
        call: suppress::call,
    },
    ToolDefinition {
        name: fix::TOOL_NAME,
        requires_auth: false,
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
        descriptor: search_symbols::descriptor,
        call: search_symbols::call,
    },
    // GCTX-011 / ADR-084: read-only file-keyed dependents traversal. Same
    // read-only posture as `search_symbols`; the authority gate is the
    // daemon-side workspace-root admission (C3 / CE-8), not the MCP auth cache.
    ToolDefinition {
        name: find_dependents::TOOL_NAME,
        requires_auth: false,
        descriptor: find_dependents::descriptor,
        call: find_dependents::call,
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

        assert_eq!(tools.len(), 10);
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
            ],
        );
        for tool in tools {
            assert_eq!(tool.descriptor()["name"], tool.name);
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
        assert!(find("anvil_does_not_exist").is_none());
    }
}
