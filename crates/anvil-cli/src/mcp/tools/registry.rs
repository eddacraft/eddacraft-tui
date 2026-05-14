use serde_json::Value;

use super::{check, gate, status, validate_write};

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

        assert_eq!(tools.len(), 4);
        assert_eq!(tools[0].name, validate_write::TOOL_NAME);
        assert_eq!(tools[0].descriptor()["name"], validate_write::TOOL_NAME);
        assert_eq!(tools[1].name, status::TOOL_NAME);
        assert_eq!(tools[1].descriptor()["name"], status::TOOL_NAME);
        assert_eq!(tools[2].name, check::TOOL_NAME);
        assert_eq!(tools[2].descriptor()["name"], check::TOOL_NAME);
        assert_eq!(tools[3].name, gate::TOOL_NAME);
        assert_eq!(tools[3].descriptor()["name"], gate::TOOL_NAME);
    }

    #[test]
    fn registry_finds_known_tools_and_rejects_unknown() {
        assert!(find(validate_write::TOOL_NAME).is_some());
        assert!(find(status::TOOL_NAME).is_some());
        assert!(find(check::TOOL_NAME).is_some());
        assert!(find(gate::TOOL_NAME).is_some());
        assert!(find("anvil_suppress").is_none());
    }
}
