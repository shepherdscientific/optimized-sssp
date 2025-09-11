import os, random, sys, ctypes
from pathlib import Path
sys.path.append(str(Path(__file__).resolve().parent))
import rust_sssp
from ctypes import Structure, c_uint32, byref

class BucketStats(Structure):
    _fields_=[('buckets_visited',c_uint32),('light_pass_repeats',c_uint32),('max_bucket_index',c_uint32),('restarts',c_uint32),('delta_x1000',c_uint32),('heavy_ratio_x1000',c_uint32)]

HAS_STATS = hasattr(rust_sssp._lib,'sssp_get_bucket_stats')
if HAS_STATS:
    rust_sssp._lib.sssp_get_bucket_stats.argtypes=[ctypes.POINTER(BucketStats)]  # type: ignore

def gen_graph(n,density,seed=1234):
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

def test_heavy_ratio_band():
    if not HAS_STATS:
        print('skip: stats not available')
        return
    # environment band (default 0.05-0.25)
    n=20000; density=2.5
    offs,tg,wt=gen_graph(n,density)
    rust_sssp.run_stoc(offs,tg,wt,0)
    bs=BucketStats(); rust_sssp._lib.sssp_get_bucket_stats(byref(bs))
    heavy = bs.heavy_ratio_x1000 / 1000.0
    assert heavy > 0.0, f"heavy ratio zero (delta likely too large): {heavy}" 
    assert 0.0 < heavy <= 0.95, f"heavy ratio out of plausible bounds: {heavy}" 
    print(f"Heavy ratio OK: {heavy:.3f} restarts={bs.restarts} delta={(bs.delta_x1000/1000.0):.4f}")

if __name__=='__main__':
    test_heavy_ratio_band()
