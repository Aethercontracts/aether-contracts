//! ═══════════════════════════════════════════════════════════════════════════════
//! Master Authenticity Scoring Engine
//! ═══════════════════════════════════════════════════════════════════════════════
//!
//! Orchestrates all four verified mathematical cores into a single
//! auditable credibility score for creator accounts.
//!
//! Score = w₁·(1 - pod_ratio) + w₂·(1 - bot_ratio) + w₃·content_quality
//!
//! ═══════════════════════════════════════════════════════════════════════════════

use sha2::{Sha256, Digest};
use crate::pod_detector::PodDetector;
use crate::bot_filter::BotFilter;
use crate::types::*;

/// Default scoring weights (sum to 1.0)
const W_POD: f64 = 0.35;   // Engagement pod penalty weight
const W_BOT: f64 = 0.35;   // Bot ratio penalty weight
const W_CONTENT: f64 = 0.30; // Content quality weight

/// The master authenticity engine
pub struct AuthenticityEngine {
    pod_detector: PodDetector,
    bot_filter: BotFilter,
    /// Content quality score (0-1), supplied by Epsilon NLP tier
    content_score: f64,
    /// Custom scoring weights
    weights: (f64, f64, f64),
}

impl AuthenticityEngine {
    pub fn new() -> Self {
        Self {
            pod_detector: PodDetector::new(),
            bot_filter: BotFilter::new(),
            content_score: 0.5, // default until NLP provides score
            weights: (W_POD, W_BOT, W_CONTENT),
        }
    }

    /// Configure custom weights (must sum to 1.0)
    pub fn with_weights(mut self, w_pod: f64, w_bot: f64, w_content: f64) -> Self {
        assert!((w_pod + w_bot + w_content - 1.0).abs() < 1e-6, "Weights must sum to 1.0");
        self.weights = (w_pod, w_bot, w_content);
        self
    }

    /// Configure the Chebyshev k-parameter for bot detection
    pub fn with_chebyshev_k(mut self, k: f64) -> Self {
        self.bot_filter = BotFilter::with_k(k);
        self
    }

    /// Configure the epsilon-neighborhood for pod detection
    pub fn with_pod_epsilon(mut self, epsilon: f64) -> Self {
        self.pod_detector = PodDetector::with_epsilon(epsilon);
        self
    }

    /// Set the content quality score from the Epsilon NLP tier
    pub fn set_content_score(&mut self, score: f64) {
        self.content_score = score.clamp(0.0, 1.0);
    }

    /// Run a full authenticity audit.
    ///
    /// # Arguments
    /// * `creator` - Creator identity
    /// * `events` - Engagement events from social platforms
    /// * `followers` - Follower behavioral features
    ///
    /// # Returns
    /// Complete `AuditReport` with all mathematical guarantees included.
    pub fn audit(
        &mut self,
        creator: CreatorId,
        events: &[EngagementEvent],
        followers: &[FollowerFeatures],
    ) -> Result<AuditReport, ScoringError> {
        // Phase 1: Pod detection via Betti approximation
        self.pod_detector.reset();
        self.pod_detector.ingest_events(events)?;
        let pod_report = self.pod_detector.analyze();

        // Phase 2: Bot filtering via Chebyshev bounds
        self.bot_filter.reset();
        self.bot_filter.ingest_followers(followers)?;
        let bot_report = self.bot_filter.analyze()?;

        // Phase 3: Weighted scoring
        let (w1, w2, w3) = self.weights;
        let pod_component = w1 * (1.0 - pod_report.pod_ratio);
        let bot_component = w2 * (1.0 - bot_report.bot_ratio);
        let content_component = w3 * self.content_score;

        let overall_score = (pod_component + bot_component + content_component) * 100.0;
        let overall_score = overall_score.clamp(0.0, 100.0);

        // Phase 4: Confidence calculation
        // Higher confidence when we have more data and lower FPR bounds
        let data_confidence = ((events.len() as f64).ln() / 10.0).min(1.0);
        let fpr_confidence = 1.0 - bot_report.max_fpr;
        let confidence = (data_confidence + fpr_confidence) / 2.0;

        // Phase 5: Generate audit hash (SHA-256 of all inputs + outputs)
        let mut hasher = Sha256::new();
        hasher.update(format!("{:?}", creator).as_bytes());
        hasher.update(format!("{}", overall_score).as_bytes());
        hasher.update(format!("{:?}", pod_report).as_bytes());
        hasher.update(format!("{:?}", bot_report).as_bytes());
        let audit_hash = format!("{:x}", hasher.finalize());

        Ok(AuditReport {
            creator,
            overall_score,
            pod_report,
            bot_report,
            content_score: self.content_score,
            confidence,
            audit_hash,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }
}

impl Default for AuthenticityEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_organic_events(count: usize) -> Vec<EngagementEvent> {
        (0..count)
            .map(|i| EngagementEvent {
                actor_id: format!("user_{}", i),
                kind: EngagementKind::Like,
                timestamp: 1000 + (i as i64) * 3600,
                content_id: format!("post_{}", i % 5),
                latency_seconds: Some(600.0 + (i as f64 * 50.0)),
            })
            .collect()
    }

    fn make_organic_followers(count: usize) -> Vec<FollowerFeatures> {
        (0..count)
            .map(|i| FollowerFeatures {
                account_id: format!("follower_{}", i),
                post_frequency: 1.0 + (i as f64 * 0.1),
                engagement_velocity: 500.0 + (i as f64 * 30.0),
                ff_ratio: 1.0 + (i as f64 * 0.05),
                account_age_days: 200.0 + (i as f64 * 20.0),
                profile_completeness: 0.8,
                avg_comment_length: 30.0 + (i as f64),
                content_diversity: 10.0 + (i as f64),
                has_avatar: true,
            })
            .collect()
    }

    #[test]
    fn test_organic_creator_high_score() {
        let mut engine = AuthenticityEngine::new();
        engine.set_content_score(0.9);

        let creator = CreatorId {
            platform: Platform::Instagram,
            handle: "@organic_creator".to_string(),
            internal_id: "org_001".to_string(),
        };

        let events = make_organic_events(30);
        let followers = make_organic_followers(20);

        let report = engine.audit(creator, &events, &followers).unwrap();

        assert!(
            report.overall_score > 50.0,
            "Organic creator should score > 50, got {}",
            report.overall_score
        );
        assert!(!report.audit_hash.is_empty());
    }

    #[test]
    fn test_score_bounded_0_to_100() {
        let mut engine = AuthenticityEngine::new();

        let creator = CreatorId {
            platform: Platform::TikTok,
            handle: "@test".to_string(),
            internal_id: "t_001".to_string(),
        };

        let events = make_organic_events(15);
        let followers = make_organic_followers(15);

        let report = engine.audit(creator, &events, &followers).unwrap();

        assert!(report.overall_score >= 0.0 && report.overall_score <= 100.0);
        assert!(report.confidence >= 0.0 && report.confidence <= 1.0);
    }

    #[test]
    fn test_weights_must_sum_to_one() {
        let result = std::panic::catch_unwind(|| {
            AuthenticityEngine::new().with_weights(0.5, 0.5, 0.5) // sum = 1.5
        });
        assert!(result.is_err());
    }
}
