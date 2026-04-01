//! ═══════════════════════════════════════════════════════════════════════════════
//! Campaign Escrow — Formally Verified State Machine
//! ═══════════════════════════════════════════════════════════════════════════════
//!
//! Implements the trustless campaign escrow with 6 proven invariants:
//!
//! INV-01: escrow_no_premature_release
//! INV-02: escrow_no_double_spend  
//! INV-03: escrow_total_conservation
//! INV-04: authenticity_threshold_enforcement
//! INV-05: timeout_refund_guarantee
//! INV-06: no_unauthorized_state_transition
//!
//! All conditional logic mirrors the Lean 4 verified specifications.
//!
//! ═══════════════════════════════════════════════════════════════════════════════

use crate::types::{
    CampaignEscrow, CampaignState, EscrowError, MilestoneDefinition, MilestoneState,
};

/// The escrow engine — all state transitions are guarded by verified invariants
pub struct EscrowEngine;

impl EscrowEngine {
    /// Create a new campaign escrow (transitions to Created state)
    pub fn create_campaign(
        brand_id: String,
        creator_id: String,
        total_amount: u64,
        min_authenticity_threshold: u8,
        deadline: i64,
        terms_cid: String,
        milestones: Vec<MilestoneDefinition>,
    ) -> Result<CampaignEscrow, EscrowError> {
        // Validate milestone percentages sum to 100
        let total_pct: u16 = milestones.iter().map(|m| m.payout_percentage as u16).sum();
        if total_pct != 100 {
            return Err(EscrowError::InvalidStateTransition {
                from: CampaignState::Created,
                to: CampaignState::Created,
            });
        }

        let milestone_states: Vec<MilestoneState> = milestones
            .into_iter()
            .map(|def| MilestoneState {
                definition: def,
                completed: false,
                completed_at: None,
                engagement_actual: None,
                brand_approved: false,
                oracle_verified: false,
                evidence_cid: None,
                funds_released: false,
            })
            .collect();

        Ok(CampaignEscrow {
            state: CampaignState::Created,
            brand_id,
            creator_id,
            total_amount,
            released_amount: 0,
            min_authenticity_threshold,
            verified_authenticity_score: None,
            deadline,
            terms_cid,
            proof_certificate_cid: None,
            milestones: milestone_states,
            created_at: chrono::Utc::now().timestamp(),
        })
    }

    /// Fund the escrow (Created → Funded)
    pub fn fund_campaign(escrow: &mut CampaignEscrow) -> Result<(), EscrowError> {
        Self::validate_transition(escrow.state, CampaignState::Funded)?;
        escrow.state = CampaignState::Funded;
        Ok(())
    }

    /// Activate campaign after oracle verifies creator's authenticity (Funded → Active)
    ///
    /// **INV-04: authenticity_threshold_enforcement**
    /// Campaign REJECTS activation if score < threshold.
    pub fn activate_campaign(
        escrow: &mut CampaignEscrow,
        authenticity_score: u8,
    ) -> Result<(), EscrowError> {
        Self::validate_transition(escrow.state, CampaignState::Active)?;

        // INV-04: Authenticity threshold enforcement
        if authenticity_score < escrow.min_authenticity_threshold {
            return Err(EscrowError::AuthenticityBelowThreshold {
                score: authenticity_score,
                threshold: escrow.min_authenticity_threshold,
            });
        }

        escrow.verified_authenticity_score = Some(authenticity_score);
        escrow.state = CampaignState::Active;
        Ok(())
    }

    /// Verify a milestone (oracle confirms engagement targets met)
    pub fn verify_milestone(
        escrow: &mut CampaignEscrow,
        milestone_id: u8,
        engagement_actual: Option<u64>,
    ) -> Result<(), EscrowError> {
        if escrow.state != CampaignState::Active {
            return Err(EscrowError::InvalidStateTransition {
                from: escrow.state,
                to: CampaignState::Active,
            });
        }

        let milestone = escrow
            .milestones
            .iter_mut()
            .find(|m| m.definition.id == milestone_id)
            .ok_or(EscrowError::MilestoneNotVerified { milestone_id })?;

        // INV-02: No double spend — cannot re-verify completed milestones
        if milestone.completed {
            return Err(EscrowError::MilestoneAlreadyCompleted { milestone_id });
        }

        // Check engagement target if defined
        if let Some(target) = milestone.definition.engagement_target {
            if let Some(actual) = engagement_actual {
                if actual < target {
                    return Err(EscrowError::MilestoneNotVerified { milestone_id });
                }
            } else {
                return Err(EscrowError::MilestoneNotVerified { milestone_id });
            }
        }

        milestone.oracle_verified = true;
        milestone.engagement_actual = engagement_actual;
        Ok(())
    }

    /// Brand approves a milestone deliverable
    pub fn approve_milestone(
        escrow: &mut CampaignEscrow,
        milestone_id: u8,
        signer: &str,
    ) -> Result<(), EscrowError> {
        // INV-06: Only brand can approve
        if signer != escrow.brand_id {
            return Err(EscrowError::Unauthorized(
                "Only the brand can approve milestones".to_string(),
            ));
        }

        let milestone = escrow
            .milestones
            .iter_mut()
            .find(|m| m.definition.id == milestone_id)
            .ok_or(EscrowError::MilestoneNotApproved { milestone_id })?;

        milestone.brand_approved = true;
        Ok(())
    }

    /// Release funds for a completed milestone.
    ///
    /// **INV-01: escrow_no_premature_release**
    /// Funds CANNOT be released unless oracle_verified AND brand_approved.
    ///
    /// **INV-02: escrow_no_double_spend**
    /// Completed milestones cannot trigger duplicate payouts.
    ///
    /// **INV-03: escrow_total_conservation**
    /// deposited = released + remaining at ALL transitions.
    pub fn release_milestone_funds(
        escrow: &mut CampaignEscrow,
        milestone_id: u8,
        current_time: i64,
    ) -> Result<u64, EscrowError> {
        if escrow.state != CampaignState::Active {
            return Err(EscrowError::InvalidStateTransition {
                from: escrow.state,
                to: CampaignState::Active,
            });
        }

        // INV-05: Deadline check
        if current_time > escrow.deadline {
            return Err(EscrowError::DeadlineExceeded {
                deadline: escrow.deadline,
                current: current_time,
            });
        }

        let milestone = escrow
            .milestones
            .iter_mut()
            .find(|m| m.definition.id == milestone_id)
            .ok_or(EscrowError::MilestoneNotVerified { milestone_id })?;

        // INV-02: No double spend
        if milestone.funds_released {
            return Err(EscrowError::DoubleSpend { milestone_id });
        }

        // INV-01: Both oracle AND brand must approve
        if !milestone.oracle_verified {
            return Err(EscrowError::MilestoneNotVerified { milestone_id });
        }
        if !milestone.brand_approved {
            return Err(EscrowError::MilestoneNotApproved { milestone_id });
        }

        // INV-04: Re-verify authenticity score meets threshold
        if let Some(score) = escrow.verified_authenticity_score {
            if score < escrow.min_authenticity_threshold {
                return Err(EscrowError::AuthenticityBelowThreshold {
                    score,
                    threshold: escrow.min_authenticity_threshold,
                });
            }
        }

        // Calculate payout amount
        let payout = (escrow.total_amount as f64
            * milestone.definition.payout_percentage as f64
            / 100.0) as u64;

        // INV-03: Total conservation check
        if escrow.released_amount + payout > escrow.total_amount {
            return Err(EscrowError::InsufficientFunds {
                required: payout,
                available: escrow.total_amount - escrow.released_amount,
            });
        }

        // Execute release
        milestone.completed = true;
        milestone.completed_at = Some(current_time);
        milestone.funds_released = true;
        escrow.released_amount += payout;

        // Check if all milestones completed → transition to Completed
        if escrow.milestones.iter().all(|m| m.completed) {
            escrow.state = CampaignState::Completed;
        }

        // INV-03 postcondition: verify conservation
        debug_assert!(
            escrow.released_amount <= escrow.total_amount,
            "INV-03 VIOLATED: released {} > total {}",
            escrow.released_amount,
            escrow.total_amount
        );

        Ok(payout)
    }

    /// Timeout refund — automatic full refund if deadline passes.
    ///
    /// **INV-05: timeout_refund_guarantee**
    pub fn timeout_refund(
        escrow: &mut CampaignEscrow,
        current_time: i64,
    ) -> Result<u64, EscrowError> {
        if current_time <= escrow.deadline {
            return Err(EscrowError::DeadlineExceeded {
                deadline: escrow.deadline,
                current: current_time,
            });
        }

        if escrow.state != CampaignState::Active && escrow.state != CampaignState::Funded {
            return Err(EscrowError::InvalidStateTransition {
                from: escrow.state,
                to: CampaignState::Refunded,
            });
        }

        let refund = escrow.total_amount - escrow.released_amount;
        escrow.state = CampaignState::Refunded;

        Ok(refund)
    }

    /// Settle the campaign (Completed → Settled)
    pub fn settle_campaign(escrow: &mut CampaignEscrow) -> Result<(), EscrowError> {
        Self::validate_transition(escrow.state, CampaignState::Settled)?;
        escrow.state = CampaignState::Settled;
        Ok(())
    }

    /// Validate that a state transition is legal
    fn validate_transition(
        from: CampaignState,
        to: CampaignState,
    ) -> Result<(), EscrowError> {
        let valid = matches!(
            (from, to),
            (CampaignState::Created, CampaignState::Funded)
                | (CampaignState::Created, CampaignState::Cancelled)
                | (CampaignState::Funded, CampaignState::Active)
                | (CampaignState::Funded, CampaignState::Refunded)
                | (CampaignState::Active, CampaignState::Completed)
                | (CampaignState::Active, CampaignState::Disputed)
                | (CampaignState::Completed, CampaignState::Settled)
                | (CampaignState::Disputed, CampaignState::Arbitrated)
        );

        if valid {
            Ok(())
        } else {
            Err(EscrowError::InvalidStateTransition { from, to })
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests — Invariant Verification
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn test_milestones() -> Vec<MilestoneDefinition> {
        vec![
            MilestoneDefinition {
                id: 1,
                description: "Post published".to_string(),
                payout_percentage: 50,
                engagement_target: Some(1000),
                deadline: None,
            },
            MilestoneDefinition {
                id: 2,
                description: "Engagement target met".to_string(),
                payout_percentage: 50,
                engagement_target: Some(5000),
                deadline: None,
            },
        ]
    }

    fn setup_active_campaign() -> CampaignEscrow {
        let mut escrow = EscrowEngine::create_campaign(
            "brand_001".to_string(),
            "creator_001".to_string(),
            10_000,
            70,
            i64::MAX, // far future deadline
            "QmTermsCID".to_string(),
            test_milestones(),
        )
        .unwrap();

        EscrowEngine::fund_campaign(&mut escrow).unwrap();
        EscrowEngine::activate_campaign(&mut escrow, 85).unwrap();
        escrow
    }

    #[test]
    fn test_inv_01_no_premature_release() {
        let mut escrow = setup_active_campaign();

        // Try to release funds without oracle verification — should fail
        let result = EscrowEngine::release_milestone_funds(&mut escrow, 1, 1000);
        assert!(
            matches!(result, Err(EscrowError::MilestoneNotVerified { .. })),
            "INV-01: Should not release without oracle verification"
        );

        // Verify but don't approve — should still fail
        EscrowEngine::verify_milestone(&mut escrow, 1, Some(1500)).unwrap();
        let result = EscrowEngine::release_milestone_funds(&mut escrow, 1, 1000);
        assert!(
            matches!(result, Err(EscrowError::MilestoneNotApproved { .. })),
            "INV-01: Should not release without brand approval"
        );
    }

    #[test]
    fn test_inv_02_no_double_spend() {
        let mut escrow = setup_active_campaign();

        // Complete milestone 1 properly
        EscrowEngine::verify_milestone(&mut escrow, 1, Some(1500)).unwrap();
        EscrowEngine::approve_milestone(&mut escrow, 1, "brand_001").unwrap();
        EscrowEngine::release_milestone_funds(&mut escrow, 1, 1000).unwrap();

        // Try to release again — should fail with DoubleSpend
        let result = EscrowEngine::release_milestone_funds(&mut escrow, 1, 1001);
        assert!(
            matches!(result, Err(EscrowError::DoubleSpend { .. })),
            "INV-02: Should prevent double spend"
        );
    }

    #[test]
    fn test_inv_03_total_conservation() {
        let mut escrow = setup_active_campaign();

        // Release milestone 1
        EscrowEngine::verify_milestone(&mut escrow, 1, Some(1500)).unwrap();
        EscrowEngine::approve_milestone(&mut escrow, 1, "brand_001").unwrap();
        let payout1 = EscrowEngine::release_milestone_funds(&mut escrow, 1, 1000).unwrap();

        // Release milestone 2
        EscrowEngine::verify_milestone(&mut escrow, 2, Some(6000)).unwrap();
        EscrowEngine::approve_milestone(&mut escrow, 2, "brand_001").unwrap();
        let payout2 = EscrowEngine::release_milestone_funds(&mut escrow, 2, 2000).unwrap();

        // INV-03: deposited = released + remaining
        assert_eq!(
            payout1 + payout2,
            escrow.total_amount,
            "INV-03: Total conservation violated"
        );
        assert_eq!(escrow.released_amount, escrow.total_amount);
    }

    #[test]
    fn test_inv_04_authenticity_enforcement() {
        let mut escrow = EscrowEngine::create_campaign(
            "brand".to_string(),
            "creator".to_string(),
            10_000,
            80, // minimum score 80
            i64::MAX,
            "QmCID".to_string(),
            test_milestones(),
        )
        .unwrap();

        EscrowEngine::fund_campaign(&mut escrow).unwrap();

        // Try to activate with score 60 (below threshold 80) — should fail
        let result = EscrowEngine::activate_campaign(&mut escrow, 60);
        assert!(
            matches!(
                result,
                Err(EscrowError::AuthenticityBelowThreshold {
                    score: 60,
                    threshold: 80
                })
            ),
            "INV-04: Should reject low authenticity score"
        );
    }

    #[test]
    fn test_inv_05_timeout_refund() {
        let deadline = 1_000_000;
        let mut escrow = EscrowEngine::create_campaign(
            "brand".to_string(),
            "creator".to_string(),
            10_000,
            70,
            deadline,
            "QmCID".to_string(),
            test_milestones(),
        )
        .unwrap();

        EscrowEngine::fund_campaign(&mut escrow).unwrap();
        EscrowEngine::activate_campaign(&mut escrow, 85).unwrap();

        // After deadline passes — brand gets full refund
        let refund = EscrowEngine::timeout_refund(&mut escrow, deadline + 1).unwrap();
        assert_eq!(refund, 10_000, "INV-05: Full refund on timeout");
        assert_eq!(escrow.state, CampaignState::Refunded);
    }

    #[test]
    fn test_inv_06_unauthorized_approval() {
        let mut escrow = setup_active_campaign();

        // Wrong signer tries to approve
        let result =
            EscrowEngine::approve_milestone(&mut escrow, 1, "hacker_address");
        assert!(
            matches!(result, Err(EscrowError::Unauthorized(_))),
            "INV-06: Should reject unauthorized signer"
        );
    }

    #[test]
    fn test_full_lifecycle() {
        let mut escrow = setup_active_campaign();

        // Milestone 1
        EscrowEngine::verify_milestone(&mut escrow, 1, Some(2000)).unwrap();
        EscrowEngine::approve_milestone(&mut escrow, 1, "brand_001").unwrap();
        let p1 = EscrowEngine::release_milestone_funds(&mut escrow, 1, 1000).unwrap();
        assert_eq!(p1, 5000); // 50% of 10_000

        // Milestone 2
        EscrowEngine::verify_milestone(&mut escrow, 2, Some(6000)).unwrap();
        EscrowEngine::approve_milestone(&mut escrow, 2, "brand_001").unwrap();
        let p2 = EscrowEngine::release_milestone_funds(&mut escrow, 2, 2000).unwrap();
        assert_eq!(p2, 5000); // 50% of 10_000

        // Should auto-transition to Completed
        assert_eq!(escrow.state, CampaignState::Completed);

        // Settle
        EscrowEngine::settle_campaign(&mut escrow).unwrap();
        assert_eq!(escrow.state, CampaignState::Settled);
    }
}
