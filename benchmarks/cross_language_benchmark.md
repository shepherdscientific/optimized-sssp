# Cross-Language SSSP Benchmark Harness (Rust vs Native Go / C#)

This document describes the added autotune + light/heavy metrics and proposes a harness (scripts to be added) comparing:

- Rust Baseline (binary heap Dijkstra)
- Rust STOC (fixed multiplier) `sssp_run_stoc`
- Rust STOC Autotune `sssp_run_stoc_autotune`
- Native Go Dijkstra
- Native C# Dijkstra

## New Rust API

C symbols:
- `sssp_run_baseline`
- `sssp_run_stoc`
- `sssp_run_stoc_autotune`
- `sssp_info_light_relaxations(const SsspResultInfo*)`
- `sssp_info_heavy_relaxations(const SsspResultInfo*)`
- `sssp_version`

Struct (breaking change):
```
struct SsspResultInfo {
  uint64_t relaxations;
  uint64_t light_relaxations;
  uint64_t heavy_relaxations;
  uint32_t settled;
  int32_t  error_code;
};
```

## Autotune Environment Variables
- `SSSP_STOC_AUTOTUNE_SET` e.g. `"1.5,2,3,4,6"`
- `SSSP_STOC_AUTOTUNE_LIMIT` default 2048 (nodes to settle during trial)
- (Legacy) `SSSP_STOC_DELTA_MULT` still used for fixed variant

## Planned Harness (next step)
1. Python driver loads/generates identical random graph CSR once.
2. Runs each variant and records wall-clock + relax counters.
3. For Go and C#, invokes their binaries (build once) with matching graph parameters (or re-generates with same seed + serialization for parity).
4. Outputs JSON + Markdown summary table.

Pending Implementation: A Python script `benchmarks/cross_language_runner.py` (not yet added) to orchestrate runs. This will be added in a subsequent commit if desired.

