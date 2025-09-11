# Language Wrappers

Minimal optional wrappers to call the Rust SSSP core (baseline + delta-stepping) from Go and C#.

## Rust Build
```
cargo build --release -p sssp_core
```
Shared library output: `implementations/rust/sssp_core/target/release/libsssp_core.(so|dylib|dll)`.
Ensure its directory is on your dynamic loader path (e.g. `export DYLD_LIBRARY_PATH=...`).

## Go Usage
```
cd wrappers/go
go get ./...
# build your app; ensure library path set
```
Example:
```go
res, _ := sssp.Run(n, offsets, targets, weights, 0, 1) // mode 1 = stoc
```

## C# Usage
```
cd wrappers/csharp
DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1 dotnet build
```
Copy or reference the native library directory; run with environment variable to locate the dylib/so.

## Modes
0 baseline (Dijkstra)
1 delta-stepping (fixed multiplier)
2 delta-stepping autotuned

## Stats Fields
Relaxations, LightRelaxations, HeavyRelaxations, Settled, ErrorCode, Version.
