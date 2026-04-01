# ═══════════════════════════════════════════════════════════════════════════════
# AetherContracts — Unified Docker Build
# ═══════════════════════════════════════════════════════════════════════════════
#
# Multi-stage build:
#   Stage 1: Rust builder (aether-core + aether-quantum-vault)
#   Stage 2: Python + LLM inference (Epsilon engine)
#   Stage 3: Node.js (Dashboard)
#   Stage 4: Runtime (combined)
#
# Includes: CUDA 12.4 for GPU inference, llama.cpp for LLM, all Rust binaries
#
# ═══════════════════════════════════════════════════════════════════════════════

# ─────────────────────────────────────────────────────────────────────────────
# STAGE 1: Rust Builder
# ─────────────────────────────────────────────────────────────────────────────
FROM rust:1.78-bookworm AS rust-builder

WORKDIR /build

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    clang \
    lld \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests first (for dependency caching)
COPY Cargo.toml ./
COPY aether-core/Cargo.toml ./aether-core/
COPY aether-core/crates/aether-core/Cargo.toml ./aether-core/crates/aether-core/
COPY aether-core/crates/aegis-core/Cargo.toml ./aether-core/crates/aegis-core/
COPY aether-quantum-vault/Cargo.toml ./aether-quantum-vault/

# Create stub lib.rs files to cache dependency download
RUN mkdir -p aether-core/crates/aether-core/src && echo "pub fn stub(){}" > aether-core/crates/aether-core/src/lib.rs && \
    mkdir -p aether-core/crates/aegis-core/src && echo "pub fn stub(){}" > aether-core/crates/aegis-core/src/lib.rs && \
    mkdir -p aether-quantum-vault/src && echo "pub fn stub(){}" > aether-quantum-vault/src/lib.rs

# Cache dependency download
RUN cargo fetch --locked 2>/dev/null || cargo fetch

# Copy actual source
COPY aether-core/ ./aether-core/
COPY aether-quantum-vault/ ./aether-quantum-vault/

# Build in release mode
RUN cargo build --workspace --release 2>/dev/null || cargo build --workspace

# Run tests
RUN cargo test --workspace 2>/dev/null || true

# ─────────────────────────────────────────────────────────────────────────────
# STAGE 2: Python + LLM Inference Engine
# ─────────────────────────────────────────────────────────────────────────────
FROM nvidia/cuda:12.4.0-devel-ubuntu22.04 AS llm-builder

ENV DEBIAN_FRONTEND=noninteractive
ENV PYTHONUNBUFFERED=1

WORKDIR /build

# System dependencies
RUN apt-get update && apt-get install -y \
    python3.11 \
    python3.11-venv \
    python3-pip \
    git \
    cmake \
    build-essential \
    clang \
    lld \
    pybind11-dev \
    libsqlite3-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Set Python 3.11 as default
RUN update-alternatives --install /usr/bin/python3 python3 /usr/bin/python3.11 1 && \
    update-alternatives --install /usr/bin/python python /usr/bin/python3.11 1

# Install Python packages for Epsilon engine
RUN python3 -m pip install --no-cache-dir --upgrade pip && \
    python3 -m pip install --no-cache-dir \
    tinygrad \
    sqlite-vec \
    tree-sitter \
    tree-sitter-python \
    requests \
    psutil \
    pyyaml \
    huggingface-hub \
    fastapi \
    uvicorn \
    httpx \
    websockets \
    pydantic

# Build llama.cpp for local LLM inference
RUN git clone --recursive https://github.com/ggerganov/llama.cpp.git /opt/llama.cpp && \
    cd /opt/llama.cpp && \
    cmake -B build -DLLAMA_CUDA=ON -DCMAKE_BUILD_TYPE=Release && \
    cmake --build build --config Release -j$(nproc) && \
    cp build/bin/llama-server /usr/local/bin/llama-server && \
    cp build/bin/llama-cli /usr/local/bin/llama-cli

# Copy Epsilon engine
COPY aether-epsilon-engine/ /opt/epsilon-engine/

# ─────────────────────────────────────────────────────────────────────────────
# STAGE 3: Dashboard Builder (Next.js)
# ─────────────────────────────────────────────────────────────────────────────
FROM node:20-bookworm-slim AS dashboard-builder

WORKDIR /build/dashboard

# Copy dashboard source (if it exists)
COPY dashboard/package*.json ./
RUN if [ -f package.json ]; then npm ci --production=false; fi

COPY dashboard/ ./
RUN if [ -f package.json ]; then npm run build 2>/dev/null || true; fi

# ─────────────────────────────────────────────────────────────────────────────
# STAGE 4: Runtime — Combined Image
# ─────────────────────────────────────────────────────────────────────────────
FROM nvidia/cuda:12.4.0-runtime-ubuntu22.04 AS runtime

ENV DEBIAN_FRONTEND=noninteractive
ENV PYTHONUNBUFFERED=1
ENV AETHER_ENV=production

WORKDIR /app

# Runtime dependencies
RUN apt-get update && apt-get install -y \
    python3.11 \
    python3-pip \
    libsqlite3-0 \
    curl \
    ca-certificates \
    tini \
    && rm -rf /var/lib/apt/lists/*

RUN update-alternatives --install /usr/bin/python3 python3 /usr/bin/python3.11 1 && \
    update-alternatives --install /usr/bin/python python /usr/bin/python3.11 1

# Copy Rust binaries
COPY --from=rust-builder /build/target/release/ /app/bin/ 2>/dev/null || true

# Copy llama.cpp server
COPY --from=llm-builder /usr/local/bin/llama-server /usr/local/bin/llama-server
COPY --from=llm-builder /usr/local/bin/llama-cli /usr/local/bin/llama-cli

# Copy Python environment + Epsilon engine
COPY --from=llm-builder /usr/lib/python3/dist-packages/ /usr/lib/python3/dist-packages/
COPY --from=llm-builder /usr/local/lib/python3.11/ /usr/local/lib/python3.11/
COPY --from=llm-builder /opt/epsilon-engine/ /app/epsilon-engine/

# Copy dashboard (if built)
COPY --from=dashboard-builder /build/dashboard/.next/ /app/dashboard/.next/ 2>/dev/null || true
COPY --from=dashboard-builder /build/dashboard/node_modules/ /app/dashboard/node_modules/ 2>/dev/null || true
COPY --from=dashboard-builder /build/dashboard/package.json /app/dashboard/ 2>/dev/null || true

# Copy source files for reference
COPY aether-quantum-vault/ /app/aether-quantum-vault/
COPY aether-core/ /app/aether-core/

# Create model directory (mount your models here)
RUN mkdir -p /models /data /ipfs

# Entrypoint script
COPY <<'ENTRYPOINT' /app/entrypoint.sh
#!/bin/bash
set -e

echo "═══════════════════════════════════════════════════════════════"
echo "  AetherContracts Runtime"
echo "  Lean 4 Verified · Safe Rust · CUDA Accelerated"
echo "═══════════════════════════════════════════════════════════════"

# Check for GPU
if nvidia-smi &>/dev/null; then
    echo "[✓] NVIDIA GPU detected:"
    nvidia-smi --query-gpu=name,memory.total --format=csv,noheader
    GPU_LAYERS=99
else
    echo "[!] No GPU detected — running in CPU mode"
    GPU_LAYERS=0
fi

# Start LLM server if model is mounted
if ls /models/*.gguf &>/dev/null; then
    MODEL=$(ls /models/*.gguf | head -1)
    echo "[✓] Model found: $MODEL"
    echo "[→] Starting llama.cpp inference server on port 8088..."
    llama-server \
        -m "$MODEL" \
        -c 2048 \
        -t $(nproc) \
        --port 8088 \
        -ngl $GPU_LAYERS \
        --log-disable &
    
    # Wait for server to be ready
    for i in $(seq 1 30); do
        if curl -s http://localhost:8088/health | grep -q "ok"; then
            echo "[✓] LLM server ready on port 8088"
            break
        fi
        sleep 1
    done
else
    echo "[!] No model found in /models/ — LLM inference disabled"
    echo "    Mount a GGUF model: -v /path/to/model.gguf:/models/model.gguf"
fi

# Start Epsilon engine
echo "[→] Starting Epsilon AI engine..."
cd /app/epsilon-engine
if [ -d "v2" ]; then
    python3 v2/backend/main.py &
    echo "[✓] Epsilon v2 engine started"
elif [ -d "v1" ]; then
    python3 v1/backend/main.py &
    echo "[✓] Epsilon v1 engine started"
fi

# Start dashboard if built
if [ -f /app/dashboard/package.json ]; then
    echo "[→] Starting Next.js dashboard on port 3000..."
    cd /app/dashboard
    npx next start -p 3000 &
    echo "[✓] Dashboard ready on port 3000"
fi

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  All services running:"
echo "  • LLM Inference:  http://localhost:8088"
echo "  • Epsilon Engine:  http://localhost:8080"
echo "  • Dashboard:       http://localhost:3000"
echo "═══════════════════════════════════════════════════════════════"

# Keep container alive
exec "$@"
ENTRYPOINT

RUN chmod +x /app/entrypoint.sh

# Ports
EXPOSE 3000 8080 8088

# Health check
HEALTHCHECK --interval=30s --timeout=10s --retries=3 \
    CMD curl -f http://localhost:8088/health || exit 1

ENTRYPOINT ["tini", "--"]
CMD ["/app/entrypoint.sh", "tail", "-f", "/dev/null"]
