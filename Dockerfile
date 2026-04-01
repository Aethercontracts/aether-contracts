# ═══════════════════════════════════════════════════════════════════════════════
# AetherContracts — Unified Docker Build (CPU-first, GPU optional)
# ═══════════════════════════════════════════════════════════════════════════════
#
# Multi-stage build:
#   Stage 1: Rust builder (scoring engine)
#   Stage 2: LLM builder (llama.cpp CPU)
#   Stage 3: Runtime (Rust + LLM + Python bridge + baked Qwen model)
#
# Model: Qwen2.5-0.5B-Instruct (Q5_K_M, ~490MB) — baked into image
# Inference: Deterministic (seed=42, temp=0.0, top_k=1)
#
# Usage:
#   docker compose up --build          # Full MVP stack
#   docker compose run simulator       # Prove determinism
#
# ═══════════════════════════════════════════════════════════════════════════════

# ─────────────────────────────────────────────────────────────────────────────
# STAGE 1: Rust Builder — Scoring Engine
# ─────────────────────────────────────────────────────────────────────────────
FROM rust:1.78-bookworm AS rust-builder

WORKDIR /build

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev cmake clang lld \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests (dependency caching)
COPY Cargo.toml ./
COPY aether-core/crates/aether-core/Cargo.toml ./aether-core/crates/aether-core/
COPY aether-core/crates/aether-lang/Cargo.toml ./aether-core/crates/aether-lang/
COPY aether-core/crates/aegis-core/Cargo.toml ./aether-core/crates/aegis-core/
COPY aether-quantum-vault/Cargo.toml ./aether-quantum-vault/

# Stub sources for dependency caching
RUN mkdir -p aether-core/crates/aether-core/src && echo "pub fn stub(){}" > aether-core/crates/aether-core/src/lib.rs && \
    mkdir -p aether-core/crates/aether-lang/src && echo "pub fn stub(){}" > aether-core/crates/aether-lang/src/lib.rs && \
    mkdir -p aether-core/crates/aegis-core/src && echo "pub fn stub(){}" > aether-core/crates/aegis-core/src/lib.rs && \
    mkdir -p aether-quantum-vault/src && echo "pub fn stub(){}" > aether-quantum-vault/src/lib.rs && \
    echo 'fn main(){}' > aether-quantum-vault/src/main.rs

# Cache dependency fetch
RUN cargo fetch --locked 2>/dev/null || cargo fetch

# Copy real source
COPY aether-core/ ./aether-core/
COPY aether-quantum-vault/ ./aether-quantum-vault/

# Build release binaries
RUN cargo build --workspace --release 2>&1 || cargo build --workspace 2>&1

# ─────────────────────────────────────────────────────────────────────────────
# STAGE 2: LLM Builder — llama.cpp (CPU-only)
# ─────────────────────────────────────────────────────────────────────────────
FROM ubuntu:22.04 AS llm-builder

ENV DEBIAN_FRONTEND=noninteractive

WORKDIR /build

RUN apt-get update && apt-get install -y \
    git cmake build-essential clang lld \
    && rm -rf /var/lib/apt/lists/*

# Build llama.cpp for CPU inference
RUN git clone --depth 1 --recursive https://github.com/ggerganov/llama.cpp.git /opt/llama.cpp && \
    cd /opt/llama.cpp && \
    cmake -B build \
        -DGGML_CUDA=OFF \
        -DGGML_METAL=OFF \
        -DCMAKE_BUILD_TYPE=Release && \
    cmake --build build --config Release -j$(nproc) && \
    cp build/bin/llama-server /usr/local/bin/llama-server && \
    cp build/bin/llama-cli /usr/local/bin/llama-cli

# ─────────────────────────────────────────────────────────────────────────────
# STAGE 3: Runtime — Everything Combined + Baked Model
# ─────────────────────────────────────────────────────────────────────────────
FROM ubuntu:22.04 AS runtime

ENV DEBIAN_FRONTEND=noninteractive
ENV PYTHONUNBUFFERED=1
ENV AETHER_ENV=production
ENV AETHER_SEED=42
ENV AETHER_CONFIG=/app/config.yaml

WORKDIR /app

# Runtime dependencies
RUN apt-get update && apt-get install -y \
    python3 python3-pip \
    curl ca-certificates tini \
    && rm -rf /var/lib/apt/lists/*

# Python packages for bridge API
COPY requirements.txt /app/requirements.txt
RUN python3 -m pip install --no-cache-dir -r /app/requirements.txt

# Copy Rust binaries
COPY --from=rust-builder /build/target/release/aether-quantum-vault /usr/local/bin/aether-quantum-vault


# Copy llama.cpp binaries
COPY --from=llm-builder /usr/local/bin/llama-server /usr/local/bin/llama-server
COPY --from=llm-builder /usr/local/bin/llama-cli /usr/local/bin/llama-cli

# Download Qwen2.5-0.5B-Instruct GGUF (baked into image)
RUN mkdir -p /models && \
    echo "[→] Downloading Qwen2.5-0.5B-Instruct (Q5_K_M)..." && \
    huggingface-cli download \
        Qwen/Qwen2.5-0.5B-Instruct-GGUF \
        qwen2.5-0.5b-instruct-q5_k_m.gguf \
        --local-dir /models \
        --local-dir-use-symlinks False && \
    echo "[✓] Model downloaded" && \
    ls -lh /models/

# Copy application files
COPY config.yaml /app/config.yaml
COPY scripts/ /app/scripts/
COPY aether-quantum-vault/ /app/aether-quantum-vault/
COPY aether-core/ /app/aether-core/

# Create data directories
RUN mkdir -p /data /ipfs

# Make scripts executable
RUN chmod +x /app/scripts/*.sh 2>/dev/null || true

# Copy entrypoint
COPY scripts/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

# Ports
EXPOSE 8080 8088 8090

# Health check — bridge API is the unified health endpoint
HEALTHCHECK --interval=30s --timeout=10s --retries=5 \
    CMD curl -f http://localhost:8090/health || exit 1

ENTRYPOINT ["tini", "--"]
CMD ["/app/entrypoint.sh", "tail", "-f", "/dev/null"]
