use sssp_core::{sssp_run_spec_phase1, SsspResultInfo};
use std::ffi::c_int;

fn main(){
    // Simple triangle graph 0->1 (1.0), 0->2 (2.0), 1->2 (0.25)
    let n: u32 = 3;
    let offsets: [u32;4] = [0,2,3,3];
    let targets: [u32;3] = [1,2,2];
    let weights: [f32;3] = [1.0,2.0,0.25];
    let mut dist = vec![0f32; n as usize];
    let mut pred = vec![-1i32; n as usize];
    let mut info = SsspResultInfo { relaxations:0, light_relaxations:0, heavy_relaxations:0, settled:0, error_code:0 };
    unsafe {
        let rc: c_int = sssp_run_spec_phase1(n, offsets.as_ptr(), targets.as_ptr(), weights.as_ptr(), 0, dist.as_mut_ptr(), pred.as_mut_ptr(), &mut info as *mut _);
        println!("rc={} relax={} settled={} dist={:?} pred={:?}", rc, info.relaxations, info.settled, dist, pred);
    }
}