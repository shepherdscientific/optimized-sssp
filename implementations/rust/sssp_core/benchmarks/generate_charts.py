#!/usr/bin/env python3
"""Generate benchmark charts and heatmaps from JSON produced by native Rust harness.

Input JSON formats supported:
1. Array of per-run objects (fields: n, m, avg_degree, baseline_ms, phase3_ms, boundary_chain_ms, phase3_speedup, boundary_chain_speedup, relaxations_*) across multiple sizes and degrees.
2. Grid JSON (optional future): {"runs":[...same objects...]}.

Outputs (written to benchmarks/ by default):
- benchmark_sample.png (line chart baseline/spec times + speedup axis) if multiple sizes for single degree
- benchmark_multi_degree.png (facet or multi-line for degrees)
- heatmap_speedup.png (matrix n vs degree of phase3 speedup)
- heatmap_baseline.png (matrix n vs degree of baseline time)

Usage:
  python benchmarks/generate_charts.py --input benchmarks/native_sample.json \
      --out-prefix benchmarks/native_sample --phase phase3

Flags:
  --input PATH (JSON)
  --out-prefix PREFIX (without extension)
  --phase phase3|boundary_chain (which speedup/time to emphasize)
  --no-bc (omit boundary chain lines)
"""
import argparse, json, math, os
from typing import List, Dict, Any
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns

PHASE_FIELDS = {
    'phase3': ('phase3_ms','phase3_speedup'),
    'boundary_chain': ('boundary_chain_ms','boundary_chain_speedup')
}

def load_runs(path: str) -> List[Dict[str,Any]]:
    with open(path,'r') as f: data = json.load(f)
    if isinstance(data, dict) and 'runs' in data: return data['runs']
    if isinstance(data, list): return data
    raise ValueError('Unrecognized JSON format')


def build_tables(runs: List[Dict[str,Any]]):
    # group by degree
    by_deg: Dict[float, List[Dict[str,Any]]] = {}
    for r in runs:
        deg = float(r.get('avg_degree',0.0))
        by_deg.setdefault(deg, []).append(r)
    for lst in by_deg.values():
        lst.sort(key=lambda x: x['n'])
    return by_deg


def plot_line_charts(by_deg, phase_key, out_prefix, include_bc):
    primary_ms_field, speedup_field = PHASE_FIELDS[phase_key]
    degrees = sorted(by_deg.keys())
    # If one degree: size vs time (baseline + phase + speedup secondary axis)
    if len(degrees) == 1:
        deg = degrees[0]
        runs = by_deg[deg]
        sizes = [r['n'] for r in runs]
        base = [r['baseline_ms'] for r in runs]
        spec = [r[primary_ms_field] for r in runs]
        speed = [r[speedup_field] for r in runs]
        fig, ax1 = plt.subplots(figsize=(7,4))
        ax1.plot(sizes, base, label='Baseline ms', marker='o')
        ax1.plot(sizes, spec, label=f'{phase_key} ms', marker='o')
        ax1.set_xlabel('n')
        ax1.set_ylabel('Time (ms)')
        ax2 = ax1.twinx()
        ax2.plot(sizes, speed, label='Speedup', color='green', linestyle='--', marker='x')
        ax2.set_ylabel('Speedup')
        ax1.set_xscale('log')
        ax1.set_yscale('log')
        ax1.legend(loc='upper left')
        ax2.legend(loc='lower right')
        fig.tight_layout()
        fig.savefig(f'{out_prefix}_benchmark.png', dpi=130)
        plt.close(fig)
    else:
        # multi-degree line chart for speedup
        fig, ax = plt.subplots(figsize=(7,4))
        for deg in degrees:
            runs = by_deg[deg]
            sizes = [r['n'] for r in runs]
            speed = [r[speedup_field] for r in runs]
            ax.plot(sizes, speed, marker='o', label=f'deg={deg:g}')
        ax.set_xscale('log'); ax.set_xlabel('n'); ax.set_ylabel('Speedup'); ax.set_title(f'{phase_key} speedup vs baseline'); ax.legend(fontsize='small')
        fig.tight_layout(); fig.savefig(f'{out_prefix}_speedup_multi_degree.png', dpi=130); plt.close(fig)
    # Optional boundary chain comparative chart if multiple degrees and included
    if include_bc:
        bc_field = 'boundary_chain_ms'
        fig, ax = plt.subplots(figsize=(7,4))
        for deg in degrees:
            runs = by_deg[deg]
            sizes = [r['n'] for r in runs]
            vals = [r[bc_field] for r in runs]
            ax.plot(sizes, vals, marker='o', label=f'deg={deg:g}')
        ax.set_xscale('log'); ax.set_yscale('log'); ax.set_xlabel('n'); ax.set_ylabel('Boundary chain ms'); ax.set_title('Boundary chain times'); ax.legend(fontsize='small')
        fig.tight_layout(); fig.savefig(f'{out_prefix}_boundary_chain_times.png', dpi=130); plt.close(fig)


def plot_heatmaps(by_deg, phase_key, out_prefix):
    primary_ms_field, speedup_field = PHASE_FIELDS[phase_key]
    degrees = sorted(by_deg.keys())
    sizes = sorted({r['n'] for lst in by_deg.values() for r in lst})
    # Build matrices
    speed_mat = np.full((len(degrees), len(sizes)), np.nan)
    base_mat = np.full((len(degrees), len(sizes)), np.nan)
    for di, deg in enumerate(degrees):
        rmap = {r['n']: r for r in by_deg[deg]}
        for si, n in enumerate(sizes):
            if n in rmap:
                speed_mat[di, si] = rmap[n][speedup_field]
                base_mat[di, si] = rmap[n]['baseline_ms']
    # Speedup heatmap
    fig, ax = plt.subplots(figsize=(8,4))
    sns.heatmap(speed_mat, annot=False, cmap='viridis', xticklabels=sizes, yticklabels=[f'deg={d:g}' for d in degrees], ax=ax)
    ax.set_xlabel('n'); ax.set_ylabel('avg_degree'); ax.set_title(f'{phase_key} speedup')
    fig.tight_layout(); fig.savefig(f'{out_prefix}_heatmap_speedup.png', dpi=130); plt.close(fig)
    # Baseline heatmap (log color)
    fig, ax = plt.subplots(figsize=(8,4))
    # Avoid log(0); add small epsilon
    norm_base = np.log10(base_mat + 1e-9)
    sns.heatmap(norm_base, annot=False, cmap='magma', xticklabels=sizes, yticklabels=[f'deg={d:g}' for d in degrees], ax=ax)
    ax.set_xlabel('n'); ax.set_ylabel('avg_degree'); ax.set_title('Baseline time log10(ms)')
    fig.tight_layout(); fig.savefig(f'{out_prefix}_heatmap_baseline.png', dpi=130); plt.close(fig)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--input', required=True)
    ap.add_argument('--out-prefix', required=True)
    ap.add_argument('--phase', choices=list(PHASE_FIELDS.keys()), default='phase3')
    ap.add_argument('--no-bc', action='store_true', help='Skip boundary chain comparative charts')
    args = ap.parse_args()
    runs = load_runs(args.input)
    if not runs:
        print('No runs found, exiting.')
        return
    by_deg = build_tables(runs)
    plot_line_charts(by_deg, args.phase, args.out_prefix, include_bc=not args.no_bc)
    plot_heatmaps(by_deg, args.phase, args.out_prefix)
    print(f'Charts written with prefix {args.out_prefix}')

if __name__ == '__main__':
    main()
