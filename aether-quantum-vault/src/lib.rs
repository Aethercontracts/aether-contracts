//! ═══════════════════════════════════════════════════════════════════════════════
//! AetherContracts — Quantum Vault
//! ═══════════════════════════════════════════════════════════════════════════════
//!
//! Formally verified scoring engine and smart contract logic for the creator
//! economy. Consumes Lean 4 → Safe Rust verified primitives from `aether-core`.
//!
//! # Architecture
//!
//! ```text
//! Social Data ──► Pod Detector (Betti) ──┐
//!                Bot Filter (Chebyshev) ──┤──► Authenticity Engine ──► Score
//!                NLP (Cauchy-Schwarz) ────┘
//!
//! Score ──► Pricing Engine (Lyapunov) ──► Rate Recommendation
//! Score ──► Oracle Attestation ──► Smart Contract (Escrow)
//! ```
//!
//! # Mathematical Guarantees
//!
//! - **Cauchy-Schwarz**: Zero false-negative attention pruning
//! - **Chebyshev**: FPR ≤ 1/k² (distribution-agnostic)
//! - **Lyapunov**: |error(t+1)| ≤ |error(t)| (monotonic descent)
//! - **Betti**: β₁ overcount bounded by window overlap
//!
//! ═══════════════════════════════════════════════════════════════════════════════

pub mod types;
pub mod pod_detector;
pub mod bot_filter;
pub mod pricing;
pub mod attention;
pub mod authenticity;
pub mod escrow;
pub mod oracle;
pub mod milestone;
