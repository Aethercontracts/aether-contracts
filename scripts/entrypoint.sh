#!/bin/bash
set -e

echo "================================================================"
echo "  AetherContracts MVP Runtime"
echo "  Qwen2.5-0.5B-Instruct | Deterministic Inference | seed=42"
echo "================================================================"

CONFIG="${AETHER_CONFIG:-/app/config.yaml}"
SEED="${AETHER_SEED:-42}"

# ── Check model ──────────────────────────────────────────────────────────────
MODEL_FILE="/models/qwen2.5-0.5b-instruct-q5_k_m.gguf"
if [ ! -f "$MODEL_FILE" ]; then
    echo "[!] Model not found at $MODEL_FILE"
    echo "[→] Downloading Qwen2.5-0.5B-Instruct..."
    bash /app/scripts/download_model.sh /models
fi

echo "[✓] Model: $MODEL_FILE ($(du -h "$MODEL_FILE" | cut -f1))"

# ── Check GPU ────────────────────────────────────────────────────────────────
GPU_LAYERS=0
if nvidia-smi &>/dev/null; then
    echo "[✓] NVIDIA GPU detected"
    nvidia-smi --query-gpu=name,memory.total --format=csv,noheader
    GPU_LAYERS=99
else
    echo "[i] CPU-only mode (no GPU detected)"
fi

# ── Start LLM Server (deterministic) ────────────────────────────────────────
echo ""
echo "[→] Starting llama.cpp server (deterministic mode)..."
echo "    seed=$SEED | temp=0.0 | top_k=1 | greedy decoding"

llama-server \
    -m "$MODEL_FILE" \
    -c 2048 \
    -t $(nproc) \
    --port 8088 \
    --host 0.0.0.0 \
    -ngl $GPU_LAYERS \
    --seed $SEED \
    --log-disable &

LLM_PID=$!
echo "[✓] LLM server started (PID $LLM_PID)"

# Wait for LLM to be ready
echo "[→] Waiting for LLM health..."
for i in $(seq 1 60); do
    if curl -s http://localhost:8088/health | grep -q '"status"'; then
        echo "[✓] LLM server ready on port 8088"
        break
    fi
    if [ $i -eq 60 ]; then
        echo "[!] LLM server failed to start in 60s"
    fi
    sleep 1
done

# ── Start Rust Scoring Engine ────────────────────────────────────────────────
echo ""
echo "[→] Starting Rust scoring engine..."

# Look for the compiled binary
RUST_BIN=""
for candidate in \
    /app/bin/aether-quantum-vault \
    /app/bin/aether_quantum_vault \
    /usr/local/bin/aether-quantum-vault; do
    if [ -x "$candidate" ]; then
        RUST_BIN="$candidate"
        break
    fi
done

if [ -n "$RUST_BIN" ]; then
    echo "[✓] Found Rust binary: $RUST_BIN"
    RUST_LOG=info "$RUST_BIN" &
    RUST_PID=$!
    echo "[✓] Rust engine started (PID $RUST_PID)"

    # Wait for Rust API
    for i in $(seq 1 30); do
        if curl -s http://localhost:8080/health | grep -q '"status"'; then
            echo "[✓] Rust API ready on port 8080"
            break
        fi
        sleep 1
    done
else
    echo "[!] Rust binary not found — scoring engine unavailable"
    echo "    (Bridge API will use simulated fallback scores)"
fi

# ── Start Bridge API ────────────────────────────────────────────────────────
echo ""
echo "[→] Starting Bridge API (FastAPI)..."
cd /app
python3 scripts/aether_api.py &
BRIDGE_PID=$!
echo "[✓] Bridge API started on port 8090 (PID $BRIDGE_PID)"

# Wait for Bridge API
for i in $(seq 1 15); do
    if curl -s http://localhost:8090/health | grep -q '"status"'; then
        echo "[✓] Bridge API ready on port 8090"
        break
    fi
    sleep 1
done

# ── Summary ──────────────────────────────────────────────────────────────────
echo ""
echo "================================================================"
echo "  All services running:"
echo "  - LLM Inference:    http://localhost:8088  (Qwen 0.5B, seed=$SEED)"
echo "  - Rust Engine:      http://localhost:8080  (Axum scoring)"
echo "  - Bridge API:       http://localhost:8090  (FastAPI simulation)"
echo "  - Dashboard:        http://localhost:3000  (Next.js)"
echo ""
echo "  Quick test:"
echo "    curl http://localhost:8090/status"
echo "    curl -X POST http://localhost:8090/simulate"
echo "    curl http://localhost:8090/determinism-proof"
echo "================================================================"

# Keep container alive
exec "$@"
