//! Temporal reasoning support for context retrieval
//!
//! Provides time-based querying and relevance scoring for
//! enhanced temporal reasoning in RAG operations.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::context::Context;

/// Temporal query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalQuery {
    /// Reference time (defaults to now)
    pub reference_time: DateTime<Utc>,
    /// Maximum age for results
    pub max_age: Option<Duration>,
    /// Minimum age for results  
    pub min_age: Option<Duration>,
    /// Time window start
    pub window_start: Option<DateTime<Utc>>,
    /// Time window end
    pub window_end: Option<DateTime<Utc>>,
    /// Apply temporal decay to relevance scoring
    pub apply_decay: bool,
    /// Decay half-life in hours
    pub decay_half_life_hours: f64,
}

impl Default for TemporalQuery {
    fn default() -> Self {
        Self {
            reference_time: Utc::now(),
            max_age: None,
            min_age: None,
            window_start: None,
            window_end: None,
            apply_decay: true,
            decay_half_life_hours: 24.0, // 1 day half-life
        }
    }
}

impl TemporalQuery {
    /// Create a new temporal query
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum age
    pub fn with_max_age(mut self, hours: i64) -> Self {
        self.max_age = Some(Duration::hours(hours));
        self
    }

    /// Set minimum age
    pub fn with_min_age(mut self, hours: i64) -> Self {
        self.min_age = Some(Duration::hours(hours));
        self
    }

    /// Set time window
    pub fn with_window(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.window_start = Some(start);
        self.window_end = Some(end);
        self
    }

    /// Query for recent contexts (last N hours)
    pub fn recent(hours: i64) -> Self {
        Self::new().with_max_age(hours)
    }

    /// Query for contexts from today
    pub fn today() -> Self {
        let now = Utc::now();
        let start_of_day = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        
        Self {
            reference_time: now,
            window_start: Some(start_of_day),
            window_end: Some(now),
            ..Default::default()
        }
    }

    /// Query for contexts from this week
    pub fn this_week() -> Self {
        Self::new().with_max_age(24 * 7)
    }

    /// Check if a context matches temporal criteria
    pub fn matches(&self, ctx: &Context) -> bool {
        let age = self.reference_time - ctx.created_at;

        // Check max age
        if let Some(max) = self.max_age {
            if age > max {
                return false;
            }
        }

        // Check min age
        if let Some(min) = self.min_age {
            if age < min {
                return false;
            }
        }

        // Check time window
        if let Some(start) = self.window_start {
            if ctx.created_at < start {
                return false;
            }
        }

        if let Some(end) = self.window_end {
            if ctx.created_at > end {
                return false;
            }
        }

        true
    }

    /// Calculate temporal relevance score (0.0 to 1.0)
    /// Uses exponential decay based on age
    pub fn relevance_score(&self, ctx: &Context) -> f64 {
        if !self.apply_decay {
            return 1.0;
        }

        let age_hours = ctx.age_hours();
        
        // Exponential decay: score = 0.5^(age/half_life)
        let decay_factor = 0.5_f64.powf(age_hours / self.decay_half_life_hours);
        
        // Combine with importance
        let importance = ctx.metadata.importance as f64;
        
        // Weighted combination (70% temporal, 30% importance)
        0.7 * decay_factor + 0.3 * importance
    }
}

/// Temporal statistics for a set of contexts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalStats {
    /// Number of contexts
    pub count: usize,
    /// Oldest context timestamp
    pub oldest: Option<DateTime<Utc>>,
    /// Newest context timestamp  
    pub newest: Option<DateTime<Utc>>,
    /// Average age in hours
    pub avg_age_hours: f64,
    /// Distribution by time bucket
    pub distribution: TimeDistribution,
}

/// Distribution of contexts over time
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimeDistribution {
    /// Last hour
    pub last_hour: usize,
    /// Last 24 hours
    pub last_day: usize,
    /// Last week
    pub last_week: usize,
    /// Last month
    pub last_month: usize,
    /// Older
    pub older: usize,
}

impl TemporalStats {
    /// Compute temporal statistics from contexts
    pub fn from_contexts(contexts: &[Context]) -> Self {
        if contexts.is_empty() {
            return Self {
                count: 0,
                oldest: None,
                newest: None,
                avg_age_hours: 0.0,
                distribution: TimeDistribution::default(),
            };
        }

        let now = Utc::now();
        let mut oldest: Option<DateTime<Utc>> = None;
        let mut newest: Option<DateTime<Utc>> = None;
        let mut total_age_hours = 0.0;
        let mut distribution = TimeDistribution::default();

        for ctx in contexts {
            // Update oldest/newest
            if oldest.map(|o| ctx.created_at < o).unwrap_or(true) {
                oldest = Some(ctx.created_at);
            }
            if newest.map(|n| ctx.created_at > n).unwrap_or(true) {
                newest = Some(ctx.created_at);
            }

            // Accumulate age
            let age_hours = ctx.age_hours();
            total_age_hours += age_hours;

            // Update distribution
            if age_hours < 1.0 {
                distribution.last_hour += 1;
            } else if age_hours < 24.0 {
                distribution.last_day += 1;
            } else if age_hours < 24.0 * 7.0 {
                distribution.last_week += 1;
            } else if age_hours < 24.0 * 30.0 {
                distribution.last_month += 1;
            } else {
                distribution.older += 1;
            }
        }

        Self {
            count: contexts.len(),
            oldest,
            newest,
            avg_age_hours: total_age_hours / contexts.len() as f64,
            distribution,
        }
    }
}

/// Human-readable time formatting for context age
pub fn format_age(ctx: &Context) -> String {
    let age_secs = ctx.age_seconds();
    
    if age_secs < 60 {
        format!("{}s ago", age_secs)
    } else if age_secs < 3600 {
        format!("{}m ago", age_secs / 60)
    } else if age_secs < 86400 {
        format!("{}h ago", age_secs / 3600)
    } else {
        format!("{}d ago", age_secs / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextDomain;

    #[test]
    fn test_temporal_query_recent() {
        let query = TemporalQuery::recent(24);
        assert!(query.max_age.is_some());
    }

    #[test]
    fn test_relevance_decay() {
        let query = TemporalQuery::new();
        let ctx = Context::new("Test", ContextDomain::General);
        
        let score = query.relevance_score(&ctx);
        // Fresh context should have high score
        assert!(score > 0.9);
    }

    #[test]
    fn test_temporal_stats() {
        let contexts = vec![
            Context::new("Test 1", ContextDomain::General),
            Context::new("Test 2", ContextDomain::Code),
        ];

        let stats = TemporalStats::from_contexts(&contexts);
        assert_eq!(stats.count, 2);
        assert!(stats.avg_age_hours < 1.0); // Just created
    }
}
