# Minimal API (Rust core + Python wrapper)

This repository has been trimmed to two exported single-source shortest path algorithms via a stable C ABI and a thin Python ctypes wrapper:

* Baseline Dijkstra (binary heap)
* Delta-stepping (named STOC variant here) + autotune variant

## Rust C ABI
Symbols:
```
int32_t sssp_run_baseline(..., SsspResultInfo* info);
int32_t sssp_run_stoc(..., SsspResultInfo* info);
int32_t sssp_run_stoc_autotune(..., SsspResultInfo* info);
uint32_t sssp_version(); // currently 4
uint64_t sssp_info_light_relaxations(const SsspResultInfo*);
uint64_t sssp_info_heavy_relaxations(const SsspResultInfo*);
```

Result struct:
```
typedef struct SsspResultInfo {
    uint64_t relaxations;
    uint64_t light_relaxations;
    uint64_t heavy_relaxations;
    uint32_t settled;
    int32_t  error_code; // 0 success
} SsspResultInfo;
```

## Environment Variables
```
SSSP_STOC_DELTA_MULT       # multiplier for fixed delta (default 3.0)
SSSP_STOC_AUTOTUNE_SET     # comma list of multipliers for autotune (default 1.5,2,3,4,6)
SSSP_STOC_AUTOTUNE_LIMIT   # node settle cap in trial runs (default 2048)
```

## Python Usage
```python
from rust_sssp import run_baseline, run_stoc, run_stoc_autotune
dist, pred, stats = run_stoc(offsets, targets, weights, source=0)
print(stats)
```

Offsets, targets, weights form a CSR graph:
* offsets: length n+1 (u's outgoing edges are e in [offsets[u], offsets[u+1]))
* targets: length m
* weights: length m (float32 internally)

## Scaling Analysis
Use `benchmarks/scaling_analysis.py` to produce empirical factors vs theoretical m·log n and m·log^{2/3} n terms:
```bash
python benchmarks/scaling_analysis.py --sizes 4000,8000,16000,32000 --density 2.0 --repeat 2
```
Output JSON fields include normalized factors:
* baseline_time_per_mlogn
* stoc_time_per_mlog23

Relative stability across n suggests consistency with the assumed complexity classes (heuristic, not a proof).

## Versioning
Increment `sssp_version` on any breaking ABI change (most recent bump: struct rename to light/heavy fields -> 4).

## Contributing
Focus on clarity, correctness, and instrumentation improvements; multi-language layers intentionally removed.
// Compute shortest paths (V2 recommended)
result, err := graph.ComputeShortestPathsV2(0)
if err != nil {
    log.Fatal(err)
}

fmt.Printf("Distance to node 1: %.2f\n", result.Distances[1])
```

### C# Library Usage

```csharp
using OptimizedSSSP.Algorithm;
using OptimizedSSP.Proto;

// Create graph from protobuf
var graph = new Graph();
// ... populate graph data

// Use V2 algorithm
var algorithm = new OptimizedSSSPV2Algorithm(graph);
var result = algorithm.ComputeShortestPaths(sourceNode: 0);

Console.WriteLine($"Execution time: {result.Metrics.TotalExecutionTimeNs / 1e6:F2} ms");
```

## 🔧 Core API Reference

### Go API

#### Graph Creation
```go
// Create empty graph
graph := optimized_sssp.NewGraph()

// Generate random test graph
graph, err := optimized_sssp.GenerateRandomGraph(
    nodeCount: 10000,
    edgeDensity: 2.5,
    seed: 12345
)
```

#### Graph Manipulation
```go
// Add nodes and edges
graph.AddNode(nodeID)
graph.AddEdge(source, target, weight)

// Query graph structure  
nodeCount := graph.NodeCount()
edgeCount := graph.EdgeCount()
edges := graph.GetEdges(nodeID)
```

#### Algorithm Execution
```go
// V2 Algorithm (recommended for large graphs)
result, err := graph.ComputeShortestPathsV2(sourceNode)

// V1 Algorithm (original implementation)
result, err := graph.ComputeShortestPaths(sourceNode)

// Dijkstra (for comparison)
result, err := graph.ComputeShortestPathsDijkstra(sourceNode)

// Compare all algorithms
results, err := graph.CompareAlgorithms(sourceNode)
```

### C# API

#### Algorithm Initialization
```csharp
// V2 Algorithm (corrected)
var algorithmV2 = new OptimizedSSSPV2Algorithm(graph);
var result = algorithmV2.ComputeShortestPaths(sourceNode);

// V1 Algorithm (original)  
var algorithmV1 = new OptimizedSSSPAlgorithm(graph);
var result = algorithmV1.ComputeShortestPaths(sourceNode);

// Dijkstra Reference
var dijkstra = new DijkstraAlgorithm(graph);
var result = dijkstra.ComputeShortestPaths(sourceNode);
```

## 📊 Result Structure

### ShortestPathResult (Go)
```go
type ShortestPathResult struct {
    SourceNode         int32                  // Starting node
    Distances          map[int32]float64      // Distance to each node
    Predecessors       map[int32]int32        // Shortest path tree
    ExecutionTime      int64                  // Nanoseconds
    Algorithm          string                 // "V1", "V2", "Dijkstra"
    Correctness        bool                   // Verified against Dijkstra
    
    // V2 Algorithm Metrics
    FrontierShrinks    int                    // Number of frontier reductions
    DijkstraRounds     int                    // Dijkstra iterations
    BellmanFordRounds  int                    // Bellman-Ford iterations
    ComplexityAnalysis map[string]interface{} // Theoretical analysis
}
```

### AlgorithmMetrics (Protobuf/C#)
```protobuf
message AlgorithmMetrics {
    int64 total_execution_time_ns = 1;
    int32 layers_created = 6;
    int32 clusters_formed = 7;
    int32 distance_updates = 10;
    map<string, double> custom_metrics = 15;  // V2 specific data
}
```

## 🎯 Performance Characteristics

### When to Use Each Algorithm

| Graph Size | V1 Original | V2 Corrected | Dijkstra | Recommendation |
|------------|-------------|--------------|----------|----------------|
| < 1K nodes | Slower | ~Same | **Fastest** | Use **Dijkstra** |
| 1K-100K nodes | Slower | ~Same | **Fastest** | Use **Dijkstra** or **V2** |
| 100K-1M nodes | Much slower | **Faster** | Good | Use **V2** |
| > 1M nodes | Much slower | **Fastest** | Good | Use **V2** |

### Complexity Comparison
- **V1**: O(m log n) - *incorrect implementation*
- **V2**: O(m log^(2/3) n) - *correct implementation*  
- **Dijkstra**: O(m + n log n) - *classical algorithm*

## 🔍 Advanced Usage Examples

### Custom Graph Loading
```go
// Load from JSON
jsonData := `{
  "nodes": [0, 1, 2, 3],
  "edges": [
    {"source": 0, "target": 1, "weight": 2.5},
    {"source": 1, "target": 2, "weight": 1.0}
  ]
}`
graph, err := optimized_sssp.LoadGraphFromJSON([]byte(jsonData))
```

### Performance Analysis
```go
result, _ := graph.ComputeShortestPathsV2(0)

// Access detailed metrics
analysis := result.ComplexityAnalysis
fmt.Printf("Theoretical complexity: %s\n", analysis["theoretical_complexity"])
fmt.Printf("Processed %d/%d nodes\n", 
    analysis["processed_nodes"], 
    analysis["nodes_n"])
```

### Cross-Language Data Exchange
```go
// Go side - export to protobuf
result, _ := graph.ComputeShortestPathsV2(0)
protobufData, _ := proto.Marshal(result.internal) // Hypothetical export

// C# side - import from protobuf  
var result = ShortestPathResult.Parser.ParseFrom(protobufData);
Console.WriteLine($"Distance to node 5: {result.Distances[5]}");
```

## 🚀 Production Integration

### Go Module Integration
```go
// go.mod
module your-application

require (
    github.com/optimized-sssp-benchmark/go v1.0.0
)

// main.go
import "github.com/optimized-sssp-benchmark/go/pkg/optimized_sssp"

func routingService(graph *optimized_sssp.Graph, source int32) map[int32]float64 {
    result, err := graph.ComputeShortestPathsV2(source)
    if err != nil {
        return nil
    }
    return result.Distances
}
```

### C# NuGet Integration
```xml
<PackageReference Include="OptimizedSSP" Version="1.0.0" />
```

```csharp
public class ShortestPathService
{
    public Dictionary<int, double> ComputeDistances(Graph graph, int source)
    {
        var algorithm = new OptimizedSSSPV2Algorithm(graph);
        var result = algorithm.ComputeShortestPaths(source);
        return result.Distances.ToDictionary(kv => (int)kv.Key, kv => kv.Value);
    }
}
```

## 🛡️ Error Handling & Best Practices

### Go Error Handling
```go
result, err := graph.ComputeShortestPathsV2(sourceNode)
if err != nil {
    switch {
    case strings.Contains(err.Error(), "algorithm failed"):
        // Handle algorithmic errors
        log.Printf("Algorithm error: %v", err)
        
    case strings.Contains(err.Error(), "invalid source"):
        // Handle invalid input
        return fmt.Errorf("source node %d not found in graph", sourceNode)
        
    default:
        // Handle unexpected errors
        return fmt.Errorf("unexpected error: %v", err)
    }
}

// Verify correctness for critical applications
dijkstraResult, _ := graph.ComputeShortestPathsDijkstra(sourceNode)
if !optimized_sssp.VerifyCorrectness(result, dijkstraResult) {
    log.Warning("Algorithm correctness verification failed")
}
```

### Performance Monitoring
```go
// Monitor execution time
start := time.Now()
result, err := graph.ComputeShortestPathsV2(sourceNode)
duration := time.Since(start)

// Log performance metrics
log.Printf("Graph: %d nodes, %d edges", graph.NodeCount(), graph.EdgeCount())
log.Printf("Execution: %.2fms, Frontier shrinks: %d", 
    duration.Seconds()*1000, result.FrontierShrinks)

// Alert on performance anomalies
expectedTime := estimateExecutionTime(graph.NodeCount(), graph.EdgeCount())
if duration > expectedTime*2 {
    log.Warning("Execution time %.2fx slower than expected", 
        float64(duration)/float64(expectedTime))
}
```

## 📈 Scaling Guidelines

### Memory Usage
- **V2 Algorithm**: O(n + frontier_size) memory
- **Frontier size**: Typically √n to n/10 depending on graph topology
- **Large graphs**: Monitor memory with `runtime.ReadMemStats()`

### Parallelization
- Currently single-threaded
- For all-pairs: Run multiple sources in parallel goroutines/tasks
- Memory isolation: Create separate graph instances per thread

### Graph Size Recommendations
- **Development/Testing**: Up to 100K nodes
- **Production**: 1M+ nodes (V2 algorithm shows best performance)
- **Enterprise**: Multi-million nodes (requires memory optimization)

This API provides a complete solution for integrating advanced shortest path algorithms into production systems with full cross-language compatibility.
\n+## 🔄 Current Minimal Rust Core (Simplified Scope)
The active Rust core (v3+) now exposes only classical Dijkstra (baseline) and a delta-stepping (STOC-style) variant plus an autotuned variant. Legacy V1/V2/cluster/layer constructs described above have been removed from the Rust FFI surface during simplification; documentation above remains for historical reference of higher-level language implementations.
\n+### C ABI Exports
```
int32_t sssp_run_baseline(..., SsspResultInfo* info);
int32_t sssp_run_stoc(..., SsspResultInfo* info);          // fixed multiplier (env SSSP_STOC_DELTA_MULT default 3.0)
int32_t sssp_run_stoc_autotune(..., SsspResultInfo* info); // probes candidates (env SSSP_STOC_AUTOTUNE_SET, LIMIT)
uint32_t sssp_version();
uint64_t sssp_info_light_relaxations(const SsspResultInfo*); // helper accessors
uint64_t sssp_info_heavy_relaxations(const SsspResultInfo*);
```
\n+### Result Struct (BREAKING compared to earlier heap field names)
```
typedef struct SsspResultInfo {
    uint64_t relaxations;        // total successful relax operations
    uint64_t light_relaxations;  // light-edge relaxations (delta-stepping)
    uint64_t heavy_relaxations;  // heavy-edge relaxations (delta-stepping)
    uint32_t settled;            // nodes settled
    int32_t  error_code;         // 0 success
} SsspResultInfo;
```
\n+### Autotune Environment Variables
```
SSSP_STOC_AUTOTUNE_SET   e.g. "1.5,2,3,4,6" (candidate multipliers; default 1.5,2,3,4,6)
SSSP_STOC_AUTOTUNE_LIMIT number of nodes to settle in trial runs (default 2048)
SSSP_STOC_DELTA_MULT     fixed multiplier when using sssp_run_stoc (default 3.0)
```
\n+### Python Convenience Functions
```
rust_sssp.run_baseline(...)
rust_sssp.run_stoc(...)
rust_sssp.run_stoc_autotune(...)
```
Return stats dict keys: relaxations, light_relaxations, heavy_relaxations, settled, variant, version.
\n+### Go Convenience Functions
```
RunRustBaseline(...)
RunRustStoc(...)
RunRustStocAutotune(...)
```
Each returns RustStats with Relaxations, LightRelaxations, HeavyRelaxations, Settled.
\n+### C# Convenience Method
```
RustSsspNative.Run(mode, ...)
// mode: 0 baseline, 1 stoc, 2 stoc autotune
```
\n+### Cross-Language Benchmark Script
`benchmarks/cross_language_runner.py` orchestrates a single graph across Rust (all three modes), Go Dijkstra, C# Dijkstra and emits consolidated JSON.
