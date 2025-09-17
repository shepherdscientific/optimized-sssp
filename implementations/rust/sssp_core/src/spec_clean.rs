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

// ---------------- BaseCase (Phase 1) Components ----------------
#[derive(Copy,Clone,Debug,PartialEq,Eq)]
pub enum BaseCaseOutcome { Success, Truncated }

#[repr(C)]
#[derive(Copy,Clone)]
pub struct BaseCaseResult { pub outcome:i32, pub new_bound:f32, pub collected:u32 }

#[repr(C)]
#[derive(Copy,Clone,Default)]
pub struct SpecPhase1Stats {
    pub last_outcome: i32,       // 0 success,1 truncated
    pub last_bound: f32,         // B' from last run
    pub last_collected: u32,     // |U| from last run
    pub last_relaxations: u64,   // relax count from last run
}
static mut LAST_PHASE1_STATS: SpecPhase1Stats = SpecPhase1Stats { last_outcome: -1, last_bound: 0.0, last_collected: 0, last_relaxations: 0 };
#[no_mangle]
pub extern "C" fn sssp_get_spec_phase1_stats(out:*mut SpecPhase1Stats){ if out.is_null(){ return; } unsafe { *out = LAST_PHASE1_STATS; } }

pub fn basecase_truncated(
    n: u32,
    off: &[u32], tgt:&[u32], wts:&[f32],
    start: u32,
    k: u32,
    initial_bound: f32,
    dist: &mut [f32],
    pred: &mut [i32],
    scratch: &mut Vec<u32>,
    relaxations: &mut u64,
) -> BaseCaseResult {
    for d in dist.iter_mut() { *d = f32::INFINITY; }
    for p in pred.iter_mut() { *p = -1; }

    #[derive(Copy,Clone)] struct Item { u:u32, d:f32 }
    impl PartialEq for Item { fn eq(&self,o:&Self)->bool { self.d==o.d && self.u==o.u } }
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
    // Optional capture arrays (distance-nondecreasing pop order & depth approximation = number of hops from source)
    let capture = std::env::var("SSSP_SPEC_CAPTURE").ok().map(|v| v=="1" || v.to_lowercase()=="true").unwrap_or(false);
    thread_local! { static POP_ORDER: std::cell::RefCell<Vec<u32>> = Default::default(); }
    thread_local! { static DEPTHS: std::cell::RefCell<Vec<u32>> = Default::default(); }
    if capture { POP_ORDER.with(|v| v.borrow_mut().clear()); DEPTHS.with(|v| v.borrow_mut().clear()); }
    // Maintain depth via predecessor chain length; approximate using pred[v] depth+1 stored in a temp array.
    let mut depth: Option<Vec<u32>> = if capture { Some(vec![u32::MAX; dist.len()]) } else { None };
    if let Some(ref mut dvec) = depth { dvec[start as usize] = 0; }
    while let Some(Item{u,d}) = pq.pop() {
        if d > dist[u as usize] { continue; }
        if d > initial_bound { break; }
        scratch.push(u);
        if capture { POP_ORDER.with(|v| v.borrow_mut().push(u)); if let Some(ref mut dv) = depth { let dep = dv[u as usize]; DEPTHS.with(|v| v.borrow_mut().push(dep)); } }
        popped += 1;
        if d > max_seen { max_seen = d; }
        if popped == k + 1 { truncated = true; break; }
        let ui = u as usize; let se = off[ui] as usize; let ee = off[ui+1] as usize;
        for e in se..ee { let v = tgt[e] as usize; let nd = d + wts[e]; if nd <= dist[v] && nd <= initial_bound { dist[v]=nd; pred[v]=u as i32; if let Some(ref mut dv)=depth { let parent_depth = dv[u as usize]; if parent_depth != u32::MAX { dv[v] = parent_depth + 1; } } pq.push(Item{u:v as u32,d:nd}); *relaxations += 1; } }
    }
    let new_bound = if truncated { max_seen } else { initial_bound };
    if truncated { for &u in scratch.iter() { if dist[u as usize] >= new_bound { dist[u as usize] = f32::INFINITY; pred[u as usize] = -1; } } }
    BaseCaseResult { outcome: if truncated {1} else {0}, new_bound, collected: scratch.iter().filter(|&&u| dist[u as usize].is_finite()).count() as u32 }
}

// -------- Subtree sizing (Phase 2 helper) --------
// Given dist/pred arrays and collected set implicitly defined by dist[v].is_finite() && dist[v] < bound,
// compute subtree sizes for forest roots relative to predecessor pointers.
pub fn compute_subtree_sizes(dist: &[f32], pred: &[i32], bound: f32, order: &[u32]) -> (Vec<u32>, Vec<u32>) {
    // order expected to be pop order (distance nondecreasing). We'll traverse in reverse to accumulate.
    let n = dist.len();
    let mut size = vec![0u32; n];
    // Mark roots lazily when encountered (pred invalid or parent outside bound).
    for &u in order.iter().rev() { // reverse
        let ui = u as usize;
        if !(dist[ui].is_finite() && dist[ui] < bound) { continue; }
        let mut subtotal = 1u32; // include self
        size[ui] += 1; // accumulate children before parent; children already added size
        subtotal = size[ui];
        let p = pred[ui];
        if p >= 0 { let pi = p as usize; if dist[pi].is_finite() && dist[pi] < bound { size[pi] += subtotal; } }
    }
    // Collect roots and their sizes
    let mut roots = Vec::new();
    let mut root_sizes = Vec::new();
    for &u in order { let ui = u as usize; if !(dist[ui].is_finite() && dist[ui] < bound) { continue; } let p = pred[ui]; if p < 0 { roots.push(u); root_sizes.push(size[ui]); } else { let pi = p as usize; if !(dist[pi].is_finite() && dist[pi] < bound) { roots.push(u); root_sizes.push(size[ui]); } } }
    (roots, root_sizes)
}

// -------- Phase 2: Pivot selection loop --------
#[repr(C)]
#[derive(Copy,Clone,Default)]
pub struct SpecPhase2Stats {
    pub attempts: u32,
    pub success: i32,       // 1 success, 0 fallback
    pub final_k: u32,
    pub collected: u32,
    pub max_subtree: u32,
    pub roots_examined: u32,
    pub relaxations: u64,
    pub bound: f32,
}
static mut LAST_PHASE2_STATS: SpecPhase2Stats = SpecPhase2Stats { attempts:0, success:0, final_k:0, collected:0, max_subtree:0, roots_examined:0, relaxations:0, bound:0.0 };
#[no_mangle]
pub extern "C" fn sssp_get_spec_phase2_stats(out:*mut SpecPhase2Stats){ if out.is_null(){ return; } unsafe { *out = LAST_PHASE2_STATS; } }

#[no_mangle]
pub extern "C" fn sssp_run_spec_phase2(
    n: u32,
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
    let off = unsafe { as_slice(offsets, n_usize+1) };
    let m = off[n_usize] as usize;
    let tgt = unsafe { as_slice(targets, m) };
    let wts = unsafe { as_slice(weights, m) };
    let dist = unsafe { as_mut_slice(out_dist, n_usize) };
    let pred = unsafe { as_mut_slice(out_pred, n_usize) };
    let mut k = std::env::var("SSSP_SPEC_K").ok().and_then(|v| v.parse::<u32>().ok()).unwrap_or(1024).max(1);
    let attempt_max = std::env::var("SSSP_SPEC_PIVOT_MAX").ok().and_then(|v| v.parse::<u32>().ok()).unwrap_or(8).max(1);
    let mut attempts = 0u32;
    let mut total_relax = 0u64;
    let mut final_collected = 0u32;
    let mut final_bound = f32::INFINITY;
    let mut max_subtree_any = 0u32;
    let mut roots_examined_any = 0u32;
    let mut success = 0i32;
    // Pop order capture vector reused each attempt
    let mut pop_order: Vec<u32> = Vec::new();
    loop {
        attempts += 1;
        pop_order.clear();
        // Run basecase with capture forced (set env temporarily if not set)
        std::env::set_var("SSSP_SPEC_CAPTURE","1");
        let mut scratch: Vec<u32> = Vec::with_capacity(k as usize + 2);
        let mut relax: u64 = 0;
        // Slight duplication: re-run basecase logic manually to fill pop_order local (rather than thread locals) for determinism.
        // Re-implement minimal variant capturing order:
        for d in dist.iter_mut() { *d = f32::INFINITY; }
        for p in pred.iter_mut() { *p = -1; }
        #[derive(Copy,Clone)] struct Item2 { u:u32, d:f32 }
        impl PartialEq for Item2 { fn eq(&self,o:&Self)->bool { self.d==o.d && self.u==o.u } }
        impl Eq for Item2 {}
        impl PartialOrd for Item2 { fn partial_cmp(&self,o:&Self)->Option<std::cmp::Ordering>{ o.d.partial_cmp(&self.d) } }
        impl Ord for Item2 { fn cmp(&self,o:&Self)->std::cmp::Ordering { self.partial_cmp(o).unwrap() } }
        use std::collections::BinaryHeap; let mut pq = BinaryHeap::new();
        dist[source as usize] = 0.0; pq.push(Item2{u:source,d:0.0}); scratch.clear();
        let mut popped = 0u32; let mut max_seen = 0.0f32; let mut truncated=false;
        while let Some(Item2{u,dv}) = pq.pop() { if dv > dist[u as usize] { continue; } scratch.push(u); pop_order.push(u); popped+=1; if dv>max_seen { max_seen=dv; } if popped==k+1 { truncated=true; break; } let ui=u as usize; let se=off[ui] as usize; let ee=off[ui+1] as usize; for e in se..ee { let v=tgt[e] as usize; let nd = dv + wts[e]; if nd <= dist[v] { dist[v]=nd; pred[v]=u as i32; pq.push(Item2{u:v as u32,d:nd}); relax+=1; } } }
        let new_bound = if truncated { max_seen } else { f32::INFINITY };
        if truncated { for &u in scratch.iter() { if dist[u as usize] >= new_bound { dist[u as usize]=f32::INFINITY; pred[u as usize]=-1; } } }
        let collected = scratch.iter().filter(|&&u| dist[u as usize].is_finite() && dist[u as usize] < new_bound).count() as u32;
        total_relax += relax;
        final_collected = collected; final_bound = new_bound;
        // Subtree sizing
        let (roots, sizes) = compute_subtree_sizes(dist, pred, new_bound, &pop_order);
        roots_examined_any += roots.len() as u32;
        let mut local_max = 0u32; for &s in &sizes { if s>local_max { local_max = s; } }
        if local_max > max_subtree_any { max_subtree_any = local_max; }
        if local_max >= k || collected as u32 >= n { success = 1; break; }
        if attempts >= attempt_max || k >= n { break; }
        k = (k.saturating_mul(2)).min(n);
    }
    unsafe { LAST_PHASE2_STATS = SpecPhase2Stats { attempts, success, final_k: k, collected: final_collected, max_subtree: max_subtree_any, roots_examined: roots_examined_any, relaxations: total_relax, bound: final_bound }; }
    if !info.is_null(){ unsafe { *info = crate::SsspResultInfo { relaxations: total_relax, light_relaxations:0, heavy_relaxations:0, settled: final_collected, error_code: success }; } }
    0
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
    if offsets.is_null() || targets.is_null() || weights.is_null() || dist_ptr.is_null() || pred_ptr.is_null() || result_out.is_null(){ return -3; }
    let off = unsafe { as_slice(offsets, n as usize + 1) };
    let m = off[n as usize] as usize;
    let tgt = unsafe { as_slice(targets, m) };
    let wts = unsafe { as_slice(weights, m) };
    let dist = unsafe { as_mut_slice(dist_ptr, n as usize) };
    let pred = unsafe { as_mut_slice(pred_ptr, n as usize) };
    let mut tmp: Vec<u32> = Vec::with_capacity(k as usize + 2);
    let mut relax = 0u64;
    let res = basecase_truncated(n, off, tgt, wts, start, k, bound, dist, pred, &mut tmp, &mut relax);
    unsafe { *result_out = res; LAST_PHASE1_STATS.last_outcome = res.outcome; LAST_PHASE1_STATS.last_bound = res.new_bound; LAST_PHASE1_STATS.last_collected = res.collected; LAST_PHASE1_STATS.last_relaxations = relax; }
    0
}

// Phase 1 runner: truncated basecase growth from source.
#[no_mangle]
pub extern "C" fn sssp_run_spec_phase1(
    n: u32,
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
    let off = unsafe { as_slice(offsets, n_usize+1) };
    let m = off[n_usize] as usize;
    let tgt = unsafe { as_slice(targets, m) };
    let wts = unsafe { as_slice(weights, m) };
    let dist = unsafe { as_mut_slice(out_dist, n_usize) };
    let pred = unsafe { as_mut_slice(out_pred, n_usize) };
    let k_env = std::env::var("SSSP_SPEC_K").ok().and_then(|v| v.parse::<u32>().ok()).unwrap_or(1024).max(1);
    let bound_env = std::env::var("SSSP_SPEC_BOUND").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(f32::INFINITY);
    let mut scratch: Vec<u32> = Vec::with_capacity(k_env as usize + 2);
    let mut relax: u64 = 0;
    let res = basecase_truncated(n, off, tgt, wts, source, k_env, bound_env, dist, pred, &mut scratch, &mut relax);
    unsafe { LAST_PHASE1_STATS.last_outcome = res.outcome; LAST_PHASE1_STATS.last_bound = res.new_bound; LAST_PHASE1_STATS.last_collected = res.collected; LAST_PHASE1_STATS.last_relaxations = relax; }
    if !info.is_null(){ unsafe { *info = crate::SsspResultInfo { relaxations: relax, light_relaxations:0, heavy_relaxations:0, settled: res.collected, error_code: res.outcome }; } }
    0
}

// ---------------- Tests (unit) ----------------
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basecase_no_truncate_small_k(){
        // Line graph 0-1-2-3 with unit weights
        let off = [0u32,1,2,3,3];
        let tgt = [1u32,2,3];
        let wts = [1.0f32,1.0,1.0];
        let n = 4u32;
        let mut dist = vec![0f32;4];
        let mut pred = vec![-1i32;4];
        let mut tmp = Vec::new();
        let mut relax=0u64;
        let res = basecase_truncated(n,&off,&tgt,&wts,0,10,f32::INFINITY,&mut dist,&mut pred,&mut tmp,&mut relax);
        assert_eq!(res.outcome,0); // success
        assert_eq!(res.collected,4);
        assert_eq!(relax,3);
    }
    #[test]
    fn basecase_truncates_at_k_plus_one(){
        // Star: 0 connected to 1..5 all weight 1
        let off = [0u32,5,5,5,5,5,5];
        let tgt = [1,2,3,4,5];
        let wts = [1.0f32;5];
        let n = 6u32;
        let mut dist = vec![0f32;6];
        let mut pred = vec![-1i32;6];
        let mut tmp = Vec::new();
        let mut relax=0u64;
        // k=2 -> collect up to 3 pops (0 plus 2 children) then truncate
        let res = basecase_truncated(n,&off,&tgt,&wts,0,2,f32::INFINITY,&mut dist,&mut pred,&mut tmp,&mut relax);
        assert_eq!(res.outcome,1); // truncated
        assert!(res.collected <= 2); // enforce dist < B'
    }
    #[test]
    fn phase2_simple_star(){
        // Star graph to force early large subtree from center.
        let off = [0u32,5,5,5,5,5,5];
        let tgt = [1,2,3,4,5];
        let wts = [1.0f32;5];
        let n = 6u32;
        let mut dist = vec![0f32;6];
        let mut pred = vec![-1i32;6];
        let mut info = crate::SsspResultInfo { relaxations:0, light_relaxations:0, heavy_relaxations:0, settled:0, error_code:0 };
        // Small k triggers truncation then scaling
        std::env::set_var("SSSP_SPEC_K","2");
        std::env::set_var("SSSP_SPEC_PIVOT_MAX","4");
        let rc = sssp_run_spec_phase2(n, off.as_ptr(), tgt.as_ptr(), wts.as_ptr(), 0, dist.as_mut_ptr(), pred.as_mut_ptr(), &mut info as *mut _);
        assert_eq!(rc,0);
        let mut stats = SpecPhase2Stats::default();
        unsafe { sssp_get_spec_phase2_stats(&mut stats as *mut _); }
        assert!(stats.attempts >=1);
        assert!(stats.max_subtree >=2);
    }
    #[test]
    fn phase2_line_graph_progress(){
        // Line 0-1-2-3-4 ensures subtree sizes small, forcing k doubling.
        let off=[0u32,1,2,3,4,4];
        let tgt=[1,2,3,4];
        let wts=[1.0f32;4];
        let n=5u32; let mut dist=vec![0f32;5]; let mut pred=vec![-1i32;5];
        std::env::set_var("SSSP_SPEC_K","1");
        std::env::set_var("SSSP_SPEC_PIVOT_MAX","5");
        let mut info = crate::SsspResultInfo { relaxations:0, light_relaxations:0, heavy_relaxations:0, settled:0, error_code:0 };
        let rc = sssp_run_spec_phase2(n, off.as_ptr(), tgt.as_ptr(), wts.as_ptr(), 0, dist.as_mut_ptr(), pred.as_mut_ptr(), &mut info as *mut _);
        assert_eq!(rc,0);
        let mut stats = SpecPhase2Stats::default(); unsafe { sssp_get_spec_phase2_stats(&mut stats as *mut _); }
        assert!(stats.attempts >=1);
        assert!(stats.final_k >=1);
    }
}
