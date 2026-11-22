use super::tool_schema::tool_schema;
use crate::protocol::{ModelSettings, Target};
use serde_json::{Value, json};

pub(super) fn model_request(
    settings: &ModelSettings,
    input_list: &[Value],
    device_list: &[Value],
    host: &Target,
) -> Value {
    let mut request = json!({
        "model": settings.model,
        "store": false,
        "stream": true,
        "include": ["reasoning.encrypted_content"],
        "parallel_tool_calls": false,
        "instructions": format!("{}\nAvailable devices {}\nCurrent loop host {}",
            include_str!("instructions.txt"), json!(device_list), json!(host)),
        "input": input_list,
        "tools": tool_schema(),
    });
    super::search::trim_legacy_results(request["input"].as_array_mut().unwrap());
    if let Some(effort) = &settings.reasoning_effort {
        request["reasoning"] = json!({"effort": effort});
    }
    request
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_settings_are_independent_of_later_selections() {
        let mut next_turn = ModelSettings {
            model: "first".into(),
            reasoning_effort: Some("high".into()),
        };
        let active_turn = next_turn.clone();
        next_turn.model = "second".into();
        next_turn.reasoning_effort = None;
        let host = Target {
            id: "service".into(),
            name: "Agent service".into(),
        };
        let input_list = vec![json!({"role":"user", "content":"hello"})];

        let active_request = model_request(&active_turn, &input_list, &[], &host);
        assert_eq!(active_request["model"], "first");
        assert_eq!(active_request["reasoning"]["effort"], "high");
        assert_eq!(active_request["input"], json!(input_list));

        let next_request = model_request(&next_turn, &input_list, &[], &host);
        assert_eq!(next_request["model"], "second");
        assert!(next_request.get("reasoning").is_none());
    }
}
