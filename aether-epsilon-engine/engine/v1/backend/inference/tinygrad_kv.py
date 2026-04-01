import numpy as np
from tinygrad.tensor import Tensor
import math
from typing import Union, Tuple, Dict, Any

class INT8KVCache:
    def __init__(self, n_layers: int = 32, n_heads: int = 32, max_tokens: int = 512, d_head: int = 64) -> None:
        assert n_layers > 0, "n_layers must be strictly positive"
        assert n_heads > 0, "n_heads must be strictly positive"
        assert max_tokens > 0, "max_tokens must be strictly positive"
        assert d_head > 0, "d_head must be strictly positive"
        
        self.n_layers: int = n_layers
        self.n_heads: int  = n_heads
        self.max_tokens: int = max_tokens
        self.d_head: int   = d_head
        self.pos: int      = 0
        self.n_tokens: int = 0
        
        self.k: np.ndarray = np.zeros((n_layers, n_heads, max_tokens, d_head), dtype=np.int8)
        self.v = np.zeros((n_layers, n_heads, max_tokens, d_head), dtype=np.int8)
        total_mb = (self.k.nbytes + self.v.nbytes) / (1024 ** 2)
        print(f"[INT8KVCache] Allocated {total_mb:.1f} MB "
              f"({n_layers}L x {n_heads}H x {max_tokens}T x {d_head}D x INT8 x 2)")

    def write(self, layer: int, keys: Union[Tensor, np.ndarray], values: Union[Tensor, np.ndarray]) -> None:
        assert 0 <= layer < self.n_layers, f"Layer {layer} out of bounds [0, {self.n_layers})"
        
        if isinstance(keys, Tensor):
            keys = keys.numpy()
        if isinstance(values, Tensor):
            values = values.numpy()
            
        # Ensure shape matches exactly (n_heads, d_head) for a single token
        assert keys.shape == (self.n_heads, self.d_head), f"Keys shape {keys.shape} != {(self.n_heads, self.d_head)}"
        assert values.shape == (self.n_heads, self.d_head), f"Values shape {values.shape} != {(self.n_heads, self.d_head)}"
        
        # Guard against NaN/Inf values causing erratic behavior
        assert not np.isnan(keys).any() and not np.isinf(keys).any(), "Keys contain NaN or Inf"
        assert not np.isnan(values).any() and not np.isinf(values).any(), "Values contain NaN or Inf"
        
        self.k[layer, :, self.pos] = np.clip(keys,   -127, 127).astype(np.int8)
        self.v[layer, :, self.pos] = np.clip(values, -127, 127).astype(np.int8)

    def advance(self) -> None:
        self.n_tokens = min(self.n_tokens + 1, self.max_tokens)
        self.pos = (self.pos + 1) % self.max_tokens

    def read(self, layer: int) -> Tuple[np.ndarray, np.ndarray]:
        assert 0 <= layer < self.n_layers, f"Layer {layer} out of bounds"
        valid = self.n_tokens
        keys   = self.k[layer, :, :valid].astype(np.float32)
        values = self.v[layer, :, :valid].astype(np.float32)
        return keys, values

    def read_as_tensors(self, layer: int) -> Tuple[Tensor, Tensor]:
        keys, values = self.read(layer)
        return Tensor(keys), Tensor(values)

    def reset(self) -> None:
        self.pos      = 0
        self.n_tokens = 0

    def memory_used_mb(self) -> float:
        bytes_per_token = self.n_layers * self.n_heads * self.d_head * 2
        return (self.n_tokens * bytes_per_token) / (1024 ** 2)

    def utilisation(self) -> float:
        return self.n_tokens / self.max_tokens


class SparseAttentionKVCache(INT8KVCache):
    def __init__(self, top_k: int = 64, **kwargs: Any) -> None:
        super().__init__(**kwargs)
        assert top_k > 0, "top_k must be strictly positive"
        assert top_k <= self.max_tokens, "top_k cannot exceed max_tokens"
        self.top_k = top_k
        print(f"[SparseAttention] top_k={self.top_k} — attends to "
              f"at most {self.top_k} of {self.max_tokens} cached tokens")

    def sparse_read(self, layer: int, query: Union[Tensor, np.ndarray]) -> Tuple[np.ndarray, np.ndarray]:
        assert 0 <= layer < self.n_layers, f"Layer {layer} out of bounds"
        if isinstance(query, Tensor):
            query = query.numpy()
            
        assert query.shape == (self.n_heads, self.d_head), f"Query shape {query.shape} != {(self.n_heads, self.d_head)}"
        assert not np.isnan(query).any() and not np.isinf(query).any(), "Query contains NaN or Inf"
        
        all_keys, all_values = self.read(layer)
        valid = self.n_tokens
        if valid <= self.top_k:
            return all_keys, all_values
        scores = np.sum(query[:, np.newaxis, :] * all_keys, axis=-1)
        scores = scores / math.sqrt(self.d_head)
        
        # Numerical stability guard for mean scores across heads
        avg_scores = scores.mean(axis=0)
        assert avg_scores.shape == (valid,), "Averaged scores shape mismatch"
        
        top_k_idx = np.argpartition(avg_scores, -self.top_k)[-self.top_k:]
        top_k_idx = top_k_idx[np.argsort(avg_scores[top_k_idx])[::-1]]
        return all_keys[:, top_k_idx, :], all_values[:, top_k_idx, :]

    def get_stats(self) -> Dict[str, Any]:
        return {
            "tokens_cached":     self.n_tokens,
            "max_tokens":        self.max_tokens,
            "utilisation_pct":   round(self.utilisation() * 100, 1),
            "memory_used_mb":    round(self.memory_used_mb(), 2),
            "sparse_top_k":      self.top_k,
            "attention_savings": f"{self.max_tokens // self.top_k}x",
        }
