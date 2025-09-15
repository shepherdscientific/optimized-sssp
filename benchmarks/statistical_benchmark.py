#!/usr/bin/env python3
"""Statistical benchmarking for baseline vs spec_clean.
Runs multiple repetitions over a grid of (n, density) values, captures timing
statistics (median, mean, stdev, min, max) and speedup distribution.
Outputs JSON + heatmap of median speedup.
"""
import time, math, random, json, argparse, statistics, os
from pathlib import Path
import numpy as np
import sys
ROOT = Path(__file__).resolve().parent.parent
sys.path.append(str(ROOT / 'implementations' / 'python'))
import rust_sssp  # type: ignore

# Graph generator (same as other benchmarks for consistency)
def generate_graph(n:int, density:float, seed:int):
    rnd = random.Random(seed)
    target_edges = int(density * n)
    adj = [[] for _ in range(n)]
    for _ in range(target_edges):
        u = rnd.randrange(n); v = rnd.randrange(n)
        if u==v: continue
        w = rnd.random()*9+1
        adj[u].append((v,w))
    offsets=[0]; targets=[]; weights=[]
    for u in range(n):
        for v,w in adj[u]:
            targets.append(v); weights.append(w)
        offsets.append(len(targets))
    return offsets, targets, weights

def time_variant(fn, offsets, targets, weights, src):
    t0 = time.perf_counter(); fn(offsets, targets, weights, src); return (time.perf_counter()-t0)*1000.0

def stats(values):
    return {
        'n': len(values),
        'median': statistics.median(values),
        'mean': statistics.fmean(values),
        'stdev': statistics.pstdev(values) if len(values)>1 else 0.0,
        'min': min(values),
        'max': max(values)
    }

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--sizes', default='5000,10000,25000,50000,100000')
    ap.add_argument('--densities', default='1.0,2.0,3.0')
    ap.add_argument('--reps', type=int, default=7)
    ap.add_argument('--seed', type=int, default=42)
    ap.add_argument('--output', default='stat_benchmark.json')
    ap.add_argument('--heatmap', default='stat_speedup_heatmap.png')
    args = ap.parse_args()

    sizes=[int(s) for s in args.sizes.split(',') if s]
    densities=[float(d) for d in args.densities.split(',') if d]

    grid_results=[]
    speedup_matrix = np.zeros((len(sizes), len(densities)))

    for i,n in enumerate(sizes):
        for j,d in enumerate(densities):
            base_times=[]; spec_times=[]
            # Re-generate graph each repetition (captures noise); could reuse to isolate algorithmic variance
            for r in range(args.reps):
                offsets,targets,weights = generate_graph(n,d, seed=args.seed + r)
                src=0
                tb = time_variant(rust_sssp.run_baseline, offsets, targets, weights, src)
                if rust_sssp._HAS_SPEC_CLEAN:
                    ts = time_variant(rust_sssp.run_spec_clean, offsets, targets, weights, src)
                else:
                    ts = float('nan')
                base_times.append(tb); spec_times.append(ts)
            b_stats = stats(base_times); s_stats = stats(spec_times)
            # Per-repetition speedups
            speedups = [ (b/s) if s>0 else float('nan') for b,s in zip(base_times,spec_times) ]
            speedup_stats = stats(speedups)
            speedup_matrix[i,j] = speedup_stats['median']
            grid_results.append({
                'n': n,
                'density': d,
                'baseline': b_stats,
                'spec': s_stats,
                'speedup': speedup_stats,
                'raw': {'baseline_ms': base_times, 'spec_ms': spec_times, 'speedup': speedups}
            })
            print(f"n={n} d={d} base_med={b_stats['median']:.2f}ms spec_med={s_stats['median']:.2f}ms spd_med={speedup_stats['median']:.3f}x")

    payload = {
        'config': vars(args),
        'results': grid_results
    }
    with open(args.output,'w') as f: json.dump(payload,f,indent=2)
    print(f"Wrote {args.output}")

    # Heatmap
    try:
        import matplotlib.pyplot as plt
        import seaborn as sns  # type: ignore
    except Exception:
        print('matplotlib/seaborn not available; skipping heatmap')
        return
    plt.figure(figsize=(1.8+1.2*len(densities), 1.8+0.6*len(sizes)))
    sns.heatmap(speedup_matrix, annot=True, fmt='.2f', cmap='viridis',
                xticklabels=[str(d) for d in densities], yticklabels=[str(s) for s in sizes])
    plt.xlabel('Density (avg out-degree)')
    plt.ylabel('Nodes (n)')
    plt.title('Spec Clean Speedup (median baseline/spec)')
    plt.tight_layout()
    plt.savefig(args.heatmap, dpi=140)
    print(f"Saved heatmap to {args.heatmap}")

if __name__ == '__main__':
    main()
