using System;
using OptimizedSssp;

class SmokeTest
{
    static void Main(){
        uint n=3;
        uint[] offsets={0,1,2,2};
        uint[] targets={1,2};
        float[] weights={1f,2f};
        var r = RustSssp.Run(n, offsets, targets, weights, 0, 0);
        if (Math.Abs(r.Distances[2]-3f) > 1e-6) throw new Exception("distance mismatch");
        Console.WriteLine("OK baseline distance to 2 = " + r.Distances[2]);
    }
}
