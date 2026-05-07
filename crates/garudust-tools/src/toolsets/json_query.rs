use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde::Deserialize;
use serde_json::{json, Value};

/// Apply a simple jq-style path expression to a JSON value.
///
/// Supported syntax:
/// - `.`           — identity (return root)
/// - `.field`      — object field
/// - `.[0]`        — array index (negative indices count from end)
/// - `.[*]`        — all array elements (returns newline-separated JSON)
/// - `keys`        — list keys of an object
/// - `length`      — number of elements / string length
/// - Chaining: `.field.[0].nested`
fn apply_path(value: &Value, expr: &str) -> Result<Value, String> {
    let expr = expr.trim();

    if expr == "." {
        return Ok(value.clone());
    }
    if expr == "keys" {
        return match value {
            Value::Object(m) => Ok(Value::Array(
                m.keys().map(|k| Value::String(k.clone())).collect(),
            )),
            _ => Err("keys: not an object".into()),
        };
    }
    if expr == "length" {
        return Ok(match value {
            Value::Array(a) => json!(a.len()),
            Value::Object(m) => json!(m.len()),
            Value::String(s) => json!(s.len()),
            Value::Null => json!(0),
            _ => return Err("length: unsupported type".into()),
        });
    }

    // Chain: split on the first segment and recurse.
    let (head, tail) = split_first_segment(expr)?;

    let next = if head == "." || head.is_empty() {
        value.clone()
    } else if let Some(idx_str) = head.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        // Array index or wildcard
        match idx_str {
            "*" => {
                let arr = value
                    .as_array()
                    .ok_or_else(|| format!("[*]: not an array (got {value})"))?;
                let items: Vec<String> = arr
                    .iter()
                    .map(|v| serde_json::to_string(v).unwrap_or_default())
                    .collect();
                // Wildcard returns early — further chaining not supported after [*]
                return Ok(Value::String(items.join("\n")));
            }
            n => {
                let idx: i64 = n
                    .parse()
                    .map_err(|_| format!("invalid array index '{n}'"))?;
                let arr = value
                    .as_array()
                    .ok_or_else(|| format!("[{n}]: not an array"))?;
                let i = if idx < 0 {
                    let neg =
                        usize::try_from(-idx).map_err(|_| format!("index {idx} out of range"))?;
                    arr.len()
                        .checked_sub(neg)
                        .ok_or_else(|| format!("index {idx} out of bounds"))?
                } else {
                    usize::try_from(idx).map_err(|_| format!("index {idx} out of range"))?
                };
                arr.get(i)
                    .cloned()
                    .ok_or_else(|| format!("index {idx} out of bounds (len {})", arr.len()))?
            }
        }
    } else {
        // Object field (strip leading dot)
        let field = head.strip_prefix('.').unwrap_or(head);
        value
            .get(field)
            .cloned()
            .ok_or_else(|| format!("field '{field}' not found"))?
    };

    if tail.is_empty() {
        Ok(next)
    } else {
        apply_path(&next, tail)
    }
}

/// Split `expr` into (first_segment, remainder).
fn split_first_segment(expr: &str) -> Result<(&str, &str), String> {
    if expr.is_empty() {
        return Ok((".", ""));
    }
    // Leading dot: consume it and continue
    if expr == "." {
        return Ok((".", ""));
    }
    // Array index: [n] or [*]
    if expr.starts_with('[') {
        let end = expr.find(']').ok_or("unmatched '['")?;
        let (head, rest) = expr.split_at(end + 1);
        let rest = rest.strip_prefix('.').unwrap_or(rest);
        return Ok((head, rest));
    }
    // Field name (strip leading dot)
    let s = expr.strip_prefix('.').unwrap_or(expr);
    // Find next separator (. or [)
    let end = s.find(['.', '[']).unwrap_or(s.len());
    let (field, rest) = s.split_at(end);
    let head = field; // without the leading dot
    let rest = rest.strip_prefix('.').unwrap_or(rest);
    Ok((head, rest))
}

// ── Tool impl ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct JsonQueryInput {
    json: String,
    query: String,
}

pub struct JsonQuery;

#[async_trait]
impl Tool for JsonQuery {
    fn name(&self) -> &'static str {
        "json_query"
    }

    fn description(&self) -> &'static str {
        "Apply a jq-style path expression to a JSON string and return the matching value. Supports field access (.field), array index (.[0]), wildcard (.[*]), keys, and length."
    }

    fn toolset(&self) -> &'static str {
        "web"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "json": {
                    "type": "string",
                    "description": "The JSON string to query."
                },
                "query": {
                    "type": "string",
                    "description": "Path expression, e.g. '.users.[0].name', 'keys', 'length', '.[*]'."
                }
            },
            "required": ["json", "query"]
        })
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let input: JsonQueryInput =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let value: Value = serde_json::from_str(&input.json)
            .map_err(|e| ToolError::InvalidArgs(format!("invalid JSON: {e}")))?;

        let result = apply_path(&value, &input.query).map_err(ToolError::Execution)?;

        let output = match &result {
            Value::String(s) => s.clone(),
            other => serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
        };

        Ok(ToolResult::ok("", output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn q(value: &Value, expr: &str) -> Result<Value, String> {
        apply_path(value, expr)
    }

    #[test]
    fn identity() {
        let v = json!({"a": 1});
        assert_eq!(q(&v, ".").unwrap(), v);
    }

    #[test]
    fn field_access() {
        let v = json!({"name": "alice", "age": 30});
        assert_eq!(q(&v, ".name").unwrap(), json!("alice"));
        assert_eq!(q(&v, ".age").unwrap(), json!(30));
    }

    #[test]
    fn nested_field() {
        let v = json!({"user": {"name": "bob"}});
        assert_eq!(q(&v, ".user.name").unwrap(), json!("bob"));
    }

    #[test]
    fn array_index() {
        let v = json!([10, 20, 30]);
        assert_eq!(q(&v, ".[0]").unwrap(), json!(10));
        assert_eq!(q(&v, ".[2]").unwrap(), json!(30));
    }

    #[test]
    fn negative_array_index() {
        let v = json!([1, 2, 3]);
        assert_eq!(q(&v, ".[-1]").unwrap(), json!(3));
    }

    #[test]
    fn array_wildcard() {
        let v = json!([1, 2, 3]);
        let result = q(&v, ".[*]").unwrap();
        assert_eq!(result, json!("1\n2\n3"));
    }

    #[test]
    fn keys_on_object() {
        let v = json!({"b": 2, "a": 1});
        let result = q(&v, "keys").unwrap();
        let keys: Vec<String> = result
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k.as_str().unwrap().to_string())
            .collect();
        assert!(keys.contains(&"a".to_string()));
        assert!(keys.contains(&"b".to_string()));
    }

    #[test]
    fn length_array() {
        let v = json!([1, 2, 3]);
        assert_eq!(q(&v, "length").unwrap(), json!(3));
    }

    #[test]
    fn length_string() {
        let v = json!("hello");
        assert_eq!(q(&v, "length").unwrap(), json!(5));
    }

    #[test]
    fn missing_field_errors() {
        let v = json!({"a": 1});
        assert!(q(&v, ".b").is_err());
    }

    #[test]
    fn field_then_index() {
        let v = json!({"items": [10, 20, 30]});
        assert_eq!(q(&v, ".items.[1]").unwrap(), json!(20));
    }
}
