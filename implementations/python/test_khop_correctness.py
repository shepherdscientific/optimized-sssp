import random, math
from rust_sssp import run_baseline, run_khop

# Simple random graph generator (reuse logic similar to benchmark script)

def generate_graph(n: int, density: float, seed: int):
    rng = random.Random(seed)
    target_edges = int(density * n)
    adj = [[] for _ in range(n)]
    for _ in range(target_edges):
        u = rng.randrange(n); v = rng.randrange(n)
        if u == v: continue
        w = rng.uniform(1.0, 10.0)
        adj[u].append((v,w))
    offsets=[0]; targets=[]; weights=[]
    for u in range(n):
        if adj[u]: adj[u].sort(key=lambda x: x[0])
        for (v,w) in adj[u]:
            targets.append(v); weights.append(w)
        offsets.append(len(targets))
    return offsets, targets, weights


def check_graph(n, density, seed, eps=1e-4):
    offs, tgts, wts = generate_graph(n, density, seed)
    src=0
    dist_b, _, _ = run_baseline(offs, tgts, wts, src)
    dist_k, _, _ = run_khop(offs, tgts, wts, src)
    # Distances can be inf (represented by large float) if unreachable; compare only finite ones.
    mismatches=0
    for i,(db,dk) in enumerate(zip(dist_b, dist_k)):
        if math.isinf(db) and math.isinf(dk):
            continue
        if abs(db-dk) > eps:
            mismatches += 1
    return mismatches


def main():
    # Small to moderate sizes
    tests = [ (n,2.0) for n in [50,100,200,400] ]
    seeds = list(range(5))
    total=0
    for (n,d) in tests:
        for s in seeds:
            mm = check_graph(n,d,s)
            if mm>0:
                print(f"FAIL n={n} seed={s} mismatches={mm}")
                total += 1
            else:
                print(f"OK   n={n} seed={s}")
    if total>0:
        raise SystemExit(f"k-hop correctness mismatches in {total} test graphs")
    print("All k-hop tests passed vs baseline within tolerance.")

if __name__ == '__main__':
    try:
        main()
    except RuntimeError as e:
        # If k-hop not available, skip
        if 'k-hop' in str(e):
            print('Skipping k-hop tests: function not available')
        else:
            raise
