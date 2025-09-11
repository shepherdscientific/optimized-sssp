import random
import time
import math
import argparse
import json
from rust_sssp import run_baseline, run_stoc

# Simple random graph generator (uniform) producing CSR arrays
# Nodes are 0..n-1; expected edges ~ density * n

def generate_graph(n: int, density: float, weight_low=1.0, weight_high=10.0, seed=12345):
    rng = random.Random(seed)
    # approximate m
    target_edges = int(density * n)
    # build adjacency lists first
    adj = [[] for _ in range(n)]
    for _ in range(target_edges):
        u = rng.randrange(n)
        v = rng.randrange(n)
        if u == v:
            continue
        w = rng.uniform(weight_low, weight_high)
        adj[u].append((v, w))
    # CSR
    offsets = [0]
    targets = []
    weights = []
    for u in range(n):
        # optionally sort targets for locality
        if adj[u]:
            adj[u].sort(key=lambda x: x[0])
        for (v, w) in adj[u]:
            targets.append(v)
            weights.append(w)
        offsets.append(len(targets))
    return offsets, targets, weights


def run_trial(n, density, seed):
    offsets, targets, weights = generate_graph(n, density, seed=seed)
    src = 0
    t0 = time.perf_counter()
    dist_b, pred_b, stats_b = run_baseline(offsets, targets, weights, src)
    t1 = time.perf_counter()
    # STOC
    try:
        dist_s, pred_s, stats_s = run_stoc(offsets, targets, weights, src)
        t2 = time.perf_counter()
        stoc_ms = (t2 - t1) * 1000.0
        stoc_speedup = (t1 - t0) / (t2 - t1) if (t2 - t1) > 0 else None
    except Exception:
        stoc_ms = None
        stoc_speedup = None
        stats_s = None

    return {
        'n': n,
        'density': density,
        'm': offsets[-1],
        'baseline_ms': (t1 - t0) * 1000.0,
        'stoc_ms': stoc_ms,
    'stoc_speedup': stoc_speedup,
    'stoc_ms': stoc_ms,
    'stoc_speedup': stoc_speedup,
        'baseline_stats': stats_b,
    'optimized_stats': None,
    'hybrid_stats': None,
    'stoc_stats': stats_s
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--sizes', type=str, default='1000,5000,10000,20000,50000')
    ap.add_argument('--density', type=float, default=2.0)
    ap.add_argument('--seed', type=int, default=12345)
    ap.add_argument('--output', type=str, default='rust_variant_bench.json')
    ap.add_argument('--plot', type=str, default='rust_variant_bench.png')
    args = ap.parse_args()

    sizes = [int(x) for x in args.sizes.split(',') if x.strip()]

    results = []
    for n in sizes:
        r = run_trial(n, args.density, args.seed)
        msg = f"n={n} baseline={r['baseline_ms']:.2f}ms"
        if r['stoc_ms'] is not None:
            msg += f" stoc={r['stoc_ms']:.2f}ms stoc_spd={(r['stoc_speedup'] or 0):.2f}x"
        print(msg)
        results.append(r)

    with open(args.output, 'w') as f:
        json.dump(results, f, indent=2)

    try:
        import matplotlib.pyplot as plt  # type: ignore
    except ImportError:
        print('matplotlib not available; skipping plot'); return

    xs = [r['n'] for r in results]
    b = [r['baseline_ms'] for r in results]
    plt.figure(figsize=(8,5))
    plt.plot(xs, b, marker='o', label='Baseline (binary heap)')
    if any(r['stoc_ms'] for r in results):
        sxs = [r['n'] for r in results if r['stoc_ms'] is not None]
        sm = [r['stoc_ms'] for r in results if r['stoc_ms'] is not None]
        plt.plot(sxs, sm, marker='D', label='STOC (delta-step)')
    plt.xlabel('Nodes (n)')
    plt.ylabel('Time (ms)')
    plt.title('Rust SSSP Variants Performance')
    plt.legend()
    plt.grid(True, alpha=0.3)
    ax2 = plt.twinx()
    if any(r['stoc_speedup'] for r in results):
        sx2 = [r['n'] for r in results if r['stoc_speedup'] is not None]
        sp_s = [r['stoc_speedup'] for r in results if r['stoc_speedup'] is not None]
        ax2.plot(sx2, sp_s, color='orange', marker='D', linestyle='--', label='STOC speedup')
    ax2.set_ylabel('Speedup (baseline / variant)')
    lines, labels = plt.gca().get_legend_handles_labels()
    lines2, labels2 = ax2.get_legend_handles_labels()
    plt.legend(lines + lines2, labels + labels2, loc='best')
    plt.tight_layout()
    plt.savefig(args.plot, dpi=150)
    print(f"Saved plot to {args.plot}")

if __name__ == '__main__':
    main()
