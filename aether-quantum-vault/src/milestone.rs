//! ═══════════════════════════════════════════════════════════════════════════════
//! Milestone Verification Engine
//! ═══════════════════════════════════════════════════════════════════════════════
//!
//! Standalone module for verifying campaign deliverables against
//! smart contract terms. Works with the escrow state machine.
//!
//! ═══════════════════════════════════════════════════════════════════════════════

use crate::types::{CampaignEscrow, CampaignState, EscrowError, MilestoneState};

/// Milestone verification utilities
pub struct MilestoneVerifier;

impl MilestoneVerifier {
    /// Check if a milestone's engagement target has been met
    pub fn check_engagement(milestone: &MilestoneState, actual: u64) -> bool {
        match milestone.definition.engagement_target {
            Some(target) => actual >= target,
            None => true, // No target = auto-pass
        }
    }

    /// Check if a milestone is within its deadline
    pub fn check_deadline(milestone: &MilestoneState, current_time: i64) -> bool {
        match milestone.definition.deadline {
            Some(deadline) => current_time <= deadline,
            None => true, // No per-milestone deadline
        }
    }

    /// Check if a milestone is fully releasable (all conditions met)
    ///
    /// The formal release predicate:
    /// ```text
    /// release_funds(milestone) IFF:
    ///   milestone.oracle_verified == true
    ///   AND milestone.brand_approved == true
    ///   AND campaign.state == ACTIVE
    ///   AND score >= threshold
    ///   AND time <= deadline
    ///   AND milestone.funds_released == false
    /// ```
    pub fn is_releasable(
        escrow: &CampaignEscrow,
        milestone_id: u8,
        current_time: i64,
    ) -> Result<bool, EscrowError> {
        if escrow.state != CampaignState::Active {
            return Ok(false);
        }

        let milestone = escrow
            .milestones
            .iter()
            .find(|m| m.definition.id == milestone_id)
            .ok_or(EscrowError::MilestoneNotVerified { milestone_id })?;

        // Check all six conditions
        let oracle_ok = milestone.oracle_verified;
        let brand_ok = milestone.brand_approved;
        let not_released = !milestone.funds_released;
        let deadline_ok = current_time <= escrow.deadline;
        let auth_ok = escrow
            .verified_authenticity_score
            .map(|s| s >= escrow.min_authenticity_threshold)
            .unwrap_or(false);
        let milestone_deadline_ok = Self::check_deadline(milestone, current_time);

        Ok(oracle_ok && brand_ok && not_released && deadline_ok && auth_ok && milestone_deadline_ok)
    }

    /// Get a summary of all milestones and their completion status
    pub fn summary(escrow: &CampaignEscrow) -> Vec<MilestoneSummary> {
        escrow
            .milestones
            .iter()
            .map(|m| {
                let payout = (escrow.total_amount as f64
                    * m.definition.payout_percentage as f64
                    / 100.0) as u64;

                MilestoneSummary {
                    id: m.definition.id,
                    description: m.definition.description.clone(),
                    payout_amount: payout,
                    oracle_verified: m.oracle_verified,
                    brand_approved: m.brand_approved,
                    completed: m.completed,
                    funds_released: m.funds_released,
                }
            })
            .collect()
    }
}

/// Human-readable milestone summary
#[derive(Debug, Clone)]
pub struct MilestoneSummary {
    pub id: u8,
    pub description: String,
    pub payout_amount: u64,
    pub oracle_verified: bool,
    pub brand_approved: bool,
    pub completed: bool,
    pub funds_released: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::escrow::EscrowEngine;
    use crate::types::MilestoneDefinition;

    #[test]
    fn test_releasable_check() {
        let milestones = vec![
            MilestoneDefinition {
                id: 1,
                description: "Post".to_string(),
                payout_percentage: 100,
                engagement_target: Some(1000),
                deadline: None,
            },
        ];

        let mut escrow = EscrowEngine::create_campaign(
            "brand".into(), "creator".into(), 10_000, 70,
            i64::MAX, "QmCID".into(), milestones,
        ).unwrap();

        EscrowEngine::fund_campaign(&mut escrow).unwrap();
        EscrowEngine::activate_campaign(&mut escrow, 80).unwrap();

        // Not releasable yet — not verified or approved
        assert!(!MilestoneVerifier::is_releasable(&escrow, 1, 1000).unwrap());

        // Verify
        EscrowEngine::verify_milestone(&mut escrow, 1, Some(1500)).unwrap();
        assert!(!MilestoneVerifier::is_releasable(&escrow, 1, 1000).unwrap());

        // Approve
        EscrowEngine::approve_milestone(&mut escrow, 1, "brand").unwrap();
        assert!(MilestoneVerifier::is_releasable(&escrow, 1, 1000).unwrap());
    }
}
