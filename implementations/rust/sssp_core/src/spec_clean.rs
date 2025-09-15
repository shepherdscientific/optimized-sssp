//! Clean restart scaffold for BMSSP (spec) implementation.
//! Phase 1: correctness parity with baseline Dijkstra using simple multi-level shell.
//! Later phases will introduce pivot selection, interval refinement, and batching.

use core::slice;
use std::cmp::Ordering;
#[repr(C)]
#[derive(Copy,Clone)]
pub struct SpecHeapStats { pub pushes:u64, pub pops:u64, pub max_size:u64 }
static mut LAST_SPEC_HEAP_STATS: SpecHeapStats = SpecHeapStats { pushes:0, pops:0, max_size:0 };
#[no_mangle]
pub extern "C" fn sssp_get_spec_heap_stats(out:*mut SpecHeapStats){ if out.is_null(){ return; } unsafe{ *out = LAST_SPEC_HEAP_STATS; } }

#[inline(always)] fn as_slice<'a, T>(ptr:*const T, len:usize)->&'a [T]{ unsafe{ slice::from_raw_parts(ptr,len) } }
#[inline(always)] fn as_mut_slice<'a, T>(ptr:*mut T, len:usize)->&'a mut [T]{ unsafe{ slice::from_raw_parts_mut(ptr,len) } }

#[derive(Copy,Clone)] struct H{d:f32,v:u32}
impl PartialEq for H { fn eq(&self,o:&Self)->bool{ self.d==o.d && self.v==o.v } }
impl Eq for H {}
impl Ord for H { fn cmp(&self,o:&Self)->Ordering { // reverse for min-heap semantics in BinaryHeap-like ordering
    if self.d<o.d { Ordering::Greater } else if self.d>o.d { Ordering::Less } else { Ordering::Equal }
} }
impl PartialOrd for H { fn partial_cmp(&self,o:&Self)->Option<Ordering>{ Some(self.cmp(o)) } }

// Lightweight custom min-heap (binary heap) with explicit sift ops (mirrors baseline style)
struct MinHeap{data:Vec<H>, pushes:u64, pops:u64, max_size:u64}
impl MinHeap{
    #[inline] fn with_cap(c:usize)->Self{ Self{ data:Vec::with_capacity(c), pushes:0, pops:0, max_size:0 } }
    #[inline] fn push(&mut self, h:H){ self.data.push(h); self.pushes+=1; if self.data.len() as u64> self.max_size { self.max_size = self.data.len() as u64; } self.sift_up(self.data.len()-1); }
    #[inline] fn pop(&mut self)->Option<H>{ let n=self.data.len(); if n==0 {return None;} self.data.swap(0,n-1); let out=self.data.pop(); self.pops+=1; if !self.data.is_empty(){ self.sift_down(0);} out }
    #[inline] fn sift_up(&mut self, mut i:usize){ while i>0 { let p=(i-1)/2; if self.data[i].d < self.data[p].d { self.data.swap(i,p); i=p;} else { break; } } }
    #[inline] fn sift_down(&mut self, mut i:usize){ let n=self.data.len(); loop { let l=i*2+1; if l>=n { break; } let r=l+1; let mut b=l; if r<n && self.data[r].d < self.data[l].d { b=r; } if self.data[b].d < self.data[i].d { self.data.swap(i,b); i=b; } else { break; } } }
}

// Baseline Dijkstra (used inside spec for now)
fn dijkstra(off:&[u32], tgt:&[u32], wts:&[f32], dist:&mut [f32], mut pred:Option<&mut [i32]>, src:u32, relaxations:&mut u64){
    for d in dist.iter_mut(){ *d = f32::INFINITY; }
    if let Some(p) = pred.as_ref(){ for _ in p.iter() { /* touch for potential prefetch */ } }
    if let Some(p) = pred.as_mut(){ for v in p.iter_mut(){ *v = -1; } }
    dist[src as usize] = 0.0;
    let mut pq = MinHeap::with_cap(dist.len().min(1024));
    pq.push(H{d:0.0,v:src});
    while let Some(H{d,v}) = pq.pop(){
        if d>unsafe{ *dist.get_unchecked(v as usize) } { continue; }
        let u = v as usize; let s = off[u] as usize; let e = off[u+1] as usize; let base = d;
        for idx in s..e { let wv = tgt[idx] as usize; let nd = base + wts[idx]; let cur = unsafe{ *dist.get_unchecked(wv) }; if nd < cur { unsafe{ *dist.get_unchecked_mut(wv)=nd; } if let Some(p)=pred.as_mut(){ unsafe{ *p.get_unchecked_mut(wv)=u as i32; } } *relaxations+=1; pq.push(H{d:nd,v:wv as u32}); } }
    }
    unsafe { LAST_SPEC_HEAP_STATS = SpecHeapStats { pushes: pq.pushes, pops: pq.pops, max_size: pq.max_size }; }
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
    if offsets.is_null() || targets.is_null() || weights.is_null() || out_dist.is_null(){ return -3; }
    let n_usize = n as usize;
    let off = as_slice(offsets, n_usize+1);
    let m = *off.last().unwrap() as usize;
    let tgt = as_slice(targets, m);
    let wts = as_slice(weights, m);
    let dist = as_mut_slice(out_dist, n_usize);
    let pred_opt = if out_pred.is_null() { None } else { Some(as_mut_slice(out_pred, n_usize)) };
    let mut relax:u64=0;
    dijkstra(off, tgt, wts, dist, pred_opt, source, &mut relax);
    if !info.is_null(){ unsafe { *info = crate::SsspResultInfo { relaxations: relax, light_relaxations:0, heavy_relaxations:0, settled: n, error_code:0 }; } }
    0
}

#[derive(Copy,Clone,Debug,PartialEq,Eq)]
pub enum BaseCaseOutcome {
    Success,      // collected < k+1 vertices (no truncation)
    Truncated,    // hit k+1 limit; distances ≥ new_bound are excluded
}

#[repr(C)]
pub struct BaseCaseResult {
    pub outcome: i32,      // 0=Success, 1=Truncated
    pub new_bound: f32,    // B' (if truncated B' = max_dist_in_prefix, else original bound)
    pub collected: u32,    // |U|
}

pub fn basecase_truncated(
    n: u32,
    off: &[u32], tgt:&[u32], wts:&[f32],
    start: u32,
    k: u32,
    initial_bound: f32,
    dist: &mut [f32],
    pred: &mut [i32],
    scratch: &mut Vec<u32>,
) -> BaseCaseResult {
    // Reset (caller may reuse arrays)
    for d in dist.iter_mut() { *d = f32::INFINITY; }
    for p in pred.iter_mut() { *p = -1; }

    // Simple binary-heap (reuse spec_clean’s heap H if desired; inline minimal here)
    #[derive(Copy,Clone)] struct Item { u:u32, d:f32 }
    impl PartialEq for Item { fn eq(&self, o:&Self)->bool { self.d == o.d && self.u==o.u } }
    impl Eq for Item {}
    impl PartialOrd for Item { fn partial_cmp(&self,o:&Self)->Option<std::cmp::Ordering>{ o.d.partial_cmp(&self.d) } }
    impl Ord for Item { fn cmp(&self,o:&Self)->std::cmp::Ordering { self.partial_cmp(o).unwrap() } }

    use std::collections::BinaryHeap;
    let mut pq = BinaryHeap::new();
    dist[start as usize] = 0.0;
    pq.push(Item{u:start,d:0.0});
    scratch.clear();

    let mut popped = 0u32;
    let mut max_seen = 0.0f32;
    let mut truncated = false;

    while let Some(Item{u,d}) = pq.pop() {
        if d > dist[u as usize] { continue; }
        if d > initial_bound { break; } // respect incoming bound B
        scratch.push(u);
        popped += 1;
        if d > max_seen { max_seen = d; }
        if popped == k + 1 {
            truncated = true;
            break;
        }
        // Relax
        let ui = u as usize;
        let start_e = off[ui] as usize;
        let end_e = off[ui+1] as usize;
        for e in start_e..end_e {
            let v = tgt[e] as usize;
            let nd = d + wts[e];
            if nd <= dist[v] && nd <= initial_bound {
                dist[v] = nd;
                pred[v] = u as i32;
                pq.push(Item{u: v as u32, d: nd});
            }
        }
    }

    // If truncated: new bound B' = max distance among collected excluding the last overflow rule:
    let new_bound = if truncated { max_seen } else { initial_bound };

    // If truncated, enforce U = { v : dist[v] < B' }
    if truncated {
        for &u in scratch.iter() {
            if dist[u as usize] >= new_bound {
                dist[u as usize] = f32::INFINITY;
                pred[u as usize] = -1;
            }
        }
    }

    BaseCaseResult {
        outcome: if truncated { 1 } else { 0 },
        new_bound,
        collected: scratch.iter().filter(|&&u| dist[u as usize].is_finite()).count() as u32,
    }
}

#[no_mangle]
pub extern "C" fn sssp_spec_basecase_probe(
    n: u32,
    offsets: *const u32,
    targets: *const u32,
    weights: *const f32,
    start: u32,
    k: u32,
    bound: f32,
    dist_ptr: *mut f32,
    pred_ptr: *mut i32,
    result_out: *mut BaseCaseResult,
) -> i32 {
    let off = unsafe { as_slice(offsets, n as usize + 1) };
    let m = off[n as usize] as usize;
    let tgt = unsafe { as_slice(targets, m) };
    let wts = unsafe { as_slice(weights, m) };
    let dist = unsafe { as_mut_slice(dist_ptr, n as usize) };
    let pred = unsafe { as_mut_slice(pred_ptr, n as usize) };
    let mut tmp: Vec<u32> = Vec::with_capacity(k as usize + 2);
    let res = basecase_truncated(n, off, tgt, wts, start, k, bound, dist, pred, &mut tmp);
    unsafe { *result_out = res; }
    0
}
