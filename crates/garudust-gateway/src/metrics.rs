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
