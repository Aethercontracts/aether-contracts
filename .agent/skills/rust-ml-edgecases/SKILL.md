---
name: rust-ml-edgecases
description: Workflow for writing or modifying Rust ML, Mathematics, and Smart Contract logic with an Edge-Case First Methodology.
---

# Rust ML & Formal Logic: Edge-Case First Methodology

You are operating within the `AetherContracts` formally verified ecosystem. 
The mathematical primitives (Cauchy-Schwarz, Lyapunov, Betti, Chebyshev) power a financial creator economy platform. Panics are unacceptable. Probabilistic bugs are exploitable.

**CRITICAL DIRECTIVE**: For *every problem* or *every code modification* involving Rust ML, Topology, or Smart Contracts, you **MUST first try to make as many edge cases as possible.**

## The Edge-Case First Workflow

Before writing any implementation code or proposing a structural change, you must explicitly document the boundaries of your computation:

### 1. Vector & Tensor Edge Cases (ML)
* What happens if the input embedding is an empty vector? `[]`
* What happens if the tensor contains `NaN` or `Infinity`?
* What happens if dimensions mismatch (e.g., `NLP_DIM = 4` but query is length 3)?
* What happens if the matrix is perfectly sparse (all zeros) or perfectly dense (all ones)?

### 2. Numerical Stability Edge Cases
* What happens on divide-by-zero scenarios (e.g., standard deviation when all values are exactly the same)?
* What happens on `f64` underflow or overflow?
* How does floating-point jitter affect your strict inequalities (e.g., `<` vs `<=`)?

### 3. Topological Bounds Edge Cases
* Are Betti numbers naturally bounded? What happens to the Vietoris-Rips complex if all points are at the exact same coordinate?
* Does Chebyshev's bound `$1/k^2$` break if $k \le 1.0$? (Yes, it must be asserted.)

### 4. Smart Contract Escrow Edge Cases
* What if the `current_time` delta is massive (overflow)?
* What if malicious users trigger state transitions concurrently?
* What if the authenticity score is precisely on the boundary (`== threshold`)?

## Implementation Mandate

1. **List the Edge Cases**: Always output a markdown list of everything that could mathematically break.
2. **Defensive Rust**: Replace all `.unwrap()` with `Result` types. If a state is truly mathematically impossible based on prior constraints, use `unreachable!()` or `debug_assert!()` with a formal proof comment explaining why.
3. **Property Bounds**: Code must handle the enumerated edge cases flawlessly.
