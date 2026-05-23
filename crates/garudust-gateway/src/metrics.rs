use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;

#[derive(Default)]
pub struct Metrics {
    pub requests_total: AtomicU64,
    pub requests_active: AtomicU64,
    pub tokens_in_total: AtomicU64,
    pub tokens_out_total: AtomicU64,
    pub errors_total: AtomicU64,
    pub platform_messages: DashMap<String, AtomicU64>,
    pub platform_errors: DashMap<String, AtomicU64>,
    pub agent_iterations_total: AtomicU64,
    pub sessions_active: AtomicU64,
}

impl Metrics {
    pub fn inc_request(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        self.requests_active.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_active(&self) {
        self.requests_active.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn add_tokens(&self, input: u32, output: u32) {
        self.tokens_in_total
            .fetch_add(u64::from(input), Ordering::Relaxed);
        self.tokens_out_total
            .fetch_add(u64::from(output), Ordering::Relaxed);
    }

    pub fn inc_error(&self) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_platform_message(&self, platform: &str) {
        self.platform_messages
            .entry(platform.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_platform_error(&self, platform: &str) {
        self.platform_errors
            .entry(platform.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_iterations(&self, n: u32) {
        self.agent_iterations_total
            .fetch_add(u64::from(n), Ordering::Relaxed);
    }

    pub fn set_sessions_active(&self, n: u64) {
        self.sessions_active.store(n, Ordering::Relaxed);
    }

    pub fn prometheus_text(&self) -> String {
        let req_total = self.requests_total.load(Ordering::Relaxed);
        let req_active = self.requests_active.load(Ordering::Relaxed);
        let tok_in = self.tokens_in_total.load(Ordering::Relaxed);
        let tok_out = self.tokens_out_total.load(Ordering::Relaxed);
        let errors = self.errors_total.load(Ordering::Relaxed);
        let iterations = self.agent_iterations_total.load(Ordering::Relaxed);
        let sessions = self.sessions_active.load(Ordering::Relaxed);

        let mut out = format!(
            "# HELP garudust_requests_total Total HTTP chat requests received\n\
             # TYPE garudust_requests_total counter\n\
             garudust_requests_total {req_total}\n\
             # HELP garudust_requests_active Currently running requests\n\
             # TYPE garudust_requests_active gauge\n\
             garudust_requests_active {req_active}\n\
             # HELP garudust_tokens_total Tokens consumed\n\
             # TYPE garudust_tokens_total counter\n\
             garudust_tokens_total{{direction=\"in\"}} {tok_in}\n\
             garudust_tokens_total{{direction=\"out\"}} {tok_out}\n\
             # HELP garudust_errors_total Total request errors\n\
             # TYPE garudust_errors_total counter\n\
             garudust_errors_total {errors}\n\
             # HELP garudust_agent_iterations_total Total agent loop iterations\n\
             # TYPE garudust_agent_iterations_total counter\n\
             garudust_agent_iterations_total {iterations}\n\
             # HELP garudust_sessions_active Currently active sessions\n\
             # TYPE garudust_sessions_active gauge\n\
             garudust_sessions_active {sessions}\n"
        );

        // Per-platform message counters (sorted for deterministic output)
        let mut pm: Vec<(String, u64)> = self
            .platform_messages
            .iter()
            .map(|e| (e.key().clone(), e.value().load(Ordering::Relaxed)))
            .collect();
        pm.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        if !pm.is_empty() {
            out.push_str(
                "# HELP garudust_platform_messages_total Messages received per platform\n\
                 # TYPE garudust_platform_messages_total counter\n",
            );
            for (platform, count) in &pm {
                let _ = writeln!(
                    out,
                    "garudust_platform_messages_total{{platform=\"{platform}\"}} {count}"
                );
            }
        }

        // Per-platform error counters (sorted for deterministic output)
        let mut pe: Vec<(String, u64)> = self
            .platform_errors
            .iter()
            .map(|e| (e.key().clone(), e.value().load(Ordering::Relaxed)))
            .collect();
        pe.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        if !pe.is_empty() {
            out.push_str(
                "# HELP garudust_platform_errors_total Agent errors per platform\n\
                 # TYPE garudust_platform_errors_total counter\n",
            );
            for (platform, count) in &pe {
                let _ = writeln!(
                    out,
                    "garudust_platform_errors_total{{platform=\"{platform}\"}} {count}"
                );
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_start_at_zero() {
        let m = Metrics::default();
        assert_eq!(m.requests_total.load(Ordering::Relaxed), 0);
        assert_eq!(m.errors_total.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn inc_request_increments_total_and_active() {
        let m = Metrics::default();
        m.inc_request();
        m.inc_request();
        assert_eq!(m.requests_total.load(Ordering::Relaxed), 2);
        assert_eq!(m.requests_active.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn dec_active_decrements_without_affecting_total() {
        let m = Metrics::default();
        m.inc_request();
        m.dec_active();
        assert_eq!(m.requests_total.load(Ordering::Relaxed), 1);
        assert_eq!(m.requests_active.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn add_tokens_accumulates_correctly() {
        let m = Metrics::default();
        m.add_tokens(100, 50);
        m.add_tokens(200, 75);
        assert_eq!(m.tokens_in_total.load(Ordering::Relaxed), 300);
        assert_eq!(m.tokens_out_total.load(Ordering::Relaxed), 125);
    }

    #[test]
    fn prometheus_text_contains_expected_metric_names() {
        let m = Metrics::default();
        m.inc_request();
        m.inc_error();
        m.add_tokens(10, 5);
        let text = m.prometheus_text();
        assert!(text.contains("garudust_requests_total 1"));
        assert!(text.contains("garudust_errors_total 1"));
        assert!(text.contains("direction=\"in\"} 10"));
        assert!(text.contains("direction=\"out\"} 5"));
    }

    #[test]
    fn platform_message_counters_track_per_platform() {
        let m = Metrics::default();
        m.inc_platform_message("telegram");
        m.inc_platform_message("telegram");
        m.inc_platform_message("discord");

        let tg = m
            .platform_messages
            .get("telegram")
            .unwrap()
            .load(Ordering::Relaxed);
        let dc = m
            .platform_messages
            .get("discord")
            .unwrap()
            .load(Ordering::Relaxed);
        assert_eq!(tg, 2);
        assert_eq!(dc, 1);

        let text = m.prometheus_text();
        assert!(text.contains("platform=\"discord\"} 1"));
        assert!(text.contains("platform=\"telegram\"} 2"));
    }

    #[test]
    fn platform_error_counters_track_per_platform() {
        let m = Metrics::default();
        m.inc_platform_error("slack");
        m.inc_platform_error("slack");

        let count = m
            .platform_errors
            .get("slack")
            .unwrap()
            .load(Ordering::Relaxed);
        assert_eq!(count, 2);

        let text = m.prometheus_text();
        assert!(text.contains("platform=\"slack\"} 2"));
    }

    #[test]
    fn add_iterations_accumulates() {
        let m = Metrics::default();
        m.add_iterations(3);
        m.add_iterations(5);
        assert_eq!(m.agent_iterations_total.load(Ordering::Relaxed), 8);
    }

    #[test]
    fn set_sessions_active_stores_value() {
        let m = Metrics::default();
        m.set_sessions_active(42);
        assert_eq!(m.sessions_active.load(Ordering::Relaxed), 42);
        m.set_sessions_active(0);
        assert_eq!(m.sessions_active.load(Ordering::Relaxed), 0);
    }
}
