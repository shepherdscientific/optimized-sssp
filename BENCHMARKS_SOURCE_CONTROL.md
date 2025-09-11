# 📊 Benchmark Source Control Strategy

## 🎯 **Recommendation: SELECTIVE INCLUSION**

**YES, benchmark runners should be under source control, but with strategic exclusions for large files.**

## 📁 **What TO Include in Source Control**

### ✅ **Source Code (Essential)**
```
✓ cmd/benchmark/main.go          # Benchmark runner source
✓ cmd/benchmark_v2/main.go       # V2 comparison runner  
✓ cmd/benchmark_v3/main.go       # V3 cache-optimized runner
✓ pkg/algorithm/*.go             # All algorithm implementations
✓ pkg/generator/*.go             # Graph generation utilities
✓ schemas/*.proto                # Protocol buffer schemas
```

### ✅ **Small Results (Examples & CI)**
```
✓ benchmarks/results/*_1k.json        # Small examples (~100KB)
✓ benchmarks/results/*_5k.json        # Medium examples (~500KB) 
✓ benchmarks/results/*_10k.json       # Larger examples (~1MB)
✓ benchmarks/results/v3_*.json        # V3 performance demos
✓ benchmarks/results/csharp_*.json    # Cross-language validation
✓ benchmarks/results/*.md             # Analysis reports
```

### ✅ **Configuration & Documentation**
```
✓ Makefile                           # Build automation
✓ README_API.md                      # Usage documentation
✓ examples/go_library_usage.go       # Integration examples
✓ .gitignore                         # Source control rules
```

## ❌ **What to EXCLUDE from Source Control**

### ❌ **Large Result Files (>10MB)**
```
✗ benchmarks/results/*_1M.json       # 131MB - too large!
✗ benchmarks/results/*_2M.json       # 263MB - excessive!
✗ benchmarks/results/v2_*.json       # 37-263MB - bloated
✗ benchmarks/results/go_benchmark.json # 124MB - redundant
```

### ❌ **Build Artifacts**
```
✗ implementations/go/benchmark        # 5.8MB compiled binary
✗ implementations/go/benchmark_v2     # 5.6MB compiled binary
✗ implementations/go/benchmark_v3     # 5.7MB compiled binary
✗ implementations/csharp/bin/         # .NET build outputs
✗ implementations/csharp/obj/         # .NET intermediate files
```

### ❌ **Generated Code** 
```
✗ implementations/go/github.com/      # Generated protobuf Go code
✗ implementations/csharp/obj/Debug/net8.0/*.cs # Generated protobuf C# code
```

## 🛠️ **Implementation Strategy**

### **1. Updated .gitignore** ✅
The repository now includes a comprehensive `.gitignore` that:
- **Excludes** large result files (>10MB)
- **Includes** small examples for documentation/CI
- **Excludes** build artifacts and generated code
- **Includes** all source code and schemas

### **2. Build Automation**
```makefile
# Makefile targets for reproducing results
benchmark-small:
	cd implementations/go && go build -o benchmark ./cmd/benchmark
	./implementations/go/benchmark -nodes=1000 -density=2.5

benchmark-v3-large:
	cd implementations/go && go build -o benchmark_v3 ./cmd/benchmark_v3  
	./implementations/go/benchmark_v3 -nodes=1000000 -density=2.5
```

### **3. Documentation Strategy**
Instead of storing large result files, we maintain:
- **Analysis reports** (`.md` files with key findings)
- **Small example results** for validation
- **Reproduction instructions** in README

## 📈 **Benefits of This Approach**

### ✅ **Repository Health**
- **Small repo size**: ~50MB instead of 500MB+
- **Fast clones**: No large binary downloads
- **Clean history**: Focus on code changes, not data

### ✅ **Development Efficiency**  
- **Fast builds**: No unnecessary large files
- **Clear diffs**: Code changes visible, not data noise
- **CI/CD friendly**: Quick pipeline execution

### ✅ **Reproducibility**
- **Source code preserved**: Anyone can rebuild benchmarks
- **Small examples included**: Validation and testing possible
- **Build scripts provided**: Easy reproduction of results

### ✅ **Professional Standards**
- **Industry best practice**: Source code in, data artifacts out
- **Academic compliance**: Methods preserved, results reproducible
- **Open source ready**: Clean, focused repository

## 🔄 **Workflow for Large Benchmarks**

### **For Development:**
```bash
# Generate fresh results locally
make benchmark-v3-large

# Analyze results
cat benchmarks/results/v3_1M.json | jq '.summary'

# Save analysis (not raw data) to source control
echo "Performance: 1.34x speedup achieved" >> CHANGELOG.md
```

### **For CI/CD:**
```yaml
# GitHub Actions example
- name: Run Performance Tests
  run: |
    make benchmark-small
    # Validate correctness, not absolute performance
    if ! grep -q "correctness.*true" benchmarks/results/v3_1k.json; then
      exit 1
    fi
```

### **For Research/Publication:**
```bash
# Generate full dataset locally
make benchmark-all-scales

# Archive results separately (Zenodo, institutional storage)
tar -czf benchmark-data-v1.0.tar.gz benchmarks/results/*_[1-9]M.json

# Include DOI in paper, not raw files in repo
```

## 🎯 **Final Recommendation**

**Store benchmark SOURCE CODE in version control, exclude large RESULT FILES.**

This approach:
1. ✅ **Preserves reproducibility** (anyone can rebuild)
2. ✅ **Maintains clean repository** (focused on code)  
3. ✅ **Enables validation** (small examples included)
4. ✅ **Supports development** (fast, efficient workflows)
5. ✅ **Meets professional standards** (industry best practices)

The benchmark runners are valuable intellectual property and should absolutely be preserved in source control. The large result files are ephemeral data artifacts that can be regenerated and should be managed separately.