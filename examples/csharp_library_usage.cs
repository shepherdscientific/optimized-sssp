using OptimizedSSSP.Library;
using System;
using System.Linq;

namespace OptimizedSSSP.Examples;

/// <summary>
/// Examples of using the Optimized SSSP Library - C# API
/// </summary>
class LibraryUsageExamples
{
    static void Main(string[] args)
    {
        Console.WriteLine("🚀 Optimized SSSP Library - C# Examples");
        Console.WriteLine("==========================================\n");

        // Example 1: Manual graph construction
        Console.WriteLine("=== Example 1: Manual Graph Construction ===");
        ManualGraphExample();

        // Example 2: Random graph generation  
        Console.WriteLine("\n=== Example 2: Random Graph Generation ===");
        RandomGraphExample();

        // Example 3: Algorithm comparison
        Console.WriteLine("\n=== Example 3: Algorithm Comparison ===");
        AlgorithmComparisonExample();

        // Example 4: Large-scale performance test
        Console.WriteLine("\n=== Example 4: Large-Scale Performance ===");
        LargeScaleExample();

        // Example 5: JSON import/export
        Console.WriteLine("\n=== Example 5: JSON Import/Export ===");
        JsonExample();
    }

    static void ManualGraphExample()
    {
        // Create a new graph
        var graph = new OptimizedSSSPGraph();

        // Add nodes manually
        for (int i = 0; i < 5; i++)
        {
            graph.AddNode(i);
        }

        // Add edges (creating a small test graph)
        graph.AddEdge(0, 1, 2.5);
        graph.AddEdge(0, 2, 1.0);
        graph.AddEdge(1, 3, 3.0);
        graph.AddEdge(2, 1, 1.5);
        graph.AddEdge(2, 3, 4.0);
        graph.AddEdge(3, 4, 2.0);

        Console.WriteLine($"Created graph with {graph.NodeCount} nodes and {graph.EdgeCount} edges");

        // Compute shortest paths using V2 algorithm
        var result = graph.ComputeShortestPathsV2(sourceNode: 0);

        Console.WriteLine($"Shortest paths from node {result.SourceNode}:");
        foreach (var kvp in result.Distances.Where(d => d.Value < double.MaxValue))
        {
            Console.WriteLine($"  Node {kvp.Key}: distance {kvp.Value:F2}");
        }
        Console.WriteLine($"Execution time: {result.ExecutionTimeMs:F2} ms");
    }

    static void RandomGraphExample()
    {
        // Generate a random graph
        var graph = OptimizedSSSPGraph.GenerateRandomGraph(
            nodeCount: 1000, 
            edgeDensity: 2.5, 
            seed: 12345);

        Console.WriteLine($"Generated random graph: {graph.NodeCount} nodes, {graph.EdgeCount} edges");

        // Use the V2 algorithm (recommended)
        int sourceNode = 0;
        var result = graph.ComputeShortestPathsV2(sourceNode);

        int reachableCount = result.Distances.Count(d => d.Value < double.MaxValue);

        Console.WriteLine($"Results: {reachableCount}/{graph.NodeCount} nodes reachable from node {sourceNode}");
        Console.WriteLine($"Execution time: {result.ExecutionTimeMs:F2} ms");
        Console.WriteLine($"Frontier shrinks: {result.FrontierShrinks}");
        Console.WriteLine($"Algorithm rounds: {result.DijkstraRounds} Dijkstra + {result.BellmanFordRounds} Bellman-Ford");
    }

    static void AlgorithmComparisonExample()
    {
        // Generate a medium-sized test graph
        var graph = OptimizedSSSPGraph.GenerateRandomGraph(
            nodeCount: 10000, 
            edgeDensity: 2.5, 
            seed: 67890);

        int sourceNode = 0;
        Console.WriteLine($"Comparing algorithms on {graph.NodeCount} node graph...");

        // Compare all available algorithms
        var comparison = graph.CompareAlgorithms(sourceNode);

        Console.WriteLine("\n" + comparison.GetSummary());

        // Show detailed analysis for V2
        if (comparison.Results.TryGetValue("V2_Corrected", out var v2Result) && 
            v2Result.ComplexityAnalysis != null)
        {
            Console.WriteLine($"\nV2 Complexity Analysis:");
            if (v2Result.ComplexityAnalysis.TryGetValue("theoretical_complexity", out var complexity))
            {
                Console.WriteLine($"  Theoretical: {complexity}");
            }
        }

        Console.WriteLine($"\nBest performing algorithm: {comparison.BestAlgorithm}");
    }

    static void LargeScaleExample()
    {
        // Test on a large graph to demonstrate V2 advantages
        int nodeCount = 100000;
        var graph = OptimizedSSSPGraph.GenerateRandomGraph(
            nodeCount: nodeCount, 
            edgeDensity: 2.5, 
            seed: 98765);

        Console.WriteLine($"Large-scale test: {graph.NodeCount} nodes, {graph.EdgeCount} edges");

        int sourceNode = 0;

        // Test V2 algorithm
        Console.Write("Running V2 algorithm... ");
        var startTime = DateTime.Now;
        var v2Result = graph.ComputeShortestPathsV2(sourceNode);
        var v2Duration = DateTime.Now - startTime;
        Console.WriteLine($"{v2Duration.TotalMilliseconds:F2} ms");

        // Test Dijkstra for comparison
        Console.Write("Running Dijkstra algorithm... ");
        startTime = DateTime.Now;
        var dijkstraResult = graph.ComputeShortestPathsDijkstra(sourceNode);
        var dijkstraDuration = DateTime.Now - startTime;
        Console.WriteLine($"{dijkstraDuration.TotalMilliseconds:F2} ms");

        // Verify correctness
        bool correct = true;
        const double tolerance = 1e-9;
        
        foreach (var kvp in v2Result.Distances)
        {
            if (dijkstraResult.Distances.TryGetValue(kvp.Key, out double dijkstraDist))
            {
                if (Math.Abs(kvp.Value - dijkstraDist) > tolerance)
                {
                    correct = false;
                    break;
                }
            }
            else
            {
                correct = false;
                break;
            }
        }

        // Display results
        Console.WriteLine("\nLarge-Scale Results:");
        Console.WriteLine($"V2 Algorithm:     {v2Duration.TotalMilliseconds:F2} ms");
        Console.WriteLine($"Dijkstra:         {dijkstraDuration.TotalMilliseconds:F2} ms");
        Console.WriteLine($"Speedup Factor:   {dijkstraDuration.TotalMilliseconds / v2Duration.TotalMilliseconds:F2}x");
        Console.WriteLine($"Correctness:      {(correct ? "✓" : "✗")}");

        if (v2Result.ComplexityAnalysis != null && 
            v2Result.ComplexityAnalysis.TryGetValue("theoretical_complexity", out var complexity))
        {
            Console.WriteLine($"Theoretical:      {complexity}");
        }

        Console.WriteLine($"Frontier Shrinks: {v2Result.FrontierShrinks}");
        Console.WriteLine($"Algorithm Mix:    {v2Result.DijkstraRounds} Dijkstra + {v2Result.BellmanFordRounds} Bellman-Ford rounds");
    }

    static void JsonExample()
    {
        // Create a simple graph
        var graph = new OptimizedSSSPGraph();
        graph.AddNode(0);
        graph.AddNode(1);
        graph.AddNode(2);
        graph.AddEdge(0, 1, 5.0);
        graph.AddEdge(1, 2, 3.0);
        graph.AddEdge(0, 2, 10.0);

        // Export to JSON
        string graphJson = graph.ToJson();
        Console.WriteLine("Graph JSON:");
        Console.WriteLine(graphJson);

        // Compute shortest paths
        var result = graph.ComputeShortestPathsV2(0);

        // Export result to JSON
        string resultJson = result.ToJson();
        Console.WriteLine("\nResult JSON:");
        Console.WriteLine(resultJson);

        // Load graph from JSON (roundtrip test)
        var loadedGraph = OptimizedSSSPGraph.LoadFromJson(graphJson);
        Console.WriteLine($"\nLoaded graph: {loadedGraph.NodeCount} nodes, {loadedGraph.EdgeCount} edges");

        // Verify loaded graph produces same results
        var loadedResult = loadedGraph.ComputeShortestPathsV2(0);
        bool identical = result.Distances.All(kvp => 
            loadedResult.Distances.TryGetValue(kvp.Key, out double loadedDist) && 
            Math.Abs(kvp.Value - loadedDist) < 1e-9);

        Console.WriteLine($"Roundtrip verification: {(identical ? "✓ Identical" : "✗ Different")}");
    }
}

/// <summary>
/// Advanced usage example for production integration
/// </summary>
public class ProductionIntegrationExample
{
    /// <summary>
    /// Example routing service using the Optimized SSSP library
    /// </summary>
    public class RoutingService
    {
        private readonly OptimizedSSSPGraph _graph;

        public RoutingService(OptimizedSSSPGraph graph)
        {
            _graph = graph;
        }

        /// <summary>
        /// Finds shortest distances from a source to all reachable nodes
        /// </summary>
        public Dictionary<int, double> FindShortestDistances(int sourceNode)
        {
            try
            {
                // Use V2 for best performance on large graphs
                var result = _graph.ComputeShortestPathsV2(sourceNode);
                
                if (!result.IsSuccessful)
                {
                    throw new InvalidOperationException($"Shortest path computation failed: {result.ErrorMessage}");
                }

                // Filter out unreachable nodes
                return result.Distances
                    .Where(kvp => kvp.Value < double.MaxValue)
                    .ToDictionary(kvp => kvp.Key, kvp => kvp.Value);
            }
            catch (Exception ex)
            {
                // Log error and fallback to Dijkstra
                Console.WriteLine($"V2 algorithm failed, falling back to Dijkstra: {ex.Message}");
                
                var dijkstraResult = _graph.ComputeShortestPathsDijkstra(sourceNode);
                return dijkstraResult.Distances
                    .Where(kvp => kvp.Value < double.MaxValue)
                    .ToDictionary(kvp => kvp.Key, kvp => kvp.Value);
            }
        }

        /// <summary>
        /// Finds the shortest path between two specific nodes
        /// </summary>
        public (double Distance, List<int> Path) FindShortestPath(int sourceNode, int targetNode)
        {
            var result = _graph.ComputeShortestPathsV2(sourceNode);

            if (!result.Distances.TryGetValue(targetNode, out double distance) || 
                distance >= double.MaxValue)
            {
                return (double.PositiveInfinity, new List<int>());
            }

            // Reconstruct path using predecessors
            var path = new List<int>();
            int current = targetNode;

            while (current != -1 && current != sourceNode)
            {
                path.Add(current);
                result.Predecessors.TryGetValue(current, out current);
            }

            if (current == sourceNode)
            {
                path.Add(sourceNode);
                path.Reverse();
            }
            else
            {
                // Path reconstruction failed
                return (distance, new List<int>());
            }

            return (distance, path);
        }

        /// <summary>
        /// Performance monitoring for production deployment
        /// </summary>
        public void RunPerformanceMonitoring(int sampleSourceNode)
        {
            var comparison = _graph.CompareAlgorithms(sampleSourceNode);
            
            // Log performance metrics
            Console.WriteLine($"Performance monitoring results:");
            Console.WriteLine(comparison.GetSummary());

            // Alert on performance anomalies
            if (comparison.Results.TryGetValue("V2_Corrected", out var v2Result) &&
                comparison.Results.TryGetValue("Dijkstra", out var dijkstraResult))
            {
                double expectedSpeedup = EstimateExpectedSpeedup(_graph.NodeCount, _graph.EdgeCount);
                double actualSpeedup = dijkstraResult.ExecutionTimeMs / v2Result.ExecutionTimeMs;

                if (actualSpeedup < expectedSpeedup * 0.5)
                {
                    Console.WriteLine($"⚠️ Performance Warning: Expected {expectedSpeedup:F2}x speedup, got {actualSpeedup:F2}x");
                }
                else if (actualSpeedup > expectedSpeedup * 1.5)
                {
                    Console.WriteLine($"✅ Performance Excellent: Expected {expectedSpeedup:F2}x speedup, got {actualSpeedup:F2}x");
                }
            }
        }

        private double EstimateExpectedSpeedup(int nodes, int edges)
        {
            // Rough heuristic based on our benchmarks
            if (nodes < 100000) return 0.9;  // Small graphs favor Dijkstra
            if (nodes < 1000000) return 1.2; // Medium graphs show advantage
            return 1.3; // Large graphs show clear advantage
        }
    }
}