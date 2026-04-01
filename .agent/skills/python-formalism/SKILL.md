---
name: python-formalism
description: Workflow for writing or modifying Epsilon IDE Python backend code with an Edge-Case First Methodology and strict mathematical typing.
---

# Epsilon IDE Python Formalism: Edge-Case First Methodology

You are writing and modifying Python orchestrator logic, tensor implementations (e.g., TinyGrad/NumPy), and agent tools for the `AetherContracts` inference engine (Epsilon IDE). 

An LLM agent operates via the tools you provide. If a tool silently absorbs an error or returns ambiguous text, the agent hallucinates.

**CRITICAL DIRECTIVE**: For *every problem* or *every code modification* involving the Python Epsilon Engine or Agent Tools, you **MUST first try to make as many edge cases as possible.**

## The Edge-Case First Workflow

Before writing any implementation code or proposing a structural change, you must explicitly document the boundaries of your computation:

### 1. File System Edge Cases (`filesystem.py` or similar)
* What happens if the target path is a symlink pointing to `/etc/shadow` or an infinite loop?
* What happens if the file exists but lacks read/write permissions?
* What happens on concurrent write contentions (e.g., locking failures)?
* What happens if the file size is 0 bytes vs exactly exactly 500,000 bytes?
* What if `edit_file` text matches multiple lines exactly?

### 2. Inference Tensor Edge Cases (`tinygrad_kv.py` or AI model loaders)
* What happens if `seq_len` precisely matches the max sequence bounds?
* What happens if an INT8 clipping value exceeds 127 or drops below -128?
* What happens if the top-k operation runs on a query where available valid tokens are LESS than `$k$`?
* What happens on tensor `.view()` mismatch when switching precision types (e.g. BF16 to FP32)?

### 3. Asynchronous Orchestration Edge Cases (`aether_link.py`)
* What happens when the async Queue hits capacity? 
* What happens on disconnect/timeout during a WebSocket streaming event?
* What happens if an API response violates the Pydantic JSON Schema entirely?

## Implementation Mandate

1. **List the Edge Cases**: Output a markdown list enumerating every single edge case before you implement the code.
2. **Strict Type Hinting**: Every function must carry `list[str]`, `Optional[int]`, `dict[str, Any]` bounds and be fully standard type-compliant.
3. **Formal Error Types**: Do NOT return magic strings like `"ERROR: something failed"`. If a tool needs to instruct the agent to halt, return a formally structured JSON envelope (e.g. `{"status": "error", "code": 403, "description": "Permission denied"}`) or raise explicit Python Exceptions that the orchestrator routes predictably.
4. **Assert Tensors**: Any numpy or tinygrad function must include `assert keys.shape == (...)` type runtime checks to formally define the tensor contract.
