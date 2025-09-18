## Makefile (Rust core + Benchmark Scaffolding)

.PHONY: build clean scaling variants bench-sample bench-grid charts bench-python-sample md-table refresh-bench-assets help

RUST_BENCH_DIR=implementations/rust/sssp_core/benchmarks
PY_BENCH_SCRIPT=implementations/python/benchmark_rust_variants.py
GRID_JSON=$(RUST_BENCH_DIR)/native_grid.json
GRID_PREFIX=$(RUST_BENCH_DIR)/native_grid
SAMPLE_JSON=benchmarks/rust_spec_baseline_sample.json

build:
	cargo build --release --manifest-path implementations/rust/sssp_core/Cargo.toml

clean:
	rm -rf implementations/rust/sssp_core/target
	rm -f scaling_results.json rust_variant_bench.json $(GRID_JSON) $(GRID_PREFIX)_*.png $(SAMPLE_JSON) benchmarks/bench_table.md

variants: build
	python3 $(PY_BENCH_SCRIPT) --sizes 2000,4000,8000 --density 2.0

scaling: build
	python3 benchmarks/scaling_analysis.py --sizes 4000,8000,16000,32000 --density 2.0 --repeat 2

# Run native Rust harness sample (single degree)
bench-sample: build
	cargo run --manifest-path implementations/rust/sssp_core/Cargo.toml --example bench_spec -- --sizes 25000,50000,100000,250000 --degrees 4 --seed 42 --out $(SAMPLE_JSON) --full-parity --no-boundary-parity

# Run multi-degree grid for heatmaps & speedup chart
bench-grid: build
	cargo run --manifest-path implementations/rust/sssp_core/Cargo.toml --example bench_spec -- --sizes 25000,50000,100000 --degrees 2,4,8 --seed 42 --out $(GRID_JSON) --full-parity --no-boundary-parity

# Generate charts from grid JSON
charts: $(GRID_JSON)
	python3 $(RUST_BENCH_DIR)/generate_charts.py --input $(GRID_JSON) --out-prefix $(GRID_PREFIX) --phase phase3

# Legacy Python benchmark sample (baseline vs spec)
bench-python-sample: build
	python3 $(PY_BENCH_SCRIPT) --sizes 25000,50000,100000,250000,500000,1000000 --density 2.0 --seed 42 --output $(SAMPLE_JSON) --plot benchmarks/rust_spec_baseline_sample.png

# Convert SAMPLE_JSON to Markdown table snippet
md-table: $(SAMPLE_JSON)
	python3 benchmarks/json_to_md_table.py --input $(SAMPLE_JSON) --output benchmarks/bench_table.md

# Convenience: run grid + charts + table (sample) end-to-end
refresh-bench-assets: bench-grid charts bench-python-sample md-table

help:
	@echo 'Targets:'
	@echo '  build                - build rust core'
	@echo '  variants             - run baseline vs stoc timing (legacy)'
	@echo '  scaling              - run scaling analysis (legacy)'
	@echo '  bench-sample         - run native harness sample sizes (degree=4)'
	@echo '  bench-grid           - run native multi-degree grid benchmark'
	@echo '  charts               - generate PNG charts from grid JSON'
	@echo '  bench-python-sample  - run legacy python benchmark sample set'
	@echo '  md-table             - produce Markdown table from sample JSON'
	@echo '  refresh-bench-assets - grid + charts + python sample + md table'
	@echo '  clean                - remove build + result artifacts'