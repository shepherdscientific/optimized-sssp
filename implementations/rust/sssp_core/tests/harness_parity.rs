use sssp_core::{
    sssp_run_baseline, sssp_run_spec_phase1, sssp_run_spec_phase2, sssp_run_spec_phase3, sssp_run_spec_boundary_chain,
    SsspResultInfo,
};

// CSR graph representation helper
struct CsrGraph { n:u32, offsets: Vec<u32>, targets: Vec<u32>, weights: Vec<f32> }

fn path_graph(n:u32, w:f32) -> CsrGraph {
    let mut offsets = Vec::with_capacity(n as usize + 1); offsets.push(0);
    let mut targets = Vec::new(); let mut weights = Vec::new();
    for u in 0..n { if u+1 < n { targets.push(u+1); weights.push(w); } offsets.push(targets.len() as u32); }
    CsrGraph { n, offsets, targets, weights }
}

fn star_graph(k:u32, w:f32) -> CsrGraph { // center 0, leaves 1..k
    let n = k+1; let mut offsets=Vec::with_capacity(n as usize +1); offsets.push(0); let mut targets=Vec::new(); let mut weights=Vec::new();
    // center edges
    for leaf in 1..n { targets.push(leaf); weights.push(w); }
    offsets.push(targets.len() as u32);
    for _ in 1..n { offsets.push(targets.len() as u32); }
    CsrGraph { n, offsets, targets, weights }
}

fn complete_graph(n:u32, w:f32) -> CsrGraph {
    let mut offsets = Vec::with_capacity(n as usize +1); offsets.push(0); let mut targets=Vec::new(); let mut weights=Vec::new();
    for u in 0..n { for v in 0..n { if u!=v { targets.push(v); weights.push(w); } } offsets.push(targets.len() as u32); }
    CsrGraph { n, offsets, targets, weights }
}

fn bridge_cliques(a:u32, b:u32, w:f32) -> CsrGraph { // two cliques connected by single bridge edge a-1 -> a
    let n = a + b; let mut offsets=Vec::with_capacity(n as usize +1); offsets.push(0); let mut targets=Vec::new(); let mut weights=Vec::new();
    for u in 0..a { for v in 0..a { if u!=v { targets.push(v); weights.push(w); } } if u==a-1 { targets.push(a); weights.push(w); } offsets.push(targets.len() as u32); }
    for u in a..n { for v in a..n { if u!=v { targets.push(v); weights.push(w); } } offsets.push(targets.len() as u32); }
    CsrGraph { n, offsets, targets, weights }
}

fn hash_dist(dist:&[f32]) -> u64 { // simple mixing; not cryptographic
    let mut h: u64 = 1469598103934665603; // FNV offset basis
    for (i,&d) in dist.iter().enumerate() { let bits = d.to_bits() as u64 ^ ((i as u64).wrapping_mul(1099511628211)); h ^= bits; h = h.wrapping_mul(1099511628211); }
    h
}

fn run_variant(
    which: &str,
    g: &CsrGraph,
    source:u32,
) -> (Vec<f32>, Vec<i32>, SsspResultInfo) {
    let mut dist = vec![0f32; g.n as usize];
    let mut pred = vec![-1i32; g.n as usize];
    let mut info = SsspResultInfo { relaxations:0, light_relaxations:0, heavy_relaxations:0, settled:0, error_code:0 };
    unsafe {
        let rc = match which {
            "baseline" => sssp_run_baseline(g.n, g.offsets.as_ptr(), g.targets.as_ptr(), g.weights.as_ptr(), source, dist.as_mut_ptr(), pred.as_mut_ptr(), &mut info as *mut _),
            "phase1" => sssp_run_spec_phase1(g.n, g.offsets.as_ptr(), g.targets.as_ptr(), g.weights.as_ptr(), source, dist.as_mut_ptr(), pred.as_mut_ptr(), &mut info as *mut _),
            "phase2" => sssp_run_spec_phase2(g.n, g.offsets.as_ptr(), g.targets.as_ptr(), g.weights.as_ptr(), source, dist.as_mut_ptr(), pred.as_mut_ptr(), &mut info as *mut _),
            "phase3" => sssp_run_spec_phase3(g.n, g.offsets.as_ptr(), g.targets.as_ptr(), g.weights.as_ptr(), source, dist.as_mut_ptr(), pred.as_mut_ptr(), &mut info as *mut _),
            "chain"  => sssp_run_spec_boundary_chain(g.n, g.offsets.as_ptr(), g.targets.as_ptr(), g.weights.as_ptr(), source, dist.as_mut_ptr(), pred.as_mut_ptr(), &mut info as *mut _),
            _ => panic!("unknown variant")
        }; assert_eq!(rc,0, "variant {} returned rc {}", which, rc);
    }
    (dist,pred,info)
}

fn assert_parity(base:&[f32], other:&[f32], tol:f32){
    let mut diffs = Vec::new();
    for i in 0..base.len() { let a=base[i]; let b=other[i];
        if a.is_finite() || b.is_finite() {
            let scale = 1.0f32.max(a.abs()).max(b.abs());
            if (a-b).abs() > tol * scale { diffs.push((i,a,b)); if diffs.len() > 16 { break; } }
        }
    }
    if !diffs.is_empty() {
        let mut msg = String::from("distance parity mismatch; first differences: ");
        for (i,a,b) in &diffs { msg.push_str(&format!("[{}:{} vs {}] ", i,a,b)); }
        panic!("{} ({} diffs; n={})", msg, diffs.len(), base.len());
    }
}

// Simple deterministic pseudo-random directed graph generator
fn pseudo_random_graph(n:u32, m:u32, seed:u64, w_min:f32, w_max:f32) -> CsrGraph {
    assert!(n>=2);
    let mut adj: Vec<Vec<(u32,f32)>> = vec![Vec::new(); n as usize];
    let mut state = seed | 1; // ensure non-zero
    let mut next_u32 = || { // xorshift64*
        state ^= state >> 12; state ^= state << 25; state ^= state >> 27; state = state.wrapping_mul(2685821657736338717); (state >> 32) as u32
    };
    let span = w_max - w_min;
    let mut edges = 0u32; let target_edges = m.min(n.saturating_mul(n-1));
    let mut attempts = 0u32; let attempt_limit = target_edges * 10 + 1000;
    while edges < target_edges && attempts < attempt_limit {
        attempts += 1;
        let u = next_u32() % n; let mut v = next_u32() % n; if u==v { continue; }
        // avoid duplicate exact edge (linear scan small expected degree)
        if adj[u as usize].iter().any(|(x,_)| *x==v) { continue; }
        let w = w_min + span * ((next_u32() as f32) / (u32::MAX as f32));
        adj[u as usize].push((v,w)); edges += 1;
    }
    for list in &mut adj { list.sort_by_key(|(v,_)| *v); }
    let mut offsets = Vec::with_capacity(n as usize +1); offsets.push(0);
    let mut targets = Vec::new(); let mut weights = Vec::new();
    for u in 0..n as usize { for (v,w) in &adj[u] { targets.push(*v); weights.push(*w); } offsets.push(targets.len() as u32); }
    CsrGraph { n, offsets, targets, weights }
}

#[test]
fn parity_core_small_graphs(){
    let graphs = vec![
        path_graph(10,1.0),
        star_graph(12,1.0),
        bridge_cliques(4,4,1.0),
        complete_graph(6,1.0),
    ];
    std::env::set_var("SSSP_SPEC_K","3");
    std::env::set_var("SSSP_SPEC_PIVOT_MAX","4");
    std::env::set_var("SSSP_SPEC_CHAIN_K","3");
    for g in &graphs {
        let (bdist,_bpred,_binfo) = run_variant("baseline", g, 0);
        let bhash = hash_dist(&bdist);
        for variant in ["phase1","phase2","phase3","chain"] { let (dist,_pred,_info) = run_variant(variant,g,0); assert_parity(&bdist,&dist,1e-5); let h = hash_dist(&dist); assert_eq!(bhash,h, "hash mismatch variant {}", variant); }
    }
}

#[test]
fn parity_random_graphs(){
    std::env::set_var("SSSP_SPEC_K","4");
    std::env::set_var("SSSP_SPEC_PIVOT_MAX","5");
    for seed in 1..=5u64 { // moderate size to keep runtime reasonable
        let g = pseudo_random_graph(40, 160, seed * 7919, 0.5, 3.5);
        let (bdist,_bp,_bi) = run_variant("baseline", &g, 0);
        let bhash = hash_dist(&bdist);
        for variant in ["phase1","phase2","phase3","chain"] { let (dist,_p,_i) = run_variant(variant,&g,0); assert_parity(&bdist,&dist,1e-4); let h = hash_dist(&dist); assert_eq!(bhash,h, "hash mismatch variant {} seed {}", variant, seed); }
    }
}
