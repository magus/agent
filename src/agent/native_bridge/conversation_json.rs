use crate::client::chat::ChatClient;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::io;

pub(super) fn load(
    chat: &ChatClient,
    service: &str,
    id: &str,
    events: bool,
    decode_nested: bool,
) -> io::Result<Value> {
    let mut bytes = Vec::new();
    let mut snapshot: Option<(u64, u64)> = None;
    loop {
        let request = json!({"action":"conversation_json","id":id,"events":events,
            "offset":bytes.len(),"snapshot":snapshot});
        let page = chat.request_at(service, request)?;
        let version: (u64, u64) = serde_json::from_value(page["snapshot"].clone())?;
        let chunk = STANDARD
            .decode(page["bytes"].as_str().unwrap_or(""))
            .map_err(io::Error::other)?;
        bytes.extend_from_slice(&chunk);
        if bytes.len() as u64 == version.1 {
            break;
        }
        if chunk.is_empty() || bytes.len() as u64 > version.1 {
            return Err(io::Error::other("Incomplete conversation JSON."));
        }
        snapshot = Some(version);
    }

    let mut document: Value = if events {
        // A snapshot taken during a write may end partway through the last line.
        let mut event_list = Vec::<Value>::new();
        for line in bytes.split_inclusive(|byte| *byte == b'\n') {
            if line.ends_with(b"\n") {
                event_list.push(serde_json::from_slice(line)?);
            }
        }
        Value::Array(event_list)
    } else {
        serde_json::from_slice(&bytes)?
    };
    if decode_nested {
        expand_tool_json(&mut document);
    }
    Ok(Value::String(serde_json::to_string_pretty(&document)?))
}

fn expand_tool_json(value: &mut Value) {
    match value {
        Value::Array(list) => list.iter_mut().for_each(expand_tool_json),
        Value::Object(fields) => {
            let field = match fields.get("type").and_then(Value::as_str) {
                Some("function_call" | "response.function_call_arguments.done") => "arguments",
                Some("function_call_output") => "output",
                _ => "action_json",
            };
            if let Some(slot) = fields.get_mut(field)
                && let Some(text) = slot.as_str()
                && let Ok(parsed @ (Value::Array(_) | Value::Object(_))) =
                    serde_json::from_str(text)
            {
                *slot = parsed;
            }
            fields.values_mut().for_each(expand_tool_json);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_tool_fields_but_preserves_message_text_and_partial_arguments() {
        let mut doc = json!([
            {"type":"function_call","arguments":r#"{"action_json":"{\"operation\":\"capture\"}"}"#},
            {"type":"function_call_output","output":"[1,2]"},
            {"type":"function_call_output","output":"not json"},
            {"type":"message","text":"[1,2]"},
            {"type":"response.function_call_arguments.delta","delta":"{"}
        ]);
        expand_tool_json(&mut doc);
        assert_eq!(doc[0]["arguments"]["action_json"]["operation"], "capture");
        assert_eq!(doc[1]["output"], json!([1, 2]));
        assert_eq!(doc[2]["output"], "not json");
        assert_eq!(doc[3]["text"], "[1,2]");
        assert_eq!(doc[4]["delta"], "{");
    }
}
