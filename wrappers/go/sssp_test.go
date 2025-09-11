package sssp

import "testing"

func TestRunBaselineSmall(t *testing.T) {
	// Simple 3-node chain 0->1->2
	off := []uint32{0, 1, 2, 2}
	tgt := []uint32{1, 2}
	wts := []float32{1.0, 2.0}
	res, err := Run(3, off, tgt, wts, 0, 0)
	if err != nil {
		t.Fatalf("err: %v", err)
	}
	if len(res.Dist) != 3 {
		t.Fatalf("unexpected dist len")
	}
	if res.Dist[2] != 3.0 {
		t.Fatalf("expected distance 3 got %v", res.Dist[2])
	}
}
