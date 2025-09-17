use std::time::Instant;
use sssp_core::{sssp_run_baseline, sssp_run_spec_phase3, sssp_run_spec_boundary_chain, sssp_run_spec_recursive, sssp_get_spec_recursion_stats, sssp_get_spec_recursion_frame_count, sssp_get_spec_recursion_frame, SpecRecursionStats};
use rand::{SeedableRng, rngs::SmallRng, Rng};
use std::fs::File; use std::io::Write;

fn make_random_graph(n: usize, avg_degree: f32, seed: u64) -> (Vec<u32>, Vec<u32>, Vec<f32>) {
    let mut rng = SmallRng::seed_from_u64(seed);
    let m_est = (n as f32 * avg_degree) as usize;
    let mut adj: Vec<Vec<(u32,f32)>> = vec![Vec::new(); n];
    for _ in 0..m_est { let u = rng.gen_range(0..n as u32); let v = rng.gen_range(0..n as u32); if u==v { continue; } let w = rng.gen_range(1.0..5.0); adj[u as usize].push((v,w)); }
    // Build CSR
    let mut offsets = Vec::with_capacity(n+1); offsets.push(0u32); let mut targets = Vec::new(); let mut weights = Vec::new();
    for u in 0..n { for &(v,w) in &adj[u] { targets.push(v); weights.push(w); } offsets.push(targets.len() as u32); }
    (offsets, targets, weights)
}

type SsspResultInfo = sssp_core::SsspResultInfo;

fn run_one(n: usize, avg_degree: f32, seed: u64, check_boundary: bool, do_recursion: bool) -> serde_json::Value {
    let (off, tgt, wt) = make_random_graph(n, avg_degree, seed);
    let m = wt.len();
    let mut dist_b = vec![f32::INFINITY; n]; let mut pred_b = vec![-1i32; n]; let mut info_b = SsspResultInfo{ relaxations:0, light_relaxations:0, heavy_relaxations:0, settled:0, error_code:0 };
    let mut dist_p3 = vec![f32::INFINITY; n]; let mut pred_p3 = vec![-1i32; n]; let mut info_p3 = SsspResultInfo{ relaxations:0, light_relaxations:0, heavy_relaxations:0, settled:0, error_code:0 };
    let mut dist_bc = vec![f32::INFINITY; n]; let mut pred_bc = vec![-1i32; n]; let mut info_bc = SsspResultInfo{ relaxations:0, light_relaxations:0, heavy_relaxations:0, settled:0, error_code:0 };
    unsafe {
        let t0=Instant::now(); sssp_run_baseline(n as u32, off.as_ptr(), tgt.as_ptr(), wt.as_ptr(), 0, dist_b.as_mut_ptr(), pred_b.as_mut_ptr(), &mut info_b as *mut _); let dt_base = t0.elapsed().as_secs_f64()*1000.0;
        let t1=Instant::now(); sssp_run_spec_phase3(n as u32, off.as_ptr(), tgt.as_ptr(), wt.as_ptr(), 0, dist_p3.as_mut_ptr(), pred_p3.as_mut_ptr(), &mut info_p3 as *mut _); let dt_p3 = t1.elapsed().as_secs_f64()*1000.0;
        let t2=Instant::now(); sssp_run_spec_boundary_chain(n as u32, off.as_ptr(), tgt.as_ptr(), wt.as_ptr(), 0, dist_bc.as_mut_ptr(), pred_bc.as_mut_ptr(), &mut info_bc as *mut _); let dt_bc = t2.elapsed().as_secs_f64()*1000.0;
    let (_dt_rec, rec_obj) = if do_recursion {
            let mut dist_r = vec![f32::INFINITY; n]; let mut pred_r = vec![-1i32; n]; let mut info_r = SsspResultInfo{ relaxations:0, light_relaxations:0, heavy_relaxations:0, settled:0, error_code:0 };
            let tr=Instant::now(); sssp_run_spec_recursive(n as u32, off.as_ptr(), tgt.as_ptr(), wt.as_ptr(), 0, dist_r.as_mut_ptr(), pred_r.as_mut_ptr(), &mut info_r as *mut _); let dt_rec = tr.elapsed().as_secs_f64()*1000.0;
            // Collect stats & frame details
            let mut stats = SpecRecursionStats{frames:0,total_relaxations:0,baseline_relaxations:0,seed_k:0,chain_segments:0,chain_total_collected:0,inv_checks:0,inv_failures:0};
            sssp_get_spec_recursion_stats(&mut stats as *mut _);
            let frame_count = sssp_get_spec_recursion_frame_count();
            let mut frames_json = Vec::new();
            // manual fetch since we don't have wrapper: define local struct
            #[repr(C)] #[derive(Copy,Clone,Default)] struct FrameDetail { id:u32,bound:f32,k_used:u32,segment_size:u32,truncated:i32,relaxations:u64,pivots_examined:u32,max_subtree:u32,depth:u32,parent_id:u32,pruning_ratio_f32:f32,bound_improvement_f32:f32,pivot_success_rate_f32:f32 }
            for i in 0..frame_count { let mut fd = FrameDetail::default(); let rc = sssp_get_spec_recursion_frame(i, &mut fd as *mut _ as *mut _); if rc==0 { frames_json.push(serde_json::json!({
                "id":fd.id, "bound":fd.bound, "k_used":fd.k_used, "segment_size":fd.segment_size,
                "truncated":fd.truncated==1, "relaxations":fd.relaxations,
                "pivots_examined":fd.pivots_examined, "max_subtree":fd.max_subtree,
                "depth":fd.depth, "parent_id":fd.parent_id,
                "pruning_ratio":fd.pruning_ratio_f32, "bound_improvement":fd.bound_improvement_f32,
                "pivot_success_rate":fd.pivot_success_rate_f32
            })); } }
            (dt_rec, Some(serde_json::json!({
                "recursion_ms": dt_rec,
                "frames": stats.frames,
                "total_relaxations": stats.total_relaxations,
                "baseline_relaxations": stats.baseline_relaxations,
                "seed_k": stats.seed_k,
                "chain_segments": stats.chain_segments,
                "chain_total_collected": stats.chain_total_collected,
                "inv_checks": stats.inv_checks,
                "inv_failures": stats.inv_failures,
                "frame_details": frames_json
            })))
        } else { (0.0, None) };
        for i in 0..n {
            let db = dist_b[i]; let dp3 = dist_p3[i]; let dbc = dist_bc[i];
            if db.is_infinite() && dp3.is_infinite() { /* ok */ } else { assert!((db-dp3).abs() < 1e-4, "phase3 parity fail at {}", i); }
            if check_boundary {
                if db.is_infinite() && dbc.is_infinite() { /* ok */ } else { assert!((db-dbc).abs() < 1e-4, "boundary_chain parity fail at {}", i); }
            }
        }
        let mut obj = serde_json::json!({
            "n": n, "m": m, "avg_degree": avg_degree,
            "baseline_ms": dt_base, "phase3_ms": dt_p3, "boundary_chain_ms": dt_bc,
            "phase3_speedup": dt_base/dt_p3.max(1e-9),
            "boundary_chain_speedup": dt_base/dt_bc.max(1e-9),
            "relaxations_baseline": info_b.relaxations,
            "relaxations_phase3": info_p3.relaxations,
            "relaxations_boundary_chain": info_bc.relaxations
        });
        if let Some(rj) = rec_obj { if let serde_json::Value::Object(ref mut map) = obj { map.insert("recursion".to_string(), rj); } }
        obj
    }
}

fn main(){
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a=="--help") { eprintln!("Usage: bench_spec --sizes 10000,20000 --degrees 2,4,8 --seed 42 --out benchmarks/native_sample.json [--no-boundary-parity] [--full-parity] [--recursion]"); return; }
    let do_recursion = args.iter().any(|a| a=="--recursion");
    let sizes_arg = args.iter().position(|a| a=="--sizes").and_then(|i| args.get(i+1)).cloned().unwrap_or("10000,20000".into());
    let degrees_arg = args.iter().position(|a| a=="--degrees").and_then(|i| args.get(i+1)).cloned();
    let single_degree: f32 = args.iter().position(|a| a=="--avg-degree").and_then(|i| args.get(i+1)).and_then(|v| v.parse().ok()).unwrap_or(4.0);
    let seed: u64 = args.iter().position(|a| a=="--seed").and_then(|i| args.get(i+1)).and_then(|v| v.parse().ok()).unwrap_or(42);
    let out_path = args.iter().position(|a| a=="--out").and_then(|i| args.get(i+1)).unwrap_or(&"benchmarks/native_sample.json".to_string()).clone();
    let full_parity = args.iter().any(|a| a=="--full-parity"); // forces boundary chain parity check even if experimental
    let skip_boundary_parity = args.iter().any(|a| a=="--no-boundary-parity");
    if full_parity { // Set large env knobs to avoid truncation
        std::env::set_var("SSSP_SPEC_K","100000000");
        std::env::set_var("SSSP_SPEC_PIVOT_MAX","100000000");
        std::env::set_var("SSSP_SPEC_CHAIN_K","100000000");
        std::env::set_var("SSSP_SPEC_CHAIN_MAX_SEG","100000000");
    }
    let sizes: Vec<usize> = sizes_arg.split(',').filter_map(|s| s.parse().ok()).collect();
    let degrees: Vec<f32> = if let Some(darg) = degrees_arg { darg.split(',').filter_map(|s| s.parse().ok()).collect() } else { vec![single_degree] };
    let mut results = Vec::new();
    for s in &sizes { for &deg in &degrees { results.push(run_one(*s, deg, seed, full_parity && !skip_boundary_parity, do_recursion)); } }
    let json = serde_json::Value::Array(results);
    if let Some(dir) = std::path::Path::new(&out_path).parent() { std::fs::create_dir_all(dir).ok(); }
    let mut f=File::create(&out_path).expect("create out"); f.write_all(serde_json::to_string_pretty(&json).unwrap().as_bytes()).unwrap();
    eprintln!("wrote {}", out_path);
}
