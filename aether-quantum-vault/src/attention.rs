//! ═══════════════════════════════════════════════════════════════════════════════
//! Cauchy-Schwarz NLP Inference Acceleration
//! ═══════════════════════════════════════════════════════════════════════════════
//!
//! Repurposes `aether_core::aether::{BlockMetadata, HierarchicalBlockTree}`
//! for accelerating NLP inference on creator content (comments, captions,
//! engagement text) with zero-false-negative guarantees.
//!
//! # Guarantee
//!
//! Cauchy-Schwarz: score(q, block) ≤ ‖q‖ · (‖μ‖ + r)
//!
//! If the upper bound is below threshold, the ENTIRE block can be skipped.
//! No relevant token is ever dropped — zero false negatives.
//!
//! ═══════════════════════════════════════════════════════════════════════════════

use aether_core::aether::{BlockMetadata, HierarchicalBlockTree};

/// Dimension for NLP embedding vectors (simplified for demonstration)
const NLP_DIM: usize = 4;

/// Block size for token grouping
const BLOCK_SIZE: usize = 64;

/// NLP inference accelerator using verified Cauchy-Schwarz pruning
pub struct AttentionAccelerator {
    /// Hierarchical block tree from the verified core
    tree: HierarchicalBlockTree<NLP_DIM>,
    /// Block metadata cache
    blocks: Vec<BlockMetadata<NLP_DIM>>,
    /// Input embeddings (token representations)
    embeddings: Vec<[f64; NLP_DIM]>,
}

impl AttentionAccelerator {
    pub fn new() -> Self {
        Self {
            tree: HierarchicalBlockTree::new(),
            blocks: Vec::new(),
            embeddings: Vec::new(),
        }
    }

    /// Load token embeddings from NLP model output.
    ///
    /// Groups tokens into blocks of 64 and computes BlockMetadata for each.
    pub fn load_embeddings(&mut self, embeddings: Vec<[f64; NLP_DIM]>) {
        self.embeddings = embeddings;
        self.blocks.clear();

        // Group into blocks of BLOCK_SIZE and compute metadata
        for chunk in self.embeddings.chunks(BLOCK_SIZE) {
            let refs: Vec<&[f64; NLP_DIM]> = chunk.iter().collect();
            let block = BlockMetadata::from_points(
                &refs.iter().map(|p| **p).collect::<Vec<_>>(),
            );
            self.blocks.push(block);
        }

        // Build hierarchical tree for multi-level pruning
        self.tree.build_from_blocks(&self.blocks);
    }

    /// Perform hierarchical attention query with Cauchy-Schwarz pruning.
    ///
    /// Returns the indices of blocks that CANNOT be pruned (must be computed).
    /// All other blocks are safely skipped with zero false negatives.
    pub fn query(&self, query: &[f64; NLP_DIM], threshold: f64) -> [bool; 128] {
        self.tree.hierarchical_query(query, threshold)
    }

    /// Compute the pruning ratio — fraction of blocks safely skipped.
    pub fn pruning_ratio(&self, query: &[f64; NLP_DIM], threshold: f64) -> f64 {
        let active_mask = self.query(query, threshold);
        self.tree.pruning_ratio(&active_mask)
    }

    /// Get the number of blocks
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }
}

impl Default for AttentionAccelerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_and_query() {
        let mut acc = AttentionAccelerator::new();

        // Create 128 synthetic embeddings (2 blocks of 64)
        let embeddings: Vec<[f64; NLP_DIM]> = (0..128)
            .map(|i| {
                let v = i as f64 / 128.0;
                [v, v * 0.5, 1.0 - v, 0.5]
            })
            .collect();

        acc.load_embeddings(embeddings);
        assert_eq!(acc.block_count(), 2);

        // Query with some vector
        let query = [0.5, 0.25, 0.5, 0.5];
        let active = acc.query(&query, 0.1);
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn test_pruning_ratio_bounded() {
        let mut acc = AttentionAccelerator::new();

        let embeddings: Vec<[f64; NLP_DIM]> = (0..256)
            .map(|i| {
                let v = i as f64 / 256.0;
                [v, v, v, v]
            })
            .collect();

        acc.load_embeddings(embeddings);

        let query = [0.5, 0.5, 0.5, 0.5];
        let ratio = acc.pruning_ratio(&query, 0.01);

        // Pruning ratio should be between 0 and 1
        assert!(ratio >= 0.0 && ratio <= 1.0);
    }
}
