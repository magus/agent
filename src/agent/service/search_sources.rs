use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

// Resolve display metadata only when the user opens a search row.
pub(super) fn load(directory: &Path, entry_index: usize) -> io::Result<Value> {
    let file = File::open(directory.join("conversation.json"))?;
    let saved: Value = serde_json::from_reader(BufReader::new(file))?;
    let entries = saved["entry_list"]
        .as_array()
        .ok_or_else(|| io::Error::other("Missing transcript"))?;
    if entries
        .get(entry_index)
        .is_none_or(|item| item["role"] != "Search")
    {
        return Err(io::Error::other("Search row is unavailable."));
    }
    let ordinal = entries[..entry_index]
        .iter()
        .filter(|item| item["role"] == "Search")
        .count();
    let history = saved["input_list"]
        .as_array()
        .ok_or_else(|| io::Error::other("Missing history"))?;
    let is_search = |item: &Value| item["type"] == "function_call" && item["name"] == "search";
    let (start, call) = history
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            is_search(item)
                && item["arguments"]
                    .as_str()
                    .and_then(|args| serde_json::from_str::<Value>(args).ok())
                    .is_some_and(|args| args["query"].is_string())
        })
        .nth(ordinal)
        .ok_or_else(|| io::Error::other("Search call is unavailable."))?;

    let mut links = BTreeMap::new();
    let mut hosted_ids = HashSet::new();
    for item in &history[start + 1..] {
        if is_search(item) || item["role"] == "user" {
            break;
        }
        match item["type"].as_str() {
            Some("function_call_output") if item["call_id"] == call["call_id"] => {
                if let Some(result) = item["output"]
                    .as_str()
                    .and_then(|text| serde_json::from_str::<Value>(text).ok())
                    && let Some(searches) = result["searches"].as_array()
                {
                    for search in searches {
                        collect_sources(search, &mut links);
                    }
                    for parts in result["messages"].as_array().into_iter().flatten() {
                        collect_citations(parts, &mut links);
                    }
                    break;
                }
            }
            Some("web_search_call") => {
                if let Some(id) = item["id"].as_str() {
                    hosted_ids.insert(id);
                }
                collect_sources(item, &mut links);
            }
            Some("message") if !hosted_ids.is_empty() => {
                collect_citations(&item["content"], &mut links);
            }
            _ => {}
        }
    }
    if !hosted_ids.is_empty() {
        let file = match File::open(directory.join("openai-events.jsonl")) {
            Ok(file) => Some(file),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        if let Some(file) = file {
            for line in BufReader::new(file).lines() {
                let line = line?;
                if !line.contains("web_search_call") {
                    continue;
                }
                let Ok(record) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                let event = &record["event"];
                let mut collect = |item: &Value| {
                    if item["id"]
                        .as_str()
                        .is_some_and(|id| hosted_ids.contains(id))
                    {
                        collect_sources(item, &mut links);
                    }
                };
                match event["type"].as_str() {
                    Some("response.output_item.done") => collect(&event["item"]),
                    Some("response.completed") => {
                        for item in event["response"]["output"].as_array().into_iter().flatten() {
                            collect(item);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(json!(
        links
            .into_iter()
            .map(|(url, title)| json!({"url":url,"citation":title}))
            .collect::<Vec<_>>()
    ))
}

fn collect_sources(item: &Value, links: &mut BTreeMap<String, Option<String>>) {
    for source in item["action"]["sources"].as_array().into_iter().flatten() {
        if let Some(url) = source["url"].as_str() {
            links.entry(url.into()).or_default();
        }
    }
}

fn collect_citations(parts: &Value, links: &mut BTreeMap<String, Option<String>>) {
    for part in parts.as_array().into_iter().flatten() {
        for citation in part["annotations"].as_array().into_iter().flatten() {
            if citation["type"] == "url_citation"
                && let Some(url) = citation["url"].as_str()
            {
                links.insert(
                    url.into(),
                    Some(citation["title"].as_str().unwrap_or(url).into()),
                );
            }
        }
    }
}
