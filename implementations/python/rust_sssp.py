import ctypes
import os
import math
from typing import Tuple, Dict, List

# Locate shared library (assumes built in rust/sssp_core/target/release)
LIB_PATH_CANDIDATES = [
    os.path.join(os.path.dirname(__file__), '..', 'rust', 'sssp_core', 'target', 'release', 'libsssp_core.dylib'),
    os.path.join(os.path.dirname(__file__), '..', 'rust', 'sssp_core', 'target', 'release', 'libsssp_core.so'),
    os.path.join(os.path.dirname(__file__), '..', 'rust', 'sssp_core', 'target', 'release', 'sssp_core.dll'),
]

_lib = None
for p in LIB_PATH_CANDIDATES:
    if os.path.exists(p):
        _lib = ctypes.CDLL(p)
        break
if _lib is None:
    raise RuntimeError("Rust SSSP shared library not found. Build with `cargo build --release`.")

class SsspResultInfo(ctypes.Structure):
    _fields_ = [
        ("relaxations", ctypes.c_uint64),
    ("light_relaxations", ctypes.c_uint64),
    ("heavy_relaxations", ctypes.c_uint64),
        ("settled", ctypes.c_uint32),
        ("error_code", ctypes.c_int32),
    ]

_lib.sssp_run_baseline.restype = ctypes.c_int32
_lib.sssp_run_baseline.argtypes = [ctypes.c_uint32, ctypes.POINTER(ctypes.c_uint32), ctypes.POINTER(ctypes.c_uint32), ctypes.POINTER(ctypes.c_float), ctypes.c_uint32, ctypes.POINTER(ctypes.c_float), ctypes.POINTER(ctypes.c_int32), ctypes.POINTER(SsspResultInfo)]
_HAS_STOC = hasattr(_lib, 'sssp_run_stoc')
if _HAS_STOC:
    _lib.sssp_run_stoc.restype = ctypes.c_int32
    _lib.sssp_run_stoc.argtypes = _lib.sssp_run_baseline.argtypes
_HAS_STOC_AUTOTUNE = hasattr(_lib, 'sssp_run_stoc_autotune')
if _HAS_STOC_AUTOTUNE:
    _lib.sssp_run_stoc_autotune = getattr(_lib, 'sssp_run_stoc_autotune')
    _lib.sssp_run_stoc_autotune.restype = ctypes.c_int32
    _lib.sssp_run_stoc_autotune.argtypes = _lib.sssp_run_baseline.argtypes
_HAS_STOC_AUTO_ADAPT = hasattr(_lib, 'sssp_run_stoc_auto_adapt')
if _HAS_STOC_AUTO_ADAPT:
    _lib.sssp_run_stoc_auto_adapt.restype = ctypes.c_int32
    _lib.sssp_run_stoc_auto_adapt.argtypes = _lib.sssp_run_baseline.argtypes
_HAS_KHOP = hasattr(_lib, 'sssp_run_khop')
if _HAS_KHOP:
    _lib.sssp_run_khop.restype = ctypes.c_int32
    _lib.sssp_run_khop.argtypes = _lib.sssp_run_baseline.argtypes
_HAS_DEFAULT = hasattr(_lib, 'sssp_run_default')
if _HAS_DEFAULT:
    _lib.sssp_run_default.restype = ctypes.c_int32
    _lib.sssp_run_default.argtypes = _lib.sssp_run_baseline.argtypes
_lib.sssp_version.restype = ctypes.c_uint32

# Optional bucket stats FFI
class _BucketStats(ctypes.Structure):
    _fields_=[('buckets_visited',ctypes.c_uint32),('light_pass_repeats',ctypes.c_uint32),('max_bucket_index',ctypes.c_uint32),('restarts',ctypes.c_uint32),('delta_x1000',ctypes.c_uint32),('heavy_ratio_x1000',ctypes.c_uint32)]
_HAS_BUCKET_STATS = hasattr(_lib, 'sssp_get_bucket_stats') and hasattr(_lib,'sssp_get_last_delta')
if _HAS_BUCKET_STATS:
    _lib.sssp_get_bucket_stats.argtypes=[ctypes.POINTER(_BucketStats)]
    _lib.sssp_get_last_delta.restype=ctypes.c_float

def get_bucket_stats():
    if not _HAS_BUCKET_STATS:
        return None
    bs=_BucketStats(); _lib.sssp_get_bucket_stats(ctypes.byref(bs))
    return {
        'buckets_visited': bs.buckets_visited,
        'light_pass_repeats': bs.light_pass_repeats,
        'max_bucket_index': bs.max_bucket_index,
        'restarts': bs.restarts,
        'delta': bs.delta_x1000 / 1000.0,
        'heavy_ratio': bs.heavy_ratio_x1000 / 1000.0,
    }

class _BaselineHeapStats(ctypes.Structure):
    _fields_=[('pushes',ctypes.c_uint64),('pops',ctypes.c_uint64),('max_size',ctypes.c_uint64)]
_HAS_BASE_HEAP = hasattr(_lib,'sssp_get_baseline_heap_stats')
if _HAS_BASE_HEAP:
    _lib.sssp_get_baseline_heap_stats.argtypes=[ctypes.POINTER(_BaselineHeapStats)]

def get_baseline_heap_stats():
    if not _HAS_BASE_HEAP:
        return None
    hs=_BaselineHeapStats(); _lib.sssp_get_baseline_heap_stats(ctypes.byref(hs))
    return {'pushes': hs.pushes, 'pops': hs.pops, 'max_size': hs.max_size}


def run_baseline(offsets, targets, weights, source: int):
    return _run(offsets, targets, weights, source, False)

def run_optimized(*_args, **_kwargs):
    raise RuntimeError("Optimized variant removed; only baseline and stoc available")

def run_hybrid(*_args, **_kwargs):
    raise RuntimeError("Hybrid variant removed; only baseline and stoc available")

def run_stoc(offsets, targets, weights, source: int):
    if not _HAS_STOC:
        raise RuntimeError("STOC (delta-stepping) function not available in loaded library")
    return _run(offsets, targets, weights, source, 'stoc')

def run_stoc_autotune(offsets, targets, weights, source: int):
    if not _HAS_STOC_AUTOTUNE:
        raise RuntimeError("STOC autotune function not available in loaded library")
    return _run(offsets, targets, weights, source, 'stoc_autotune')

def run_stoc_auto_adapt(offsets, targets, weights, source: int):
    if not _HAS_STOC_AUTO_ADAPT:
        raise RuntimeError("Unified autotune+adaptive function not available")
    return _run(offsets, targets, weights, source, 'stoc_auto_adapt')

def run_khop(offsets, targets, weights, source: int):
    if not _HAS_KHOP:
        raise RuntimeError("k-hop experimental function not available")
    return _run(offsets, targets, weights, source, 'khop')

def run_default(offsets, targets, weights, source: int):
    if _HAS_DEFAULT:
        return _run(offsets, targets, weights, source, 'default')
    # Fallback to batched khop if default not present
    return run_khop(offsets, targets, weights, source)

def _run(offsets, targets, weights, source: int, mode):
    n = len(offsets) - 1
    m = len(targets)
    assert len(weights) == m
    OffArr = (ctypes.c_uint32 * (n + 1))(*offsets)
    TgtArr = (ctypes.c_uint32 * m)(*targets)
    WArr = (ctypes.c_float * m)(*weights)
    DistArr = (ctypes.c_float * n)()
    PredArr = (ctypes.c_int32 * n)()
    info = SsspResultInfo()
    if mode == 'stoc':
        fn = _lib.sssp_run_stoc; variant = 'stoc'
    elif mode == 'stoc_autotune':
        fn = _lib.sssp_run_stoc_autotune; variant = 'stoc_autotune'
    elif mode == 'stoc_auto_adapt':
        fn = _lib.sssp_run_stoc_auto_adapt; variant = 'stoc_auto_adapt'
    elif mode == 'khop':
        fn = _lib.sssp_run_khop; variant = 'khop'
    elif mode == 'default':
        fn = _lib.sssp_run_default; variant = 'default'
    else:
        fn = _lib.sssp_run_baseline; variant = 'baseline'
    rc = fn(n, OffArr, TgtArr, WArr, source, DistArr, PredArr, ctypes.byref(info))
    if rc != 0:
        raise RuntimeError(f"Rust core returned error {rc}")
    return (
        [DistArr[i] for i in range(n)],
        [PredArr[i] for i in range(n)],
        {
            'relaxations': info.relaxations,
            'light_relaxations': info.light_relaxations,
            'heavy_relaxations': info.heavy_relaxations,
            'settled': info.settled,
            'version': _lib.sssp_version(),
            'variant': variant
        }
    )
