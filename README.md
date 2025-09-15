docs/ (will be pruned further)
go mod tidy
go run cmd/benchmark/main.go --help
# Optimized SSSP (Clean Spec Implementation)

From classical Dijkstra toward a structured path for O(m log^{2/3} n).

Current focus: parity core + instrumentation. (Former delta-stepping code is legacy only.)

## 1. Purpose
This repository is a clean-room trajectory toward an eventually layered BMSSP-style single-source shortest path algorithm targeting the theoretical O(m log^{2/3} n) bound. We start from a rigorously instrumented, parity-correct MinHeap Dijkstra (`spec_clean`) and will introduce one structural mechanism per phase (pivots, bounded recursion, batched prepends, selective frontier growth) with empirical and invariant checks at each step.

## 2. What Exists Right Now
| Component | Status | Notes |
|-----------|--------|-------|
| Baseline binary-heap Dijkstra | ✅ Stable | Reference correctness oracle |
| `spec_clean` (custom min-heap) | ✅ Parity | ~1–6% constant-factor improvement; identical relax counts |
| Heap instrumentation (pushes/pops/max) | ✅ | Guides structural comparisons |
| Benchmark (baseline vs spec) + overlays | ✅ | Produces JSON + plot + speedup axis |
| Statistical grid benchmark & heatmap | ✅ (script) | Captures variation across (n,density) |
| BMSSP hierarchical mechanics | ⏳ Not started | Will be layered incrementally |
| Pivot selection / wave gathering | ⏳ | Maps to FindPivots in spec snapshot |
| Bounded multi-level recursion (l,t,k) | ⏳ | Core of theoretical improvement |
| BatchPrepend / segmented DS `D` | ⏳ | Needed for amortized bounds |

## 3. Spec Snapshot (Canonical Target)
See `docs/SSSP_SPEC_SNAPSHOT.md` (frozen). Mermaid summary of the recursive BMSSP skeleton:

```mermaid
flowchart TD
  Start[BMSSP Start] --> Check{Level zero?}
  Check -- yes --> Base[BaseCase truncated]
  Check -- no --> Pivots[Find pivots]
  Pivots --> InitD[Init structure D]
  InitD --> Loop{D not empty AND under limit}
  Loop --> Pull[Pull block]
  Pull --> Recurse[Recurse level-1]
  Recurse --> Relax[Relax edges]
  Relax --> Insert[Insert future]
  Insert --> Batch[Batch prepend]
  Batch --> Loop
  Loop -- done --> Done[Return Result]
```

Key bounded parameters (from snapshot):
```text
k = floor(log^{1/3} n)
t = floor(log^{2/3} n)
Level l frontier limit: |S| ≤ 2^{l t}
Pull capacity M = 2^{(l-1) t}
```

## 4. Gap Analysis: Current vs Spec
| Spec Element | Needed For | Implemented? | Planned Phase |
|--------------|-----------|--------------|---------------|
| Equality relax (<=) invariant | Tight forest reuse | ✅ (baseline & spec_clean) | — |
| BaseCase truncated growth (k+1 guard) | Size control | ❌ | Phase 1 (next) |
| Pivot discovery (k waves / BF style) | Shrink frontier | ❌ | Phase 2 |
| Forest root filtering (≥ k subtree) | Bound pivot count | ❌ | Phase 2 |
| Data structure D (Pull / BatchPrepend) | Amortized selection | ❌ | Phase 3 |
| Boundary B'/B management chain | Disjoint U_i sets | ❌ | Phase 3 |
| Multi-level recursion l=0..L | Hierarchical scaling | ❌ | Phase 4 |
| Invariant checks (S-size, dependency) | Safety proofs | ❌ | Ongoing (each phase) |
| Instrumented counters per recursion | Empirical validation | ❌ | Phase 4 |

## 5. Benchmarks

### 5.1 Sample (Parity Range)
![Sample Baseline vs Spec](benchmarks/rust_spec_baseline_sample.png)

| n | m | Baseline ms | Spec ms | Speedup |
|---|---|-------------|---------|---------|
| 25,000 | 49,996 | 22.39 | 22.06 | 1.015x |
| 50,000 | 99,996 | 47.63 | 47.09 | 1.012x |
| 100,000 | 199,998 | 116.81 | 111.53 | 1.047x |
| 250,000 | 499,996 | 343.90 | 334.29 | 1.029x |
| 500,000 | 999,998 | 746.58 | 702.16 | 1.063x |
| 1,000,000 | 1,999,999 | 1,549.55 | 1,501.69 | 1.032x |

### 5.2 Extended (Larger Sizes)
![Large Benchmark](benchmarks/rust_spec_baseline_big.png)

| n | m | Baseline ms | Spec ms | Speedup |
|---|---|-------------|---------|---------|
| 25,000 | 49,996 | 21.48 | 21.74 | 0.99x |
| 50,000 | 99,996 | 47.29 | 46.66 | 1.01x |
| 100,000 | 199,998 | 124.87 | 112.23 | 1.11x |
| 250,000 | 499,996 | 343.51 | 327.77 | 1.05x |
| 500,000 | 999,998 | 720.15 | 706.19 | 1.02x |
| 1,000,000 | 1,999,999 | 1541.73 | 1503.30 | 1.03x |
| 2,500,000 | 4,999,999 | 3424.53 | 3331.04 | 1.03x |
| 5,000,000 | 9,999,999 | 7032.10 | 6909.83 | 1.02x |
| 10,000,000 | 19,999,998 | 21555.25 | 18749.72 | 1.15x |

Observed speedup pattern: modest (1.0–1.06x typical) with occasional higher outlier at largest n (cache residency & branch profile effects). No asymptotic change expected yet.

### 5.3 Statistical Grid & Heatmap
Scripts produce heatmaps summarizing median speedup across (n,density). Example placeholders (regenerate via statistical script):

Random small sampling heatmap:
![Speedup Heatmap (Sample)](benchmarks/stat_full_heatmap.png)

Larger run heatmap:
![Speedup Heatmap (Large)](benchmarks/stat_full_heatmap_big.png)

## 6. Reproduce Benchmarks
```bash
cargo build --release -p sssp_core
python implementations/python/benchmark_rust_variants.py \
  --sizes 25000,50000,100000,250000,500000,1000000 \
  --density 2.0 --seed 42 \
  --output benchmarks/rust_spec_baseline_sample.json \
  --plot benchmarks/rust_spec_baseline_sample.png

# Extended sizes
python implementations/python/benchmark_rust_variants.py \
  --sizes 25000,50000,100000,250000,500000,1000000,2500000,5000000,10000000 \
  --density 2.0 --seed 42 \
  --output benchmarks/rust_spec_baseline_big.json \
  --plot benchmarks/rust_spec_baseline_big.png
```

## 7. Instrumentation Snapshot
Example JSON fields (per size):
```jsonc
{
  "baseline_ms": 1541.73,
  "spec_ms": 1503.30,
  "spec_speedup": 1.03,
  "baseline_heap": { "pushes": 839761, "pops": 839761, "max_size": 164079 },
  "spec_heap": { "pushes": 839761, "pops": 839761, "max_size": 164079 }
}
```
Heap identity confirms no structural optimization applied yet—future phases should drive divergence (reduced pushes or lower max_size plateau) or justify conceptual changes.

## 8. Development Roadmap (Rolling)
1. Implement BaseCase truncation & k parameter wiring.
2. Add pivot wave primitive (bounded k relax waves) + subtree sizing.
3. Prototype minimal D with Pull + BatchPrepend; measure overhead vs pure heap.
4. Integrate multi-level recursion, verifying invariants after each level with debug asserts.
5. Add instrumentation counters per level: pulls, batch prepends, successful pivots, truncated basecases.
6. Statistical validation: track growth of |U| per level vs theoretical caps.
7. Optimization passes: memory pooling, distance type specialization, adjacency ordering.

## 9. How Close Are We?
Current work corresponds to a “Level 0 always” regime: we run a full unrestricted Dijkstra (effectively BaseCase without truncation) every time. The theoretical improvements rely on constraining growth and layering dependent exploration. Thus: we are at Step 0 structurally, with instrumentation prepared to observe changes as soon as truncation & pivot phases land.

## 10. Legacy / Deprecated (STOC Path)
Delta-stepping code remains only for historical comparison and will not evolve further in this branch. It may be entirely removed once BMSSP phases demonstrate stable improvement. Treat any STOC references as archival.

## 11. Contributing
Please limit changes to:
* Parity-preserving enhancements (faster heap, memory layout) WITH instrumentation deltas.
* Incremental BMSSP phase implementations following snapshot.
* Improved benchmark/statistical tooling.

Out of scope: reintroducing multi-language scaffolding or unrelated algorithm variants at this stage.

## 12. License
MIT License – see [LICENSE](LICENSE).

## 13. References
* BMSSP / hierarchical SSSP theoretical notes (internal snapshot): `docs/SSSP_SPEC_SNAPSHOT.md`
* Classical Dijkstra analysis
* Planned empirical methodology (statistical scripts under `benchmarks/`)

---
<sub>Spec version synchronized with snapshot commit; update requires explicit snapshot delta + version bump.</sub>