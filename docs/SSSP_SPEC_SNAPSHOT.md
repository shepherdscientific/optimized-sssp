# SSSP Spec Snapshot (Derived from `sssp.txt` / paper excerpt)

This file captures the exact algorithm description we will implement, mapping paper terminology to code constructs.

## Parameters
- n = |V|, m = |E|, assume m = O(n) (constant degree transform if needed)
- k = floor(log^{1/3} n)
- t = floor(log^{2/3} n)

## High-Level Strategy
Divide-and-conquer over vertex distance ranges using recursive bounded multi-source shortest path (BMSSP) subproblems.
Each BMSSP level l (0..L where L = ceil(log n / t)) works with:
- Frontier set S size ≤ 2^{l t}
- Upper bound B > max_{x in S} d_hat[x]
- Invariant: Any incomplete vertex v with d(v) < B has a shortest path that visits some complete vertex in S.
Goal: discover all vertices whose true distance < B' (B' ≤ B) that depend on S and make them complete.

## Subroutines
### BaseCase (l = 0)
- S = {x}, x complete.
- Run mini-Dijkstra from x limited by bound B until either k+1 vertices gathered or frontier exhausted.
- If < k+1 vertices collected: successful, B' = B, U = collected set.
- Else: let B* = max distance in collected set; return B' = B*, U = {v : dist[v] < B'}.

### FindPivots(B, S)
Perform up to k Bellman-Ford style relaxation waves rooted at S (bounded by B) to collect W (union of wave layers).
If |W| exceeds k|S| early: return P = S, W (partial pivot reduction).
Else construct forest F of tight edges (distance equality) restricted to W; pick P = roots in S whose tree has ≥ k vertices.
Guarantee: size(P) ≤ |W|/k, every vertex in dependent universe either completed (in W) or depends on some pivot in P.

### BMSSP(l, B, S)
If l=0 -> BaseCase.
Else:
1. (P, W) = FindPivots(B, S)
2. Initialize data-structure D (capacity parameter M = 2^{(l-1) t}) with keys P and values dist_hat.
3. Loop while D non-empty and |U| < k * 2^{l t}:
   a. (S_i, B_i) = D.Pull()  (smallest ≤ M keys plus separating boundary B_i)
   b. Recurse: (B'_i, U_i) = BMSSP(l-1, B_i, S_i)
   c. Add U_i to U.
   d. Relax edges out of U_i (<= condition). Insert improved distances ≥ B_i into D; stage improvements in [B'_i, B_i) for BatchPrepend.
   e. BatchPrepend staged improvements plus (x, dist_hat[x]) for x in S_i with dist in [B'_i, B_i).
4. Termination:
   - Success: D empty -> return B' = B, U ∪ {x in W: dist_hat[x] < B}
   - Partial: |U| reaches k*2^{l t} (or loop ends with D non-empty) -> choose B' = last B'_i, return U plus restricted W.

## Data Structure D
Supports operations with amortized bounds given N insertions, block size limit M:
- Insert(key,value)
- BatchPrepend(batch) where batch all smaller than existing minima
- Pull(): extract up to M smallest keys with boundary x separating from remainder.

## Edge Relaxation Equality Rule
Always perform relax when d_hat[u] + w <= d_hat[v] (allow equality) to preserve tight-edge forest reuse across levels.

## Output
Final top-level call BMSSP(L, ∞, {s}) must succeed (since |U| ≤ n << k*2^{L t}); returns all vertices with correct distances.

## Invariants to Enforce in Code
1. S size constraint per level.
2. Incomplete vertex dependency invariant before each BMSSP call.
3. Distances only decrease; equality relax allowed.
4. Disjointness of U_i sets among siblings (enforced by strictly increasing B_i sequences).
5. Pivot bound: |P| ≤ |U| / k on success or ≤ |S| on partial.

This snapshot is frozen for implementation reference. Any deviation must update this file and bump a SPEC_VERSION constant.
