//! ═══════════════════════════════════════════════════════════════════════════════
//! Shared Data Types for AetherContracts
//! ═══════════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════════
// Creator & Social Graph Types
// ═══════════════════════════════════════════════════════════════════════════════

/// A unique creator identity across platforms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorId {
    pub platform: Platform,
    pub handle: String,
    pub internal_id: String,
}

/// Supported social media platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platform {
    Instagram,
    TikTok,
    YouTube,
    X,
    Twitch,
}

/// Raw engagement data point from a social platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementEvent {
    /// Who engaged (follower/commenter)
    pub actor_id: String,
    /// What kind of engagement
    pub kind: EngagementKind,
    /// Unix timestamp
    pub timestamp: i64,
    /// Target content ID (post/video/tweet)
    pub content_id: String,
    /// Optional: response latency in seconds (for pod detection)
    pub latency_seconds: Option<f64>,
}

/// Types of engagement actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngagementKind {
    Like,
    Comment,
    Share,
    Save,
    Follow,
    View,
    Reply,
}

/// Behavioral feature vector for a single follower/account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowerFeatures {
    pub account_id: String,
    /// Posts per day (normalized)
    pub post_frequency: f64,
    /// Average time between follow and first engagement (seconds)
    pub engagement_velocity: f64,
    /// Follower-to-following ratio
    pub ff_ratio: f64,
    /// Account age in days
    pub account_age_days: f64,
    /// Profile completeness score (0-1)
    pub profile_completeness: f64,
    /// Average comment length (characters)
    pub avg_comment_length: f64,
    /// Number of unique content interactions
    pub content_diversity: f64,
    /// Whether the account has a profile picture
    pub has_avatar: bool,
}

impl FollowerFeatures {
    /// Collapse the feature vector into a single liveness score for the
    /// Chebyshev-bounded ManifoldHeap.
    ///
    /// This is the "behavioral fingerprint" — a single scalar that
    /// summarizes how organic an account's behavior is.
    pub fn liveness_score(&self) -> f64 {
        let mut score = 0.0;

        // Post frequency: organic accounts post 0.1-3 times per day
        // Bots either post 0 (lurkers) or 10+ (spam)
        score += if self.post_frequency > 0.05 && self.post_frequency < 5.0 {
            1.0
        } else {
            0.2
        };

        // Engagement velocity: real users take time to discover content
        // Pod members and bots engage within minutes
        score += if self.engagement_velocity > 300.0 {
            1.5 // 5+ minutes = organic
        } else if self.engagement_velocity > 60.0 {
            0.8
        } else {
            0.1 // Under 60 seconds = suspicious
        };

        // FF ratio: real users follow < 2x their follower count
        score += if self.ff_ratio > 0.1 && self.ff_ratio < 3.0 {
            1.0
        } else {
            0.2
        };

        // Account age: older accounts are more trustworthy
        score += if self.account_age_days > 365.0 {
            1.5
        } else if self.account_age_days > 90.0 {
            1.0
        } else if self.account_age_days > 30.0 {
            0.5
        } else {
            0.1
        };

        // Profile completeness
        score += self.profile_completeness;

        // Comment quality: bots leave short generic comments
        score += if self.avg_comment_length > 20.0 {
            1.0
        } else if self.avg_comment_length > 5.0 {
            0.5
        } else {
            0.1
        };

        // Content diversity: organic users interact with varied content
        score += (self.content_diversity / 10.0).min(1.5);

        // Avatar presence
        score += if self.has_avatar { 0.5 } else { 0.0 };

        score
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Scoring & Report Types
// ═══════════════════════════════════════════════════════════════════════════════

/// Complete authenticity audit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub creator: CreatorId,
    pub overall_score: f64,
    pub pod_report: PodReport,
    pub bot_report: BotReport,
    pub content_score: f64,
    pub confidence: f64,
    pub audit_hash: String,
    pub timestamp: i64,
}

/// Pod detection results from Betti approximation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodReport {
    /// Number of engagement pods detected (β₁ spikes)
    pub pods_detected: u32,
    /// Betti-0: connected components in the engagement graph
    pub betti_0: u32,
    /// Betti-1: 1-dimensional loops (pod rings)
    pub betti_1: u32,
    /// Pod score: 0.0 (no pods) to 1.0 (fully artificial)
    pub pod_ratio: f64,
    /// Total actors analyzed
    pub actors_analyzed: usize,
    /// Detected pod ring sizes
    pub ring_sizes: Vec<u32>,
    /// The overcount bound from the verified approximation
    pub overcount_bound: u32,
}

/// Bot detection results from Chebyshev-bounded filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotReport {
    /// Number of accounts flagged as bots
    pub bots_flagged: usize,
    /// Total accounts analyzed
    pub total_accounts: usize,
    /// Bot ratio: flagged / total
    pub bot_ratio: f64,
    /// The Chebyshev k-parameter used
    pub chebyshev_k: f64,
    /// Maximum false-positive rate guarantee: FPR ≤ 1/k²
    pub max_fpr: f64,
    /// Population mean liveness
    pub population_mean: f64,
    /// Population std deviation
    pub population_std: f64,
    /// The boundary: mean - k*std
    pub safety_boundary: f64,
}

/// Price recommendation from Lyapunov-stable PD governor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceRecommendation {
    /// Recommended rate (in platform currency)
    pub recommended_rate: f64,
    /// Current listed rate
    pub current_rate: f64,
    /// Error magnitude |optimal - current|
    pub error_magnitude: f64,
    /// Lyapunov energy: V = e²/2 (should be decreasing)
    pub lyapunov_energy: f64,
    /// Number of PD adaptation steps taken
    pub adaptation_steps: u32,
    /// Has the price converged? (error < threshold)
    pub converged: bool,
    /// Price trajectory history (last N steps)
    pub trajectory: Vec<f64>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Smart Contract Types
// ═══════════════════════════════════════════════════════════════════════════════

/// Campaign lifecycle states (formally verified state machine)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampaignState {
    /// Brand has created the campaign but not funded it
    Created,
    /// Funds deposited into escrow
    Funded,
    /// Oracle verified creator's authenticity; campaign is live
    Active,
    /// All milestones completed
    Completed,
    /// All funds released to creator
    Settled,
    /// Campaign cancelled by mutual agreement
    Cancelled,
    /// Funds returned to brand (timeout or cancel)
    Refunded,
    /// A milestone is under dispute
    Disputed,
    /// Dispute has been resolved by arbitrator
    Arbitrated,
}

/// A campaign milestone definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneDefinition {
    pub id: u8,
    pub description: String,
    /// Percentage of total escrow for this milestone (0-100)
    pub payout_percentage: u8,
    /// Required engagement target (optional)
    pub engagement_target: Option<u64>,
    /// Deadline for this specific milestone (Unix timestamp)
    pub deadline: Option<i64>,
}

/// On-chain milestone state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneState {
    pub definition: MilestoneDefinition,
    pub completed: bool,
    pub completed_at: Option<i64>,
    pub engagement_actual: Option<u64>,
    pub brand_approved: bool,
    pub oracle_verified: bool,
    pub evidence_cid: Option<String>,
    pub funds_released: bool,
}

/// Campaign escrow account (mirrors on-chain state)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignEscrow {
    pub state: CampaignState,
    pub brand_id: String,
    pub creator_id: String,
    pub total_amount: u64,
    pub released_amount: u64,
    pub min_authenticity_threshold: u8,
    pub verified_authenticity_score: Option<u8>,
    pub deadline: i64,
    pub terms_cid: String,
    pub proof_certificate_cid: Option<String>,
    pub milestones: Vec<MilestoneState>,
    pub created_at: i64,
}

/// Oracle attestation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticityAttestation {
    pub creator_id: String,
    pub score: u8,
    pub chebyshev_k: f64,
    pub max_fpr_percent: f64,
    pub betti_1_count: u32,
    pub pods_detected: u32,
    pub audit_report_cid: String,
    pub cab_certificate_cid: Option<String>,
    pub attested_at: i64,
    pub oracle_signature: String,
}

/// CAB certificate chain for proof verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CABCertificate {
    pub runtime_version: String,
    pub theorem_count: u16,
    pub proof_lines: u16,
    pub lean4_proof_cid: String,
    pub safe_rust_cid: String,
    pub wasm_binary_cid: String,
    pub deployed_at: i64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Error Types
// ═══════════════════════════════════════════════════════════════════════════════

/// Errors from the scoring engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScoringError {
    InsufficientData { required: usize, provided: usize },
    InvalidFeatureVector(String),
    GraphCapacityExceeded { max_points: usize },
    ComputationFailed(String),
}

/// Errors from the escrow contract logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EscrowError {
    InvalidStateTransition { from: CampaignState, to: CampaignState },
    InsufficientFunds { required: u64, available: u64 },
    AuthenticityBelowThreshold { score: u8, threshold: u8 },
    MilestoneNotVerified { milestone_id: u8 },
    MilestoneNotApproved { milestone_id: u8 },
    MilestoneAlreadyCompleted { milestone_id: u8 },
    DeadlineExceeded { deadline: i64, current: i64 },
    Unauthorized(String),
    DoubleSpend { milestone_id: u8 },
}

impl core::fmt::Display for ScoringError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InsufficientData { required, provided } => {
                write!(f, "Need {} data points, got {}", required, provided)
            }
            Self::InvalidFeatureVector(msg) => write!(f, "Invalid features: {}", msg),
            Self::GraphCapacityExceeded { max_points } => {
                write!(f, "Graph capacity {} exceeded", max_points)
            }
            Self::ComputationFailed(msg) => write!(f, "Computation failed: {}", msg),
        }
    }
}

impl core::fmt::Display for EscrowError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidStateTransition { from, to } => {
                write!(f, "Cannot transition from {:?} to {:?}", from, to)
            }
            Self::InsufficientFunds { required, available } => {
                write!(f, "Need {} lamports, have {}", required, available)
            }
            Self::AuthenticityBelowThreshold { score, threshold } => {
                write!(f, "Auth score {} below threshold {}", score, threshold)
            }
            Self::MilestoneNotVerified { milestone_id } => {
                write!(f, "Milestone {} not verified by oracle", milestone_id)
            }
            Self::MilestoneNotApproved { milestone_id } => {
                write!(f, "Milestone {} not approved by brand", milestone_id)
            }
            Self::MilestoneAlreadyCompleted { milestone_id } => {
                write!(f, "Milestone {} already completed (INV-02: no double spend)", milestone_id)
            }
            Self::DeadlineExceeded { deadline, current } => {
                write!(f, "Deadline {} passed, current time {}", deadline, current)
            }
            Self::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            Self::DoubleSpend { milestone_id } => {
                write!(f, "INVARIANT VIOLATION INV-02: double spend on milestone {}", milestone_id)
            }
        }
    }
}

impl std::error::Error for ScoringError {}
impl std::error::Error for EscrowError {}
