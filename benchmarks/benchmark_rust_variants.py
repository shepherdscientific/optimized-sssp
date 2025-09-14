import random
import time
import argparse
import json
import sys, os
sys.path.append(os.path.join(os.path.dirname(__file__), '..', 'implementations', 'python'))
from rust_sssp import run_baseline, run_stoc, run_khop, run_stoc_auto_adapt
try:
    from rust_sssp import run_default  # may not exist yet
    HAS_DEFAULT = True
except Exception:
    HAS_DEFAULT = False


def generate_graph(n: int, density: float, weight_low=1.0, weight_high=10.0, seed=12345):
    rng = random.Random(seed)
    target_edges = int(density * n)
    adj = [[] for _ in range(n)]
    for _ in range(target_edges):
        u = rng.randrange(n); v = rng.randrange(n)
        if u == v: continue
        w = rng.uniform(weight_low, weight_high)
        adj[u].append((v, w))
    offsets=[0]; targets=[]; weights=[]
    for u in range(n):
        if adj[u]: adj[u].sort(key=lambda x: x[0])
        for (v,w) in adj[u]:
            targets.append(v); weights.append(w)
        offsets.append(len(targets))
    return offsets, targets, weights


def run_trial(n, density, seed):
    offsets, targets, weights = generate_graph(n, density, seed=seed)
    src = 0
    t0=time.perf_counter(); dist_b, pred_b, stats_b = run_baseline(offsets, targets, weights, src); t1=time.perf_counter()
    try:
        dist_s, pred_s, stats_s = run_stoc(offsets, targets, weights, src); t2=time.perf_counter(); stoc_ms=(t2-t1)*1000; stoc_speed=(t1-t0)/(t2-t1) if (t2-t1)>0 else None
    except Exception:
        stoc_ms=None; stoc_speed=None; stats_s=None; t2=t1
    try:
        dist_k, pred_k, stats_k = run_khop(offsets, targets, weights, src); tk=time.perf_counter(); khop_ms=(tk-t2)*1000; khop_speed=(t1-t0)/(tk-t2) if (tk-t2)>0 else None
    except Exception:
        khop_ms=None; khop_speed=None; stats_k=None; tk=t2
    default_ms=None; default_speed=None; stats_d=None
    if HAS_DEFAULT:
        try:
            dist_d, pred_d, stats_d = run_default(offsets, targets, weights, src); td=time.perf_counter(); default_ms=(td-tk)*1000; default_speed=(t1-t0)/(td-tk) if (td-tk)>0 else None
        except Exception:
            pass
    return {
        'n': n,
        'm': offsets[-1],
        'density': density,
        'baseline_ms': (t1-t0)*1000,
        'stoc_ms': stoc_ms,
        'khop_ms': khop_ms,
        'default_ms': default_ms,
        'stoc_speedup': stoc_speed,
        'khop_speedup': khop_speed,
        'default_speedup': default_speed,
        'baseline_stats': stats_b,
        'stoc_stats': stats_s,
        'khop_stats': stats_k,
        'default_stats': stats_d,
    }


def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--sizes', default='2000,4000,8000,16000,32000')
    ap.add_argument('--density', type=float, default=2.0)
    ap.add_argument('--seed', type=int, default=12345)
    ap.add_argument('--output', default='rust_variant_bench.json')
    ap.add_argument('--plot', default='rust_variant_bench.png')
    args=ap.parse_args()
    sizes=[int(s) for s in args.sizes.split(',') if s]
    results=[]
    for n in sizes:
        r=run_trial(n, args.density, args.seed)
        msg=f"n={n} base={r['baseline_ms']:.2f}ms"
        if r['stoc_ms'] is not None: msg+=f" stoc={r['stoc_ms']:.2f}ms"
        if r['khop_ms'] is not None: msg+=f" khop={r['khop_ms']:.2f}ms"
        if r['default_ms'] is not None: msg+=f" default={r['default_ms']:.2f}ms"
        print(msg)
        results.append(r)
    with open(args.output,'w') as f: json.dump(results,f,indent=2)
    try:
        import matplotlib.pyplot as plt
    except ImportError:
        print('matplotlib not available; skipping plot'); return
    xs=[r['n'] for r in results]
    plt.figure(figsize=(8,5))
    plt.plot(xs, [r['baseline_ms'] for r in results], marker='o', label='Baseline')
    if any(r['stoc_ms'] for r in results):
        plt.plot([r['n'] for r in results if r['stoc_ms']], [r['stoc_ms'] for r in results if r['stoc_ms']], marker='d', label='STOC')
    if any(r['khop_ms'] for r in results):
        plt.plot([r['n'] for r in results if r['khop_ms']], [r['khop_ms'] for r in results if r['khop_ms']], marker='s', label='KHOP batch')
    if any(r['default_ms'] for r in results):
        plt.plot([r['n'] for r in results if r['default_ms']], [r['default_ms'] for r in results if r['default_ms']], marker='^', label='Default')
    plt.xlabel('n'); plt.ylabel('Time (ms)'); plt.title('Variants'); plt.legend(); plt.grid(alpha=0.3)
    plt.tight_layout(); plt.savefig(args.plot, dpi=140)
    print(f"Saved plot to {args.plot}")

if __name__=='__main__':
    main()
