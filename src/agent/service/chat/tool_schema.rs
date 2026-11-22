use serde_json::{Value, json};

pub(super) fn tool_schema() -> Value {
    let search: Value =
        serde_json::from_str(include_str!("search.json")).expect("valid search schema");
    json!([
        search,
        {"type":"function", "name":"shell", "description":"Run a shell command on an explicitly selected authorized host. Use only for shell tasks, never as a substitute for screenshot-based computer use.",
         "strict":true, "parameters":{"type":"object", "properties":{
             "host":{"type":"string"}, "command":{"type":"string"}, "cwd":{"type":"string", "description":"Absolute working directory. Use / when unknown."}, "timeout_seconds":{"type":"integer"}
         }, "required":["host","command","cwd","timeout_seconds"], "additionalProperties":false}},
        {"type":"function", "name":"computer", "description":"Capture the desktop or send mouse and keyboard input. action_json is a JSON object using the documented computer actions. Always inspect a fresh screenshot before input. Never use accessibility trees or app-specific APIs.",
         "strict":true, "parameters":{"type":"object", "properties":{
             "host":{"type":"string"}, "action_json":{"type":"string"}
         }, "required":["host","action_json"], "additionalProperties":false}}
    ])
}
