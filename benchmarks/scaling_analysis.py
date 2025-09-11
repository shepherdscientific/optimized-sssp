#!/usr/bin/env python3
"""Empirical scaling analysis for baseline vs STOC.
Generates synthetic graphs increasing n, fixed density d, measures m=|E| and time.
Fits time against: m * log(n)**(2/3) (STOC target) vs m * log(n) (baseline expected) using linear regression of constant factors.
Outputs JSON + printed ratios.
"""
import math, time, random, argparse, json, statistics, sys, os
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent
sys.path.append(str(ROOT / 'implementations' / 'python'))
import rust_sssp  # type: ignore

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

def measure(offsets,targets,weights,mode):
    src=0
    t0=time.perf_counter()
    if mode=='baseline':
        rust_sssp.run_baseline(offsets,targets,weights,src)
    else:
        rust_sssp.run_stoc(offsets,targets,weights,src)
    return (time.perf_counter()-t0)

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--sizes',default='2000,4000,8000,16000,32000')
    ap.add_argument('--density',type=float,default=2.0)
    ap.add_argument('--repeat',type=int,default=3)
    ap.add_argument('--seed',type=int,default=12345)
    ap.add_argument('--output',default='scaling_results.json')
    args=ap.parse_args()
    sizes=[int(s) for s in args.sizes.split(',') if s]
    rows=[]
    for n in sizes:
        offsets,targets,weights=generate_graph(n,args.density,args.seed)
        m=offsets[-1]
        base_times=[measure(offsets,targets,weights,'baseline') for _ in range(args.repeat)]
        stoc_times=[measure(offsets,targets,weights,'stoc') for _ in range(args.repeat)]
        bt=statistics.median(base_times); st=statistics.median(stoc_times)
        logn=math.log(n)
        metric_log= m*logn
        metric_stoc = m*(logn**(2/3))
        rows.append({
            'n':n,'m':m,'baseline_time_s':bt,'stoc_time_s':st,
            'm_log_n':metric_log,'m_log23_n':metric_stoc,
            'baseline_time_per_mlogn': bt/metric_log if metric_log>0 else None,
            'stoc_time_per_mlog23': st/metric_stoc if metric_stoc>0 else None
        })
        print(f"n={n} m={m} baseline={bt*1000:.2f}ms stoc={st*1000:.2f}ms")
    with open(args.output,'w') as f: json.dump(rows,f,indent=2)
    # Simple factor stability summary
    print('\nFactor stability:')
    b_factors=[r['baseline_time_per_mlogn'] for r in rows if r['baseline_time_per_mlogn']]
    s_factors=[r['stoc_time_per_mlog23'] for r in rows if r['stoc_time_per_mlog23']]
    if b_factors: print('baseline factor median',statistics.median(b_factors),'ratio max/min',max(b_factors)/min(b_factors))
    if s_factors: print('stoc factor median',statistics.median(s_factors),'ratio max/min',max(s_factors)/min(s_factors))

if __name__=='__main__':
    main()
