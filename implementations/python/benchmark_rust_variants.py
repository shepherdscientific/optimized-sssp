import random
import time
import math
import argparse
import json
from rust_sssp import run_baseline, _HAS_SPEC_CLEAN, run_spec_clean
from rust_sssp import get_baseline_heap_stats, get_spec_heap_stats  # added

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


def run_trial(n, density, seed, verify_spec=True):
    offsets, targets, weights = generate_graph(n, density, seed=seed)
    src = 0
    t0 = time.perf_counter()
    dist_b, pred_b, stats_b = run_baseline(offsets, targets, weights, src)
    t1 = time.perf_counter()
    base_heap = get_baseline_heap_stats()
    spec_ms = None
    spec_speedup = None
    spec_stats = None
    spec_parity_ok = None
    spec_heap = None
    if _HAS_SPEC_CLEAN:
        t_spec0 = time.perf_counter()
        dist_spec, pred_spec, spec_stats = run_spec_clean(offsets, targets, weights, src)
        t_spec1 = time.perf_counter()
        spec_ms = (t_spec1 - t_spec0) * 1000.0
        spec_speedup = (t1 - t0) / (t_spec1 - t_spec0) if (t_spec1 - t_spec0) > 0 else None
        spec_heap = get_spec_heap_stats()
        if verify_spec:
            # Parity: distances identical (allow tiny float epsilon)
            mismatches = [i for i,(db,ds) in enumerate(zip(dist_b, dist_spec)) if abs(db-ds) > 1e-6 or (math.isinf(db) != math.isinf(ds))]
            spec_parity_ok = (len(mismatches) == 0)
            if not spec_parity_ok:
                print(f"[WARN] spec_clean parity mismatches n={n} count={len(mismatches)} sample={mismatches[:10]}")
    return {
        'n': n,
        'density': density,
        'm': offsets[-1],
        'baseline_ms': (t1 - t0) * 1000.0,
        'spec_ms': spec_ms,
        'spec_speedup': spec_speedup,
        'spec_parity_ok': spec_parity_ok,
        'baseline_stats': stats_b,
        'spec_stats': spec_stats,
        'baseline_heap': base_heap,
        'spec_heap': spec_heap,
        'spec_only': True
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
        msg = f"n={n} base={r['baseline_ms']:.2f}ms"
        if r.get('baseline_heap') and r['baseline_heap'].get('max_size') is not None:
            msg += f" baseH={r['baseline_heap']['max_size']}"
        if r.get('spec_ms') is not None:
            msg += f" spec={r['spec_ms']:.2f}ms"
            if r.get('spec_speedup') is not None:
                msg += f" spec_spd={(r['spec_speedup'] or 0):.2f}x"
            if r.get('spec_heap') and r['spec_heap'] and r['spec_heap'].get('max_size') is not None:
                msg += f" specH={r['spec_heap']['max_size']}"
            if r.get('spec_parity_ok') is not None:
                msg += " parity=OK" if r['spec_parity_ok'] else " parity=FAIL"
        print(msg)
        results.append(r)

    # We'll append fit constants later after computing overlays; store intermediate
    raw_results = results

    try:
        import matplotlib.pyplot as plt  # type: ignore
    except ImportError:
        print('matplotlib not available; skipping plot'); return

    xs = [r['n'] for r in results]
    b = [r['baseline_ms'] for r in results]
    m_vals = [r['m'] for r in results]
    fig = plt.figure(figsize=(8,5))
    ax_time = fig.add_subplot(111)
    ax_time.plot(xs, b, marker='o', label='Baseline empirical')
    if any(r.get('spec_ms') for r in results):
        sm_spec = [r['spec_ms'] for r in results if r.get('spec_ms') is not None]
        sx_spec = [r['n'] for r in results if r.get('spec_ms') is not None]
    ax_time.plot(sx_spec, sm_spec, marker='s', label='Spec Clean (parity)')
    # (STOC removed)
    ax_time.set_xlabel('Nodes (n)')
    ax_time.set_ylabel('Time (ms)')
    ax_time.set_title('Rust SSSP Variants Performance')
    # Fit constants for overlays using least squares on provided sizes
    import numpy as _np
    logn = _np.array([math.log(n) for n in xs])
    m_arr = _np.array(m_vals, dtype=float)
    baseline_arr = _np.array(b)
    # Model1: c1 * (m + n log n) ~ treat m ~ density*n so keep both terms
    comp1 = m_arr + _np.array(xs) * logn
    # Model2: c2 * m * (log n)**(2/3)
    comp2 = m_arr * (logn ** (2/3))
    # Solve c via least squares (c = (x.y)/(x.x))
    def fit_const(comp, y):
        denom = float((comp*comp).sum())
        return float((comp*y).sum()/denom) if denom>0 else 0.0
    c1 = fit_const(comp1, baseline_arr)
    c2 = fit_const(comp2, baseline_arr)
    # Scale: comp terms are counts; multiply by constant to approximate ms directly
    overlay1 = c1 * comp1
    overlay2 = c2 * comp2
    ax_time.plot(xs, overlay1, linestyle='--', color='gray', label='O(m + n log n) fit')
    ax_time.plot(xs, overlay2, linestyle='--', color='purple', label='O(m log^{2/3} n) fit')
    ax_time.grid(True, alpha=0.3)
    ax_speed = ax_time.twinx()
    speedup_handles = []
    speedup_labels = []
    # (STOC speedup removed)
    if any(r.get('spec_speedup') for r in results):
        sx2b = [r['n'] for r in results if r.get('spec_speedup') is not None]
        sp_spec = [r['spec_speedup'] for r in results if r.get('spec_speedup') is not None]
        h_spec, = ax_speed.plot(sx2b, sp_spec, color='green', marker='s', linestyle='--', label='Spec speedup')
        speedup_handles.append(h_spec); speedup_labels.append('Spec speedup')
    ax_speed.set_ylabel('Speedup (baseline / variant)')
    # Primary legend on time axis
    ph, pl = ax_time.get_legend_handles_labels()
    ax_time.legend(ph, pl, loc='upper left')
    if speedup_handles:
        ax_speed.legend(speedup_handles, speedup_labels, loc='lower right')
    plt.tight_layout()
    plt.savefig(args.plot, dpi=150)
    print(f"Saved plot to {args.plot}")
    # Write JSON with fits
    output_payload = {
        'config': {
            'sizes': sizes,
            'density': args.density,
            'seed': args.seed,
        },
        'results': raw_results,
        'fits': {
            'c1_m_plus_nlogn': c1,
            'c2_m_log23_n': c2,
            'model_points': { 'overlay1_ms': list(map(float, overlay1)), 'overlay2_ms': list(map(float, overlay2)) }
        }
    }
    with open(args.output, 'w') as f:
        json.dump(output_payload, f, indent=2)
    print(f"Saved JSON with fit constants to {args.output}")

if __name__ == '__main__':
    main()
