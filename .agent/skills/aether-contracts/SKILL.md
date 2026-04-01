---
name: aether-contracts
description: Formally Verified Aether Contracts development & workflows.
---

# Aether Contracts Platform Guidelines

This skill defines the architectural mandates and development principles for AetherContracts: a formally verified creator economy platform using AI-scored authenticity and trustless blockchain escrow.

## Core Architectural Layers

1.  **AETHER Core (`aether-core`)**
    *   All math must be grounded in topological principles. 
    *   Contains Betti number verification, Chebyshev boundaries, Cauchy-Schwarz, and Lyapunov stabilizers. 
    *   No heuristics — we strictly use formal bounds (e.g., FPR $\le 1/k^2$).

2.  **Quantum Vault (`aether-quantum-vault`)**
    *   State machine for the campaign escrow.
    *   Strict enforcement of INV-01 through INV-06:
        *   INV-01: NO PREMATURE RELEASE (Requires Oracle + Brand signature).
        *   INV-02: NO DOUBLE SPEND.
        *   INV-03: TOTAL CONSERVATION (deposited = released + remaining).
        *   INV-04: AUTHENTICITY ENFORCEMENT (score $\ge$ threshold).
        *   INV-05: TIMEOUT REFUND GUARANTEE.
        *   INV-06: UNAUTHORIZED STATE TRANSITION ZERO-TOLERANCE.
    *   Written in Safe Rust. No `unsafe` blocks.

3.  **Epsilon IDE Engine Integration (`aether-epsilon-engine`)**
    *   The Python-based multi-tier AI inference runtime.
    *   Tiers: Fast (Shallow embedding), Balanced (Attention parsing), Deep (LLM generation).
    *   Bridged into Rust via the `AttentionAccelerator` (Cauchy-Schwarz pruning).
    *   LLM Inference is offloaded locally to `llama.cpp` using local hardware via CUDA 12.4 in Docker.

## CAB (Certified Artifact Builder)
*   Every audit report must be fingerprinted into a cryptographic Hash.
*   IPFS + On-chain attestation via the `OracleEngine`.

## Execution Patterns
*   Always structure the Rust codebase linearly in workspaces to enable multi-crate development.
*   The Monorepo is housed at `Aethercontracts/aether-contracts`.
*   All testing involves rigorous formal property-based checks on the `EscrowEngine`.
*   To start integration: check `docker-compose.yml` to spin up the Rust+Python backend, `llama.cpp` endpoint, and Next.js frontend Dashboard.
