#!/usr/bin/env python3
"""Cross-language benchmark harness.
Generates one random graph (CSR) and benchmarks:
  - Rust baseline
  - Rust stoc (fixed multiplier)
  - Rust stoc autotune
  - Go native dijkstra
  - C# native dijkstra
Outputs JSON summary. Requires prior build of Rust, Go, C#.
"""
import os, json, time, subprocess, random, math, statistics, argparse, tempfile, shutil, sys
from pathlib import Path

# Assume python bindings available
sys.path.append(str(Path(__file__).parent.parent / 'implementations' / 'python'))
import rust_sssp  # type: ignore

# Simple graph generator (match existing semantics loosely)
def generate_random_graph(n: int, density: float, weight_range=(1.0,10.0), seed=0):
    rnd = random.Random(seed)
    avg_out = density
    edges = []
    offsets = [0]
    for u in range(n):
        deg = max(1, int(rnd.expovariate(1/avg_out)))
        targets = set()
        for _ in range(deg):
            v = rnd.randrange(n)
            if v == u: continue
            targets.add(v)
        for v in targets:
            w = rnd.uniform(*weight_range)
            edges.append((v, w))
        offsets.append(len(edges))
    targets = [v for (v, _) in edges]
    weights = [w for (_, w) in edges]
    return offsets, targets, weights

def run_rust(offsets, targets, weights, source, mode):
    if mode == 'baseline':
        dist, pred, info = rust_sssp.run_baseline(offsets, targets, weights, source)
    elif mode == 'stoc':
        dist, pred, info = rust_sssp.run_stoc(offsets, targets, weights, source)
    elif mode == 'stoc_autotune':
        dist, pred, info = rust_sssp.run_stoc_autotune(offsets, targets, weights, source)
    else:
        raise ValueError(mode)
    return dist, pred, info

def run_go(n, density, iterations, seed, verbose=False):
    go_root = ROOT / 'implementations' / 'go'
    cmd = ["go", "run", "./cmd/benchmark", "--nodes", str(n), "--density", str(density), "--iterations", str(iterations), "--seed", str(seed), "--verify", "false"]
    start = time.time()
    out = subprocess.check_output(cmd, cwd=go_root, stderr=subprocess.STDOUT, text=True)
    elapsed = time.time() - start
    # naive parse: last "Dijkstra Algorithm:" line in output
    dijkstra_ms = None
    for line in out.splitlines():
        if "Dijkstra Algorithm:" in line:
            try:
                dijkstra_ms = float(line.split(':')[2].strip().split()[0])
            except Exception:
                pass
    return {"elapsed_wall_s": elapsed, "parsed_dijkstra_ms": dijkstra_ms, "raw_output": out if verbose else None}

def run_csharp(n, density, iterations, seed, verbose=False):
    cs_proj = ROOT/'implementations'/'csharp'
    build_dir = cs_proj/'bin'/'Debug'
    # build (debug) if not present
    subprocess.check_call(["dotnet","build"], cwd=cs_proj)
    exe = next((build_dir).rglob('*OptimizedSSSP.dll'), None)
    if exe is None:
        raise RuntimeError('Could not locate built C# dll')
    cmd = ["dotnet", str(exe), "--nodes", str(n), "--density", str(density), "--iterations", str(iterations), "--seed", str(seed), "--verbose", "false"]
    start = time.time()
    out = subprocess.check_output(cmd, cwd=ROOT, stderr=subprocess.STDOUT, text=True)
    elapsed = time.time() - start
    dijkstra_ms = None
    for line in out.splitlines():
        if "Dijkstra Algorithm:" in line:
            try:
                dijkstra_ms = float(line.split(':')[2].strip().split()[0])
            except Exception:
                pass
    return {"elapsed_wall_s": elapsed, "parsed_dijkstra_ms": dijkstra_ms, "raw_output": out if verbose else None}

ROOT = Path(__file__).resolve().parent.parent

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--nodes', type=int, default=10000)
    ap.add_argument('--density', type=float, default=2.5)
    ap.add_argument('--seed', type=int, default=12345)
    ap.add_argument('--iterations', type=int, default=1)
    ap.add_argument('--output', type=str, default='cross_language_results.json')
    ap.add_argument('--verbose', action='store_true')
    args = ap.parse_args()

    offsets, targets, weights = generate_random_graph(args.nodes, args.density, seed=args.seed)
    source = 0

    results = {}
    for mode in ['baseline','stoc','stoc_autotune']:
        t0 = time.time()
        dist, pred, info = run_rust(offsets, targets, weights, source, mode)
        dt = time.time() - t0
        results[f'rust_{mode}'] = {
            'time_s': dt,
            'relaxations': info['relaxations'],
            'light_relaxations': info.get('light_relaxations'),
            'heavy_relaxations': info.get('heavy_relaxations'),
            'settled': info['settled'],
        }
    # Go & C# native runs (only dijkstra extracted)
    go_res = run_go(args.nodes, args.density, args.iterations, args.seed, verbose=args.verbose)
    cs_res = run_csharp(args.nodes, args.density, args.iterations, args.seed, verbose=args.verbose)
    results['go_dijkstra'] = go_res
    results['csharp_dijkstra'] = cs_res

    with open(args.output,'w') as f:
        json.dump({'config': vars(args), 'results': results, 'timestamp': time.time()}, f, indent=2)
    print(f"Wrote results to {args.output}")

if __name__ == '__main__':
    main()
