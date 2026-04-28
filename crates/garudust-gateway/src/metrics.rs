use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct Metrics {
    pub requests_total: AtomicU64,
    pub requests_active: AtomicU64,
    pub tokens_in_total: AtomicU64,
    pub tokens_out_total: AtomicU64,
    pub errors_total: AtomicU64,
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

    pub fn prometheus_text(&self) -> String {
        let req_total = self.requests_total.load(Ordering::Relaxed);
        let req_active = self.requests_active.load(Ordering::Relaxed);
        let tok_in = self.tokens_in_total.load(Ordering::Relaxed);
        let tok_out = self.tokens_out_total.load(Ordering::Relaxed);
        let errors = self.errors_total.load(Ordering::Relaxed);

        format!(
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
             garudust_errors_total {errors}\n"
        )
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
}
