/// Per-million-token prices (USD) for known models.
/// Prices are approximate and updated manually. Models not listed here return `None`.
struct ModelPrice {
    input_per_m: f64,
    output_per_m: f64,
}

fn lookup(model: &str) -> Option<ModelPrice> {
    // Normalise: strip provider prefix ("anthropic/claude-...") and minor version suffixes.
    let name = model
        .rsplit_once('/')
        .map_or(model, |(_, n)| n)
        .to_lowercase();

    // Match by prefix so minor variants (e.g. -20251001) are covered.
    let p = if name.starts_with("claude-opus-4") {
        ModelPrice {
            input_per_m: 15.0,
            output_per_m: 75.0,
        }
    } else if name.starts_with("claude-sonnet-4") {
        ModelPrice {
            input_per_m: 3.0,
            output_per_m: 15.0,
        }
    } else if name.starts_with("claude-haiku-4") {
        ModelPrice {
            input_per_m: 0.8,
            output_per_m: 4.0,
        }
    } else if name.starts_with("claude-opus-3") {
        ModelPrice {
            input_per_m: 15.0,
            output_per_m: 75.0,
        }
    } else if name.starts_with("claude-sonnet-3") {
        ModelPrice {
            input_per_m: 3.0,
            output_per_m: 15.0,
        }
    } else if name.starts_with("claude-haiku-3") {
        ModelPrice {
            input_per_m: 0.25,
            output_per_m: 1.25,
        }
    } else if name.starts_with("gpt-4o-mini") {
        ModelPrice {
            input_per_m: 0.15,
            output_per_m: 0.60,
        }
    } else if name.starts_with("gpt-4o") {
        ModelPrice {
            input_per_m: 2.50,
            output_per_m: 10.0,
        }
    } else if name.starts_with("gpt-4-turbo") || name.starts_with("gpt-4-1106") {
        ModelPrice {
            input_per_m: 10.0,
            output_per_m: 30.0,
        }
    } else if name.starts_with("gpt-3.5") {
        ModelPrice {
            input_per_m: 0.50,
            output_per_m: 1.50,
        }
    } else if name.starts_with("gemini-2.5-pro") {
        ModelPrice {
            input_per_m: 1.25,
            output_per_m: 10.0,
        }
    } else if name.starts_with("gemini-2.5-flash") {
        ModelPrice {
            input_per_m: 0.15,
            output_per_m: 0.60,
        }
    } else if name.starts_with("gemini-2.0-flash") {
        ModelPrice {
            input_per_m: 0.10,
            output_per_m: 0.40,
        }
    } else if name.starts_with("gemini-1.5-pro") {
        ModelPrice {
            input_per_m: 3.50,
            output_per_m: 10.50,
        }
    } else if name.starts_with("gemini-1.5-flash") {
        ModelPrice {
            input_per_m: 0.075,
            output_per_m: 0.30,
        }
    // DeepSeek
    } else if name.starts_with("deepseek-r1") {
        ModelPrice {
            input_per_m: 0.55,
            output_per_m: 2.19,
        }
    } else if name.starts_with("deepseek-v3") {
        ModelPrice {
            input_per_m: 0.27,
            output_per_m: 1.10,
        }
    // Mistral
    } else if name.starts_with("mistral-large") {
        ModelPrice {
            input_per_m: 2.00,
            output_per_m: 6.00,
        }
    } else if name.starts_with("mistral-small") {
        ModelPrice {
            input_per_m: 0.10,
            output_per_m: 0.30,
        }
    } else if name.starts_with("codestral") {
        ModelPrice {
            input_per_m: 0.20,
            output_per_m: 0.60,
        }
    // xAI Grok
    } else if name.starts_with("grok-3-mini") {
        ModelPrice {
            input_per_m: 0.30,
            output_per_m: 0.50,
        }
    } else if name.starts_with("grok-3") {
        ModelPrice {
            input_per_m: 3.00,
            output_per_m: 15.00,
        }
    } else if name.starts_with("grok-2") {
        ModelPrice {
            input_per_m: 2.00,
            output_per_m: 10.00,
        }
    // Perplexity Sonar
    } else if name.starts_with("sonar-pro") {
        ModelPrice {
            input_per_m: 3.00,
            output_per_m: 15.00,
        }
    } else if name.starts_with("sonar-reasoning") {
        ModelPrice {
            input_per_m: 1.00,
            output_per_m: 5.00,
        }
    } else if name.starts_with("sonar") {
        ModelPrice {
            input_per_m: 1.00,
            output_per_m: 1.00,
        }
    // Cohere Command
    } else if name.starts_with("command-r-plus") {
        ModelPrice {
            input_per_m: 2.50,
            output_per_m: 10.00,
        }
    } else if name.starts_with("command-r") {
        ModelPrice {
            input_per_m: 0.15,
            output_per_m: 0.60,
        }
    // Alibaba Qwen (DashScope)
    } else if name.starts_with("qwen-max") {
        ModelPrice {
            input_per_m: 4.00,
            output_per_m: 12.00,
        }
    } else if name.starts_with("qwen-plus") {
        ModelPrice {
            input_per_m: 0.80,
            output_per_m: 2.00,
        }
    } else if name.starts_with("qwen-turbo") {
        ModelPrice {
            input_per_m: 0.20,
            output_per_m: 0.60,
        }
    // Zhipu GLM
    } else if name.starts_with("glm-4-plus") {
        ModelPrice {
            input_per_m: 5.00,
            output_per_m: 5.00,
        }
    } else if name.starts_with("glm-4") {
        ModelPrice {
            input_per_m: 0.14,
            output_per_m: 0.14,
        }
    // Moonshot Kimi
    } else if name.starts_with("moonshot-v1-128k") {
        ModelPrice {
            input_per_m: 8.00,
            output_per_m: 8.00,
        }
    } else if name.starts_with("moonshot-v1-32k") {
        ModelPrice {
            input_per_m: 3.20,
            output_per_m: 3.20,
        }
    } else if name.starts_with("moonshot-v1") {
        ModelPrice {
            input_per_m: 1.60,
            output_per_m: 1.60,
        }
    // Together AI / Cerebras / Fireworks — Llama family
    } else if name.contains("llama-3.3-70b") || name.contains("llama3.3-70b") {
        ModelPrice {
            input_per_m: 0.88,
            output_per_m: 0.88,
        }
    } else if name.contains("llama-3.1-70b") || name.contains("llama3.1-70b") {
        ModelPrice {
            input_per_m: 0.88,
            output_per_m: 0.88,
        }
    } else if name.contains("llama-3.1-8b") || name.contains("llama3.1-8b") {
        ModelPrice {
            input_per_m: 0.18,
            output_per_m: 0.18,
        }
    } else {
        return None;
    };
    Some(p)
}

/// Estimate USD cost for a completed task.
/// Returns `None` if the model is not in the price table.
pub fn estimate_cost_usd(model: &str, input_tokens: u32, output_tokens: u32) -> Option<f64> {
    let p = lookup(model)?;
    let cost = (f64::from(input_tokens) / 1_000_000.0) * p.input_per_m
        + (f64::from(output_tokens) / 1_000_000.0) * p.output_per_m;
    Some(cost)
}

/// Format a usage footer line: `[N iter | Xin Yout tok | ~$Z @ model]`
pub fn usage_footer(model: &str, iters: u32, input_tokens: u32, output_tokens: u32) -> String {
    let total = input_tokens + output_tokens;
    let cost_part = estimate_cost_usd(model, input_tokens, output_tokens)
        .map(|c| format!(" | ~${c:.3}"))
        .unwrap_or_default();
    let short_model = model.rsplit_once('/').map_or(model, |(_, n)| n);
    format!(
        "[{iters} iter | {input_tokens}in {output_tokens}out {total}tok{cost_part} @ {short_model}]"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_model_has_cost() {
        let c = estimate_cost_usd("anthropic/claude-sonnet-4-6", 10_000, 2_000);
        assert!(c.is_some());
        // 10k in @ $3/M + 2k out @ $15/M = $0.030 + $0.030 = $0.060
        let c = c.unwrap();
        assert!((c - 0.060).abs() < 0.001, "expected ~$0.06, got {c}");
    }

    #[test]
    fn unknown_model_returns_none() {
        assert!(estimate_cost_usd("local/mistral-7b", 1000, 500).is_none());
    }

    #[test]
    fn footer_includes_iter_and_tokens() {
        let footer = usage_footer("claude-sonnet-4-6", 3, 1000, 500);
        assert!(footer.contains("3 iter"));
        assert!(footer.contains("1000in"));
        assert!(footer.contains("500out"));
        assert!(footer.contains("1500tok"));
    }

    #[test]
    fn footer_strips_provider_prefix() {
        let footer = usage_footer("anthropic/claude-haiku-4-5", 1, 100, 50);
        assert!(footer.contains("@ claude-haiku-4-5"));
        assert!(!footer.contains("anthropic/"));
    }

    #[test]
    fn new_providers_have_pricing() {
        for model in &[
            "deepseek-r1",
            "deepseek-v3",
            "mistral-large-2407",
            "mistral-small-2503",
            "codestral-2405",
            "grok-3",
            "grok-3-mini",
            "grok-2-1212",
            "sonar-pro",
            "sonar-reasoning",
            "sonar",
            "command-r-plus",
            "command-r-08-2024",
            "qwen-max",
            "qwen-plus",
            "qwen-turbo",
            "glm-4-plus",
            "glm-4-0520",
            "moonshot-v1-8k",
            "moonshot-v1-32k",
            "moonshot-v1-128k",
            "llama-3.3-70b-instruct-turbo",
            "llama-3.1-8b-instruct-turbo",
            "llama3.3-70b",
            "llama3.1-70b",
        ] {
            assert!(
                estimate_cost_usd(model, 1_000, 500).is_some(),
                "missing pricing for {model}"
            );
        }
    }
}
