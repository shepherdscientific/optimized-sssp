## Minimal Makefile (Rust core + Python benchmarks)

.PHONY: build clean scaling variants

build:
	cargo build --release -p sssp_core

clean:
	rm -rf implementations/rust/sssp_core/target
	rm -f scaling_results.json rust_variant_bench.json

variants: build
	python implementations/python/benchmark_rust_variants.py --sizes 2000,4000,8000 --density 2.0

scaling: build
	python benchmarks/scaling_analysis.py --sizes 4000,8000,16000,32000 --density 2.0 --repeat 2

help:
	@echo 'Targets:'
	@echo '  build    - build rust core'
	@echo '  variants - run baseline vs stoc timing'
	@echo '  scaling  - run scaling analysis'
	@echo '  clean    - remove build + result artifacts'