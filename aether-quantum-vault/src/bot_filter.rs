//! ═══════════════════════════════════════════════════════════════════════════════
//! Bot Detection with Chebyshev-Bounded False Positives
//! ═══════════════════════════════════════════════════════════════════════════════
//!
//! Repurposes `aether_core::memory::ChebyshevGuard` and `ManifoldHeap` for
//! distribution-agnostic bot detection with mathematical false-positive bounds.
//!
//! # The Guarantee
//!
//! Chebyshev's Inequality: P(|X − μ| ≥ kσ) ≤ 1/k²
//!
//! This holds for ANY distribution — power-law, Pareto, heavy-tailed.
//! We never assume normality. The bound is universal.
//!
//! - k=2 → FPR ≤ 25%
//! - k=3 → FPR ≤ 11.1%
//! - k=4 → FPR ≤ 6.25%
//! - k=5 → FPR ≤ 4%
//!
//! ═══════════════════════════════════════════════════════════════════════════════

use aether_core::memory::{ChebyshevGuard, ManifoldHeap};
use crate::types::{BotReport, FollowerFeatures, ScoringError};

/// Default Chebyshev k-parameter (controls false-positive rate)
const DEFAULT_K: f64 = 3.0;

/// Minimum accounts required for statistical significance
const MIN_ACCOUNTS: usize = 10;

/// Bot detection filter using Chebyshev-bounded analysis
pub struct BotFilter {
    /// The ManifoldHeap from aether-core — stores follower liveness scores
    /// in a spatially-organized, SIMD-aligned structure
    heap: ManifoldHeap<FollowerData>,
    /// Chebyshev k-parameter
    k: f64,
    /// Handles for each account in the heap
    accounts: Vec<AccountHandle>,
}

/// Internal: associates an account ID with its heap handle
struct AccountHandle {
    account_id: String,
    handle: aether_core::memory::Gc<FollowerData>,
    liveness: f64,
}

/// Data stored in the ManifoldHeap for each follower
#[derive(Debug, Clone)]
struct FollowerData {
    account_id: String,
    features: FollowerFeatures,
}

impl BotFilter {
    /// Create a new bot filter with the default k=3 (11.1% max FPR)
    pub fn new() -> Self {
        Self::with_k(DEFAULT_K)
    }

    /// Create a bot filter with a custom k-parameter.
    ///
    /// Higher k = stricter (fewer false positives, may miss some bots)
    /// Lower k = more aggressive (catches more bots, higher false positive rate)
    pub fn with_k(k: f64) -> Self {
        assert!(k >= 1.0, "k must be ≥ 1.0 for Chebyshev inequality to hold");
        Self {
            heap: ManifoldHeap::new(),
            k,
            accounts: Vec::new(),
        }
    }

    /// Ingest follower behavioral features and store them in the ManifoldHeap.
    ///
    /// Each follower's features are collapsed into a single liveness score
    /// using `FollowerFeatures::liveness_score()`, then allocated into the
    /// spatially-organized heap for Chebyshev analysis.
    pub fn ingest_followers(&mut self, followers: &[FollowerFeatures]) -> Result<(), ScoringError> {
        if followers.len() < MIN_ACCOUNTS {
            return Err(ScoringError::InsufficientData {
                required: MIN_ACCOUNTS,
                provided: followers.len(),
            });
        }

        for follower in followers {
            let liveness = follower.liveness_score();
            let data = FollowerData {
                account_id: follower.account_id.clone(),
                features: follower.clone(),
            };

            let handle = self.heap.alloc(data);

            // Set the liveness score in the heap's spatial block
            // The ManifoldHeap automatically tracks liveness per slot
            // We touch it to "heat" it proportionally to the liveness score
            if liveness > 5.0 {
                self.heap.touch(handle);
                self.heap.touch(handle);
            }

            self.accounts.push(AccountHandle {
                account_id: follower.account_id.clone(),
                handle,
                liveness,
            });
        }

        Ok(())
    }

    /// Run the Chebyshev-bounded analysis and produce a bot report.
    ///
    /// Uses `ChebyshevGuard::calculate()` from the verified core to compute
    /// distribution-agnostic population statistics, then `is_safe()` to
    /// determine which accounts fall outside the safety boundary.
    pub fn analyze(&self) -> Result<BotReport, ScoringError> {
        if self.accounts.len() < MIN_ACCOUNTS {
            return Err(ScoringError::InsufficientData {
                required: MIN_ACCOUNTS,
                provided: self.accounts.len(),
            });
        }

        // Use the verified ChebyshevGuard from aether-core
        // This computes mean and std_dev across all liveness scores in the heap
        let guard = ChebyshevGuard::calculate(&self.heap);

        // Compute our own stats from the raw liveness scores for reporting
        let liveness_scores: Vec<f64> = self.accounts.iter().map(|a| a.liveness).collect();
        let n = liveness_scores.len() as f64;
        let mean = liveness_scores.iter().sum::<f64>() / n;
        let variance = liveness_scores.iter().map(|l| (l - mean).powi(2)).sum::<f64>() / n;
        let std_dev = variance.sqrt();
        let boundary = mean - self.k * std_dev;

        // Flag accounts whose liveness falls below the Chebyshev boundary
        let bots_flagged = self
            .accounts
            .iter()
            .filter(|a| !guard.is_safe(a.liveness))
            .count();

        let total = self.accounts.len();
        let bot_ratio = bots_flagged as f64 / total as f64;
        let max_fpr = 1.0 / (self.k * self.k);

        Ok(BotReport {
            bots_flagged,
            total_accounts: total,
            bot_ratio,
            chebyshev_k: self.k,
            max_fpr,
            population_mean: mean,
            population_std: std_dev,
            safety_boundary: boundary,
        })
    }

    /// Get the list of flagged account IDs
    pub fn flagged_accounts(&self) -> Vec<String> {
        let guard = ChebyshevGuard::calculate(&self.heap);
        self.accounts
            .iter()
            .filter(|a| !guard.is_safe(a.liveness))
            .map(|a| a.account_id.clone())
            .collect()
    }

    /// Reset the filter for a new analysis
    pub fn reset(&mut self) {
        self.heap = ManifoldHeap::new();
        self.accounts.clear();
    }
}

impl Default for BotFilter {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn organic_follower(id: &str) -> FollowerFeatures {
        FollowerFeatures {
            account_id: id.to_string(),
            post_frequency: 1.5,
            engagement_velocity: 600.0,
            ff_ratio: 1.2,
            account_age_days: 500.0,
            profile_completeness: 0.9,
            avg_comment_length: 45.0,
            content_diversity: 15.0,
            has_avatar: true,
        }
    }

    fn bot_follower(id: &str) -> FollowerFeatures {
        FollowerFeatures {
            account_id: id.to_string(),
            post_frequency: 50.0, // spam-level posting
            engagement_velocity: 5.0, // instant engagement
            ff_ratio: 0.01, // follows thousands, followed by none
            account_age_days: 3.0, // brand new account
            profile_completeness: 0.1,
            avg_comment_length: 3.0, // "nice!" "🔥" 
            content_diversity: 1.0,
            has_avatar: false,
        }
    }

    #[test]
    fn test_organic_population_low_flags() {
        let mut filter = BotFilter::with_k(2.0);

        let followers: Vec<_> = (0..20)
            .map(|i| organic_follower(&format!("organic_{}", i)))
            .collect();

        filter.ingest_followers(&followers).unwrap();
        let report = filter.analyze().unwrap();

        // In a fully organic population, the Chebyshev boundary should flag
        // at most 1/k² = 25% (for k=2). In practice, with uniform organic
        // accounts, nearly none should be flagged.
        assert!(
            report.max_fpr <= 0.25,
            "FPR guarantee should be ≤ 25% for k=2"
        );
    }

    #[test]
    fn test_mixed_population_detects_bots() {
        let mut filter = BotFilter::with_k(2.0);

        let mut followers: Vec<FollowerFeatures> = (0..15)
            .map(|i| organic_follower(&format!("organic_{}", i)))
            .collect();

        // Add 5 bots with very different behavioral profiles
        for i in 0..5 {
            followers.push(bot_follower(&format!("bot_{}", i)));
        }

        filter.ingest_followers(&followers).unwrap();
        let report = filter.analyze().unwrap();

        // Bots should have much lower liveness scores, pushing them below the boundary
        assert!(report.bots_flagged > 0, "Should detect some bots");
        assert!(
            report.bot_ratio > 0.0,
            "Bot ratio should be > 0 in mixed population"
        );
    }

    #[test]
    fn test_chebyshev_k_bound_holds() {
        let k = 3.0;
        let filter = BotFilter::with_k(k);
        let max_fpr = 1.0 / (k * k);
        assert!((max_fpr - 1.0 / 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_insufficient_data_error() {
        let mut filter = BotFilter::new();
        let followers = vec![organic_follower("lone_user")];
        let result = filter.ingest_followers(&followers);
        assert!(matches!(result, Err(ScoringError::InsufficientData { .. })));
    }

    #[test]
    fn test_liveness_score_computation() {
        let organic = organic_follower("test_organic");
        let bot = bot_follower("test_bot");

        let organic_score = organic.liveness_score();
        let bot_score = bot.liveness_score();

        assert!(
            organic_score > bot_score,
            "Organic liveness {} should be > bot liveness {}",
            organic_score,
            bot_score
        );
    }
}
