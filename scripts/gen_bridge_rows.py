#!/home/jsc64/.local/oracle-venv/bin/python
"""
Generates tests/goldens/slices/bridge/bridge_rows.txt: slice S2 (src/bridge.rs, src/mag/convert.rs) edge rows computed with exact
Python fractions.Fraction, the independent cross-check of the exact model tests/model/bridge.rs (CONTRACT 16.S2.8 "Model selftest").
    ~/.local/oracle-venv/bin/python scripts/gen_bridge_rows.py              write the golden (only inside a popslot/poprun scope on Pop)
    ~/.local/oracle-venv/bin/python scripts/gen_bridge_rows.py --check DIR  regenerate into DIR and compare byte for byte (tools/goldens_regen.sh)
    ~/.local/oracle-venv/bin/python scripts/gen_bridge_rows.py --selftest   self-checks of this generator only

TOKENS. A number X is `0` or `<s>e<exp>:<w1>.<w2>...` with s in {+,-}, exp a decimal integer and wK 16-hex-digit u64 words, w1 nonzero:
X = s * sum_K wK * 2^(exp - 64K) (so a single word with its top bit set is `0.w1 * 2^exp`, the Mag convention, CONTRACT 1.2).
A Mag M is `0` or `m<mant>e<exp>` (mant decimal in [2^63, 2^64)). ORD is L, E or G.
ROWS (one per line, generation order, streamed; nothing but the current row is held):
    CONV X UPMANT UPEXP LOMANT LOEXP   from_exact_num(X) and from_exact_num_lower(X) (zero: 0 0 0 0)   16.S2.1, 16.S2.2
    CMPN M X ORD                       value(M) against |X|                                           16.S2.5
    CMPD A B R S ORD                   A - B against R + S                                           16.S2.6
Exponents stay within +-4096, so every value is an ordinary Fraction; the zf/Mag range edges are hand rows in tests/harness/slice_bridge.rs.
"""
import os
import sys
from fractions import Fraction

import goldlib   # first: popslot-scope guard and oracle check at import

GENERATOR_VERSION = 1
GOLDEN_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tests", "goldens", "slices", "bridge")
GOLDEN_NAME = "bridge_rows.txt"
TOP = 1 << 63
ONES = (1 << 64) - 1
SUITE_ID = 0x5332   # "S2"


def value(x):
    if x == 0:
        return Fraction(0)
    s, e, ws = x
    v = sum(Fraction(w) * Fraction(2) ** (e - 64 * (k + 1)) for k, w in enumerate(ws))
    return -v if s else v


def tok(x):
    if x == 0:
        return "0"
    s, e, ws = x
    return "%se%d:%s" % ("-" if s else "+", e, ".".join("%016x" % w for w in ws))


def mag_value(m):
    return Fraction(0) if m == 0 else Fraction(m[0]) * Fraction(2) ** (m[1] - 64)


def mtok(m):
    return "0" if m == 0 else "m%de%d" % m


def floor_log2(a):
    """e with 2^e <= a < 2^(e+1) for a Fraction a > 0."""
    e = a.numerator.bit_length() - a.denominator.bit_length()
    while Fraction(2) ** e > a:
        e -= 1
    while Fraction(2) ** (e + 1) <= a:
        e += 1
    return e


def to_mag(a, up):
    """The least (up) / greatest (down) Mag bounding a >= 0, as (mant, exp) or 0."""
    if a == 0:
        return 0
    e = floor_log2(a) + 1           # a in [2^(e-1), 2^e): the 0.m * 2^e convention
    scaled = a * Fraction(2) ** (64 - e)
    mant = -((-scaled.numerator) // scaled.denominator) if up else scaled.numerator // scaled.denominator
    if mant == 1 << 64:
        return (TOP, e + 1)
    return (mant, e)


def ordc(a, b):
    return "L" if a < b else ("E" if a == b else "G")


def pred(m):
    mant, e = m
    return (ONES, e - 1) if mant == TOP else (mant - 1, e)


def succ(m):
    mant, e = m
    return (TOP, e + 1) if mant == ONES else (mant + 1, e)


def fixed_values():
    yield (0, 1, [TOP])                      # 1
    yield (1, 1, [TOP])                      # -1
    yield (0, 2, [3 << 62])                  # 3
    yield (0, 0, [TOP + 1])
    yield (0, 5, [ONES])                     # all ones, no sticky
    yield (0, 5, [ONES, 1])                  # all ones + sticky: carry
    yield (1, -7, [ONES, TOP])               # carry, negative
    yield (0, 9, [TOP, 1])                   # sticky at the far end
    yield (0, 9, [TOP + 1, 0x8000000000000001])
    yield (0, 3, [1])                        # top word not normalised (one bit)
    yield (0, 3, [0x7FFFFFFFFFFFFFFF, ONES]) # 127 bits, top bit clear
    yield (1, 100, [0x0123456789ABCDEF, 0xFEDCBA9876543210])
    yield (0, -4000, [TOP, 0, 1])            # sticky two words down
    yield (0, 4000, [ONES, ONES, ONES])


def rand_value(rng, words_max, exp_lo, exp_hi):
    n = 1 + rng.next() % words_max
    ws = [rng.next() for _ in range(n)]
    if ws[0] == 0:
        ws[0] = 1
    e = exp_lo + rng.next() % (exp_hi - exp_lo + 1)
    return (rng.next() & 1, e, ws)


def cmpn_mags(x):
    a = abs(value(x))
    if a == 0:
        yield 0
        yield (TOP, 1)
        return
    up, lo = to_mag(a, True), to_mag(a, False)
    for m in (0, up, lo, pred(lo), succ(up)):
        yield m


def rows():
    rng = goldlib.SplitMix64(goldlib.stream_seed(SUITE_ID, 1))
    vals = list(fixed_values()) + [0]
    vals += [rand_value(rng, 4, -3000, 3000) for _ in range(40)]
    vals += [rand_value(rng, 16, -200, 200) for _ in range(4)]   # 1000-bit operands
    for x in vals:
        a = abs(value(x))
        up, lo = to_mag(a, True), to_mag(a, False)
        yield goldlib.fmt_row("CONV", tok(x), *(up if up else (0, 0)), *(lo if lo else (0, 0)))
    for x in vals:
        for m in cmpn_mags(x):
            yield goldlib.fmt_row("CMPN", mtok(m), tok(x), ordc(mag_value(m), abs(value(x))))
    # CMPD: pairs with the radii taken around |A - B| (touching edges, one ulp off), plus cancellation and random radii
    rng2 = goldlib.SplitMix64(goldlib.stream_seed(SUITE_ID, 2))
    pairs = [(vals[0], vals[0]), (vals[0], vals[1]), (vals[2], vals[1])]
    for _ in range(40):
        pairs.append((vals[rng2.next() % len(vals)], vals[rng2.next() % len(vals)]))
    for _ in range(6):   # cancellation: B = A plus a few low bits
        s, e, ws = rand_value(rng2, 16, -100, 100)
        ws2 = list(ws)
        ws2[-1] = ws2[-1] ^ (1 + rng2.next() % 255)
        pairs.append(((s, e, ws), (s, e, ws2)))
    for a, b in pairs:
        d = value(a) - value(b)
        ad = abs(d)
        cands = [(0, 0)]
        if ad != 0:
            up, lo = to_mag(ad, True), to_mag(ad, False)
            cands += [(up, 0), (0, lo), (lo, 0), (pred(lo), 0), (succ(up), 0)]
            half = to_mag(ad / 2, False)
            if mag_value(half) * 2 == ad:
                cands.append((half, half))       # touching: r + s = |A - B| exactly
        cands.append(((TOP, 1 + rng2.next() % 64 - 32), (TOP + rng2.next() % (1 << 62), rng2.next() % 64 - 32)))
        for r, s in cands:
            yield goldlib.fmt_row("CMPD", tok(a), tok(b), mtok(r), mtok(s), ordc(d, mag_value(r) + mag_value(s)))


def stream(path):
    with open(path, "wb") as f:
        f.write(("# gen_bridge_rows.py v%d (exact fractions.Fraction); see the docstring for the row format\n" % GENERATOR_VERSION).encode("ascii"))
        for r in rows():
            f.write((r + "\n").encode("ascii"))


def selftest():
    def check(cond, msg):
        if not cond:
            raise SystemExit("gen_bridge_rows selftest FAILED: " + msg)
    check(to_mag(Fraction(3), True) == (3 << 62, 2), "3 up")
    check(to_mag(Fraction(1), False) == (TOP, 1), "1 down")
    check(to_mag(value((0, 5, [ONES, 1])), True) == (TOP, 6), "carry")
    check(to_mag(value((0, 5, [ONES, 1])), False) == (ONES, 5), "truncate")
    check(to_mag(value((0, 3, [1])), True) == (TOP, -60), "normalise")
    check(tok((1, -2, [TOP])) == "-e-2:8000000000000000", "token")
    check(ordc(Fraction(1), Fraction(2)) == "L", "ord")


def main(argv):
    selftest()
    if argv == ["--selftest"]:
        print("gen_bridge_rows selftest OK")
        return 0
    if len(argv) == 2 and argv[0] == "--check":
        os.makedirs(argv[1], exist_ok=True)
        stream(os.path.join(argv[1], GOLDEN_NAME))
        return 1 if goldlib.check_against(argv[1], GOLDEN_DIR, [GOLDEN_NAME]) else 0
    if argv:
        raise SystemExit("usage: scripts/gen_bridge_rows.py [--check DIR | --selftest]")
    stream(os.path.join(GOLDEN_DIR, GOLDEN_NAME))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
