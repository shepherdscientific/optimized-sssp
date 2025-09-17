use std::time::Instant;
use sssp_core::{sssp_run_baseline, sssp_run_spec_phase3, sssp_run_spec_boundary_chain};
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

fn run_one(n: usize, avg_degree: f32, seed: u64) -> serde_json::Value {
    let (off, tgt, wt) = make_random_graph(n, avg_degree, seed);
    let m = wt.len();
    let mut dist_b = vec![0f32; n]; let mut pred_b = vec![-1i32; n]; let mut info_b = SsspResultInfo{ relaxations:0, light_relaxations:0, heavy_relaxations:0, settled:0, error_code:0 };
    let mut dist_p3 = vec![0f32; n]; let mut pred_p3 = vec![-1i32; n]; let mut info_p3 = SsspResultInfo{ relaxations:0, light_relaxations:0, heavy_relaxations:0, settled:0, error_code:0 };
    let mut dist_bc = vec![0f32; n]; let mut pred_bc = vec![-1i32; n]; let mut info_bc = SsspResultInfo{ relaxations:0, light_relaxations:0, heavy_relaxations:0, settled:0, error_code:0 };
    unsafe {
        let t0=Instant::now(); sssp_run_baseline(n as u32, off.as_ptr(), tgt.as_ptr(), wt.as_ptr(), 0, dist_b.as_mut_ptr(), pred_b.as_mut_ptr(), &mut info_b as *mut _); let dt_base = t0.elapsed().as_secs_f64()*1000.0;
        let t1=Instant::now(); sssp_run_spec_phase3(n as u32, off.as_ptr(), tgt.as_ptr(), wt.as_ptr(), 0, dist_p3.as_mut_ptr(), pred_p3.as_mut_ptr(), &mut info_p3 as *mut _); let dt_p3 = t1.elapsed().as_secs_f64()*1000.0;
        let t2=Instant::now(); sssp_run_spec_boundary_chain(n as u32, off.as_ptr(), tgt.as_ptr(), wt.as_ptr(), 0, dist_bc.as_mut_ptr(), pred_bc.as_mut_ptr(), &mut info_bc as *mut _); let dt_bc = t2.elapsed().as_secs_f64()*1000.0;
        for i in 0..n { assert!((dist_b[i]-dist_p3[i]).abs() < 1e-4, "phase3 parity fail at {}", i); assert!((dist_b[i]-dist_bc[i]).abs() < 1e-4, "boundary_chain parity fail at {}", i); }
        serde_json::json!({
            "n": n, "m": m, "avg_degree": avg_degree,
            "baseline_ms": dt_base, "phase3_ms": dt_p3, "boundary_chain_ms": dt_bc,
            "phase3_speedup": dt_base/dt_p3.max(1e-9),
            "boundary_chain_speedup": dt_base/dt_bc.max(1e-9),
            "relaxations_baseline": info_b.relaxations,
            "relaxations_phase3": info_p3.relaxations,
            "relaxations_boundary_chain": info_bc.relaxations
        })
    }
}

fn main(){
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a=="--help") { eprintln!("Usage: bench_spec --sizes 10000,20000 --avg-degree 4.0 --seed 42 --out benchmarks/native_sample.json"); return; }
    let sizes_arg = args.iter().position(|a| a=="--sizes").and_then(|i| args.get(i+1)).cloned().unwrap_or("10000,20000".into());
    let avg_degree: f32 = args.iter().position(|a| a=="--avg-degree").and_then(|i| args.get(i+1)).and_then(|v| v.parse().ok()).unwrap_or(4.0);
    let seed: u64 = args.iter().position(|a| a=="--seed").and_then(|i| args.get(i+1)).and_then(|v| v.parse().ok()).unwrap_or(42);
    let out_path = args.iter().position(|a| a=="--out").and_then(|i| args.get(i+1)).unwrap_or(&"benchmarks/native_sample.json".to_string()).clone();
    let sizes: Vec<usize> = sizes_arg.split(',').filter_map(|s| s.parse().ok()).collect();
    let mut results = Vec::new();
    for s in sizes { results.push(run_one(s, avg_degree, seed)); }
    let json = serde_json::Value::Array(results);
    if let Some(dir) = std::path::Path::new(&out_path).parent() { std::fs::create_dir_all(dir).ok(); }
    let mut f=File::create(&out_path).expect("create out"); f.write_all(serde_json::to_string_pretty(&json).unwrap().as_bytes()).unwrap();
    eprintln!("wrote {}", out_path);
}
