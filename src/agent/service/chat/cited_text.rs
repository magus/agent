use serde_json::Value;
use std::collections::HashSet;
use url::Url;

pub(super) fn cited_text(part: &Value) -> Option<String> {
    let text = part["text"].as_str()?;
    let mut result = text.to_owned();
    let mut seen = HashSet::new();

    for annotation in part["annotations"].as_array().into_iter().flatten() {
        if annotation["type"] != "url_citation" {
            continue;
        }
        let Some(source) = annotation["url"].as_str() else {
            continue;
        };
        let Ok(url) = Url::parse(source) else {
            continue;
        };
        let is_web_link = matches!(url.scheme(), "https" | "http");
        if !is_web_link || !seen.insert(source) {
            continue;
        }
        let Some(domain) = url.domain().or_else(|| url.host_str()) else {
            continue;
        };
        if text.contains(&format!("]({source})")) {
            continue;
        }
        result.push_str(&format!("\n\n[{domain}](<{}>)", url.as_str()));
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn shows_each_missing_citation_once() {
        let citation = json!({"type":"url_citation", "url":"https://example.org/quote"});
        let part = json!({
            "text":"See [report](https://example.com/report).",
            "annotations":[
                citation, citation,
                {"type":"url_citation", "url":"https://example.com/report"}
            ]
        });
        assert_eq!(
            cited_text(&part).unwrap(),
            "See [report](https://example.com/report).\n\n[example.org](<https://example.org/quote>)"
        );
    }
}
