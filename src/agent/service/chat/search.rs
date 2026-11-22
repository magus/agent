use serde_json::{Value, json};
use std::collections::HashSet;
use std::io;

pub(super) fn query_for_call(item: &Value) -> Option<String> {
    if item["type"] != "function_call" || item["name"] != "search" {
        return None;
    }
    let args: Value = serde_json::from_str(item["arguments"].as_str()?).ok()?;
    args["query"].as_str().map(str::to_owned)
}

pub(super) fn append_images(entry: &mut crate::protocol::Entry, call: &Value) {
    for image in call["results"].as_array().into_iter().flatten() {
        if image["type"] != "image_result" || !image["image_url"].is_string() {
            continue;
        }
        let exists = entry
            .image_result_list
            .iter()
            .any(|saved| saved["image_url"] == image["image_url"]);
        if !exists {
            entry.image_result_list.push(image.clone());
        }
    }
}

pub(super) fn hosted_tool(arguments: &str) -> io::Result<Value> {
    let args: Value = serde_json::from_str(arguments)?;
    args["query"]
        .as_str()
        .map(str::trim)
        .filter(|query| !query.is_empty())
        .ok_or_else(|| io::Error::other("Search requires a query."))?;

    let mut tool = json!({"type": "web_search"});
    for field in [
        "filters",
        "search_context_size",
        "search_content_types",
        "image_settings",
    ] {
        if !args[field].is_null() {
            tool[field] = args[field].clone();
        }
    }
    if !args["user_location"].is_null() {
        let mut location = args["user_location"]
            .as_object()
            .cloned()
            .ok_or_else(|| io::Error::other("user_location must be an object."))?;
        location.insert("type".into(), json!("approximate"));
        tool["user_location"] = Value::Object(location);
    }

    let wants_images = args["search_content_types"]
        .as_array()
        .is_some_and(|types| types.iter().any(|kind| kind == "image"));
    if !wants_images && !args["image_settings"].is_null() {
        return Err(io::Error::other(
            "Include image in search_content_types to use image_settings.",
        ));
    }
    Ok(tool)
}

pub(super) fn enable(request: &mut Value, tool: Value) {
    let include = request["include"]
        .as_array_mut()
        .expect("model request includes list");
    include.push(json!("web_search_call.action.sources"));
    if tool["search_content_types"]
        .as_array()
        .is_some_and(|types| types.iter().any(|kind| kind == "image"))
    {
        include.push(json!("web_search_call.results"));
    }
    request["tools"]
        .as_array_mut()
        .expect("model request declares tools")
        .push(tool);
    request["tool_choice"] = json!({"type":"web_search"});
}

pub(super) fn strip_sources(item: &mut Value) {
    if item["type"] == "web_search_call"
        && let Some(action) = item["action"].as_object_mut()
    {
        action.remove("sources");
    }
}

// Old chats stored the whole hosted response inside a custom function result.
pub(super) fn trim_legacy_results(input_list: &mut [Value]) {
    let mut search_ids = HashSet::new();
    for item in input_list {
        match item["type"].as_str() {
            Some("function_call") if item["name"] == "search" => {
                if let Some(id) = item["call_id"].as_str() {
                    search_ids.insert(id.to_owned());
                }
            }
            Some("function_call_output")
                if item["call_id"]
                    .as_str()
                    .is_some_and(|id| search_ids.remove(id)) =>
            {
                let Some(text) = item["output"].as_str() else {
                    continue;
                };
                let Ok(mut result) = serde_json::from_str::<Value>(text) else {
                    continue;
                };
                if let Some(searches) = result["searches"].as_array_mut() {
                    for search in searches {
                        strip_sources(search);
                    }
                    item["output"] = json!(result.to_string());
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_requested_options_reach_hosted_search() {
        let minimal = hosted_tool(r#"{"query":"today's news"}"#).unwrap();
        assert_eq!(minimal, json!({"type":"web_search"}));
        let options = json!({
            "query":"photos of recent snowfall",
            "filters":{"allowed_domains":["example.org"], "blocked_domains":["old.example.org"]},
            "search_context_size":"high",
            "user_location":{"city":"Denver"},
            "search_content_types":["text","image"],
            "image_settings":{"caption":true,"max_results":2}
        });
        let tool = hosted_tool(&options.to_string()).unwrap();
        for field in [
            "filters",
            "search_context_size",
            "search_content_types",
            "image_settings",
        ] {
            assert_eq!(tool[field], options[field]);
        }
        assert_eq!(
            tool["user_location"],
            json!({"type":"approximate","city":"Denver"})
        );
        let mut request = json!({"input":[{"role":"user","content":"prior context"}],"tools":[],"include":["reasoning.encrypted_content"]});
        enable(&mut request, tool);
        assert_eq!(request["input"][0]["content"], "prior context");
        assert_eq!(request["include"][2], "web_search_call.results");
        assert_eq!(request["tool_choice"]["type"], "web_search");
    }

    #[test]
    fn legacy_results_keep_citations_and_images_but_drop_source_lists() {
        let result = json!({"messages":[{"annotations":["citation"]}],"searches":[
            {"type":"web_search_call","action":{"sources":["extra"]},"results":["image"]}]});
        let mut input = vec![
            json!({"type":"function_call","call_id":"s","name":"search"}),
            json!({"type":"function_call_output","call_id":"s","output":result.to_string()}),
        ];
        trim_legacy_results(&mut input);
        let trimmed: Value = serde_json::from_str(input[1]["output"].as_str().unwrap()).unwrap();
        assert!(trimmed["searches"][0]["action"].get("sources").is_none());
        assert_eq!(trimmed["messages"], result["messages"]);
        assert_eq!(
            trimmed["searches"][0]["results"],
            result["searches"][0]["results"]
        );
    }
}
