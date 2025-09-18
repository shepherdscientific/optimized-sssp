#!/usr/bin/env python3
import json, argparse, sys

def fmt_ms(v):
    return f"{v:.2f}" if isinstance(v,(int,float)) else str(v)

def main():
    ap = argparse.ArgumentParser(description="Convert benchmark JSON (sample format) to markdown table.")
    ap.add_argument('--input', required=True)
    ap.add_argument('--output', required=True)
    ap.add_argument('--speedup-key', default='spec_speedup')
    args = ap.parse_args()

    with open(args.input) as f:
        data = json.load(f)

    # Accept either list at root or dict with 'results'
    if isinstance(data, list):
        results = data
    elif isinstance(data, dict):
        if 'results' in data and isinstance(data['results'], list):
            results = data['results']
        else:
            # maybe nested under 'data'
            cand = data.get('data')
            if isinstance(cand, list):
                results = cand
            else:
                print('Unrecognized JSON structure (no list root or results array)', file=sys.stderr)
                sys.exit(1)
    else:
        print('Unsupported JSON top-level type', file=sys.stderr)
        sys.exit(1)

    speed_key = args.speedup_key
    headers = ["n", "m", "baseline_ms", "spec_ms", speed_key]

    lines = []
    lines.append("| " + " | ".join(h.replace('_', ' ') for h in headers) + " |")
    lines.append("|" + "|".join(["---" for _ in headers]) + "|")

    for r in results:
        n = r.get('n')
        m = r.get('m')
        b = r.get('baseline_ms') or r.get('baseline_time_ms') or r.get('baseline')
        s = r.get('spec_ms') or r.get('phase3_ms') or r.get('spec_time_ms')
        sp = r.get(speed_key)
        if sp is None and speed_key == 'spec_speedup':
            # attempt derive
            try:
                if b and s:
                    sp = float(b) / float(s)
            except Exception:
                sp = None
        line = [
            str(n) if n is not None else '',
            str(m) if m is not None else '',
            fmt_ms(b) if b is not None else '',
            fmt_ms(s) if s is not None else '',
            (f"{float(sp):.3f}" if isinstance(sp,(int,float)) else (str(sp) if sp is not None else ''))
        ]
        lines.append("| " + " | ".join(line) + " |")

    with open(args.output, 'w') as out:
        out.write("\n".join(lines) + "\n")

    # Also print path for convenience
    print(f"Wrote table to {args.output}")

if __name__ == '__main__':
    main()
