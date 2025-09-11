package sssp

/*
#cgo LDFLAGS: -L${SRCDIR}/../../implementations/rust/sssp_core/target/release -lsssp_core
#include <stdint.h>

typedef struct SsspResultInfo {
  uint64_t relaxations;
  uint64_t light_relaxations;
  uint64_t heavy_relaxations;
  uint32_t settled;
  int32_t  error_code;
} SsspResultInfo;

int32_t sssp_run_baseline(uint32_t n, const uint32_t* offsets, const uint32_t* targets,
                          const float* weights, uint32_t source, float* out_dist,
                          int32_t* out_pred, SsspResultInfo* info);
int32_t sssp_run_stoc(uint32_t n, const uint32_t* offsets, const uint32_t* targets,
                      const float* weights, uint32_t source, float* out_dist,
                      int32_t* out_pred, SsspResultInfo* info);
int32_t sssp_run_stoc_autotune(uint32_t n, const uint32_t* offsets, const uint32_t* targets,
                      const float* weights, uint32_t source, float* out_dist,
                      int32_t* out_pred, SsspResultInfo* info);
uint32_t sssp_version();
*/
import "C"
import "unsafe"

// Result holds algorithm outputs.
type Result struct {
	Dist  []float32
	Pred  []int32
	Stats Stats
}

// Stats mirrors the Rust SsspResultInfo.
type Stats struct {
	Relaxations      uint64
	LightRelaxations uint64
	HeavyRelaxations uint64
	Settled          uint32
	ErrorCode        int32
	Version          uint32
}

// Run executes a selected variant: 0 baseline, 1 stoc, 2 autotune.
func Run(n uint32, offsets, targets []uint32, weights []float32, source uint32, mode int) (Result, error) {
	dist := make([]float32, n)
	pred := make([]int32, n)
	var info C.SsspResultInfo
	var rc C.int32_t
	switch mode {
	case 0:
		rc = C.sssp_run_baseline(C.uint32_t(n), (*C.uint32_t)(unsafe.Pointer(&offsets[0])), (*C.uint32_t)(unsafe.Pointer(&targets[0])), (*C.float)(unsafe.Pointer(&weights[0])), C.uint32_t(source), (*C.float)(unsafe.Pointer(&dist[0])), (*C.int32_t)(unsafe.Pointer(&pred[0])), &info)
	case 1:
		rc = C.sssp_run_stoc(C.uint32_t(n), (*C.uint32_t)(unsafe.Pointer(&offsets[0])), (*C.uint32_t)(unsafe.Pointer(&targets[0])), (*C.float)(unsafe.Pointer(&weights[0])), C.uint32_t(source), (*C.float)(unsafe.Pointer(&dist[0])), (*C.int32_t)(unsafe.Pointer(&pred[0])), &info)
	case 2:
		rc = C.sssp_run_stoc_autotune(C.uint32_t(n), (*C.uint32_t)(unsafe.Pointer(&offsets[0])), (*C.uint32_t)(unsafe.Pointer(&targets[0])), (*C.float)(unsafe.Pointer(&weights[0])), C.uint32_t(source), (*C.float)(unsafe.Pointer(&dist[0])), (*C.int32_t)(unsafe.Pointer(&pred[0])), &info)
	default:
		return Result{}, nil
	}
	if rc != 0 {
		return Result{}, nil
	}
	return Result{Dist: dist, Pred: pred, Stats: Stats{Relaxations: uint64(info.relaxations), LightRelaxations: uint64(info.light_relaxations), HeavyRelaxations: uint64(info.heavy_relaxations), Settled: uint32(info.settled), ErrorCode: int32(info.error_code), Version: uint32(C.sssp_version())}}, nil
}
