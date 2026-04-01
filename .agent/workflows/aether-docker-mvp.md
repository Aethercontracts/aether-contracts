---
description: Running the AetherContracts Deterministic MVP Docker Stack
---

# AetherContracts — Deterministic Docker MVP Workflow

This workflow automatically provisions and verifies the AetherContracts Docker environment, ensuring that the Qwen2.5-0.5B model and Rust ML engine operate in mathematically verifiable lockstep.

### 1. Execute the Neural Link Launcher
The primary method of interacting with the AetherContracts workspace is through the expansive Python launcher. It provides real-time health polling and orchestrates the entire Docker build step.
// turbo
```powershell
.\start.bat
```

### 2. Manual Fallback: Build and Run the Platform
If the launcher is unavailable, you can manually build the Docker images and start the detached containers. This provisions the Rust Scoring Engine, Next.js Dashboard, IPFS Node, and the Qwen 0.5B llama.cpp instance.
// turbo
```powershell
docker compose up --build -d
```

### 3. Manual Fallback: Run the Determinism Proof
Once the platform is completely healthy (API bridge, LLM, and Rust Engine), you can manually run the simulation harness. This scripts 5 identically-seeded creator audits against the system and verifies that every output SHA-256 hash perfectly matches.
// turbo
```powershell
docker compose run --rm simulator
```

### 4. Stop the Platform
When you are finished testing, bring down the architecture cleanly to free up system resources.
// turbo
```powershell
docker compose down
```
