# Implementation Status (Spec-Oriented Optimized SSSP)

This document now tracks the incremental realization of the spec elements needed for the hierarchical tight-forest reuse algorithm (working codename: BMSSP). We shift from earlier layering / clustering prototype claims to a concrete phased integration path aligned with the current Rust core.

## Phase Matrix

| Spec Element | Purpose | Status | Notes |
|--------------|---------|--------|-------|
| Equality relax (<=) invariant | Enables tight forest reuse across waves | ✅ (baseline & spec_clean) | Already enforced in all relax points |
| BaseCase truncated growth (k+1 guard) | Size control of U prefix | ✅ (Phase 1) | Exported: `sssp_run_spec_phase1`, probe: `sssp_spec_basecase_probe` |
| Pivot discovery (k waves / BF style) | Shrinks active frontier | ⏳ Planned | Will batch incremental Dijkstra waves until size ≥ k+1 or distance pivot stabilizes |
| Forest root filtering (≥ k subtree) | Bounds pivot count | ⏳ Planned | Requires subtree size accounting per root candidate |
| Data structure D (Pull / BatchPrepend) | Amortized selection | ⏳ Planned | Combines small bucketed frontier with append-invariant buffers |
| Boundary B'/B management chain | Maintain disjoint U_i | ⏳ Planned | Formalize transitions: (B, U) -> (B', U') with safety checks |
| Multi-level recursion l=0..L | Hierarchical scaling | ⏳ Planned | Recursively apply basecase+pivot on contracted quotient graph |
| Invariant checks (S-size, dependency) | Safety proofs | 🏗 Ongoing | Will gate debug builds via `SSSP_SPEC_CHECK=1` |
| Instrumented counters per recursion | Empirical validation | ⏳ Phase 4 | Aggregated JSON export for benchmark harness |

## Current Rust Additions (Phase 1)

Added in `spec_clean.rs`:
- `basecase_truncated` internal with relax counting & k+1 cutoff.
- Public probe: `sssp_spec_basecase_probe` for experimentation / tuning.
- Phase 1 runner: `sssp_run_spec_phase1` (env: `SSSP_SPEC_K`, `SSSP_SPEC_BOUND`).
- Phase 1 stats struct & getter: `SpecPhase1Stats` via `sssp_get_spec_phase1_stats`.

## Environment Controls (Phase 1)
| Variable | Meaning | Default |
|----------|---------|---------|
| `SSSP_SPEC_K` | Basecase k (prefix target) | 1024 |
| `SSSP_SPEC_BOUND` | Distance bound B (float) | +∞ |
| `SSSP_SPEC_CHECK` | Enable future invariant assertions | off |

## Immediate Next Steps
1. Implement pivot accumulation loop (Phase 2 start) over repeated truncated expansions.
2. Track subtree sizes to filter forest roots meeting ≥ k criterion.
3. Introduce DS D skeleton (batched frontier append + pull interface).
4. Add debug invariant macros gated by `SSSP_SPEC_CHECK`.

## Testing Plan Outline
- Deterministic synthetic graphs (ring, star, layered ladder) to validate truncation boundaries.
- Random geometric graphs for distribution of B' relative to k.
- Cross-verify Phase1 distance prefix with baseline Dijkstra restricted to nodes dist < B'.

## Instrumentation Roadmap
Planned cumulative stats (per recursion level later):
```
struct RecursionStats {
    level: u32,
    k: u32,
    base_relax: u64,
    pivots: u32,
    forests_built: u32,
    avg_subtree: f32,
    restarts: u32,
    time_ns: u64,
}
```
Export intent: `sssp_spec_dump_stats(JSONptr,len)` (Phase 4).

## Rationale Shift
Earlier document sections referencing a fully realized O(m log^(2/3) n) layered/cluster pipeline were aspirational scaffolding. We now ground the roadmap strictly in implemented code paths present in this repository (Rust core + cross-language wrappers). Historical narrative retained in git history; this file reflects real progress only.

## Changelog (Recent)
- 2025-09-17: Phase 1 basecase integration, stats getter, updated roadmap.

## License
MIT (see repository `LICENSE`).

---
For questions or to propose refinements to the phase ordering, open an issue referencing the matrix entry.