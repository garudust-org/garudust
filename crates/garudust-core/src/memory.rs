use async_trait::async_trait;
use chrono::{NaiveDate, Utc};

use crate::{config::MemoryExpiryConfig, error::AgentError};

pub const ENTRY_DELIMITER: &str = "\n§\n";

#[derive(Debug, Clone, PartialEq, Default)]
pub enum MemoryCategory {
    Fact,
    Preference,
    Skill,
    Project,
    #[default]
    Other,
}

impl MemoryCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fact => "fact",
            Self::Preference => "preference",
            Self::Skill => "skill",
            Self::Project => "project",
            Self::Other => "other",
        }
    }

    pub fn from_name(s: &str) -> Self {
        match s {
            "fact" => Self::Fact,
            "preference" => Self::Preference,
            "skill" => Self::Skill,
            "project" => Self::Project,
            _ => Self::Other,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Fact => "Facts",
            Self::Preference => "Preferences",
            Self::Skill => "Skills",
            Self::Project => "Project",
            Self::Other => "Other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub category: MemoryCategory,
    pub content: String,
    pub created_at: String, // YYYY-MM-DD
}

impl MemoryEntry {
    pub fn new(category: MemoryCategory, content: String) -> Self {
        let created_at = Utc::now().format("%Y-%m-%d").to_string();
        Self {
            category,
            content,
            created_at,
        }
    }

    /// Parse `[category|date] content` or plain `content` (backward compat → Other).
    fn parse(raw: &str) -> Self {
        let raw = raw.trim();
        if let Some(rest) = raw.strip_prefix('[') {
            if let Some(bracket_end) = rest.find(']') {
                let meta = &rest[..bracket_end];
                let content = rest[bracket_end + 1..].trim().to_string();
                let mut parts = meta.splitn(2, '|');
                let cat_str = parts.next().unwrap_or("other");
                let date = parts.next().unwrap_or("").to_string();
                return Self {
                    category: MemoryCategory::from_name(cat_str),
                    content,
                    created_at: date,
                };
            }
        }
        Self {
            category: MemoryCategory::Other,
            content: raw.to_string(),
            created_at: String::new(),
        }
    }

    fn serialize(&self) -> String {
        format!(
            "[{}|{}] {}",
            self.category.as_str(),
            self.created_at,
            self.content
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct MemoryContent {
    pub entries: Vec<MemoryEntry>,
}

impl MemoryContent {
    pub fn parse(raw: &str) -> Self {
        let entries = raw
            .split(ENTRY_DELIMITER)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(MemoryEntry::parse)
            .collect();
        Self { entries }
    }

    pub fn serialize(&self) -> String {
        self.entries
            .iter()
            .map(MemoryEntry::serialize)
            .collect::<Vec<_>>()
            .join(ENTRY_DELIMITER)
    }

    /// Remove entries older than the per-category thresholds defined in `config`.
    /// Entries with no `created_at` date are never expired.
    /// Returns the number of entries removed.
    pub fn expire(&mut self, config: &MemoryExpiryConfig) -> usize {
        let today = Utc::now().date_naive();
        let before = self.entries.len();

        self.entries.retain(|e| {
            let max_days = match e.category {
                MemoryCategory::Fact => config.fact_days,
                MemoryCategory::Project => config.project_days,
                MemoryCategory::Other => config.other_days,
                MemoryCategory::Preference => config.preference_days,
                MemoryCategory::Skill => config.skill_days,
            };

            let Some(days) = max_days else {
                return true; // no limit for this category
            };

            let Ok(date) = NaiveDate::parse_from_str(&e.created_at, "%Y-%m-%d") else {
                return true; // no parseable date → keep
            };

            let age = (today - date).num_days();
            age <= i64::from(days)
        });

        before - self.entries.len()
    }

    /// Max entries returned by [`Self::prefetch`] to prevent context bloat.
    const PREFETCH_LIMIT: usize = 8;

    /// Five-or-more-char English stop words excluded from keyword matching.
    /// Only words that survive the `alpha.len() < 5` filter need to be listed here.
    const STOP_WORDS: &'static [&'static str] = &[
        "there", "about", "which", "where", "their", "those", "these", "every", "after", "other",
        "never", "still", "under", "again", "being", "since", "while", "shall", "might", "until",
        "above", "below", "maybe", "often", "quite", "would", "could", "whose", "whether",
        "however", "although", "because", "without", "within", "around", "before", "should",
        "through", "always", "almost", "already",
    ];

    /// Keyword-match recall: entries whose content contains any significant word
    /// from `query`. Tokens < 5 alphabetic chars and stop words are excluded to
    /// reduce false positives. Returns at most [`Self::PREFETCH_LIMIT`] entries,
    /// newest first.
    pub fn prefetch(&self, query: &str) -> Vec<&MemoryEntry> {
        let words: Vec<String> = query
            .split_whitespace()
            .filter_map(|w| {
                let alpha: String = w.chars().filter(|c| c.is_alphabetic()).collect();
                if alpha.len() < 5 {
                    return None;
                }
                let lower = alpha.to_lowercase();
                if Self::STOP_WORDS.contains(&lower.as_str()) {
                    return None;
                }
                Some(lower)
            })
            .collect();
        if words.is_empty() {
            return vec![];
        }
        let mut hits: Vec<&MemoryEntry> = self
            .entries
            .iter()
            .filter(|e| {
                let lower = e.content.to_lowercase();
                words.iter().any(|w| lower.contains(w.as_str()))
            })
            .collect();
        // Prefer newest entries (created_at is YYYY-MM-DD, lexicographically sortable).
        hits.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        hits.truncate(Self::PREFETCH_LIMIT);
        hits
    }

    /// Format prefetch hits as a compact block for injection into the user message.
    /// Returns empty string when no hits.
    pub fn prefetch_for_prompt(&self, query: &str) -> String {
        let hits = self.prefetch(query);
        if hits.is_empty() {
            return String::new();
        }
        hits.iter()
            .map(|e| {
                if e.created_at.is_empty() {
                    format!("- {} [{}]", e.content, e.category.display_name())
                } else {
                    format!(
                        "- {} [{}] ({})",
                        e.content,
                        e.category.display_name(),
                        e.created_at
                    )
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Grouped markdown for the system prompt.
    pub fn serialize_for_prompt(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        let order = [
            MemoryCategory::Fact,
            MemoryCategory::Preference,
            MemoryCategory::Skill,
            MemoryCategory::Project,
            MemoryCategory::Other,
        ];

        let mut sections = Vec::new();
        for cat in &order {
            let items: Vec<&MemoryEntry> =
                self.entries.iter().filter(|e| &e.category == cat).collect();
            if items.is_empty() {
                continue;
            }
            let lines: Vec<String> = items
                .iter()
                .map(|e| {
                    if e.created_at.is_empty() {
                        format!("- {}", e.content)
                    } else {
                        format!("- {} ({})", e.content, e.created_at)
                    }
                })
                .collect();
            sections.push(format!("## {}\n{}", cat.display_name(), lines.join("\n")));
        }

        sections.join("\n\n")
    }
}

#[async_trait]
pub trait MemoryStore: Send + Sync + 'static {
    async fn read_memory(&self) -> Result<MemoryContent, AgentError>;
    async fn write_memory(&self, content: &MemoryContent) -> Result<(), AgentError>;

    async fn read_user_profile(&self) -> Result<String, AgentError>;
    async fn write_user_profile(&self, content: &str) -> Result<(), AgentError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── MemoryCategory ────────────────────────────────────────────────────────

    #[test]
    fn category_roundtrip() {
        for (s, cat) in [
            ("fact", MemoryCategory::Fact),
            ("preference", MemoryCategory::Preference),
            ("skill", MemoryCategory::Skill),
            ("project", MemoryCategory::Project),
            ("other", MemoryCategory::Other),
        ] {
            assert_eq!(MemoryCategory::from_name(s), cat);
            assert_eq!(cat.as_str(), s);
        }
    }

    #[test]
    fn unknown_category_becomes_other() {
        assert_eq!(MemoryCategory::from_name("unknown"), MemoryCategory::Other);
        assert_eq!(MemoryCategory::from_name(""), MemoryCategory::Other);
    }

    // ── MemoryEntry parse ─────────────────────────────────────────────────────

    #[test]
    fn parse_structured_entry() {
        let e = MemoryEntry::parse("[fact|2026-04-27] user likes Rust");
        assert_eq!(e.category, MemoryCategory::Fact);
        assert_eq!(e.content, "user likes Rust");
        assert_eq!(e.created_at, "2026-04-27");
    }

    #[test]
    fn parse_all_category_prefixes() {
        for cat in ["fact", "preference", "skill", "project", "other"] {
            let raw = format!("[{cat}|2026-01-01] some content");
            let e = MemoryEntry::parse(&raw);
            assert_eq!(e.category, MemoryCategory::from_name(cat));
            assert_eq!(e.content, "some content");
        }
    }

    #[test]
    fn parse_plain_entry_backward_compat() {
        let e = MemoryEntry::parse("old plain memory entry");
        assert_eq!(e.category, MemoryCategory::Other);
        assert_eq!(e.content, "old plain memory entry");
        assert!(e.created_at.is_empty());
    }

    #[test]
    fn parse_trims_whitespace() {
        let e = MemoryEntry::parse("  [fact|2026-04-27]   trimmed content  ");
        assert_eq!(e.content, "trimmed content");
    }

    // ── MemoryContent parse / serialize ───────────────────────────────────────

    #[test]
    fn parse_empty_string() {
        let mc = MemoryContent::parse("");
        assert!(mc.entries.is_empty());
    }

    #[test]
    fn parse_whitespace_only() {
        let mc = MemoryContent::parse("   \n  ");
        assert!(mc.entries.is_empty());
    }

    #[test]
    fn serialize_roundtrip() {
        let raw = "[fact|2026-04-27] entry one\n§\n[preference|2026-04-27] entry two";
        let mc = MemoryContent::parse(raw);
        assert_eq!(mc.entries.len(), 2);
        let serialized = mc.serialize();
        let mc2 = MemoryContent::parse(&serialized);
        assert_eq!(mc2.entries.len(), 2);
        assert_eq!(mc2.entries[0].content, "entry one");
        assert_eq!(mc2.entries[1].content, "entry two");
        assert_eq!(mc2.entries[0].category, MemoryCategory::Fact);
        assert_eq!(mc2.entries[1].category, MemoryCategory::Preference);
    }

    #[test]
    fn serialize_roundtrip_backward_compat() {
        // Plain entries (no prefix) must survive roundtrip as Other
        let raw = "plain old entry\n§\n[fact|2026-01-01] new entry";
        let mc = MemoryContent::parse(raw);
        assert_eq!(mc.entries.len(), 2);
        assert_eq!(mc.entries[0].category, MemoryCategory::Other);
        assert_eq!(mc.entries[0].content, "plain old entry");
    }

    // ── serialize_for_prompt ─────────────────────────────────────────────────

    #[test]
    fn prompt_empty_when_no_entries() {
        let mc = MemoryContent::default();
        assert!(mc.serialize_for_prompt().is_empty());
    }

    #[test]
    fn prompt_groups_by_category() {
        let raw = "[preference|2026-04-27] short answers\n§\n[fact|2026-04-27] Rust is fast\n§\n[preference|2026-04-27] no emojis";
        let mc = MemoryContent::parse(raw);
        let prompt = mc.serialize_for_prompt();

        // Facts come before Preferences in the defined order
        let fact_pos = prompt.find("## Facts").unwrap();
        let pref_pos = prompt.find("## Preferences").unwrap();
        assert!(fact_pos < pref_pos);

        assert!(prompt.contains("- Rust is fast"));
        assert!(prompt.contains("- short answers"));
        assert!(prompt.contains("- no emojis"));
    }

    #[test]
    fn prompt_skips_empty_categories() {
        let raw = "[fact|2026-04-27] only facts here";
        let mc = MemoryContent::parse(raw);
        let prompt = mc.serialize_for_prompt();
        assert!(prompt.contains("## Facts"));
        assert!(!prompt.contains("## Preferences"));
        assert!(!prompt.contains("## Skills"));
        assert!(!prompt.contains("## Other"));
    }

    #[test]
    fn prompt_shows_date_when_present() {
        let raw = "[fact|2026-04-27] dated entry";
        let mc = MemoryContent::parse(raw);
        let prompt = mc.serialize_for_prompt();
        assert!(prompt.contains("(2026-04-27)"));
    }

    #[test]
    fn prompt_omits_date_for_plain_entries() {
        let raw = "plain entry no date";
        let mc = MemoryContent::parse(raw);
        let prompt = mc.serialize_for_prompt();
        assert!(prompt.contains("- plain entry no date"));
        assert!(!prompt.contains("()"));
    }

    // ── expire() ─────────────────────────────────────────────────────────────

    use crate::config::MemoryExpiryConfig;
    use chrono::{Duration, Utc};

    fn days_ago(n: i64) -> String {
        (Utc::now().date_naive() - Duration::days(n))
            .format("%Y-%m-%d")
            .to_string()
    }

    fn default_expiry() -> MemoryExpiryConfig {
        MemoryExpiryConfig::default() // fact=90, project=30, other=60
    }

    #[test]
    fn expire_removes_old_fact() {
        let raw = format!("[fact|{}] old fact", days_ago(91));
        let mut mc = MemoryContent::parse(&raw);
        let removed = mc.expire(&default_expiry());
        assert_eq!(removed, 1);
        assert!(mc.entries.is_empty());
    }

    #[test]
    fn expire_keeps_recent_fact() {
        let raw = format!("[fact|{}] recent fact", days_ago(10));
        let mut mc = MemoryContent::parse(&raw);
        let removed = mc.expire(&default_expiry());
        assert_eq!(removed, 0);
        assert_eq!(mc.entries.len(), 1);
    }

    #[test]
    fn expire_removes_old_project() {
        let raw = format!("[project|{}] stale project", days_ago(31));
        let mut mc = MemoryContent::parse(&raw);
        let removed = mc.expire(&default_expiry());
        assert_eq!(removed, 1);
    }

    #[test]
    fn expire_never_removes_preference() {
        let raw = format!("[preference|{}] old preference", days_ago(999));
        let mut mc = MemoryContent::parse(&raw);
        let removed = mc.expire(&default_expiry());
        assert_eq!(removed, 0);
        assert_eq!(mc.entries.len(), 1);
    }

    #[test]
    fn expire_never_removes_skill() {
        let raw = format!("[skill|{}] old skill", days_ago(999));
        let mut mc = MemoryContent::parse(&raw);
        let removed = mc.expire(&default_expiry());
        assert_eq!(removed, 0);
        assert_eq!(mc.entries.len(), 1);
    }

    #[test]
    fn expire_skips_entries_without_date() {
        let mut mc = MemoryContent::parse("plain old entry with no date");
        let removed = mc.expire(&default_expiry());
        assert_eq!(removed, 0);
        assert_eq!(mc.entries.len(), 1);
    }

    #[test]
    fn expire_returns_correct_count() {
        let raw = format!(
            "[fact|{}] keep\n§\n[fact|{}] drop one\n§\n[project|{}] drop two",
            days_ago(10),
            days_ago(91),
            days_ago(31),
        );
        let mut mc = MemoryContent::parse(&raw);
        let removed = mc.expire(&default_expiry());
        assert_eq!(removed, 2);
        assert_eq!(mc.entries.len(), 1);
        assert_eq!(mc.entries[0].content, "keep");
    }

    // ── prefetch ──────────────────────────────────────────────────────────────

    #[test]
    fn prefetch_returns_matching_entries() {
        let raw = "[preference|2026-04-29] user drinks black coffee\n§\n[fact|2026-04-29] user lives in Bangkok";
        let mc = MemoryContent::parse(raw);
        // "coffee" is 6 chars and not a stop word — should match only the first entry
        let hits = mc.prefetch("about coffee");
        assert_eq!(hits.len(), 1);
        assert!(hits[0].content.contains("coffee"));
    }

    #[test]
    fn prefetch_returns_empty_when_no_match() {
        let raw = "[preference|2026-04-29] user likes black coffee";
        let mc = MemoryContent::parse(raw);
        // "weather" doesn't appear in the entry
        let hits = mc.prefetch("current weather");
        assert!(hits.is_empty());
    }

    #[test]
    fn prefetch_is_case_insensitive() {
        let raw = "[preference|2026-04-29] user likes Black Coffee";
        let mc = MemoryContent::parse(raw);
        let hits = mc.prefetch("COFFEE");
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn prefetch_strips_punctuation_from_query_words() {
        let raw = "[preference|2026-04-29] user drinks black coffee";
        let mc = MemoryContent::parse(raw);
        let hits = mc.prefetch("coffee?");
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn prefetch_ignores_short_words() {
        let raw = "[preference|2026-04-29] user likes tea";
        let mc = MemoryContent::parse(raw);
        // all words are < 5 alpha chars — nothing should match
        let hits = mc.prefetch("is he ok now");
        assert!(hits.is_empty());
    }

    #[test]
    fn prefetch_ignores_stop_words() {
        let raw = "[preference|2026-04-29] user changed the API endpoint";
        let mc = MemoryContent::parse(raw);
        // "about", "their", "there" are >= 5 chars but "about" and "there"
        // are stop words; only "their" might vary — use a pure stop word query
        let hits = mc.prefetch("about there");
        assert!(hits.is_empty());
    }

    #[test]
    fn prefetch_caps_results_at_limit() {
        let entries: String = (0..20)
            .map(|i| {
                format!(
                    "[fact|2026-04-{:02}] keyword entry number {i}",
                    (i % 28) + 1
                )
            })
            .collect::<Vec<_>>()
            .join("\n§\n");
        let mc = MemoryContent::parse(&entries);
        let hits = mc.prefetch("keyword entry");
        assert!(hits.len() <= MemoryContent::PREFETCH_LIMIT);
    }

    #[test]
    fn prefetch_returns_newest_first() {
        let raw = "[fact|2026-01-01] keyword old entry\n§\n[fact|2026-04-29] keyword new entry";
        let mc = MemoryContent::parse(raw);
        let hits = mc.prefetch("keyword entry");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].created_at, "2026-04-29");
    }

    #[test]
    fn prefetch_for_prompt_formats_correctly() {
        let raw = "[preference|2026-04-29] user likes black coffee";
        let mc = MemoryContent::parse(raw);
        let block = mc.prefetch_for_prompt("coffee preference");
        assert!(block.contains("user likes black coffee"));
        assert!(block.contains("[Preferences]"));
        assert!(block.contains("2026-04-29"));
    }

    #[test]
    fn expire_disabled_when_days_is_none() {
        let config = MemoryExpiryConfig {
            fact_days: None,
            project_days: None,
            other_days: None,
            preference_days: None,
            skill_days: None,
        };
        let raw = format!("[fact|{}] very old", days_ago(9999));
        let mut mc = MemoryContent::parse(&raw);
        let removed = mc.expire(&config);
        assert_eq!(removed, 0);
    }
}
