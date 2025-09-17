# Multi-Level Recursion Plan (Phase 4+)

Status: DRAFT (scaffold)  
Last Updated: 2025-09-18

## 1. Objective
Introduce hierarchical (multi-level) recursive narrowing for SSSP that repeatedly segments the reachable frontier into progressively smaller candidate sets, reducing work prior to a final correctness phase. Each level should measurably reduce remaining relaxations versus a flat boundary-chain or Phase 3 run.

Success Criteria:
- Depth > 1 produces cumulative pruning (relaxations_saved(level_d) > sum_saved(level_<d)).
- Total runtime improves over Phase3 and single-layer segmentation on mid/large graphs.
- All invariants hold (soundness: no missed shorter paths; structural properties maintained).

## 2. Current Prototype (Single Layer)
Implemented: segmentation-based pre-pass collecting frames (`SpecRecursionFrameDetail`). Frames arise from repeated truncated expansions with adaptive `k` until truncation halts. After segmentation we optionally run full baseline for correctness.

Limitations:
- No deeper recursion; frame chain is linear, not hierarchical.
- No reuse of pivot loop (Phase 2) or subtree size estimation.
- No pruning integrated into final baseline (baseline still global).
- Invariants partial (monotonic bound, dependency check only).

## 3. Gap Analysis Toward Multi-Level
| Area | Needed | Status | Planned Action |
|------|--------|--------|----------------|
| Depth Control | Max depth / early stop heuristics | Missing | Add env knobs + termination logic |
| Pivot Reuse | Use phase2 subtree sizing to choose next-level seeds | Missing | Embed pivot loop per level (optional fallback) |
| Segment Refinement | Bound tightening across levels | Proto (single) | Propagate improved `B_{L+1}` |
| Frontier Representation | Efficient filtered candidate sets | Missing | Maintain per-level candidate vector & bitset |
| Work Pruning | Skip relaxations outside refined region | Missing | Conditional early ignore using distance bound |
| Invariants | Disjointness, pivot subtree bounds, monotonic relax sets | Partial | Expanded invariant suite (see section 7) |
| Metrics | Per-level pruning ratio, pivot efficacy | Partial | Extend frame detail fields |
| JSON Schema | Hierarchical nesting | Flat | Add `parent_id`, `depth` |

## 4. Design Overview
Each recursion level L receives:
- Candidate set C_L (Vec<u32>) and optional membership bitset.
- Global distance array `dist` shared across levels.
- Upper bound `B_L` (float) limiting consideration.

Steps per Level:
1. Phase 2 style pivot loop on C_L (restricted graph) to pick a strong pivot boundary (or fallback to segmentation if insufficient size gain).
2. Construct segments (subsets) whose subtree size or bound margin justifies deeper descent. Choose top-K segments (K small, often 1) for next level.
3. Compute new bound B_{L+1} = min(dist[v] + delta_margin over chosen pivot boundary) ensuring B_{L+1} > B_L.
4. Record frame with metrics & invariants.
5. Stop if any termination rule triggers; else recurse with selected candidate subset.

Finalization: Run restricted baseline over union of visited/pruned candidate sets when feasible; otherwise full baseline (fallback flag noted in stats).

## 5. Data Structures
- `dist: Vec<f32>` global.
- `pred: Vec<i32>` global.
- `candidate: Vec<u32>` per level (reused buffers via `Vec::with_capacity` & swap technique).
- `bitset: Vec<u64>` optional for O(1) candidate membership checks when pruning.
- `PivotMeta`: root id, subtree size, local bound improvement.
- Frame detail extension:
  - depth (u32)
  - parent_id (u32)
  - pruning_ratio (f32) = 1 - |C_{L+1}| / |C_L|
  - bound_improvement (f32) = (B_{L+1} - B_L)
  - pivot_success_rate (f32) = successful_pivots / attempts

## 6. Environment Knobs (to add)
| Env Var | Default | Purpose |
|---------|---------|---------|
| `SSSP_SPEC_ML_DEPTH_MAX` | 4 | Hard depth cap |
| `SSSP_SPEC_ML_MIN_SEG_GAIN` | 1.10 | Required subtree size improvement ratio to continue |
| `SSSP_SPEC_ML_MIN_PRUNE_RATIO` | 0.05 | Minimal pruning ratio to proceed deeper |
| `SSSP_SPEC_ML_PIVOT_ATTEMPTS` | 4 | Attempts of Phase 2 pivot sizing per level |
| `SSSP_SPEC_ML_DELTA_MARGIN` | 0.0 | Additional slack added to next bound (future tuning) |
| `SSSP_SPEC_ML_RESTRICTED_BASELINE` | 1 | Try restricted baseline on candidate union |

## 7. Invariants (Extended)
1. Monotonic Bounds: B_{L+1} > B_L (strict) until final.
2. Coverage: Union of segments chosen over levels is subset of nodes with dist < final baseline distances (soundness check via parity mode).
3. Disjoint Segment Selection (per level): Selected segments for deeper descent must not overlap (except root pivot). (Check via temporary bitset.)
4. Subtree Bound: For each pivot subtree chosen, all nodes in subtree must have dist < candidate bound.
5. Pruning Monotonicity: |C_{L+1}| <= |C_L| and pruning_ratio >= 0.
6. Relaxation Non-Regress: dist never increases; (baseline parity ensures correctness).

Metrics increment invariant failure counters; failing soft invariants can abort deeper descent if ratio of failures exceeds threshold (future env).

## 8. JSON Schema Evolution
`recursion.frame_details[]` additions:
```
{
  "id": <u32>,
  "parent_id": <u32>,
  "depth": <u32>,
  "bound": <f32>,
  "k_used": <u32>,
  "segment_size": <u32>,
  "pruning_ratio": <f32>,
  "bound_improvement": <f32>,
  "pivot_success_rate": <f32>,
  ... existing fields
}
```
Top-level recursion section: add `multi_level": true/false`, `depth_reached`, `restricted_baseline_used`.

## 9. Milestones
1. (M1) Env knobs & struct field placeholders (no logic) + doc (this file).  
2. (M2) Skeleton multi-level driver calling current segmentation as level 0 + placeholder deeper level (no extra pruning).  
3. (M3) Integrate pivot loop reuse restricted to candidate set; record subtree stats.  
4. (M4) Pruning logic & restricted baseline path.  
5. (M5) Extended invariants & failure gating.  
6. (M6) Visualization & benchmark comparative study.  

## 10. Risks / Open Questions
- Restricted baseline complexity: building induced subgraph vs filtering edges dynamically; may offset gains if heavy copy cost.
- Pivot effectiveness in sparse graphs with low branching factor: may need adaptive attempt scaling.
- Bound improvement stagnation detection: which metric best signals diminishing returns? (Candidate size delta vs relaxations delta vs subtree growth.)

## 11. Next Actions
- Complete Phase 2 stabilization plan alignment with pivot reuse (#14).
- Implement M1 (env knobs + struct fields) then expose via C ABI for early harness integration.

## 12. Appendix: Future Extensions
- Parallel per-level pivot evaluation.
- Adaptive k growth strategies based on subtree variance.
- Probabilistic sampling of candidate frontier.
