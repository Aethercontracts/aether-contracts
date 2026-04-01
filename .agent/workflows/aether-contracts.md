---
description: "AetherContracts Setup & Integration Workflow"
---

# AetherContracts Setup Workflow

This workflow automates the setup, build, and deployment processes for the AetherContracts platform, bridging the gap between the multi-tier AI inference engine and the formally verified Quantum Vault.

## 1. Local Monorepo Build Check

Verify the Rust workspace and crates compile successfully before proceeding to deployment or integration tests.

```bash
// turbo-all
cargo check --workspace
```

## 2. Docker Compose Verification

Start the platform in Docker. This pulls down the LLM inference (llama.cpp) if a model exists, the Python Epsilon Engine, and the Rust Vault binaries.

```bash
docker compose build
docker compose up -d
```

## 3. Deployment & Push to Organization

Login via GitHub and push the monorepo to the unified Aethercontracts organization.

```bash
# Requires interactive auth if not already logged in
gh auth login
gh repo create Aethercontracts/aether-contracts --public --source=. --remote=origin --push
```

## 4. Run Verification Matrix

Run the Lean 4 + Rust formal verification test suite for invariants INV-01 through INV-06 on the Escrow state machine.

```bash
// turbo-all
cargo test -p aether-quantum-vault -- --nocapture
```
