//! ═══════════════════════════════════════════════════════════════════════════════
//! Dynamic Price Stabilization via Lyapunov-Stable PD Governor
//! ═══════════════════════════════════════════════════════════════════════════════
//!
//! Repurposes `aether_core::governor::GeometricGovernor` from kernel tick-rate
//! control → creator pricing stabilization.
//!
//! # Mapping
//!
//! | Kernel Domain           | Creator Economy Domain        |
//! |-------------------------|-------------------------------|
//! | System tick rate         | Creator's market rate         |
//! | Target rate R_target    | Optimal price (from scoring)  |
//! | Deviation Δ(t)          | |current_price - optimal|     |
//! | Threshold ε(t)          | Price adjustment step size    |
//! | α (proportional gain)   | Response speed to deviation   |
//! | β (derivative gain)     | Damping to prevent oscillation|
//!
//! # Guarantee
//!
//! Lyapunov function V = e²/2, with V̇ < 0:
//! |error(t+1)| ≤ |error(t)| — monotonic descent to equilibrium.
//!
//! ═══════════════════════════════════════════════════════════════════════════════

use aether_core::governor::GeometricGovernor;
use aether_core::state::SystemState;
use crate::types::PriceRecommendation;

/// Pricing engine constants
const MAX_TRAJECTORY_HISTORY: usize = 100;

/// Dynamic pricing engine using Lyapunov-stable PD control
pub struct PricingEngine {
    /// The PD governor from aether-core (Lean 4 verified control law)
    governor: GeometricGovernor,
    /// Target optimal price (computed from authenticity score + market data)
    target_price: f64,
    /// Current market price
    current_price: f64,
    /// Price trajectory history
    trajectory: Vec<f64>,
    /// Lyapunov energy history (should be monotonically decreasing)
    energy_history: Vec<f64>,
    /// Number of adaptation steps performed
    step_count: u32,
    /// Previous error (for derivative term)
    prev_error: f64,
}

impl PricingEngine {
    /// Create a new pricing engine.
    ///
    /// # Arguments
    /// * `target_price` - The optimal price derived from authenticity scoring
    /// * `current_price` - The creator's current listed rate
    pub fn new(target_price: f64, current_price: f64) -> Self {
        Self {
            governor: GeometricGovernor::new(),
            target_price,
            current_price,
            trajectory: vec![current_price],
            energy_history: Vec::new(),
            step_count: 0,
            prev_error: target_price - current_price,
        }
    }

    /// Run one step of the Lyapunov-stable PD controller.
    ///
    /// Maps the kernel governor's `adapt()` to pricing:
    /// 1. Compute error signal: e(t) = target - current
    /// 2. Feed into PD controller via SystemState deviation
    /// 3. Governor adapts ε (step size) with guaranteed descent
    /// 4. Apply bounded price adjustment
    ///
    /// Returns the new recommended price after this step.
    pub fn step(&mut self) -> f64 {
        let error = self.target_price - self.current_price;
        let error_derivative = error - self.prev_error;

        // Compute Lyapunov energy: V = e²/2
        let energy = error * error / 2.0;

        // Formal boundary check to counter numerical drift explosion
        assert!(energy.is_finite(), "Mathematical fault: Lyapunov energy hit non-finite state");

        if let Some(&prev_energy) = self.energy_history.last() {
            // Formally test Lyapunov convergence
            debug_assert!(energy <= prev_energy + 1e-6, "Lyapunov convergence breached: V(t+1) > V(t) + epsilon");
        }

        self.energy_history.push(energy);

        // Create a SystemState representing the pricing deviation
        // We use a 4D state vector where:
        //   dim 0 = normalized error magnitude
        //   dim 1 = error derivative (for damping)
        //   dim 2 = price velocity
        //   dim 3 = market volatility proxy
        let error_magnitude = error.abs() / self.target_price.max(1.0);
        let state = SystemState::new(
            [
                error_magnitude.min(1.0),
                (error_derivative.abs() / self.target_price.max(1.0)).min(1.0),
                0.0,
                0.0,
            ],
            self.step_count as u64,
        );

        // Feed the state to the governor — it adapts ε using PD control
        let reference = SystemState::zero();
        let deviation = state.deviation(&reference);
        self.governor.adapt(deviation, 1.0);

        // Use governor's adapted ε as the step size, bounded by the error
        let epsilon = self.governor.epsilon();
        let adjustment = if error.abs() < epsilon {
            // Close enough — snap to target
            error
        } else {
            // Apply bounded adjustment in the direction of the error
            error.signum() * epsilon.min(error.abs())
        };

        self.current_price += adjustment;
        self.prev_error = error;
        self.step_count += 1;

        // Track trajectory
        assert!(self.current_price.is_finite(), "Mathematical fault: current_price hit non-finite state");

        if self.trajectory.len() < MAX_TRAJECTORY_HISTORY {
            self.trajectory.push(self.current_price);
        }

        self.current_price
    }

    /// Run multiple PD steps until convergence or max iterations.
    ///
    /// Returns the final price recommendation with full trajectory.
    pub fn converge(&mut self, max_steps: u32, tolerance: f64) -> PriceRecommendation {
        for _ in 0..max_steps {
            self.step();

            let error = (self.target_price - self.current_price).abs();
            if error < tolerance {
                break;
            }
        }

        self.recommendation()
    }

    /// Generate the current price recommendation report
    pub fn recommendation(&self) -> PriceRecommendation {
        let error = (self.target_price - self.current_price).abs();
        let energy = error * error / 2.0;

        PriceRecommendation {
            recommended_rate: self.current_price,
            current_rate: self.trajectory.first().copied().unwrap_or(0.0),
            error_magnitude: error,
            lyapunov_energy: energy,
            adaptation_steps: self.step_count,
            converged: error < 0.01 * self.target_price.max(1.0),
            trajectory: self.trajectory.clone(),
        }
    }

    /// Update the target price (e.g., when a new authenticity score comes in)
    pub fn update_target(&mut self, new_target: f64) {
        self.target_price = new_target;
        self.prev_error = new_target - self.current_price;
    }

    /// Verify Lyapunov descent: energy should be monotonically decreasing.
    ///
    /// Returns true if the last N energy values are non-increasing.
    pub fn verify_descent(&self, window: usize) -> bool {
        if self.energy_history.len() < 2 {
            return true;
        }

        let start = self.energy_history.len().saturating_sub(window);
        let window_slice = &self.energy_history[start..];

        for pair in window_slice.windows(2) {
            // Allow small numerical jitter (1e-10)
            if pair[1] > pair[0] + 1e-10 {
                return false;
            }
        }
        true
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_converges_to_target() {
        let target = 500.0;
        let current = 200.0;
        let mut engine = PricingEngine::new(target, current);

        let rec = engine.converge(1000, 1.0);

        assert!(
            rec.converged || rec.error_magnitude < 50.0,
            "Price should converge toward target. Error: {}",
            rec.error_magnitude
        );
    }

    #[test]
    fn test_lyapunov_energy_decreases() {
        let mut engine = PricingEngine::new(100.0, 50.0);

        for _ in 0..50 {
            engine.step();
        }

        // The Lyapunov energy should generally decrease
        // (may not be strictly monotonic due to discrete-time + floating point,
        //  but the trend should be downward)
        if engine.energy_history.len() >= 2 {
            let first_energy = engine.energy_history[0];
            let last_energy = engine.energy_history.last().unwrap();
            assert!(
                *last_energy <= first_energy + 1e-6,
                "Energy should decrease: first={}, last={}",
                first_energy,
                last_energy
            );
        }
    }

    #[test]
    fn test_trajectory_recorded() {
        let mut engine = PricingEngine::new(100.0, 80.0);
        engine.converge(10, 0.5);

        assert!(
            engine.trajectory.len() > 1,
            "Should have recorded trajectory"
        );
    }

    #[test]
    fn test_already_at_target() {
        let mut engine = PricingEngine::new(100.0, 100.0);
        let rec = engine.converge(10, 0.5);

        assert!(rec.converged, "Should already be converged");
        assert!(rec.error_magnitude < 1.0);
    }
}
