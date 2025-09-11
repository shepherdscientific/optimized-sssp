using System;
using System.Runtime.InteropServices;

namespace OptimizedSssp;

public static class RustSssp
{
    [StructLayout(LayoutKind.Sequential)]
    public struct SsspResultInfo
    {
        public ulong relaxations;
        public ulong light_relaxations;
        public ulong heavy_relaxations;
        public uint settled;
        public int error_code;
    }

    const string LIB = "sssp_core"; // ensure library on PATH / LD_LIBRARY_PATH / DYLD_LIBRARY_PATH

    [DllImport(LIB, EntryPoint="sssp_run_baseline", CallingConvention=CallingConvention.Cdecl)]
    private static extern int sssp_run_baseline(uint n, IntPtr offsets, IntPtr targets, IntPtr weights, uint source, IntPtr out_dist, IntPtr out_pred, ref SsspResultInfo info);
    [DllImport(LIB, EntryPoint="sssp_run_stoc", CallingConvention=CallingConvention.Cdecl)]
    private static extern int sssp_run_stoc(uint n, IntPtr offsets, IntPtr targets, IntPtr weights, uint source, IntPtr out_dist, IntPtr out_pred, ref SsspResultInfo info);
    [DllImport(LIB, EntryPoint="sssp_run_stoc_autotune", CallingConvention=CallingConvention.Cdecl)]
    private static extern int sssp_run_stoc_autotune(uint n, IntPtr offsets, IntPtr targets, IntPtr weights, uint source, IntPtr out_dist, IntPtr out_pred, ref SsspResultInfo info);
    [DllImport(LIB, EntryPoint="sssp_version", CallingConvention=CallingConvention.Cdecl)]
    private static extern uint sssp_version();

    public record Result(float[] Distances, int[] Predecessors, SsspResultInfo Info, uint Version);

    public static Result Run(uint n, uint[] offsets, uint[] targets, float[] weights, uint source, int mode)
    {
        if (offsets.Length != n + 1) throw new ArgumentException("offsets length must be n+1");
        var dist = new float[n];
        var pred = new int[n];
        var info = new SsspResultInfo();
        var hOff = GCHandle.Alloc(offsets, GCHandleType.Pinned);
        var hTgt = GCHandle.Alloc(targets, GCHandleType.Pinned);
        var hWts = GCHandle.Alloc(weights, GCHandleType.Pinned);
        var hDist = GCHandle.Alloc(dist, GCHandleType.Pinned);
        var hPred = GCHandle.Alloc(pred, GCHandleType.Pinned);
        try
        {
            int rc = mode switch {
                0 => sssp_run_baseline(n, hOff.AddrOfPinnedObject(), hTgt.AddrOfPinnedObject(), hWts.AddrOfPinnedObject(), source, hDist.AddrOfPinnedObject(), hPred.AddrOfPinnedObject(), ref info),
                1 => sssp_run_stoc(n, hOff.AddrOfPinnedObject(), hTgt.AddrOfPinnedObject(), hWts.AddrOfPinnedObject(), source, hDist.AddrOfPinnedObject(), hPred.AddrOfPinnedObject(), ref info),
                2 => sssp_run_stoc_autotune(n, hOff.AddrOfPinnedObject(), hTgt.AddrOfPinnedObject(), hWts.AddrOfPinnedObject(), source, hDist.AddrOfPinnedObject(), hPred.AddrOfPinnedObject(), ref info),
                _ => throw new ArgumentException("invalid mode")
            };
            if (rc != 0) throw new InvalidOperationException($"Rust core returned error {rc}");
            return new Result(dist, pred, info, sssp_version());
        }
        finally
        {
            hOff.Free(); hTgt.Free(); hWts.Free(); hDist.Free(); hPred.Free();
        }
    }
}
