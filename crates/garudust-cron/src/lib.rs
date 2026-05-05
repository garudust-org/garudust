//! Cron-scheduled autonomous task runner for Garudust AI agents.
//!
//! [`CronScheduler`] runs agent tasks on a cron schedule — useful for
//! recurring jobs such as daily briefings, monitoring alerts, or periodic
//! data collection, all without human input.
//!
//! # Example
//!
//! ```no_run
//! use garudust_cron::{CronScheduler, parse_job_pairs};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let jobs = parse_job_pairs("0 9 * * *=send me a morning briefing");
//!     // scheduler.start(agent, jobs).await?;
//!     Ok(())
//! }
//! ```
//!
//! Jobs are specified as `"cron_expr=task"` pairs.  Use [`parse_job_pairs`]
//! to parse a comma-separated list from a config file or environment variable.

pub mod scheduler;

pub use scheduler::CronScheduler;

/// Parse a `"cron_expr=task"` pair string (comma-separated entries).
///
/// Entries that don't contain `=` are silently skipped.
pub fn parse_job_pairs(s: &str) -> Vec<(String, String)> {
    s.split(',')
        .filter_map(|entry| {
            let (expr, task) = entry.trim().split_once('=')?;
            Some((expr.trim().to_string(), task.trim().to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_job() {
        let jobs = parse_job_pairs("0 9 * * *=morning briefing");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].0, "0 9 * * *");
        assert_eq!(jobs[0].1, "morning briefing");
    }

    #[test]
    fn parses_multiple_jobs() {
        let jobs = parse_job_pairs("0 9 * * *=briefing,0 18 * * *=summary");
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[1].0, "0 18 * * *");
        assert_eq!(jobs[1].1, "summary");
    }

    #[test]
    fn skips_entries_without_equals() {
        let jobs = parse_job_pairs("bad-entry,0 9 * * *=task");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].1, "task");
    }

    #[test]
    fn trims_whitespace() {
        let jobs = parse_job_pairs("  0 9 * * *  =  do the thing  ");
        assert_eq!(jobs[0].0, "0 9 * * *");
        assert_eq!(jobs[0].1, "do the thing");
    }

    #[test]
    fn empty_string_returns_empty() {
        assert!(parse_job_pairs("").is_empty());
    }

    #[test]
    fn task_may_contain_equals_sign() {
        let jobs = parse_job_pairs("0 * * * *=key=value");
        assert_eq!(jobs[0].1, "key=value");
    }
}
