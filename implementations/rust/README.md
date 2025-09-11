# Rust SSSP Core

High-performance single-source shortest path (Dijkstra) core exported via stable C ABI for Go, Python, and C#.

Two exposed variants:

| Function | Variant | Notes |
|----------|---------|-------|
| `sssp_run_baseline` | Baseline | Classic binary heap Dijkstra |
| `sssp_run_optimized` | Optimized | 4-ary heap + unchecked inner loop |

## C ABI
```
int32_t sssp_run_baseline(
    uint32_t n,
    const uint32_t* offsets,
    const uint32_t* targets,
    const float*   weights,
    uint32_t source,
    float* out_dist,
    int32_t* out_pred,
    struct SsspResultInfo* info);

int32_t sssp_run_optimized(
    uint32_t n,
    const uint32_t* offsets,
    const uint32_t* targets,
    const float*   weights,
    uint32_t source,
    float* out_dist,
    int32_t* out_pred,
    struct SsspResultInfo* info);

uint32_t sssp_version();
```

`offsets[n] = m`. Distances initialized to +Inf; unreachable stays +Inf.

## Build
```
cargo build --release
```
Outputs `libsssp_core.{a,dylib,so}` in `target/release`.

## Next Optimizations (planned)
- BFS / RCM node reordering utility (extern function)
- Batch pop (process small distance window)
- Optional float16/quantized weights
- Parallel frontier expansion (experimental)

## Safety
Caller owns all memory; Rust never frees caller buffers.

## License
MIT OR Apache-2.0
