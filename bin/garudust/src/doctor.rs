use std::time::Instant;

use garudust_core::config::AgentConfig;

struct Check {
    label: String,
    status: Status,
    detail: String,
}

enum Status {
    Ok,
    Warn,
    Fail,
}

impl Check {
    fn ok(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            status: Status::Ok,
            detail: detail.into(),
        }
    }
    fn warn(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            status: Status::Warn,
            detail: detail.into(),
        }
    }
    fn fail(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            status: Status::Fail,
            detail: detail.into(),
        }
    }
    fn print(&self) {
        let icon = match self.status {
            Status::Ok => "✓",
            Status::Warn => "!",
            Status::Fail => "✗",
        };
        println!("[{icon}] {}: {}", self.label, self.detail);
    }
    fn is_fail(&self) -> bool {
        matches!(self.status, Status::Fail)
    }
}

pub async fn run(config: &AgentConfig) {
    println!("Garudust Doctor");
    println!("{}", "─".repeat(48));

    let mut checks: Vec<Check> = Vec::new();

    // ── Provider & Model ─────────────────────────────────────────────────────
    checks.push(Check::ok("Provider", &config.provider));
    checks.push(Check::ok("Model", &config.model));

    // ── API Key ──────────────────────────────────────────────────────────────
    if let Some(k) = &config.api_key {
        checks.push(Check::ok("API key", redact(k)));
    } else {
        let hint = if config.provider == "anthropic" {
            "ANTHROPIC_API_KEY"
        } else {
            "OPENROUTER_API_KEY"
        };
        checks.push(Check::fail("API key", format!("not set — export {hint}")));
    }

    // ── Connectivity ─────────────────────────────────────────────────────────
    let base = config.base_url.clone().unwrap_or_else(|| {
        if config.provider == "anthropic" {
            "https://api.anthropic.com".into()
        } else {
            "https://openrouter.ai".into()
        }
    });
    let host = host_of(&base);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();
    let t = Instant::now();
    let ok = client.head(&base).send().await.is_ok();
    let ms = t.elapsed().as_millis();
    if ok {
        checks.push(Check::ok("Connectivity", format!("{host} ({ms}ms)")));
    } else {
        checks.push(Check::fail("Connectivity", format!("{host} — unreachable")));
    }

    // ── Memory dir ───────────────────────────────────────────────────────────
    let mem_dir = config.home_dir.join("memories");
    if mem_dir.exists() {
        let probe = mem_dir.join(".doctor_probe");
        let writable = std::fs::write(&probe, b"").is_ok();
        if writable {
            let _ = std::fs::remove_file(&probe);
        }
        let detail = format!(
            "{} ({})",
            mem_dir.display(),
            if writable { "writable" } else { "not writable" }
        );
        checks.push(if writable {
            Check::ok("Memory dir", detail)
        } else {
            Check::fail("Memory dir", detail)
        });
    } else {
        checks.push(Check::warn(
            "Memory dir",
            format!("{} (created on first run)", mem_dir.display()),
        ));
    }

    // ── Skills dir ───────────────────────────────────────────────────────────
    let skills_dir = config.home_dir.join("skills");
    if skills_dir.exists() {
        let n = garudust_tools::toolsets::skills::load_skills_from_dir(&skills_dir)
            .await
            .len();
        checks.push(Check::ok(
            "Skills dir",
            format!(
                "{} ({n} skill{} found)",
                skills_dir.display(),
                if n == 1 { "" } else { "s" }
            ),
        ));
    } else {
        checks.push(Check::warn(
            "Skills dir",
            format!("{} (not found — no skills loaded)", skills_dir.display()),
        ));
    }

    // ── Session DB ───────────────────────────────────────────────────────────
    let db_path = config.home_dir.join("state.db");
    match garudust_memory::SessionDb::open(&config.home_dir) {
        Ok(_) => checks.push(Check::ok(
            "Session DB",
            format!("{} (OK)", db_path.display()),
        )),
        Err(e) => checks.push(Check::fail("Session DB", format!("failed — {e}"))),
    }

    // ── Print ────────────────────────────────────────────────────────────────
    for c in &checks {
        c.print();
    }
    println!("{}", "─".repeat(48));

    let failures = checks.iter().filter(|c| c.is_fail()).count();
    if failures == 0 {
        println!("All checks passed.");
    } else {
        println!("{failures} check(s) failed.");
    }
}

fn redact(key: &str) -> String {
    if key.len() > 10 {
        format!("{}…{}", &key[..6], &key[key.len() - 4..])
    } else {
        "set".into()
    }
}

fn host_of(url: &str) -> &str {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(url)
}
