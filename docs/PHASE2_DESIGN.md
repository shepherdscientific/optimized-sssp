# Phase 2 Design (Pivot Discovery Loop)

Objective: Introduce a repeated truncated expansion (basecase) to identify a distance pivot boundary B' such that at least one root of the truncated predecessor forest has subtree size ≥ k (size threshold). This provides a bounded-growth step toward hierarchical recursion while preserving equality-relax (<=) invariant.

## Key Entities

- k: target size scale (adaptive: may grow geometrically on failure).
- Basecase(k): truncated Dijkstra-like expansion that returns prefix U with |U| ≤ k and a boundary B'.
- Root: node u in U whose predecessor p is not in U (pred[u] = -1 or dist[p] ≥ B').
- Subtree size sz(u): number of descendants (including u) within U following predecessor links (forest defined by pred restricted to U).

## Success Criterion
Accept pivot when max_root_subtree_size ≥ k. Otherwise increase k ← min(2k, n) and rerun (bounded by pivot_attempt_max).

## Termination Conditions
1. Success criterion satisfied.
2. k reached n (full exploration) -> success by exhaustion.
3. Attempts == pivot_attempt_max -> return best attempt (max subtree) even if < k (flag via stats).

## Data Captured per Attempt
- relaxations
- collected (|U|)
- new_bound B'
- pop_order (distance-nondecreasing visitation order)
- dist[], pred[] (for the attempt)
- root list with subtree sizes

## Invariants
1. Equality relax (<=) only introduces tighter or equal distances, preserving determinism for ties.
2. Distances of nodes not in U either INF or ≥ B'.
3. Forest roots precisely those with pred[v] = -1 or dist[pred[v]] ≥ B'.

## Phase 2 Public Artifacts
Added symbols (FFI):
```
sssp_run_spec_phase2
sssp_get_spec_phase2_stats
```
Environment variables:
```
SSSP_SPEC_K           # initial k (default 1024)
SSSP_SPEC_PIVOT_MAX   # maximum pivot attempts (default 8)
```

Stats struct (internal now, exposed via getter):
```
struct SpecPhase2Stats {
  attempts: u32,
  success: i32,          # 1 success, 0 fallback
  final_k: u32,
  collected: u32,        # |U| of final attempt
  max_subtree: u32,
  roots_examined: u32,
  relaxations: u64,
}
```

## Simplifications (Intentional for Iteration)
- Single-source expansion each attempt (source vertex unchanged).
- No reuse of previous attempt's partial forest (future optimization).
- No multi-level recursion yet (Phase 4).

## Future Extensions (Phases 3-4)
- Reuse of tight forest between attempts (avoid recomputation).
- DataStructureD integration for batching frontier merges.
- Boundary chain and multi-level recursion; passing contracted graph summary downward.
- Instrumentation JSON dump for per-level stats.

---
Document date: 2025-09-17