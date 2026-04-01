"""
scripts/simulate.py
====================
AetherContracts Determinism Simulation Harness.

Runs N identical audit cycles against the running stack and proves
all outputs are byte-identical via SHA-256 hash comparison.

Usage (inside Docker):
    python /app/scripts/simulate.py --runs 5
    python /app/scripts/simulate.py --runs 10 --verbose
"""

import argparse
import hashlib
import json
import sys
import time
import requests


def wait_for_service(url, name, timeout=120):
    """Block until a service becomes healthy."""
    print(f"  Waiting for {name} at {url}...", end="", flush=True)
    start = time.time()
    while time.time() - start < timeout:
        try:
            r = requests.get(url, timeout=3)
            if r.status_code == 200:
                print(f" OK ({time.time()-start:.0f}s)")
                return True
        except Exception:
            pass
        time.sleep(2)
        print(".", end="", flush=True)
    print(f" TIMEOUT after {timeout}s")
    return False


def run_simulation(bridge_url, run_id, verbose=False):
    """Execute one simulation cycle and return the hash."""
    resp = requests.post(
        f"{bridge_url}/simulate",
        json={
            "creator_handle": "@determinism_test",
            "platform": "Instagram",
            "num_followers": 100,
            "num_events": 50,
            "content_score": 0.85,
        },
        timeout=180,
    )
    resp.raise_for_status()
    data = resp.json()

    sha = data.get("sha256_hash", "ERROR")
    if verbose:
        print(f"  Run {run_id}: hash={sha[:24]}... "
              f"score={data.get('audit_result', {}).get('simulated_score', 'N/A')}")
        if data.get("llm_analysis"):
            preview = data["llm_analysis"][:80].replace("\n", " ")
            print(f"         LLM: {preview}...")

    return sha, data


def main():
    parser = argparse.ArgumentParser(description="AetherContracts Determinism Test")
    parser.add_argument("--runs", type=int, default=5, help="Number of simulation runs")
    parser.add_argument("--bridge-url", default="http://127.0.0.1:8090", help="Bridge API URL")
    parser.add_argument("--rust-url", default="http://127.0.0.1:8080", help="Rust API URL")
    parser.add_argument("--llm-url", default="http://127.0.0.1:8088", help="LLM API URL")
    parser.add_argument("--verbose", "-v", action="store_true")
    parser.add_argument("--timeout", type=int, default=120, help="Service startup timeout")
    args = parser.parse_args()

    print("=" * 60)
    print("  AetherContracts — Determinism Simulation")
    print("  Proving mathematical reproducibility")
    print("=" * 60)
    print()

    # 1. Wait for all services
    print("[1/3] Waiting for services...")
    services_ok = True
    for url, name in [
        (f"{args.rust_url}/health", "Rust Scoring Engine"),
        (f"{args.llm_url}/health", "LLM Server (Qwen 0.5B)"),
        (f"{args.bridge_url}/health", "Bridge API"),
    ]:
        if not wait_for_service(url, name, timeout=args.timeout):
            print(f"  WARNING: {name} not available, simulation may use fallbacks")
            services_ok = False

    # 2. Run N identical simulations
    print(f"\n[2/3] Running {args.runs} identical simulations...")
    hashes = []
    results = []
    for i in range(1, args.runs + 1):
        try:
            sha, data = run_simulation(args.bridge_url, i, verbose=args.verbose)
            hashes.append(sha)
            results.append(data)
        except Exception as e:
            print(f"  Run {i}: FAILED — {e}")
            hashes.append(f"ERROR_{i}")

    # 3. Verify determinism
    print(f"\n[3/3] Determinism Verification")
    print("-" * 60)

    unique_hashes = set(hashes)
    if len(unique_hashes) == 1 and "ERROR" not in list(unique_hashes)[0]:
        print(f"  PASS: All {args.runs} runs produced identical output")
        print(f"  SHA-256: {hashes[0]}")
        print(f"  Mathematical reproducibility: PROVEN")
        exit_code = 0
    else:
        print(f"  FAIL: {len(unique_hashes)} unique outputs across {args.runs} runs")
        for i, h in enumerate(hashes, 1):
            print(f"    Run {i}: {h[:32]}...")
        exit_code = 1

    print()
    print("=" * 60)

    # Get determinism proof from API
    try:
        proof = requests.get(f"{args.bridge_url}/determinism-proof", timeout=10).json()
        print(f"  API Proof: {proof.get('message', 'N/A')}")
    except Exception:
        pass

    print("=" * 60)
    sys.exit(exit_code)


if __name__ == "__main__":
    main()
