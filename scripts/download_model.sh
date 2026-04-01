#!/bin/bash
# ═══════════════════════════════════════════════════════════════════════════════
# Download Qwen2.5-0.5B-Instruct GGUF Model
# ═══════════════════════════════════════════════════════════════════════════════
#
# Downloads the Q5_K_M quantization (~490MB) from HuggingFace.
# Idempotent — skips if file already exists.
#
# ═══════════════════════════════════════════════════════════════════════════════

set -euo pipefail

MODEL_DIR="${1:-/models}"
MODEL_FILE="qwen2.5-0.5b-instruct-q5_k_m.gguf"
MODEL_PATH="${MODEL_DIR}/${MODEL_FILE}"
REPO="Qwen/Qwen2.5-0.5B-Instruct-GGUF"

echo "═══════════════════════════════════════════════════"
echo "  Qwen2.5-0.5B-Instruct GGUF Downloader"
echo "═══════════════════════════════════════════════════"

mkdir -p "${MODEL_DIR}"

if [ -f "${MODEL_PATH}" ]; then
    SIZE=$(du -h "${MODEL_PATH}" | cut -f1)
    echo "[✓] Model already exists: ${MODEL_PATH} (${SIZE})"
    exit 0
fi

echo "[→] Downloading ${MODEL_FILE} from ${REPO}..."
echo "[→] Target: ${MODEL_PATH}"

# Use huggingface-cli if available, otherwise fallback to curl
if command -v huggingface-cli &>/dev/null; then
    huggingface-cli download \
        "${REPO}" \
        "${MODEL_FILE}" \
        --local-dir "${MODEL_DIR}" \
        --local-dir-use-symlinks False
else
    # Direct URL download fallback
    URL="https://huggingface.co/${REPO}/resolve/main/${MODEL_FILE}"
    echo "[→] Using curl fallback: ${URL}"
    curl -L -o "${MODEL_PATH}" "${URL}" --progress-bar
fi

if [ -f "${MODEL_PATH}" ]; then
    SIZE=$(du -h "${MODEL_PATH}" | cut -f1)
    echo "[✓] Download complete: ${MODEL_PATH} (${SIZE})"
else
    echo "[✗] Download failed!"
    exit 1
fi
