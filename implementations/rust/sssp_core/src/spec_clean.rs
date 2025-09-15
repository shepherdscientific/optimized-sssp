//! Clean restart scaffold for BMSSP (spec) implementation.
//! Phase 1: correctness parity with baseline Dijkstra using simple multi-level shell.
//! Later phases will introduce pivot selection, interval refinement, and batching.

use core::slice;
use std::cmp::Ordering;

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
struct MinHeap{data:Vec<H>}
impl MinHeap{
    #[inline] fn with_cap(c:usize)->Self{ Self{ data:Vec::with_capacity(c) } }
    #[inline] fn push(&mut self, h:H){ self.data.push(h); self.sift_up(self.data.len()-1); }
    #[inline] fn pop(&mut self)->Option<H>{ let n=self.data.len(); if n==0 {return None;} self.data.swap(0,n-1); let out=self.data.pop(); if !self.data.is_empty(){ self.sift_down(0);} out }
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
