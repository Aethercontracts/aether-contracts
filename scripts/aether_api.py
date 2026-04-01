"""
scripts/aether_api.py
======================
FastAPI Bridge for AetherContracts MVP.
Runs on :8090
"""

import asyncio
import hashlib
import json
import os
import random
import time
from datetime import datetime, timezone
from typing import Optional

import httpx
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
import uvicorn
import yaml

CONFIG_PATH = os.environ.get("AETHER_CONFIG", "/app/config.yaml")
RUST_API    = os.environ.get("RUST_API_URL", "http://127.0.0.1:8080")
LLM_API     = os.environ.get("LLM_API_URL",  "http://127.0.0.1:8088")
SEED        = int(os.environ.get("AETHER_SEED", "42"))

IM_START = "<|im_start|>"
IM_END   = "<|im_end|>"
EOT_TOK  = "<|endoftext|>"

def load_config():
    try:
        with open(CONFIG_PATH) as f:
            return yaml.safe_load(f) or {}
    except Exception:
        return {}

config = load_config()
det = config.get("determinism", {})
DET_SEED    = det.get("seed", SEED)
DET_TEMP    = det.get("temperature", 0.0)
DET_TOP_K   = det.get("top_k", 1)
DET_TOP_P   = det.get("top_p", 1.0)
DET_REPEAT  = det.get("repeat_penalty", 1.0)

app = FastAPI(
    title="AetherContracts Simulation Bridge",
    description="Deterministic creator economy simulation powered by Qwen2.5-0.5B",
    version="1.0.0",
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

simulation_hashes = []


class CompletionRequest(BaseModel):
    prompt: str
    max_tokens: int = 512
    system_prompt: Optional[str] = None


class SimulationRequest(BaseModel):
    creator_handle: str = "@test_creator"
    platform: str = "Instagram"
    num_followers: int = 100
    num_events: int = 50
    content_score: float = 0.85


def generate_synthetic_audit_data(req, seed=42):
    """Generate seeded synthetic data. Same seed = identical data."""
    rng = random.Random(seed)
    events = []
    for i in range(req.num_events):
        events.append({
            "event_type": rng.choice(["like", "comment", "share", "save"]),
            "timestamp": 1700000000 + i * 3600,
            "account_id": f"user_{i:04d}",
            "is_organic": rng.random() > 0.1,
        })
    followers = []
    for i in range(req.num_followers):
        followers.append({
            "account_age_days": rng.randint(30, 3650),
            "post_count": rng.randint(0, 500),
            "follower_count": rng.randint(0, 10000),
            "following_count": rng.randint(10, 5000),
            "has_profile_pic": rng.random() > 0.05,
            "has_bio": rng.random() > 0.1,
            "engagement_rate": round(rng.uniform(0.001, 0.15), 4),
        })
    return {
        "creator": {
            "platform": req.platform,
            "handle": req.creator_handle,
            "internal_id": f"sim_{hashlib.md5(req.creator_handle.encode()).hexdigest()[:12]}",
        },
        "events": events,
        "followers": followers,
        "content_score": req.content_score,
    }


@app.get("/health")
async def health_check():
    rust_ok = False
    llm_ok = False
    async with httpx.AsyncClient(timeout=5) as client:
        try:
            r = await client.get(f"{RUST_API}/health")
            rust_ok = r.status_code == 200
        except Exception:
            pass
        try:
            r = await client.get(f"{LLM_API}/health")
            llm_ok = r.status_code == 200
        except Exception:
            pass
    return {
        "status": "ok" if (rust_ok and llm_ok) else "degraded",
        "services": {
            "rust_scoring_engine": "healthy" if rust_ok else "unavailable",
            "llm_inference": "healthy" if llm_ok else "unavailable",
            "bridge_api": "healthy",
        },
        "model": "Qwen2.5-0.5B-Instruct (Q5_K_M)",
        "determinism": {"seed": DET_SEED, "temperature": DET_TEMP, "top_k": DET_TOP_K},
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }


@app.post("/llm/complete")
async def llm_complete(req: CompletionRequest):
    system = req.system_prompt or (
        "You are the AetherContracts AI analyst. "
        "Analyze creator authenticity data and provide clear, structured assessments."
    )
    prompt = (
        f"{IM_START}system\n{system}{IM_END}\n"
        f"{IM_START}user\n{req.prompt}{IM_END}\n"
        f"{IM_START}assistant\n"
    )
    payload = {
        "prompt": prompt,
        "n_predict": req.max_tokens,
        "temperature": DET_TEMP,
        "top_k": DET_TOP_K,
        "top_p": DET_TOP_P,
        "repeat_penalty": DET_REPEAT,
        "seed": DET_SEED,
        "stop": [IM_END, EOT_TOK],
        "stream": False,
    }
    async with httpx.AsyncClient(timeout=120) as client:
        try:
            r = await client.post(f"{LLM_API}/completion", json=payload)
            r.raise_for_status()
            data = r.json()
            content = data.get("content", "").strip()
            content_hash = hashlib.sha256(content.encode()).hexdigest()
            return {
                "content": content,
                "tokens_predicted": data.get("tokens_predicted", 0),
                "sha256": content_hash,
                "deterministic_params": {
                    "seed": DET_SEED, "temperature": DET_TEMP,
                    "top_k": DET_TOP_K, "repeat_penalty": DET_REPEAT,
                },
            }
        except httpx.TimeoutException:
            raise HTTPException(status_code=504, detail="LLM inference timed out")
        except Exception as e:
            raise HTTPException(status_code=502, detail=f"LLM error: {e}")


@app.post("/simulate")
async def run_simulation(req: SimulationRequest = SimulationRequest()):
    run_id = len(simulation_hashes) + 1
    ts = datetime.now(timezone.utc).isoformat()

    # 1. Generate seeded synthetic data
    audit_data = generate_synthetic_audit_data(req, seed=DET_SEED)

    # 2. Send to Rust scoring engine
    audit_result = None
    async with httpx.AsyncClient(timeout=30) as client:
        try:
            r = await client.post(f"{RUST_API}/audit", json=audit_data)
            r.raise_for_status()
            audit_result = r.json()
        except Exception as e:
            audit_result = {
                "error": str(e), "fallback": True,
                "simulated_score": 87.5, "bot_ratio": 0.05, "pods_detected": 0,
            }

    # 3. Ask LLM to analyze the audit
    score_val = audit_result.get("simulated_score",
                audit_result.get("Ok", {}).get("overall_score", "N/A"))
    analysis_prompt = (
        f"Analyze this creator audit result:\n"
        f"- Authenticity Score: {score_val}\n"
        f"- Bot Ratio: {audit_result.get('bot_ratio', 'N/A')}\n"
        f"- Pods Detected: {audit_result.get('pods_detected', 'N/A')}\n"
        f"- Platform: {req.platform}\n"
        f"- Creator: {req.creator_handle}\n\n"
        f"Provide a brief risk assessment and recommendation."
    )

    llm_analysis = "LLM unavailable"
    llm_hash = ""
    try:
        llm_resp = await llm_complete(CompletionRequest(prompt=analysis_prompt, max_tokens=256))
        llm_analysis = llm_resp["content"]
        llm_hash = llm_resp["sha256"]
    except Exception as e:
        llm_analysis = f"LLM error: {e}"
        llm_hash = hashlib.sha256(llm_analysis.encode()).hexdigest()

    # 4. Hash the combined result for determinism proof
    combined = json.dumps({"audit": audit_result, "llm": llm_analysis}, sort_keys=True)
    combined_hash = hashlib.sha256(combined.encode()).hexdigest()

    entry = {
        "run_id": run_id, "timestamp": ts, "sha256": combined_hash,
        "audit_hash": hashlib.sha256(json.dumps(audit_result, sort_keys=True).encode()).hexdigest(),
        "llm_hash": llm_hash,
    }
    simulation_hashes.append(entry)

    all_match = len(set(h["sha256"] for h in simulation_hashes)) <= 1
    return {
        "run_id": run_id, "timestamp": ts,
        "audit_result": audit_result, "llm_analysis": llm_analysis,
        "sha256_hash": combined_hash, "deterministic": all_match,
        "total_runs": len(simulation_hashes),
    }


@app.get("/determinism-proof")
async def determinism_proof():
    if not simulation_hashes:
        return {"proven": False, "message": "No simulations run yet. POST /simulate first.", "runs": []}
    hashes = [h["sha256"] for h in simulation_hashes]
    unique = set(hashes)
    return {
        "proven": len(unique) == 1,
        "total_runs": len(simulation_hashes),
        "unique_hashes": len(unique),
        "hash_value": hashes[0] if len(unique) == 1 else None,
        "message": (
            f"DETERMINISTIC: {len(simulation_hashes)} runs all produced identical hash {hashes[0][:16]}..."
            if len(unique) == 1
            else f"NON-DETERMINISTIC: {len(unique)} unique hashes across {len(simulation_hashes)} runs"
        ),
        "runs": simulation_hashes,
    }


@app.get("/status")
async def system_status():
    health = await health_check()
    return {
        "system": "AetherContracts MVP",
        "version": "1.0.0",
        "model": "Qwen2.5-0.5B-Instruct (Q5_K_M)",
        "architecture": {
            "scoring_engine": "Rust (Axum) - Betti topology, Chebyshev bounds, Cauchy-Schwarz NLP",
            "llm_inference": "llama.cpp - Qwen2.5-0.5B deterministic",
            "bridge_api": "FastAPI - simulation orchestrator",
            "dashboard": "Next.js 15",
            "proof_anchoring": "IPFS (Kubo)",
        },
        "determinism": {
            "seed": DET_SEED, "temperature": DET_TEMP,
            "top_k": DET_TOP_K, "mode": "greedy (argmax)",
        },
        "health": health,
        "simulation_runs": len(simulation_hashes),
    }


if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8090, log_level="info")
