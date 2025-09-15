//! Clean restart scaffold for BMSSP (spec) implementation.
//! Phase 1: correctness parity with baseline Dijkstra using simple multi-level shell.
//! Later phases will introduce pivot selection, interval refinement, and batching.

use core::slice;
use std::collections::BinaryHeap;
use std::cmp::Ordering;

#[inline(always)] fn as_slice<'a, T>(ptr:*const T, len:usize)->&'a [T]{ unsafe{ slice::from_raw_parts(ptr,len) } }
#[inline(always)] fn as_mut_slice<'a, T>(ptr:*mut T, len:usize)->&'a mut [T]{ unsafe{ slice::from_raw_parts_mut(ptr,len) } }

#[derive(Copy,Clone)] struct H{d:f32,v:u32}
impl PartialEq for H { fn eq(&self,o:&Self)->bool{ self.d==o.d && self.v==o.v } }
impl Eq for H {}
impl Ord for H { fn cmp(&self,o:&Self)->Ordering { o.d.partial_cmp(&self.d).unwrap_or(Ordering::Equal) } }
impl PartialOrd for H { fn partial_cmp(&self,o:&Self)->Option<Ordering>{ Some(self.cmp(o)) } }

// Baseline Dijkstra (used inside spec for now)
fn dijkstra(off:&[u32], tgt:&[u32], wts:&[f32], dist:&mut [f32], pred:&mut [i32], src:u32, relaxations:&mut u64){
    let n = dist.len();
    for d in dist.iter_mut(){ *d = f32::INFINITY; }
    for p in pred.iter_mut(){ *p = -1; }
    dist[src as usize] = 0.0;
    let mut pq = BinaryHeap::new(); pq.push(H{d:0.0,v:src});
    while let Some(H{d,v}) = pq.pop(){ if d>dist[v as usize]{ continue; } let u=v as usize; let s = off[u] as usize; let e = off[u+1] as usize; for idx in s..e { let wv = tgt[idx] as usize; let nd = d + wts[idx]; if nd < dist[wv] { dist[wv]=nd; pred[wv]=u as i32; *relaxations+=1; pq.push(H{d:nd,v:wv as u32}); } } }
    // All nodes reachable settled; unreachable remain INF.
    if n>0 {}
}

// Placeholder BMSSP shell: currently just invokes Dijkstra once.
#[no_mangle]
pub extern "C" fn sssp_run_spec_clean(
    n:u32,
    offsets:*const u32,
    targets:*const u32,
    weights:*const f32,
    source:u32,
    out_dist:*mut f32,
    out_pred:*mut i32,
    info:*mut crate::SsspResultInfo,
) -> i32 {
    if n==0 { return -1; }
    if source>=n { return -2; }
    if offsets.is_null() || targets.is_null() || weights.is_null() || out_dist.is_null() || out_pred.is_null(){ return -3; }
    let n_usize = n as usize;
    let off = as_slice(offsets, n_usize+1);
    let m = *off.last().unwrap() as usize;
    let tgt = as_slice(targets, m);
    let wts = as_slice(weights, m);
    let dist = as_mut_slice(out_dist, n_usize);
    let pred = as_mut_slice(out_pred, n_usize);
    let mut relax:u64=0;
    dijkstra(off, tgt, wts, dist, pred, source, &mut relax);
    if !info.is_null(){ unsafe { *info = crate::SsspResultInfo { relaxations: relax, light_relaxations:0, heavy_relaxations:0, settled: n, error_code:0 }; } }
    0
}
