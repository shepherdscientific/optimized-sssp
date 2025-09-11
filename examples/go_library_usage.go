package main

import (
	"fmt"
	"log"
	"time"

	"github.com/optimized-sssp-benchmark/go/pkg/optimized_sssp"
)

func main() {
	// Example 1: Manual graph construction
	fmt.Println("=== Example 1: Manual Graph Construction ===")
	manualGraphExample()

	// Example 2: Random graph generation
	fmt.Println("\n=== Example 2: Random Graph Generation ===")
	randomGraphExample()

	// Example 3: Algorithm comparison
	fmt.Println("\n=== Example 3: Algorithm Comparison ===")
	algorithmComparisonExample()

	// Example 4: Large-scale performance test
	fmt.Println("\n=== Example 4: Large-Scale Performance ===")
	largeScaleExample()
}

func manualGraphExample() {
	// Create a new graph
	graph := optimized_sssp.NewGraph()

	// Add nodes manually
	for i := int32(0); i < 5; i++ {
		graph.AddNode(i)
	}

	// Add edges (creating a small test graph)
	graph.AddEdge(0, 1, 2.5)
	graph.AddEdge(0, 2, 1.0)
	graph.AddEdge(1, 3, 3.0)
	graph.AddEdge(2, 1, 1.5)
	graph.AddEdge(2, 3, 4.0)
	graph.AddEdge(3, 4, 2.0)

	fmt.Printf("Created graph with %d nodes and %d edges\n", 
		graph.NodeCount(), graph.EdgeCount())

	// Compute shortest paths using optimized algorithm
	result, err := graph.ComputeShortestPaths(0)
	if err != nil {
		log.Fatalf("Failed to compute shortest paths: %v", err)
	}

	fmt.Printf("Shortest paths from node %d:\n", result.SourceNode)
	for node, distance := range result.Distances {
		if distance < 1e100 { // Reachable nodes
			fmt.Printf("  Node %d: distance %.2f\n", node, distance)
		}
	}
	fmt.Printf("Execution time: %.2f ms\n", float64(result.ExecutionTime)/1e6)
}

func randomGraphExample() {
	// Generate a random graph
	graph, err := optimized_sssp.GenerateRandomGraph(1000, 2.5, time.Now().UnixNano())
	if err != nil {
		log.Fatalf("Failed to generate graph: %v", err)
	}

	fmt.Printf("Generated random graph: %d nodes, %d edges\n", 
		graph.NodeCount(), graph.EdgeCount())

	// Use the optimized algorithm
	sourceNode := int32(0)
	result, err := graph.ComputeShortestPaths(sourceNode)
	if err != nil {
		log.Fatalf("optimized algorithm failed: %v", err)
	}

	reachableCount := 0
	for _, distance := range result.Distances {
		if distance < 1e100 {
			reachableCount++
		}
	}

	fmt.Printf("Results: %d/%d nodes reachable from node %d\n", 
		reachableCount, graph.NodeCount(), sourceNode)
	fmt.Printf("Execution time: %.2f ms\n", float64(result.ExecutionTime)/1e6)
	fmt.Printf("Frontier shrinks: %d\n", result.FrontierShrinks)
	fmt.Printf("Dijkstra rounds: %d, Bellman-Ford rounds: %d\n", 
		result.DijkstraRounds, result.BellmanFordRounds)
}

func algorithmComparisonExample() {
	// Generate a medium-sized test graph
	graph, err := optimized_sssp.GenerateRandomGraph(10000, 2.5, 12345)
	if err != nil {
		log.Fatalf("Failed to generate graph: %v", err)
	}

	sourceNode := int32(0)
	fmt.Printf("Comparing algorithms on %d node graph...\n", graph.NodeCount())

	// Compare all available algorithms
	results, err := graph.CompareAlgorithms(sourceNode)
	if err != nil {
		log.Fatalf("Comparison failed: %v", err)
	}

	fmt.Println("\nAlgorithm Performance Comparison:")
	fmt.Println("Algorithm       | Time (ms) | Correctness")
	fmt.Println("----------------|-----------|------------")
	
	for name, result := range results {
		timeMs := float64(result.ExecutionTime) / 1e6
		correctness := "N/A"
		if result.Correctness {
			correctness = "✓"
		} else if name != "Dijkstra" { // Dijkstra is reference
			correctness = "✗"
		}
		
		fmt.Printf("%-15s | %8.2f  | %s\n", name, timeMs, correctness)
	}

	// Calculate speedup factors
	if dijkstra, ok := results["Dijkstra"]; ok {
		if optimized, ok := results["OptimizedSSP"]; ok {
			speedup := float64(dijkstra.ExecutionTime) / float64(optimized.ExecutionTime)
			fmt.Printf("\nOptimized Speedup vs Dijkstra: %.2fx %s\n", 
				speedup, 
				map[bool]string{true: "faster", false: "slower"}[speedup > 1])
		}
	}
}

func largeScaleExample() {
	// Test on a large graph to demonstrate optimized algorithm advantages
	nodeCount := int32(100000)
	graph, err := optimized_sssp.GenerateRandomGraph(nodeCount, 2.5, 67890)
	if err != nil {
		log.Fatalf("Failed to generate large graph: %v", err)
	}

	fmt.Printf("Large-scale test: %d nodes, %d edges\n", 
		graph.NodeCount(), graph.EdgeCount())

	sourceNode := int32(0)

	// Test optimized algorithm
	fmt.Println("Running optimized algorithm...")
	optimizedStart := time.Now()
	optimizedResult, err := graph.ComputeShortestPaths(sourceNode)
	optimizedDuration := time.Since(optimizedStart)
	
	if err != nil {
		log.Fatalf("optimized algorithm failed: %v", err)
	}

	// Test Dijkstra for comparison
	fmt.Println("Running Dijkstra algorithm...")
	dijkstraStart := time.Now()
	dijkstraResult, err := graph.ComputeShortestPathsDijkstra(sourceNode)
	dijkstraDuration := time.Since(dijkstraStart)
	
	if err != nil {
		log.Fatalf("Dijkstra failed: %v", err)
	}

	// Verify correctness
	correct := optimized_sssp.VerifyCorrectness(optimizedResult, dijkstraResult)

	// Display results
	fmt.Println("\nLarge-Scale Results:")
	fmt.Printf("Optimized:        %.2f ms\n", optimizedDuration.Seconds()*1000)
	fmt.Printf("Dijkstra:         %.2f ms\n", dijkstraDuration.Seconds()*1000)
	fmt.Printf("Speedup Factor:   %.2fx\n", dijkstraDuration.Seconds()/optimizedDuration.Seconds())
	fmt.Printf("Correctness:      %v\n", correct)
	
	if complexity, ok := optimizedResult.ComplexityAnalysis["theoretical_complexity"].(string); ok {
		fmt.Printf("Theoretical:      %s\n", complexity)
	}
	
	fmt.Printf("Frontier Shrinks: %d\n", optimizedResult.FrontierShrinks)
	fmt.Printf("Algorithm Mix:    %d Dijkstra + %d Bellman-Ford rounds\n", 
		optimizedResult.DijkstraRounds, optimizedResult.BellmanFordRounds)
	if optimizedResult.CacheMetrics != nil {
		fmt.Println("Cache-optimized data structures active")
	}

	// Save results to JSON for analysis
	if jsonData, err := optimizedResult.ToJSON(); err == nil {
		fmt.Println("\nResult JSON structure available for external analysis")
		fmt.Printf("JSON size: %d characters\n", len(jsonData))
	}
}