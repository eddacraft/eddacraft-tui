use serde_json::Value;

use super::{status, validate_write};

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

        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, validate_write::TOOL_NAME);
        assert_eq!(tools[0].descriptor()["name"], validate_write::TOOL_NAME);
        assert_eq!(tools[1].name, status::TOOL_NAME);
        assert_eq!(tools[1].descriptor()["name"], status::TOOL_NAME);
    }

    #[test]
    fn registry_finds_validate_write_and_rejects_unknown_tools() {
        assert!(find(validate_write::TOOL_NAME).is_some());
        assert!(find(status::TOOL_NAME).is_some());
        assert!(find("anvil_check").is_none());
    }
}
