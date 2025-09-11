#!/usr/bin/env python3
"""Generate performance plot for README.
Produces benchmarks/performance.png showing baseline vs STOC (fixed δ) vs STOC autotune + theoretical curves.

Designed to scale optionally to large n (e.g. 1,000,000) while keeping CI runs bounded via --ci-max-n.
"""
import math, random, time, statistics, argparse, json, os
from pathlib import Path
import sys
ROOT=Path(__file__).resolve().parent.parent
sys.path.append(str(ROOT/'implementations'/'python'))
import rust_sssp  # type: ignore

def gen_graph(n,density,seed):
    rnd=random.Random(seed)
    m=int(density*n)
    adj=[[] for _ in range(n)]
    for _ in range(m):
        u=rnd.randrange(n); v=rnd.randrange(n)
        if u==v: continue
        w=rnd.random()*9+1
        adj[u].append((v,w))
    offs=[0]; tg=[]; wt=[]
    for u in range(n):
        for v,w in adj[u]:
            tg.append(v); wt.append(w)
        offs.append(len(tg))
    return offs,tg,wt

def measure(offs,tg,wt,mode, autotune_env=None):
    t0=time.perf_counter()
    if mode=='baseline':
        rust_sssp.run_baseline(offs,tg,wt,0)
    elif mode=='stoc':
        rust_sssp.run_stoc(offs,tg,wt,0)
    elif mode=='stoc_autotune':
        backup={}
        if autotune_env:
            for k,v in autotune_env.items():
                backup[k]=os.environ.get(k)
                os.environ[k]=v
        try:
            rust_sssp.run_stoc_autotune(offs,tg,wt,0)
        finally:
            if autotune_env:
                for k,old in backup.items():
                    if old is None:
                        os.environ.pop(k, None)
                    else:
                        os.environ[k]=old
    else:
        raise ValueError(mode)
    return time.perf_counter()-t0

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--sizes',default='2000,4000,8000,16000,32000,64000')
    ap.add_argument('--density',type=float,default=2.0)
    ap.add_argument('--repeat',type=int,default=2)
    ap.add_argument('--output',default='benchmarks/performance.png')
    ap.add_argument('--no-autotune',action='store_true',help='Skip autotune series')
    ap.add_argument('--autotune-set',default='0.5,1,2,4',help='Comma list passed to SSSP_STOC_AUTOTUNE_SET')
    ap.add_argument('--autotune-limit',default='20000',help='Value for SSSP_STOC_AUTOTUNE_LIMIT (truncation)')
    ap.add_argument('--ci-max-n',type=int,default=128000,help='Cap max n when CI env detected')
    ap.add_argument('--annotate-heavy',action='store_true',help='Annotate heavy edge ratio (STOC) vs n on secondary axis')
    args=ap.parse_args()
    try:
        import matplotlib.pyplot as plt
    except ImportError:
        print('matplotlib required')
        return
    sizes=[int(s) for s in args.sizes.split(',') if s]
    if os.environ.get('CI'):
        sizes=[s for s in sizes if s <= args.ci_max_n]
        if not sizes:
            sizes=[args.ci_max_n]
    rows=[]
    do_autotune = (not args.no_autotune) and hasattr(rust_sssp,'run_stoc_autotune')
    autotune_env={'SSSP_STOC_AUTOTUNE_SET':args.autotune_set,'SSSP_STOC_AUTOTUNE_LIMIT':args.autotune_limit}
    for n in sizes:
        offs,tg,wt=gen_graph(n,args.density,12345)
        b=[measure(offs,tg,wt,'baseline') for _ in range(args.repeat)]
        s=[measure(offs,tg,wt,'stoc') for _ in range(args.repeat)]
        a=[]
        if do_autotune:
            # Autotune more expensive; single measurement usually fine, but honor repeat if >1.
            a=[measure(offs,tg,wt,'stoc_autotune',autotune_env=autotune_env) for _ in range(max(1,args.repeat))]
        bt=statistics.median(b); st=statistics.median(s); at=statistics.median(a) if a else None
        m=offs[-1]; logn=math.log(n)
        # Per-series stats for error bars
        row={'n':n,'m':m,'baseline_s':bt,'stoc_s':st,'stoc_autotune_s':at,'baseline_samples':b,'stoc_samples':s,'autotune_samples':a,'mlogn':m*logn,'mlog23':m*(logn**(2/3))}
        # Attempt to pull bucket stats via FFI if available
        try:
            import rust_sssp as rs
            stats = rs.get_bucket_stats() if hasattr(rs,'get_bucket_stats') else None
            if stats:
                row['bucket_stats']=stats
            bh = rs.get_baseline_heap_stats() if hasattr(rs,'get_baseline_heap_stats') else None
            if bh:
                row['baseline_heap']=bh
        except Exception:
            pass
        rows.append(row)
        if at is not None:
            print(f"n={n} baseline {bt*1000:.2f}ms stoc {st*1000:.2f}ms stoc_autotune {at*1000:.2f}ms")
        else:
            print(f"n={n} baseline {bt*1000:.2f}ms stoc {st*1000:.2f}ms")
    # Normalize theoretical curves to first baseline point for visibility
    if rows:
        k_base=rows[0]['baseline_s']/rows[0]['mlogn']
        k_stoc=rows[0]['stoc_s']/rows[0]['mlog23']
    x=[r['n'] for r in rows]
    bcurve=[r['baseline_s'] for r in rows]
    scurve=[r['stoc_s'] for r in rows]
    acurve=[r['stoc_autotune_s'] for r in rows]
    theo_base=[k_base*r['mlogn'] for r in rows]
    theo_stoc=[k_stoc*r['mlog23'] for r in rows]
    fig, ax1 = plt.subplots(figsize=(8,5))
    # Error bars (std dev) for baseline and stoc
    def series_mean_std(key):
        means=[]; stds=[]
        for r in rows:
            samples=r.get(key+'_samples',[])
            if len(samples)>1:
                means.append(sum(samples)/len(samples))
                stds.append(statistics.pstdev(samples))
            elif len(samples)==1:
                means.append(samples[0]); stds.append(0.0)
            else:
                means.append(float('nan')); stds.append(0.0)
        return means,stds
    bmeans, bstds = series_mean_std('baseline')
    smeans, sstds = series_mean_std('stoc')
    ax1.errorbar(x,bmeans,yerr=bstds,fmt='o-',capsize=3,label='Baseline (mean±σ)')
    ax1.errorbar(x,smeans,yerr=sstds,fmt='d-',capsize=3,label='STOC fixed δ (mean±σ)')
    if any(a is not None for a in acurve):
        # Only plot where we have data
        ax=[r['n'] for r in rows if r['stoc_autotune_s'] is not None]
        avy=[r['stoc_autotune_s'] for r in rows if r['stoc_autotune_s'] is not None]
        ax1.plot(ax,avy,'s-',label='STOC autotune (measured)')
    ax1.plot(x,theo_base,'--',label='c·m·log n (scaled)')
    ax1.plot(x,theo_stoc,'--',label='c·m·log^{2/3} n (scaled)')
    ax1.set_xscale('log'); ax1.set_yscale('log')
    ax1.set_xlabel('n (log scale)')
    ax1.set_ylabel('Time (s, log scale)')
    ax1.set_title('Baseline vs Delta-Stepping Scaling (with Autotune)')
    ax1.grid(alpha=0.3, which='both')

    if args.annotate_heavy:
        # Secondary axis for heavy ratio (linear scale)
        ax2 = ax1.twinx()
        hx=[]; hy=[]
        for r in rows:
            bs=r.get('bucket_stats')
            if bs and 'heavy_ratio' in bs and bs['heavy_ratio']>0:
                hx.append(r['n']); hy.append(bs['heavy_ratio'])
        if hx:
            ax2.plot(hx,hy,'^-',color='tab:red',label='Heavy ratio')
            ax2.set_ylabel('Heavy edge ratio')
            ax2.set_ylim(0,1.0)
            # Combine legends
            lines1, labels1 = ax1.get_legend_handles_labels()
            lines2, labels2 = ax2.get_legend_handles_labels()
            ax1.legend(lines1+lines2, labels1+labels2, loc='best')
        else:
            ax1.legend(loc='best')
    else:
        ax1.legend(loc='best')
    out_path=Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fig.tight_layout(); fig.savefig(out_path,dpi=140)
    # Alternative normalization appended to JSON (baseline heap metrics)
    for r in rows:
        bh = r.get('baseline_heap')
        if bh:
            pushes = bh['pushes']; pops = bh['pops']; maxh = max(1, bh['max_size'])
            denom = max(1, (pushes + pops))
            r['baseline_time_per_op'] = r['baseline_s'] / denom
            lg = math.log2(maxh)
            if lg <= 0:
                r['baseline_norm_heap'] = None
            else:
                r['baseline_norm_heap'] = r['baseline_s'] / (denom * lg)
    with open('benchmarks/performance_data.json','w') as f:
        json.dump(rows,f,indent=2)
    print(f"Wrote {args.output}")

if __name__=='__main__':
    main()
