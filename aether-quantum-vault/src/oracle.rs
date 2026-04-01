//! ═══════════════════════════════════════════════════════════════════════════════
//! Authenticity Oracle — On-Chain Score Bridge
//! ═══════════════════════════════════════════════════════════════════════════════
//!
//! Bridges the off-chain AI scoring engine to on-chain smart contracts.
//! Generates cryptographic attestations that the escrow can verify.
//!
//! ═══════════════════════════════════════════════════════════════════════════════

use sha2::{Sha256, Digest};
use crate::types::{AuditReport, AuthenticityAttestation};

/// Generates oracle attestations from audit reports
pub struct OracleEngine {
    /// Oracle identifier (in production: multisig 2-of-3)
    oracle_id: String,
}

impl OracleEngine {
    pub fn new(oracle_id: String) -> Self {
        Self { oracle_id }
    }

    /// Generate an attestation from an audit report.
    ///
    /// This creates a signed record that the escrow smart contract
    /// can verify to confirm the authenticity score is legitimate.
    pub fn attest(&self, report: &AuditReport) -> Result<AuthenticityAttestation, String> {
        // Hash the full audit report for the signature
        let report_json = serde_json::to_string(report)
            .map_err(|e| format!("Serialization error during attestation: {}", e))?;
        let mut hasher = Sha256::new();
        hasher.update(report_json.as_bytes());
        hasher.update(self.oracle_id.as_bytes());
        let signature = format!("{:x}", hasher.finalize());

        AuthenticityAttestation {
            creator_id: report.creator.internal_id.clone(),
            score: report.overall_score.round() as u8,
            chebyshev_k: report.bot_report.chebyshev_k,
            max_fpr_percent: report.bot_report.max_fpr * 100.0,
            betti_1_count: report.pod_report.betti_1,
            pods_detected: report.pod_report.pods_detected,
            audit_report_cid: report.audit_hash.clone(),
            cab_certificate_cid: None,
            attested_at: chrono::Utc::now().timestamp(),
            oracle_signature: signature,
        }
    }

    /// Verify an attestation's signature integrity
    pub fn verify_attestation(&self, attestation: &AuthenticityAttestation) -> bool {
        // In production: verify multisig cryptographic signatures
        // For now: verify the oracle_signature is non-empty and properly formatted
        !attestation.oracle_signature.is_empty() && attestation.oracle_signature.len() == 64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_attestation_generation() {
        let oracle = OracleEngine::new("oracle_001".to_string());

        let report = AuditReport {
            creator: CreatorId {
                platform: Platform::Instagram,
                handle: "@test_creator".to_string(),
                internal_id: "creator_123".to_string(),
            },
            overall_score: 85.5,
            pod_report: PodReport {
                pods_detected: 0,
                betti_0: 1,
                betti_1: 0,
                pod_ratio: 0.0,
                actors_analyzed: 100,
                ring_sizes: vec![],
                overcount_bound: 1,
            },
            bot_report: BotReport {
                bots_flagged: 5,
                total_accounts: 100,
                bot_ratio: 0.05,
                chebyshev_k: 3.0,
                max_fpr: 0.111,
                population_mean: 5.0,
                population_std: 1.5,
                safety_boundary: 0.5,
            },
            content_score: 0.9,
            confidence: 0.85,
            audit_hash: "QmAuditCID123".to_string(),
            timestamp: 1000000,
        };

        let attestation = oracle.attest(&report).unwrap();

        assert_eq!(attestation.score, 86); // 85.5 rounded
        assert_eq!(attestation.creator_id, "creator_123");
        assert!(oracle.verify_attestation(&attestation));
    }
}
