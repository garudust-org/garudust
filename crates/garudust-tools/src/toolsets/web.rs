use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    net_guard,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use serde_json::json;

// ─── Safe DNS resolver ────────────────────────────────────────────────────────

/// Custom DNS resolver that filters out private/reserved IPs at resolution time,
/// closing the TOCTOU gap between is_safe_url() and reqwest's actual connect.
struct SafeResolver;

impl Resolve for SafeResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            let all: Vec<SocketAddr> = tokio::net::lookup_host(format!("{host}:0"))
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?
                .collect();

            let safe: Vec<SocketAddr> = all
                .into_iter()
                .filter(|a| !net_guard::is_blocked_ip(a.ip()))
                .collect();

            if safe.is_empty() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("SSRF protection: '{host}' resolved only to private/reserved IPs"),
                ))
                    as Box<dyn std::error::Error + Send + Sync>);
            }

            Ok(Box::new(safe.into_iter()) as Addrs)
        })
    }
}

// ─── WebFetch ─────────────────────────────────────────────────────────────────

pub struct WebFetch;

#[async_trait]
impl Tool for WebFetch {
    fn name(&self) -> &'static str {
        "web_fetch"
    }
    fn description(&self) -> &'static str {
        "Fetch content from a URL"
    }
    fn toolset(&self) -> &'static str {
        "web"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "URL to fetch" }
            },
            "required": ["url"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let url = params["url"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("url required".into()))?;

        net_guard::is_safe_url(url)?;

        let body = http_client()?
            .get(url)
            .send()
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .text()
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        Ok(ToolResult::ok("", body))
    }
}

// ─── WebSearch ────────────────────────────────────────────────────────────────

pub struct WebSearch;

#[async_trait]
impl Tool for WebSearch {
    fn name(&self) -> &'static str {
        "web_search"
    }
    fn description(&self) -> &'static str {
        "Search the web. Uses Brave Search when BRAVE_SEARCH_API_KEY is set, DuckDuckGo otherwise."
    }
    fn toolset(&self) -> &'static str {
        "web"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "count": {
                    "type": "integer",
                    "description": "Number of results to return (1-10, default 5)",
                    "default": 5
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let query = params["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("query required".into()))?;
        let count = params["count"].as_u64().unwrap_or(5).clamp(1, 10) as usize;

        match std::env::var("BRAVE_SEARCH_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
        {
            Some(api_key) => brave_search(query, count, &api_key).await,
            None => ddg_search(query, count).await,
        }
    }
}

static HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

fn http_client() -> Result<&'static reqwest::Client, ToolError> {
    if let Some(c) = HTTP_CLIENT.get() {
        return Ok(c);
    }
    let c = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; Garudust/1.0)")
        .timeout(std::time::Duration::from_secs(10))
        .dns_resolver(Arc::new(SafeResolver))
        .build()
        .map_err(|e| ToolError::Execution(format!("HTTP client init failed: {e}")))?;
    Ok(HTTP_CLIENT.get_or_init(|| c))
}

async fn brave_search(query: &str, count: usize, api_key: &str) -> Result<ToolResult, ToolError> {
    let client = http_client()?;
    let resp = client
        .get("https://api.search.brave.com/res/v1/web/search")
        .query(&[("q", query), ("count", &count.to_string())])
        .header("Accept", "application/json")
        .header("X-Subscription-Token", api_key)
        .send()
        .await
        .map_err(|e| ToolError::Execution(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(ToolError::Execution(format!(
            "Brave Search API error {status}: {body}"
        )));
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ToolError::Execution(e.to_string()))?;

    let results = data["web"]["results"]
        .as_array()
        .ok_or_else(|| ToolError::Execution("unexpected response format".into()))?;

    if results.is_empty() {
        return Ok(ToolResult::ok("", "No results found."));
    }

    let formatted: Vec<String> = results
        .iter()
        .take(count)
        .enumerate()
        .map(|(i, r)| {
            let title = r["title"].as_str().unwrap_or("(no title)");
            let url = r["url"].as_str().unwrap_or("");
            let desc = r["description"].as_str().unwrap_or("").trim();
            format!("{}. **{}**\n   {}\n   {}", i + 1, title, url, desc)
        })
        .collect();

    Ok(ToolResult::ok("", formatted.join("\n\n")))
}

async fn ddg_search(query: &str, count: usize) -> Result<ToolResult, ToolError> {
    let resp = http_client()?
        .get("https://html.duckduckgo.com/html/")
        .query(&[("q", query)])
        .send()
        .await
        .map_err(|e| ToolError::Execution(format!("DDG search failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(ToolError::Execution(format!(
            "DDG search returned {}",
            resp.status()
        )));
    }

    let html = resp
        .text()
        .await
        .map_err(|e| ToolError::Execution(e.to_string()))?;

    let items = parse_ddg_html(&html, count);
    if items.is_empty() {
        return Ok(ToolResult::ok(
            "",
            "No results found. (DDG returned no parseable results — it may be rate-limiting.)",
        ));
    }
    Ok(ToolResult::ok("", items.join("\n\n")))
}

/// DDG redirect URLs look like: `//duckduckgo.com/l/?uddg=https%3A%2F%2F...`
/// This scrapes an undocumented HTML format and may break if DDG changes its markup.
fn parse_ddg_html(html: &str, limit: usize) -> Vec<String> {
    let mut results = Vec::new();
    let mut pos = 0;

    while results.len() < limit {
        let Some(a_start) = html[pos..].find("result__title") else {
            break;
        };
        let base = pos + a_start;
        let Some(href_start) = html[base..].find("href=\"") else {
            break;
        };
        let href_off = base + href_start + 6;
        let Some(href_end) = html[href_off..].find('"') else {
            break;
        };
        let raw_url = &html[href_off..href_off + href_end];

        let url = if let Some(uddg) = raw_url.split("uddg=").nth(1) {
            let encoded = uddg.split('&').next().unwrap_or(uddg);
            percent_decode(encoded)
        } else {
            raw_url.to_string()
        };

        let Some(gt) = html[href_off + href_end..].find('>') else {
            break;
        };
        let title_off = href_off + href_end + gt + 1;
        let Some(lt) = html[title_off..].find('<') else {
            break;
        };
        let title = html[title_off..title_off + lt]
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"");

        // Search for snippet after the closing </a> tag to avoid associating the
        // wrong snippet with this result. Limit the search window to before the
        // next result title so snippets from later results don't bleed in.
        let after_title = title_off + lt;
        let next_result_off = html[after_title..]
            .find("result__title")
            .map_or(html.len(), |o| after_title + o);
        let search_window = &html[after_title..next_result_off];

        let snippet = if let Some(snip_start) = search_window.find("result__snippet") {
            let snip_base = after_title + snip_start;
            if let Some(gt2) = html[snip_base..].find('>') {
                let snip_off = snip_base + gt2 + 1;
                // Clamp search to the same window used to find this snippet so we
                // never bleed into the next result's content.
                let window_len = next_result_off.saturating_sub(snip_off);
                let snip_search = &html[snip_off..snip_off + window_len];
                // Use the closing </div or </td to avoid truncating at inline tags like </b>
                let snip_end = snip_search
                    .find("</div")
                    .or_else(|| snip_search.find("</td"))
                    .unwrap_or_else(|| snip_search.find("</").unwrap_or(window_len));
                let raw = &html[snip_off..snip_off + snip_end];
                strip_html_tags(raw)
                    .replace("&amp;", "&")
                    .replace("&nbsp;", " ")
                    .replace("&#39;", "'")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if !url.is_empty() && !title.trim().is_empty() {
            results.push(format!(
                "{}. **{}**\n   {}\n   {}",
                results.len() + 1,
                title.trim(),
                url,
                snippet.trim()
            ));
        }

        pos = title_off + lt + 1;
    }

    results
}

fn strip_html_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

fn percent_decode(s: &str) -> String {
    let mut buf: Vec<u8> = Vec::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Some(byte) = std::str::from_utf8(&b[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
            {
                buf.push(byte);
                i += 3;
                continue;
            }
        }
        buf.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&buf).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decode_ascii() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
    }

    #[test]
    fn percent_decode_thai_utf8() {
        // ก = U+0E01, UTF-8: 0xE0 0xB8 0x81 → %E0%B8%81
        assert_eq!(percent_decode("%E0%B8%81"), "ก");
    }

    #[test]
    fn percent_decode_mixed() {
        assert_eq!(
            percent_decode("https%3A%2F%2Fexample.com"),
            "https://example.com"
        );
    }

    #[test]
    fn percent_decode_invalid_sequence_passthrough() {
        // Invalid hex after % — leave bytes as-is
        assert_eq!(percent_decode("%ZZ"), "%ZZ");
    }

    #[test]
    fn parse_ddg_html_extracts_results() {
        let html = r#"
            <div class="result__title">
              <a href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com&amp;rut=abc"
                 class="result__a">Example Title</a>
            </div>
            <div class="result__snippet">A short description here.</div>
        "#;
        let results = parse_ddg_html(html, 5);
        assert_eq!(results.len(), 1);
        assert!(results[0].contains("Example Title"));
        assert!(results[0].contains("https://example.com"));
        assert!(results[0].contains("A short description here."));
    }

    #[test]
    fn parse_ddg_html_respects_limit() {
        let item = r#"
            <div class="result__title">
              <a href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com">Title</a>
            </div>
        "#;
        let html = item.repeat(10);
        let results = parse_ddg_html(&html, 3);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn parse_ddg_html_empty_on_no_results() {
        assert!(parse_ddg_html("<html><body>no results</body></html>", 5).is_empty());
    }

    #[test]
    fn parse_ddg_html_snippet_with_bold_tags() {
        // Snippet contains <b> inline tags — must not truncate at </b>
        let html = r#"
            <div class="result__title">
              <a href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com">Gold Price</a>
            </div>
            <div class="result__snippet">Current <b>gold</b> price is high today</div>
        "#;
        let results = parse_ddg_html(html, 5);
        assert_eq!(results.len(), 1);
        assert!(
            results[0].contains("Current gold price is high today"),
            "snippet was: {}",
            results[0]
        );
    }

    #[test]
    fn strip_html_tags_basic() {
        assert_eq!(strip_html_tags("hello <b>world</b>"), "hello world");
    }

    #[test]
    fn strip_html_tags_nested() {
        assert_eq!(
            strip_html_tags("<div>foo <span>bar</span></div>"),
            "foo bar"
        );
    }
}
