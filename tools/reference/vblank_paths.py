"""Per-frame 68000 vblank-handler paths from a reference handler trace
(ticket #35, docs/reference/latch-timing.md).

    python vblank_paths.py local/captures/<dir> [--blocks]

Reads DIR/m68k_reads.tsv (run_audio_probe.py --m68k-trace) and the latch
writes of DIR/audio_taps.tsv. For each handler invocation in the trace
(the first read of 0x3b052 through the latch write): the interrupt latency
L (frame start to the first fetch, 68000 cycles at 8 MHz), the path cost P
(first fetch to the write) and the path's jump signature (every (from, to)
read pair that is not sequential: branches taken, calls, returns, the
strength-table reads). Groups the frames by signature and prints each
path's P values, L range and frames. With --blocks, also prints the
cycles between the first reads of the path's jump targets, in order — the
block costs the translation's accounting is checked against.
"""
import sys, collections
from pathlib import Path

CLK = 8_000_000.0
FRAME = 1 / 60.0
HANDLER = 0x3b052


def load_reads(path):
    rows = []
    with open(path) as f:
        next(f)
        for line in f:
            t, fr, a, d = line.rstrip('\n').split('\t')
            rows.append((float(t), int(fr), int(a, 16), int(d, 16)))
    return rows


def load_latch(path):
    out = []
    with open(path) as f:
        next(f)
        for line in f:
            t, fr, src, addr, data = line.rstrip('\n').split('\t')
            if src == 'latch':
                out.append((float(t), int(fr), int(data, 16)))
    return out


def invocations(reads, latch):
    """Yield (frame, t0, tw, reads in the handler, latch value) per run."""
    li = 0
    i = 0
    n = len(reads)
    while i < n:
        t, fr, a, d = reads[i]
        if a != HANDLER:
            i += 1
            continue
        t0 = t
        while li < len(latch) and latch[li][0] < t0:
            li += 1
        if li >= len(latch):
            return
        tw = latch[li][0]
        j = i
        body = []
        while j < n and reads[j][0] <= tw:
            body.append(reads[j])
            j += 1
        yield fr, t0, tw, body, latch[li][2]
        i = j


def signature(body):
    jumps = []
    prev = None
    for t, fr, a, d in body:
        if prev is not None and a != prev + 2 and a != prev:
            jumps.append((prev, a))
        prev = a
    return tuple(jumps)


def main(argv):
    d = Path(argv[0])
    blocks = '--blocks' in argv
    reads = load_reads(d / 'm68k_reads.tsv')
    latch = load_latch(d / 'audio_taps.tsv')
    groups = collections.OrderedDict()
    count = 0
    for fr, t0, tw, body, value in invocations(reads, latch):
        L = (t0 - fr * FRAME) * CLK
        P = (tw - t0) * CLK
        groups.setdefault(signature(body), []).append((fr, L, P, value, body))
        count += 1
    print(f"{count} handler invocations, {len(groups)} distinct paths")
    for sig, items in groups.items():
        Ps = sorted(set(round(p, 1) for _, _, p, _, _ in items))
        Ls = [round(l, 1) for _, l, _, _, _ in items]
        frames = [f for f, _, _, _, _ in items]
        more = '...' if len(frames) > 8 else ''
        print(f"\npath x{len(items)}: P={Ps} L=[{min(Ls)}..{max(Ls)}] frames={frames[:8]}{more} latch={sorted(set(v for *_, v, _ in items))}")
        shown = collections.Counter()
        for a, b in sig:
            shown[(a, b)] += 1
            if shown[(a, b)] <= 2:
                print(f"   {a:05x} -> {b:05x}")
            elif shown[(a, b)] == 3:
                print(f"   {a:05x} -> {b:05x} (x{sig.count((a, b))})")
        if blocks:
            targets = {b for _, b in sig}
            body = items[0][4]
            first = collections.OrderedDict()
            for t, fr, a, dd in body:
                if a in targets and a not in first:
                    first[a] = t
            prev_a, prev_t = HANDLER, body[0][0]
            out = []
            for a, t in first.items():
                out.append(f"{prev_a:05x}->{a:05x}:{(t - prev_t) * CLK:.0f}")
                prev_a, prev_t = a, t
            print("   blocks: " + " ".join(out))
    return 0


if __name__ == '__main__':
    raise SystemExit(main(sys.argv[1:]))
