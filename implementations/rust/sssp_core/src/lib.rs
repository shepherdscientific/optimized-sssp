//! Core high-performance SSSP implementations exposed via a stable C ABI.
//! Provides two variants only:
//!  - Dijkstra: classic binary-heap (extern `sssp_run_baseline`)
//!  - STOC / delta-stepping style: (extern `sssp_run_stoc`)
//! All other experimental variants have been removed per simplification.

use core::slice;

#[repr(C)]
pub struct SsspResultInfo {
    pub relaxations: u64,          // total relax operations
    pub light_relaxations: u64,     // light-edge relaxations (delta-stepping)
    pub heavy_relaxations: u64,     // heavy-edge relaxations (delta-stepping)
    pub settled: u32,               // nodes settled (visited)
    pub error_code: i32,            // 0 == success
}

// Baseline heap instrumentation
#[repr(C)]
pub struct BaselineHeapStats { pub pushes: u64, pub pops: u64, pub max_size: u64 }
impl Copy for BaselineHeapStats {}
impl Clone for BaselineHeapStats { fn clone(&self) -> Self { *self } }
static mut LAST_BASELINE_HEAP_STATS: BaselineHeapStats = BaselineHeapStats { pushes:0, pops:0, max_size:0 };

// Additional global instrumentation for delta-stepping (light/heavy) to correlate scaling behavior.
// Updated on each STOC / autotune final full run.
#[repr(C)]
pub struct SsspBucketStats {
    pub buckets_visited: u32,       // number of non-empty bucket indices processed
    pub light_pass_repeats: u32,    // total light-phase repeat loops (sum over buckets)
    pub max_bucket_index: u32,      // highest bucket index allocated
    pub restarts: u32,              // adaptive restarts performed (delta adjustments)
    pub delta_x1000: u32,           // final delta * 1000 (for quick inspection)
    pub heavy_ratio_x1000: u32,     // (heavy_relax / total_relax) * 1000
}

impl Copy for SsspBucketStats {}
impl Clone for SsspBucketStats { fn clone(&self) -> Self { *self } }

static mut LAST_BUCKET_STATS: SsspBucketStats = SsspBucketStats { buckets_visited: 0, light_pass_repeats: 0, max_bucket_index: 0, restarts: 0, delta_x1000: 0, heavy_ratio_x1000: 0 };
static mut LAST_DELTA: f32 = 0.0;

#[no_mangle]
pub extern "C" fn sssp_get_bucket_stats(out: *mut SsspBucketStats) {
    if out.is_null() { return; }
    unsafe { *out = LAST_BUCKET_STATS; }
}

#[no_mangle]
pub extern "C" fn sssp_get_last_delta() -> f32 { unsafe { LAST_DELTA } }

#[no_mangle]
pub extern "C" fn sssp_get_baseline_heap_stats(out: *mut BaselineHeapStats) {
    if out.is_null() { return; }
    unsafe { *out = LAST_BASELINE_HEAP_STATS; }
}

#[inline(always)]
fn as_slice<'a, T>(ptr: *const T, len: usize) -> &'a [T] {
    unsafe { slice::from_raw_parts(ptr, len) }
}
#[inline(always)]
fn as_mut_slice<'a, T>(ptr: *mut T, len: usize) -> &'a mut [T] {
    unsafe { slice::from_raw_parts_mut(ptr, len) }
}

#[derive(Copy, Clone)]
struct HeapItem { node: u32, dist: f32 }

// ---------------- Baseline binary heap ----------------
struct BinaryHeapSimple { data: Vec<HeapItem> }
impl BinaryHeapSimple {
    #[inline] fn new(cap: usize) -> Self { Self { data: Vec::with_capacity(cap) } }
    #[inline] fn push(&mut self, item: HeapItem, pushes: &mut u64) { self.data.push(item); *pushes += 1; self.sift_up(self.data.len()-1); }
    #[inline] fn pop(&mut self, pops: &mut u64) -> Option<HeapItem> {
        let len = self.data.len();
        if len == 0 { return None; }
        self.data.swap(0, len-1);
        let out = self.data.pop();
        *pops += 1;
        if !self.data.is_empty() { self.sift_down(0); }
        out
    }
    #[inline] fn sift_up(&mut self, mut idx: usize) {
        while idx > 0 {
            let parent = (idx - 1) / 2;
            if self.data[idx].dist < self.data[parent].dist { self.data.swap(idx, parent); idx = parent; } else { break; }
        }
    }
    #[inline] fn sift_down(&mut self, mut idx: usize) {
        let n = self.data.len();
        loop {
            let left = idx * 2 + 1;
            if left >= n { break; }
            let right = left + 1;
            let mut best = left;
            if right < n && self.data[right].dist < self.data[left].dist { best = right; }
            if self.data[best].dist < self.data[idx].dist { self.data.swap(idx, best); idx = best; } else { break; }
        }
    }
}


#[no_mangle]
pub extern "C" fn sssp_run_baseline(
    n: u32,
    offsets: *const u32, // len n+1
    targets: *const u32, // len m
    weights: *const f32, // len m
    source: u32,
    out_dist: *mut f32,  // len n
    out_pred: *mut i32,  // len n
    info: *mut SsspResultInfo,
) -> i32 {
    if n == 0 { return -1; }
    if source >= n { return -2; }
    if offsets.is_null() || targets.is_null() || weights.is_null() || out_dist.is_null() || out_pred.is_null() { return -3; }

    // Safety: caller promises valid lengths. Derive m from offsets[n].
    let n_usize = n as usize;
    let off = as_slice(offsets, n_usize + 1);
    let m = match off.last() { Some(v) => *v as usize, None => return -4 };
    let tgt = as_slice(targets, m);
    let wts = as_slice(weights, m);
    let dist = as_mut_slice(out_dist, n_usize);
    let pred = as_mut_slice(out_pred, n_usize);

    // Init
    for d in dist.iter_mut() { *d = f32::INFINITY; }
    for p in pred.iter_mut() { *p = -1; }
    dist[source as usize] = 0.0;

    let mut heap = BinaryHeapSimple::new( (n as usize).min(1024) );
    let mut relaxations: u64 = 0;
    let light_relaxations: u64 = 0; // unused in baseline
    let heavy_relaxations: u64 = 0; // unused in baseline
    let mut heap_pushes: u64 = 0;
    let mut heap_pops: u64 = 0;
    let mut heap_max: u64 = 0;
    heap.push(HeapItem { node: source, dist: 0.0 }, &mut heap_pushes);
    heap_max = heap_max.max(heap.data.len() as u64);

    while let Some(item) = heap.pop(&mut heap_pops) {
        if item.dist > dist[item.node as usize] { continue; }
        let start = off[item.node as usize] as usize;
        let end = off[item.node as usize + 1] as usize;
        for e in start..end {
            let v = tgt[e] as usize;
            let w = wts[e];
            let nd = item.dist + w;
            let cur = dist[v];
            if nd < cur {
                dist[v] = nd;
                pred[v] = item.node as i32;
                heap.push(HeapItem { node: v as u32, dist: nd }, &mut heap_pushes);
                if heap.data.len() as u64 > heap_max { heap_max = heap.data.len() as u64; }
                relaxations += 1;
            }
        }
    }

    if !info.is_null() { unsafe { *info = SsspResultInfo { relaxations, light_relaxations, heavy_relaxations, settled: n, error_code: 0 }; } }
    unsafe { LAST_BASELINE_HEAP_STATS = BaselineHeapStats { pushes: heap_pushes, pops: heap_pops, max_size: heap_max }; }
    0
}

#[no_mangle]
pub extern "C" fn sssp_version() -> u32 { 4 } // incremented due to SsspResultInfo breaking change

// ---------------- STOC-inspired (delta-stepping style) variant ----------------
// This implements a simplified delta-stepping algorithm (Meyer & Sanders) often
// used as a practical foundation for layering / bucket approaches referenced in
// later theoretical STOC-style improvements. We expose it under the name
// `sssp_run_stoc` per user request, though it is the classical delta-stepping
// core (single-threaded here).
// Key idea: partition edges into light (w <= delta) and heavy (w > delta).
// Process buckets i in increasing order of floor(dist/delta). For each bucket:
//  1. Repeatedly settle nodes reachable via light edges within the bucket.
//  2. Afterwards relax heavy edges from those settled nodes, inserting targets
//     into future buckets. This reduces priority queue operations to simple
//     bucket insertions and batches many light-edge relaxations.
// Expected benefit appears on graphs with many small weights creating clusters
// per distance band; on random sparse graphs overhead may still dominate.
#[no_mangle]
pub extern "C" fn sssp_run_stoc(
    n: u32,
    offsets: *const u32,
    targets: *const u32,
    weights: *const f32,
    source: u32,
    out_dist: *mut f32,
    out_pred: *mut i32,
    info: *mut SsspResultInfo,
) -> i32 {
    if n == 0 { return -1; }
    if source >= n { return -2; }
    if offsets.is_null() || targets.is_null() || weights.is_null() || out_dist.is_null() || out_pred.is_null() { return -3; }

    let n_usize = n as usize;
    let off = as_slice(offsets, n_usize + 1);
    let m = match off.last() { Some(v) => *v as usize, None => return -4 };
    let tgt = as_slice(targets, m);
    let wts = as_slice(weights, m);
    let dist = as_mut_slice(out_dist, n_usize);
    let pred = as_mut_slice(out_pred, n_usize);

    for d in dist.iter_mut() { *d = f32::INFINITY; }
    for p in pred.iter_mut() { *p = -1; }
    dist[source as usize] = 0.0;

    // Delta selection strategies: "avg" (default) or "quantile".
    fn sample_weights(wts: &[f32], cap: usize) -> Vec<f32> {
        let m = wts.len();
        let take = cap.min(m);
        let mut out = Vec::with_capacity(take);
        for i in 0..take { out.push(unsafe { *wts.get_unchecked(i) }); }
        out
    }
    let mode = std::env::var("SSSP_STOC_DELTA_MODE").unwrap_or_else(|_| "avg".to_string());
    let heavy_target_raw: f32 = std::env::var("SSSP_STOC_HEAVY_TARGET").ok().and_then(|v| v.parse().ok()).unwrap_or(0.15);
    let heavy_target: f32 = heavy_target_raw.max(0.01).min(0.9);
    let mult_env: Option<f32> = std::env::var("SSSP_STOC_DELTA_MULT").ok().and_then(|v| v.parse().ok());
    let choose_delta = || -> f32 {
        if mode == "quantile" {
            let mut samp = sample_weights(wts, 5000);
            if samp.is_empty() { return 1.0; }
            samp.sort_by(|a,b| a.partial_cmp(b).unwrap());
            let q_index = ((samp.len()-1) as f32 * (1.0 - heavy_target)).round() as usize;
            let base = samp[q_index].max(1e-4);
            let mult = mult_env.unwrap_or(1.0);
            (base * mult).clamp(1e-4, 1e6)
        } else {
            // avg mode
            let sample = core::cmp::min(1000, m);
            let mut avg = 1.0f32;
            if sample > 0 { let mut s = 0.0; for i in 0..sample { s += unsafe { *wts.get_unchecked(i) }; } avg = s / sample as f32; if avg <= 0.0 { avg = 1.0; } }
            let mult = mult_env.unwrap_or(3.0);
            (avg * mult).clamp(1e-4, 1e6)
        }
    };

    let adaptive_max: u32 = std::env::var("SSSP_STOC_ADAPT_MAX_RESTARTS").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
    // Dynamic trigger ~ log2(n)/2 bounded [3,40]
    let logn = (n as f32).ln().max(1.0);
    let adapt_trigger_buckets: u32 = std::env::var("SSSP_STOC_ADAPT_TRIGGER")
        .ok().and_then(|v| v.parse().ok())
        .unwrap_or_else(|| {
            let est = (logn / 2.0) as u32;
            est.clamp(3,40)
        });
    let heavy_min_raw: f32 = std::env::var("SSSP_STOC_HEAVY_MIN_RATIO").ok().and_then(|v| v.parse().ok()).unwrap_or(0.05);
    let heavy_min: f32 = if heavy_min_raw < 0.0 {0.0} else if heavy_min_raw > 0.9 {0.9} else { heavy_min_raw };
    let heavy_max_raw: f32 = std::env::var("SSSP_STOC_HEAVY_MAX_RATIO").ok().and_then(|v| v.parse().ok()).unwrap_or(0.25);
    let mut heavy_max: f32 = if heavy_max_raw < heavy_min + 0.01 { heavy_min + 0.01 } else { heavy_max_raw };
    if heavy_max > 0.95 { heavy_max = 0.95; }
    let mut restarts: u32 = 0;
    let adapt_trace = std::env::var("SSSP_STOC_ADAPT_TRACE").ok().map(|v| v=="1" || v.to_lowercase()=="true").unwrap_or(false);
    // Will hold (relax, light, heavy, settled, buckets_visited, light_repeat_total, bucket_cap)
    let final_stats: Option<(u64,u64,u64,u32,u32,u32,usize)>; // will be set before break
    let mut delta = choose_delta();
    loop {
        // Run with current delta
        let inv_delta = 1.0f32 / delta;
    let mut buckets: Vec<Vec<u32>> = Vec::new();
    // Heuristic reserve to reduce reallocs on early growth (light clustering typical)
    buckets.reserve((n_usize/64).max(32));
        let mut in_bucket: Vec<bool> = vec![false; n_usize];
        let mut settled: Vec<bool> = vec![false; n_usize];
        let mut relaxations: u64 = 0;
        let mut light_relax: u64 = 0;
        let mut heavy_relax: u64 = 0;
        let mut settled_count: u32 = 0;
        #[inline(always)] fn ensure_bucket(buckets: &mut Vec<Vec<u32>>, idx: usize) { if idx >= buckets.len() { buckets.resize_with(idx + 1, Vec::new); } }
        #[inline(always)] fn bucket_of(dist: f32, inv_delta: f32) -> usize { (dist * inv_delta) as usize }
        ensure_bucket(&mut buckets, 0);
        buckets[0].push(source);
        in_bucket[source as usize] = true;
        let mut current_bucket = 0usize;
        let max_bucket_cap = 4 * n_usize + 1024;
        let mut buckets_visited: u32 = 0;
        let mut light_repeat_total: u32 = 0;
        for d in dist.iter_mut() { *d = f32::INFINITY; }
        for p in pred.iter_mut() { *p = -1; }
        dist[source as usize] = 0.0;
        while current_bucket < buckets.len() {
            if buckets[current_bucket].is_empty() { current_bucket += 1; continue; }
            buckets_visited += 1;
            let mut request_light_repeat = true;
            let mut light_set: Vec<u32> = Vec::new();
            while request_light_repeat {
                light_repeat_total += 1;
                request_light_repeat = false;
                let frontier: Vec<u32> = core::mem::take(&mut buckets[current_bucket]);
                for &u_raw in &frontier { in_bucket[u_raw as usize] = false; }
                if frontier.is_empty() { break; }
                for &u_raw in &frontier {
                    let u = u_raw as usize;
                    if settled[u] { continue; }
                    settled[u] = true; settled_count += 1;
                    light_set.push(u_raw);
                    let start = off[u] as usize; let end = off[u+1] as usize;
                    let base = dist[u];
                    for e in start..end {
                        let v = unsafe { *tgt.get_unchecked(e) } as usize;
                        let w = unsafe { *wts.get_unchecked(e) };
                        if w <= delta { // light edge
                            let nd = base + w;
                            let cur = unsafe { *dist.get_unchecked(v) };
                            if nd < cur {
                                unsafe { *dist.get_unchecked_mut(v) = nd; *pred.get_unchecked_mut(v) = u as i32; }
                                let b = bucket_of(nd, inv_delta);
                                if b > max_bucket_cap { return -5; }
                                ensure_bucket(&mut buckets, b);
                                if !in_bucket[v] && !settled[v] { buckets[b].push(v as u32); in_bucket[v] = true; request_light_repeat |= b == current_bucket; }
                                relaxations += 1; light_relax += 1;
                            }
                        }
                    }
                }
            }
            // Phase 2 heavy
            for &u_raw in &light_set {
                let u = u_raw as usize;
                let start = off[u] as usize; let end = off[u+1] as usize; let base = dist[u];
                for e in start..end {
                    let v = unsafe { *tgt.get_unchecked(e) } as usize;
                    let w = unsafe { *wts.get_unchecked(e) };
                    if w > delta {
                        let nd = base + w; let cur = unsafe { *dist.get_unchecked(v) };
                        if nd < cur {
                            unsafe { *dist.get_unchecked_mut(v) = nd; *pred.get_unchecked_mut(v) = u as i32; }
                            let b = bucket_of(nd, inv_delta);
                            if b > max_bucket_cap { return -5; }
                            ensure_bucket(&mut buckets, b);
                            if !in_bucket[v] && !settled[v] { buckets[b].push(v as u32); in_bucket[v] = true; }
                            relaxations += 1; heavy_relax += 1;
                        }
                    }
                }
            }
            current_bucket += 1;
            // Adaptive restart / adjust conditions
            if buckets_visited >= adapt_trigger_buckets {
                let heavy_ratio = if relaxations==0 {0.0} else { heavy_relax as f32 / relaxations as f32 };
                if heavy_relax == 0 && restarts < adaptive_max {
                    // shrink delta to create heavy edges
                    let old = delta; delta *= 0.5;
                    restarts += 1;
                    if adapt_trace { eprintln!("[stoc-adapt] restart={} action=shrink_zero heavy_relax=0 old_delta={:.6} new_delta={:.6}", restarts, old, delta); }
                    break; // restart
                } else if heavy_ratio < heavy_min && restarts < adaptive_max {
                    let old = delta; delta *= 0.7; // small shrink
                    restarts += 1;
                    if adapt_trace { eprintln!("[stoc-adapt] restart={} action=shrink heavy_ratio={:.4} min={} old_delta={:.6} new_delta={:.6}", restarts, heavy_ratio, heavy_min, old, delta); }
                    break;
                } else if heavy_ratio > heavy_max && restarts < adaptive_max {
                    let old = delta; delta *= 1.5; // expand to reduce heavy churn
                    restarts += 1;
                    if adapt_trace { eprintln!("[stoc-adapt] restart={} action=expand heavy_ratio={:.4} max={} old_delta={:.6} new_delta={:.6}", restarts, heavy_ratio, heavy_max, old, delta); }
                    break;
                }
            }
        }
        // If we broke due to adjustment (restarts incremented) continue loop
        if restarts > 0 && (relaxations == 0 || (buckets_visited >= adapt_trigger_buckets && restarts <= adaptive_max && (heavy_relax == 0 || {
            let r = heavy_relax as f32 / relaxations.max(1) as f32; r < heavy_min || r > heavy_max
        }))) {
            if restarts <= adaptive_max { continue; }
        }
        final_stats = Some((relaxations, light_relax, heavy_relax, settled_count, buckets_visited, light_repeat_total, buckets.len()));
        unsafe { LAST_DELTA = delta; }
        break;
    }

    let (relaxations, light_relax, heavy_relax, settled_count, buckets_visited, light_repeat_total, bucket_len) = final_stats.expect("final_stats must be set before loop break");
    if !info.is_null() { unsafe { *info = SsspResultInfo { relaxations, light_relaxations: light_relax, heavy_relaxations: heavy_relax, settled: settled_count, error_code: 0 }; } }
    let heavy_ratio_x1000 = if relaxations==0 {0} else { ((heavy_relax as f64 / relaxations as f64)*1000.0) as u32 };
    unsafe { LAST_BUCKET_STATS = SsspBucketStats { buckets_visited, light_pass_repeats: light_repeat_total, max_bucket_index: (bucket_len.saturating_sub(1)) as u32, restarts, delta_x1000: (LAST_DELTA * 1000.0) as u32, heavy_ratio_x1000 }; }
    0
}

// ------------------- Light / Heavy getter helpers (C ABI) -------------------
#[no_mangle]
pub extern "C" fn sssp_info_light_relaxations(info: *const SsspResultInfo) -> u64 {
    if info.is_null() { return 0; }
    unsafe { (*info).light_relaxations }
}
#[no_mangle]
pub extern "C" fn sssp_info_heavy_relaxations(info: *const SsspResultInfo) -> u64 {
    if info.is_null() { return 0; }
    unsafe { (*info).heavy_relaxations }
}

// ------------------- Autotuned STOC (delta-stepping) -----------------------
// Tries a set of delta multipliers on a truncated run (settling up to a limit
// of nodes) and then executes the fastest multiplier on the full graph.
// Candidate set can be overridden via env: SSSP_STOC_AUTOTUNE_SET="1.5,2,3,4,6".
// Truncation limit (nodes) via env: SSSP_STOC_AUTOTUNE_LIMIT (default 2048).
use std::time::Instant;

fn parse_autotune_set() -> Vec<f32> {
    if let Ok(v) = std::env::var("SSSP_STOC_AUTOTUNE_SET") { return v.split(',').filter_map(|s| s.trim().parse().ok()).filter(|x:&f32| *x>0.0).collect(); }
    vec![1.5, 2.0, 3.0, 4.0, 6.0]
}

#[inline(always)]
fn derive_avg_weight(sample: usize, wts: &[f32]) -> f32 {
    if sample == 0 { return 1.0; }
    let mut s = 0.0; for i in 0..sample { unsafe { s += *wts.get_unchecked(i); } }
    let mut avg = s / sample as f32; if avg <= 0.0 { avg = 1.0; }
    avg
}

fn stoc_run_internal(
    n: u32,
    off: &[u32], tgt: &[u32], wts: &[f32], source: u32,
    delta: f32,
    dist: &mut [f32], pred: &mut [i32],
    truncate_after: Option<u32>,
) -> (u64,u64,u64,u32,i32) {
    let n_usize = n as usize;
    for d in dist.iter_mut() { *d = f32::INFINITY; }
    for p in pred.iter_mut() { *p = -1; }
    dist[source as usize] = 0.0;
    let inv_delta = 1.0f32 / delta;
    let mut buckets: Vec<Vec<u32>> = Vec::new();
    let mut in_bucket: Vec<bool> = vec![false; n_usize];
    let mut settled: Vec<bool> = vec![false; n_usize];
    let mut relaxations: u64 = 0; let mut light_relax: u64 = 0; let mut heavy_relax: u64 = 0; let mut settled_count: u32 = 0;
    #[inline(always)] fn ensure_bucket(buckets: &mut Vec<Vec<u32>>, idx: usize) { if idx >= buckets.len() { buckets.resize_with(idx + 1, Vec::new); } }
    #[inline(always)] fn bucket_of(dist: f32, inv_delta: f32) -> usize { (dist * inv_delta) as usize }
    ensure_bucket(&mut buckets,0); buckets[0].push(source); in_bucket[source as usize] = true;
    let mut current_bucket = 0usize; let max_bucket_cap = 4 * n_usize + 1024;
    while current_bucket < buckets.len() {
        if buckets[current_bucket].is_empty() { current_bucket += 1; continue; }
        let mut request_light_repeat = true; let mut light_set: Vec<u32> = Vec::new();
    while request_light_repeat {
            request_light_repeat = false; let frontier: Vec<u32> = core::mem::take(&mut buckets[current_bucket]); for &u_raw in &frontier { in_bucket[u_raw as usize] = false; }
            if frontier.is_empty() { break; }
            for &u_raw in &frontier { let u = u_raw as usize; if settled[u] { continue; } settled[u] = true; settled_count += 1; light_set.push(u_raw); let start = off[u] as usize; let end = off[u+1] as usize; let base = dist[u];
                for e in start..end { let v = unsafe { *tgt.get_unchecked(e) } as usize; let w = unsafe { *wts.get_unchecked(e) }; if w <= delta { let nd = base + w; let cur = unsafe { *dist.get_unchecked(v) }; if nd < cur { unsafe { *dist.get_unchecked_mut(v) = nd; *pred.get_unchecked_mut(v) = u as i32; } let b = bucket_of(nd, inv_delta); if b > max_bucket_cap { return (relaxations, light_relax, heavy_relax, settled_count, -5); } ensure_bucket(&mut buckets,b); if !in_bucket[v] && !settled[v] { buckets[b].push(v as u32); in_bucket[v] = true; request_light_repeat |= b == current_bucket; } relaxations += 1; light_relax += 1; } } }
                if let Some(limit) = truncate_after { if settled_count >= limit { break; } }
            }
            if let Some(limit) = truncate_after { if settled_count >= limit { break; } }
        }
        for &u_raw in &light_set { let u = u_raw as usize; let start = off[u] as usize; let end = off[u+1] as usize; let base = dist[u]; for e in start..end { let v = unsafe { *tgt.get_unchecked(e) } as usize; let w = unsafe { *wts.get_unchecked(e) }; if w > delta { let nd = base + w; let cur = unsafe { *dist.get_unchecked(v) }; if nd < cur { unsafe { *dist.get_unchecked_mut(v) = nd; *pred.get_unchecked_mut(v) = u as i32; } let b = bucket_of(nd, inv_delta); if b > max_bucket_cap { return (relaxations, light_relax, heavy_relax, settled_count, -5); } ensure_bucket(&mut buckets,b); if !in_bucket[v] && !settled[v] { buckets[b].push(v as u32); in_bucket[v] = true; } relaxations += 1; heavy_relax += 1; } } } }
        if let Some(limit) = truncate_after { if settled_count >= limit { break; } }
        current_bucket += 1;
    }
    (relaxations, light_relax, heavy_relax, settled_count, 0)
}

#[no_mangle]
pub extern "C" fn sssp_run_stoc_autotune(
    n: u32,
    offsets: *const u32,
    targets: *const u32,
    weights: *const f32,
    source: u32,
    out_dist: *mut f32,
    out_pred: *mut i32,
    info: *mut SsspResultInfo,
) -> i32 {
    if n == 0 { return -1; }
    if source >= n { return -2; }
    if offsets.is_null() || targets.is_null() || weights.is_null() || out_dist.is_null() || out_pred.is_null() { return -3; }
    let n_usize = n as usize; let off = as_slice(offsets, n_usize + 1); let m = match off.last() { Some(v) => *v as usize, None => return -4 }; let tgt = as_slice(targets, m); let wts = as_slice(weights, m);
    let dist = as_mut_slice(out_dist, n_usize); let pred = as_mut_slice(out_pred, n_usize);
    let sample = core::cmp::min(1000, m); let avg = derive_avg_weight(sample, wts);
    let candidates = { let mut c = parse_autotune_set(); if c.is_empty() { c.push(3.0); } c };
    let limit: u32 = std::env::var("SSSP_STOC_AUTOTUNE_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(2048).min(n);
    let mut best_mult = candidates[0]; let mut best_time = f64::INFINITY;
    let mut tmp_dist = vec![0f32; n_usize]; let mut tmp_pred = vec![0i32; n_usize];
    for &mult in &candidates { let delta = (avg * mult).clamp(0.0001, 1e6); let start = Instant::now(); let (_r,_l,_h,_s,err) = stoc_run_internal(n, off, tgt, wts, source, delta, &mut tmp_dist, &mut tmp_pred, Some(limit)); if err != 0 { continue; } let elapsed = start.elapsed().as_secs_f64(); if elapsed < best_time { best_time = elapsed; best_mult = mult; } }
    let final_delta = (avg * best_mult).clamp(0.0001, 1e6);
    let (relax, light, heavy, settled, err) = stoc_run_internal(n, off, tgt, wts, source, final_delta, dist, pred, None);
    if err != 0 { return err; }
    if !info.is_null() { unsafe { *info = SsspResultInfo { relaxations: relax, light_relaxations: light, heavy_relaxations: heavy, settled, error_code: 0 }; } }
    // Autotune internal run does not update global stats; only final full run instrumentation performed via LAST_BUCKET_STATS in sssp_run_stoc.
    0
}

// Unified: autotune to pick initial delta multiplier, then run adaptive STOC loop (same as sssp_run_stoc logic).
// Exposed as sssp_run_stoc_auto_adapt for experimentation; future: may replace separate paths.
#[no_mangle]
pub extern "C" fn sssp_run_stoc_auto_adapt(
    n: u32,
    offsets: *const u32,
    targets: *const u32,
    weights: *const f32,
    source: u32,
    out_dist: *mut f32,
    out_pred: *mut i32,
    info: *mut SsspResultInfo,
) -> i32 {
    if n == 0 { return -1; }
    if source >= n { return -2; }
    if offsets.is_null() || targets.is_null() || weights.is_null() || out_dist.is_null() || out_pred.is_null() { return -3; }
    let n_usize = n as usize; let off = as_slice(offsets, n_usize + 1); let m = match off.last() { Some(v) => *v as usize, None => return -4 };
    let tgt = as_slice(targets, m); let wts = as_slice(weights, m);
    let sample = core::cmp::min(1000, m); let avg = derive_avg_weight(sample, wts);
    let candidates = { let mut c = parse_autotune_set(); if c.is_empty() { c.push(3.0); } c };
    let limit: u32 = std::env::var("SSSP_STOC_AUTOTUNE_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(2048).min(n);
    let mode = std::env::var("SSSP_STOC_DELTA_MODE").unwrap_or_else(|_| "avg".to_string());
    // Helper to derive initial delta for a multiplier under current mode.
    let base_quantile = if mode == "quantile" {
        // Sample & pick quantile similarly to sssp_run_stoc (but without heavy_target multiplier yet).
        let heavy_target_raw: f32 = std::env::var("SSSP_STOC_HEAVY_TARGET").ok().and_then(|v| v.parse().ok()).unwrap_or(0.15);
        let heavy_target = heavy_target_raw.max(0.01).min(0.9);
        let mut samp: Vec<f32> = {
            let take = core::cmp::min(5000, m);
            let mut v = Vec::with_capacity(take);
            for i in 0..take { v.push(unsafe { *wts.get_unchecked(i) }); }
            v
        };
        if samp.is_empty() { 1.0 } else { samp.sort_by(|a,b| a.partial_cmp(b).unwrap()); let q_index = ((samp.len()-1) as f32 * (1.0 - heavy_target)).round() as usize; samp[q_index].max(1e-4) }
    } else { 0.0 }; // unused in avg mode
    let mut best_mult = candidates[0]; let mut best_time = f64::INFINITY; let mut tmp_dist = vec![0f32; n_usize]; let mut tmp_pred = vec![0i32; n_usize];
    for &mult in &candidates {
        let delta = if mode == "quantile" { (base_quantile * mult).clamp(1e-4, 1e6) } else { (avg * mult).clamp(1e-4, 1e6) };
        let start = Instant::now();
        let (_r,_l,_h,_s,err) = stoc_run_internal(n, off, tgt, wts, source, delta, &mut tmp_dist, &mut tmp_pred, Some(limit));
        if err != 0 { continue; }
        let elapsed = start.elapsed().as_secs_f64();
        if elapsed < best_time { best_time = elapsed; best_mult = mult; }
    }
    // Temporarily set multiplier env if not already set so sssp_run_stoc starts from our seed.
    let env_key = "SSSP_STOC_DELTA_MULT";
    let prev = std::env::var(env_key).ok();
    if prev.is_none() { std::env::set_var(env_key, format!("{}", best_mult)); }
    let rc = sssp_run_stoc(n, offsets, targets, weights, source, out_dist, out_pred, info);
    // Restore previous env state.
    if prev.is_none() { std::env::remove_var(env_key); }
    rc
}

// ---------------------------------------------------------------------------
// Experimental k-hop recursive frontier algorithm (very simplified prototype)
// NOT a full faithful implementation of the theoretical O(m log^{2/3} n) paper.
// Provides a pragmatic approximation for benchmarking exploration only.
// Key differences / caveats:
//  * Uses fixed k-hop expansion rounds (Bellman-Ford style limited to k) from a frontier.
//  * Determines "finished" nodes as those whose shortest path (current dist) was stabilized within k rounds of current frontier.
//  * Boundary (next frontier) = finished nodes having edges into any still-unfinished nodes.
//  * Pivot selection heuristic: choose subset of boundary with out-degree >= PIVOT_MIN_OUT (fallback all boundary if none).
//  * Recurses until no unfinished remain or depth limit exceeded.
//  * Tie-breaking: distances compared first; for exact equal (unlikely with floats) lower node id wins.
// Environment variables controlling prototype:
//    SSSP_KHOP_K (default 2)               - hop bound per recursion level
//    SSSP_KHOP_MAX_DEPTH (default 32)       - recursion depth safety cap
//    SSSP_KHOP_PIVOT_MIN_OUT (default 2)    - minimum out-degree to qualify as pivot
// Exported as sssp_run_khop (C ABI). Returns 0 on success, negative on error.
#[no_mangle]
pub extern "C" fn sssp_run_khop(
    n: u32,
    offsets: *const u32,
    targets: *const u32,
    weights: *const f32,
    source: u32,
    out_dist: *mut f32,
    out_pred: *mut i32,
    info: *mut SsspResultInfo,
) -> i32 {
    if n == 0 { return -1; }
    if source >= n { return -2; }
    if offsets.is_null() || targets.is_null() || weights.is_null() || out_dist.is_null() || out_pred.is_null() { return -3; }
    let n_usize = n as usize;
    let off = as_slice(offsets, n_usize + 1);
    let m = match off.last() { Some(v) => *v as usize, None => return -4 };
    let tgt = as_slice(targets, m);
    let wts = as_slice(weights, m);
    let dist = as_mut_slice(out_dist, n_usize);
    let pred = as_mut_slice(out_pred, n_usize);
    for d in dist.iter_mut() { *d = f32::INFINITY; }
    for p in pred.iter_mut() { *p = -1; }
    dist[source as usize] = 0.0;
    // K interpreted as batch size of Dijkstra pops processed with a simple local queue; ensures correctness since we only finalize when popped.
    let k: usize = std::env::var("SSSP_KHOP_K").ok().and_then(|v| v.parse().ok()).unwrap_or(32).max(1).min(1024) as usize;
    let mut heap = BinaryHeapSimple::new((n as usize).min(1024));
    let mut relaxations: u64 = 0;
    let mut pops: usize = 0;
    let mut heap_pushes: u64 = 0; let mut heap_pops: u64 = 0; let mut heap_max: u64 = 0;
    heap.push(HeapItem { node: source, dist: 0.0 }, &mut heap_pushes); heap_max=1;
    // Temporary vector for batch nodes (after pop) processed with adjacency relaxations.
    while let Some(item) = heap.pop(&mut heap_pops) {
        if item.dist > dist[item.node as usize] { continue; }
        // Process this popped node and up to k-1 additional pops ahead lazily collecting them.
        let mut batch: Vec<HeapItem> = Vec::with_capacity(k);
        batch.push(item);
        for _ in 1..k {
            if let Some(next) = heap.pop(&mut heap_pops) {
                if next.dist > dist[next.node as usize] { continue; }
                batch.push(next);
            } else { break; }
        }
        // Relax outgoing edges of batch nodes.
        for hi in batch.into_iter() {
            let u = hi.node as usize; let base = hi.dist;
            let start = off[u] as usize; let end = off[u+1] as usize;
            for e in start..end {
                let v = unsafe { *tgt.get_unchecked(e) } as usize;
                let w = unsafe { *wts.get_unchecked(e) };
                let nd = base + w; let cur = unsafe { *dist.get_unchecked(v) };
                if nd < cur { unsafe { *dist.get_unchecked_mut(v) = nd; *pred.get_unchecked_mut(v) = u as i32; } heap.push(HeapItem { node: v as u32, dist: nd }, &mut heap_pushes); if heap.data.len() as u64 > heap_max { heap_max = heap.data.len() as u64; } relaxations += 1; }
            }
            pops += 1;
        }
    }
    if !info.is_null() { unsafe { *info = SsspResultInfo { relaxations, light_relaxations: 0, heavy_relaxations: 0, settled: n, error_code: 0 }; } }
    // (Optionally we could update LAST_BASELINE_HEAP_STATS but keep separate)
    0
}

// Alias: default entrypoint (currently the batched k-hop variant). Exposed so wrappers can remain stable if we swap implementation later.
#[no_mangle]
pub extern "C" fn sssp_run_default(
    n: u32,
    offsets: *const u32,
    targets: *const u32,
    weights: *const f32,
    source: u32,
    out_dist: *mut f32,
    out_pred: *mut i32,
    info: *mut SsspResultInfo,
) -> i32 {
    sssp_run_khop(n, offsets, targets, weights, source, out_dist, out_pred, info)
}
