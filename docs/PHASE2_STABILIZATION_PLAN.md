# Phase 2 Stabilization Plan

Status: DRAFT  
Last Updated: 2025-09-18

## 1. Objective
Elevate current Phase 2 (pivot loop + subtree sizing) from "initial" to "stable" by tightening correctness guarantees, improving instrumentation, and enabling reuse inside multi-level recursion.

## 2. Current State (Initial)
- Adaptive doubling of `k` until a pivot subtree >= k or attempt limit (implicit)
- Subtree sizing logic integrated only in isolated attempts (not reused in recursion)
- Limited metrics: attempts, max subtree, collected count
- No persistent cache of subtree sizes or pivot quality across subsequent phases
- Minimal invariants (none specific to root selection correctness or subtree integrity)

## 3. Stability Criteria
| Dimension | Requirement |
|----------|-------------|
| Determinism | Same graph + seed yields identical pivot boundary & attempts |
| Metrics Completeness | Expose: attempts, success flag, k_sequence, per-attempt (max_subtree, roots_examined, relaxations, pivot_improvement_ratio) |
| Invariants | Validate: root eligibility, subtree size accumulation correctness, chosen pivot subtree >= all other candidate subtrees (unless tie), bound monotonicity across attempts |
| Integration | Provide API to reuse attempt results in higher phases / recursion without recomputation |
| Early Exit | Detect over-sized pivot early (subtree >= target*k_growth_threshold) to save attempts |
| Failure Handling | Graceful fallback when all attempts fail to reach target (expose fallback flag) |

## 4. New/Updated Data Structures
```
pub struct Phase2AttemptDetail {
    pub attempt_id: u32,
    pub k_in: u32,
    pub k_out: u32,            // possibly grown for next attempt
    pub bound: f32,
    pub collected: u32,
    pub max_subtree: u32,
    pub chosen_root: u32,
    pub roots_examined: u32,
    pub relaxations: u64,
    pub pivot_improvement_ratio: f32, // max_subtree / k_in
    pub success: i32,                 // 1 on boundary success
}
```
Cache object for reuse:
```
pub struct Phase2Cache {
    pub final_bound: f32,
    pub chosen_root: u32,
    pub attempt_details: Vec<Phase2AttemptDetail>,
    pub success: bool,
    pub fallback_used: bool,
}
```

## 5. Instrumentation Additions
- `sssp_get_phase2_cache_stats()` C ABI returning summary
- Optional `SSSP_SPEC_PHASE2_TRACE=1` to emit attempt-level JSON to stderr (debug mode)

## 6. Environment Knobs (Additions)
| Env | Default | Purpose |
|-----|---------|---------|
| `SSSP_SPEC_PHASE2_MAX_ATTEMPTS` | 6 | Upper bound on pivot attempts |
| `SSSP_SPEC_PHASE2_K_START` | 256 | Starting k value |
| `SSSP_SPEC_PHASE2_EAGER_SUCCESS_RATIO` | 1.50 | Early exit if max_subtree >= ratio * k_in |
| `SSSP_SPEC_PHASE2_MAX_K` | 0 (unbounded) | Hard cap on k growth if >0 |

## 7. Invariants (Implementation Checklist)
1. Root Eligibility: For every root r, either `pred[r] == -1` or `dist[pred[r]] >= attempt_bound`.
2. Subtree Accumulation: Sum of subtree sizes over all roots equals `collected`.
3. Pivot Optimality: `max_subtree` == size(chosen_root subtree). If multiple with same size, chosen_root is smallest id (tie rule) to ensure determinism.
4. Bound Monotonicity: Attempt bounds strictly increase across attempts until success/failure.
5. k Growth Monotonic: k sequence non-decreasing; never exceeds `SSSP_SPEC_PHASE2_MAX_K` if set.
6. Early Exit Condition: If early exit triggered, flag recorded and no further attempts executed.
7. No Silent Failure: If success==false, fallback_used flag set and final_bound==attempt_bound of last attempt.

## 8. Multi-Level Recursion Reuse Hooks
- Provide function `phase2_prepare_candidates(cache: &Phase2Cache, dist: &[f32], pred: &[i32]) -> Vec<u32>` returning subtree of chosen_root bounded by pivot boundary (candidate set for next level).
- Provide `phase2_bound(cache)` to feed as level bound.
- Provide `phase2_pivot_quality(cache)` metric for heuristic gating deeper recursion.

## 9. Milestones
1. (P2-M1) Add env knobs + attempt detail struct + ABI skeleton (no logic changes)  
2. (P2-M2) Implement full attempt loop with invariants & metrics  
3. (P2-M3) Add cache reuse API + candidate extraction  
4. (P2-M4) Integrate into single-layer recursion path (ensure parity)  
5. (P2-M5) Hook into multi-level skeleton (depth>1)  

## 10. Risks
- Overhead of invariants could distort benchmark timings: Mitigate via `SSSP_SPEC_PHASE2_INVARIANTS=0` disable switch.
- k growth plateau causing wasted attempts: Add heuristic comparing marginal subtree gain; abort if gain < epsilon threshold.

## 11. Open Questions
- Should pivot selection incorporate edge weight distribution variance? (Deferred)
- Benefit of sampling vs full root examination for very large attempt sets? (Future optimization)

## 12. Next Steps
Proceed with Milestone P2-M1 (struct + knobs + ABI) before multi-level recursion skeleton, enabling early integration tests.
