use sssp_core::*;
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 { eprintln!("usage: run_one <n> <density> <seed> <mode: baseline|stoc|stoc_autotune>"); std::process::exit(1); }
    let n: u32 = args[1].parse().expect("n");
    let density: f32 = args[2].parse().expect("density");
    let seed: u64 = args[3].parse().expect("seed");
    let mode = args[4].as_str();
    let mut rng = SmallRng::seed_from_u64(seed);
    let avg_deg = density;
    let m_est = (n as f32 * avg_deg) as usize;
    let mut offsets = Vec::with_capacity(n as usize + 1);
    let mut targets = Vec::with_capacity(m_est + n as usize);
    let mut weights = Vec::with_capacity(m_est + n as usize);
    offsets.push(0);
    for _u in 0..n {
        let deg = avg_deg.max(1.0) as usize;
        for _ in 0..deg { let v = rng.gen_range(0..n); let w: f32 = rng.gen_range(1.0..4.0); targets.push(v); weights.push(w); }
        offsets.push(targets.len() as u32);
    }
    let mut dist = vec![0f32; n as usize];
    let mut pred = vec![0i32; n as usize];
    let mut info = SsspResultInfo { relaxations:0, light_relaxations:0, heavy_relaxations:0, settled:0, error_code:0 };
    let rc = unsafe { match mode { "baseline" => sssp_run_baseline(n, offsets.as_ptr(), targets.as_ptr(), weights.as_ptr(), 0, dist.as_mut_ptr(), pred.as_mut_ptr(), &mut info), "stoc" => sssp_run_stoc(n, offsets.as_ptr(), targets.as_ptr(), weights.as_ptr(), 0, dist.as_mut_ptr(), pred.as_mut_ptr(), &mut info), "stoc_autotune" => sssp_run_stoc_autotune(n, offsets.as_ptr(), targets.as_ptr(), weights.as_ptr(), 0, dist.as_mut_ptr(), pred.as_mut_ptr(), &mut info), _ => { eprintln!("bad mode"); return; } } };
    if rc != 0 {
        eprintln!("error {rc}");
        return;
    }
    print!("mode={mode} n={n} m={} relax={} light={} heavy={} settled={}", targets.len(), info.relaxations, info.light_relaxations, info.heavy_relaxations, info.settled);
    if mode != "baseline" {
        unsafe {
            let mut bs = SsspBucketStats { buckets_visited:0, light_pass_repeats:0, max_bucket_index:0, restarts:0, delta_x1000:0, heavy_ratio_x1000:0 };
            extern "C" { fn sssp_get_bucket_stats(out: *mut SsspBucketStats); fn sssp_get_last_delta() -> f32; }
            sssp_get_bucket_stats(&mut bs as *mut _);
            let d = sssp_get_last_delta();
            print!(
                " buckets_visited={} light_pass_repeats={} max_bucket_index={} restarts={} final_delta={:.4} heavy_ratio={:.3}",
                bs.buckets_visited,
                bs.light_pass_repeats,
                bs.max_bucket_index,
                bs.restarts,
                d,
                (bs.heavy_ratio_x1000 as f32)/1000.0
            );
        }
    }
    println!();
}
