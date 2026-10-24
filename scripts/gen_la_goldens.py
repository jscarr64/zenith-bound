#!/usr/bin/env python3
"""Exact linear-algebra reference goldens for zenith-bound (zenith-float 2.0).

Pinned oracle: sympy 1.14.0, mpmath 1.3.0 (run with ~/.local/oracle-venv).
Usage:
    gen_la_goldens.py            write tests/goldens/la_*.txt and SHA256SUMS
    gen_la_goldens.py --check    regenerate in memory and diff against committed files

All arithmetic is exact (fractions.Fraction / sympy QQ / Python ints). Irrational
outputs are emitted as rigorous dyadic enclosures computed by exact integer square
roots, exact rational root refinement (sympy, Sturm-based), or mpmath.iv interval
arithmetic. No floats are used anywhere. Every identity and every enclosure is
re-checked here before a row is emitted.
"""
import sys, os, random, hashlib, argparse, difflib
from fractions import Fraction as F
from math import isqrt

SYMPY_PIN = "1.14.0"
MPMATH_PIN = "1.3.0"
import sympy, mpmath
from mpmath import iv
assert sympy.__version__ == SYMPY_PIN, f"sympy {sympy.__version__} != pinned {SYMPY_PIN}"
assert mpmath.__version__ == MPMATH_PIN, f"mpmath {mpmath.__version__} != pinned {MPMATH_PIN}"
sys.set_int_max_str_digits(0)

GEN = "scripts/gen_la_goldens.py"
SEED = 20261024
BITS = 256          # default enclosure width (relative)
LOG_BITS = (256, 4096)
ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
OUT = os.path.join(ROOT, "tests", "goldens")

# ---------------------------------------------------------------- formatting
def q(x):
    x = F(x)
    return f"{x.numerator}/{x.denominator}"

def dy(m, e):
    """Canonical dyadic m*2^e: m odd (or m == 0 with e == 0)."""
    if m == 0:
        return "0*2^0"
    while m % 2 == 0:
        m //= 2; e += 1
    return f"{m}*2^{e}"

def dyval(m, e):
    return F(m) * (F(2) ** e)

def ilog2(x):
    """floor(log2 |x|) for nonzero rational x, exactly."""
    x = abs(F(x)); n, d = x.numerator, x.denominator
    k = n.bit_length() - d.bit_length()
    if F(2) ** k > x: k -= 1
    if F(2) ** (k + 1) <= x: k += 1
    return k

def floor_q(x): return x.numerator // x.denominator
def ceil_q(x): return -((-x.numerator) // x.denominator)

def grid(x, bits):
    """Exponent e of the grid 2^e used for a value of magnitude |x|: e = floor(log2|x|) - bits + 1."""
    return ilog2(x) - bits + 1

def enclose_rational(x, bits=BITS):
    x = F(x)
    if x == 0: return (0, 0), (0, 0)
    e = grid(x, bits); s = x / (F(2) ** e)
    return (floor_q(s), e), (ceil_q(s), e)

def enclose_csqrt(c, s, bits=BITS):
    """Enclose y = c*sqrt(s), c, s rational, s >= 0, by exact integer isqrt."""
    c, s = F(c), F(s)
    assert s >= 0
    if c == 0 or s == 0: return (0, 0), (0, 0)
    y2 = c * c * s
    e = ilog2(y2) // 2 - bits + 1           # grid exponent ~ floor(log2 |y|) - bits + 1
    t = y2 / (F(4) ** e)                    # (|y| / 2^e)^2
    m = isqrt(floor_q(t))
    hi = m if F(m * m) == t else m + 1
    lo = m
    assert F(lo * lo) <= t <= F(hi * hi)
    if c < 0: lo, hi = -hi, -lo
    L, H = dyval(lo, e), dyval(hi, e)
    # containment check: lo <= y <= hi, exact via squares and sign
    if c > 0: assert 0 <= L and L * L <= y2 <= H * H
    else: assert H <= 0 and H * H <= y2 <= L * L
    return (lo, e), (hi, e)

def pair(lo, hi): return f"{dy(*lo)} {dy(*hi)}"

# ---------------------------------------------------------------- exact matrix helpers
def ident(n): return [[F(int(i == j)) for j in range(n)] for i in range(n)]
def mm(A, B): return [[sum((A[i][k] * B[k][j] for k in range(len(B))), F(0)) for j in range(len(B[0]))] for i in range(len(A))]
def tr(A): return [list(r) for r in zip(*A)]
def flat(A): return [x for r in A for x in r]
def issym(A): return all(A[i][j] == A[j][i] for i in range(len(A)) for j in range(len(A)))

def lu_pp(A):
    """Partial pivoting: pivot = max |U[i][k]| over i >= k, ties -> smallest i; zero column -> no step."""
    n = len(A); U = [list(map(F, r)) for r in A]; L = ident(n); perm = list(range(n))
    for k in range(n):
        p = max(range(k, n), key=lambda i: (abs(U[i][k]), -i))
        if U[p][k] == 0: continue
        if p != k:
            U[k], U[p] = U[p], U[k]; perm[k], perm[p] = perm[p], perm[k]
            for j in range(k): L[k][j], L[p][j] = L[p][j], L[k][j]
        for i in range(k + 1, n):
            l = U[i][k] / U[k][k]
            if l:
                L[i][k] = l
                U[i] = [U[i][j] - l * U[k][j] for j in range(n)]
    PA = [list(map(F, A[perm[i]])) for i in range(n)]
    assert mm(L, U) == PA
    return perm, L, U

def perm_sign(perm):
    s, seen = 1, [False] * len(perm)
    for i in range(len(perm)):
        if not seen[i]:
            j, c = i, 0
            while not seen[j]: seen[j] = True; j = perm[j]; c += 1
            if c % 2 == 0: s = -s
    return s

def det(A):
    perm, L, U = lu_pp(A); d = F(perm_sign(perm))
    for i in range(len(A)): d *= U[i][i]
    return d

def solve(A, b):
    n = len(A); perm, L, U = lu_pp(A)
    if any(U[i][i] == 0 for i in range(n)): return None
    y = []
    for i in range(n): y.append(F(b[perm[i]]) - sum((L[i][j] * y[j] for j in range(i)), F(0)))
    x = [F(0)] * n
    for i in reversed(range(n)): x[i] = (y[i] - sum((U[i][j] * x[j] for j in range(i + 1, n)), F(0))) / U[i][i]
    assert [sum((F(A[i][j]) * x[j] for j in range(n)), F(0)) for i in range(n)] == [F(v) for v in b]
    return x

def ldl(A):
    n = len(A)
    if not issym(A): return "not_symmetric"
    L = ident(n); D = [F(0)] * n
    for j in range(n):
        D[j] = F(A[j][j]) - sum((L[j][k] ** 2 * D[k] for k in range(j)), F(0))
        if D[j] <= 0: return "not_spd"
        for i in range(j + 1, n):
            L[i][j] = (F(A[i][j]) - sum((L[i][k] * L[j][k] * D[k] for k in range(j)), F(0))) / D[j]
    assert mm(mm(L, [[D[i] if i == j else F(0) for j in range(n)] for i in range(n)]), tr(L)) == [list(map(F, r)) for r in A]
    return L, D

def gram_schmidt(A):
    m, n = len(A), len(A[0]); cols = tr([list(map(F, r)) for r in A])
    Qt, Rt, d = [], ident(n), []
    for j in range(n):
        v = list(cols[j])
        for i in range(j):
            Rt[i][j] = sum((a * b for a, b in zip(Qt[i], cols[j])), F(0)) / d[i]
            v = [a - Rt[i][j] * b for a, b in zip(v, Qt[i])]
        dj = sum((a * a for a in v), F(0))
        if dj == 0: return None
        Qt.append(v); d.append(dj)
    QtM = tr(Qt)
    assert mm(QtM, Rt) == [list(map(F, r)) for r in A]
    G = mm(Qt, QtM)
    assert all(G[i][j] == (d[i] if i == j else 0) for i in range(n) for j in range(n))
    return QtM, Rt, d

# ---------------------------------------------------------------- enclosure via mpmath.iv
def raw_to_q(t):
    s, man, e, bc = t
    if man == 0: return F(0)
    v = F(man) * (F(2) ** e)
    return -v if s else v

def iv_from_q(x, bits):
    lo, hi = enclose_rational(x, bits)
    a = iv.mpf(lo[0]) * iv.mpf(2) ** lo[1]
    b = iv.mpf(hi[0]) * iv.mpf(2) ** hi[1]
    return iv.mpf([a.a, b.b])

def iv_log_abs(x, bits):
    """[lo, hi] rational enclosure of ln|x| from mpmath.iv at working precision bits+64."""
    old = iv.prec
    try:
        iv.prec = bits + 64
        r = iv.log(iv_from_q(abs(x), bits + 64))
        a, b = r._mpi_
        return raw_to_q(a), raw_to_q(b)
    finally:
        iv.prec = old

def enclose_log(x, bits):
    a, b = iv_log_abs(x, bits)
    if a == 0 and b == 0: return (0, 0), (0, 0)
    mag = max(abs(a), abs(b))
    e = grid(mag, bits)
    lo = (floor_q(a / F(2) ** e), e); hi = (ceil_q(b / F(2) ** e), e)
    # containment check: independent higher-precision enclosure must lie inside
    a2, b2 = iv_log_abs(x, 2 * bits + 64)
    assert dyval(*lo) <= a2 <= b2 <= dyval(*hi)
    if x == 1 or x == -1: assert dyval(*lo) <= 0 <= dyval(*hi)
    return lo, hi

# ---------------------------------------------------------------- eigenvalues (exact algebraic)
X = sympy.Symbol("x")
def eig_exact(A, bits=BITS):
    n = len(A)
    M = sympy.Matrix([[sympy.Rational(a.numerator, a.denominator) for a in r] for r in A])
    P = sympy.Poly(M.charpoly(X).as_expr(), X, domain="QQ")
    coeffs = [F(int(c.p), int(c.q)) for c in P.all_coeffs()]
    _, facs = P.factor_list()
    out = []
    for f, mult in facs:
        if f.degree() == 1:
            a, b = f.all_coeffs()
            rr = -b / a; r = F(int(rr.p), int(rr.q))
            lo, hi = enclose_rational(r, bits)
            for _ in range(mult): out.append((r, q(r), lo, hi))
            continue
        ivs = f.intervals()
        assert sum(k for _, k in ivs) == f.degree(), "non-real root in symmetric charpoly?"
        for (s, t), k in ivs:
            s, t = sympy.Rational(s), sympy.Rational(t)
            while True:
                if s * t > 0:
                    w = min(abs(s), abs(t)); e = grid(F(int(w.p), int(w.q)), bits)
                    if t - s <= sympy.Rational(2) ** e / 4: break
                    s, t = f.refine_root(s, t, eps=sympy.Rational(2) ** e / 8)
                else:
                    s, t = f.refine_root(s, t, eps=(t - s) / 4)
            sF, tF = F(int(s.p), int(s.q)), F(int(t.p), int(t.q))
            lo = (floor_q(sF / F(2) ** e), e); hi = (ceil_q(tF / F(2) ** e), e)
            L, H = dyval(*lo), dyval(*hi)
            assert f.count_roots(sympy.Rational(L.numerator, L.denominator), sympy.Rational(H.numerator, H.denominator)) == 1
            for _ in range(mult): out.append(((sF + tF) / 2, "alg", lo, hi))
    out.sort(key=lambda r: r[0])
    for a, b in zip(out, out[1:]):  # ordering must be unambiguous: distinct eigenvalues get disjoint enclosures
        # two rational eigenvalues closer than the grid may share a cell; the exact p/q tokens disambiguate
        if not ((a[1] != "alg" and b[1] != "alg" and a[0] <= b[0]) or (a[1] == "alg" and a[2:] == b[2:]) or dyval(*a[3]) < dyval(*b[2])): print("ORDER", n, a[1], b[1], a[2:], b[2:]); raise AssertionError
    assert sum(1 for _ in out) == n
    return coeffs, out

# ---------------------------------------------------------------- case corpus
rng = random.Random(SEED)
def rq(lo=-9, hi=9, dmax=9): return F(rng.randint(lo, hi), rng.randint(1, dmax))
def rmat(m, n, rational=True):
    return [[rq() if rational else F(rng.randint(-9, 9)) for _ in range(n)] for _ in range(m)]
def hilbert(n): return [[F(1, i + j + 1) for j in range(n)] for i in range(n)]
def spd(n, rational=True):
    B = rmat(n, n, rational); A = mm(B, tr(B))
    for i in range(n): A[i][i] += 1
    return A
def rsym(n):
    A = [[F(0)] * n for _ in range(n)]
    for i in range(n):
        for j in range(i, n): A[i][j] = A[j][i] = F(rng.randint(-9, 9))
    return A
def zero(n): return [[F(0)] * n for _ in range(n)]
H6, T6 = F(2) ** 1000000, F(1, 2 ** 1000000)
B3, T3 = F(2) ** 1000, F(1, 2 ** 1000)
B4 = F(2) ** 10000
EPS = F(1, 2 ** 200)

def square_corpus(kind):
    """Common square cases. kind in lu/solve/logdet."""
    C = []
    C += [("onebyone.pos", [[F(3, 7)]]), ("onebyone.neg", [[F(-5, 2)]]), ("onebyone.zero", [[F(0)]])]
    C += [("zero.%02d" % n, zero(n)) for n in (2, 5)]
    C += [("basic.2", [[F(4), F(3)], [F(6), F(3)]]), ("basic.3", [[F(2), F(-1), F(0)], [F(-1), F(2), F(-1)], [F(0), F(-1), F(2)]])]
    C += [("perm.swap2", [[F(0), F(1)], [F(1), F(0)]]), ("perm.zero_pivot3", [[F(0), F(2), F(1)], [F(1), F(1), F(1)], [F(3), F(0), F(2)]]),
          ("perm.cyclic4", [[F(int((j - i) % 4 == 1)) * (i + 1) for j in range(4)] for i in range(4)]),
          ("perm.tie3", [[F(1), F(2), F(3)], [F(-1), F(5), F(1)], [F(1), F(0), F(7)]])]
    C += [("singular.dup_rows3", [[F(1), F(2), F(3)], [F(1), F(2), F(3)], [F(4), F(5), F(6)]]),
          ("singular.classic3", [[F(1), F(2), F(3)], [F(4), F(5), F(6)], [F(7), F(8), F(9)]])]
    C += [("rankdef.r1_4", [[F(i + 1) * F(j + 1, 3) for j in range(4)] for i in range(4)]),
          ("rankdef.r2_5", mm(rmat(5, 2), rmat(2, 5)))]
    C += [("nearsing.2", [[F(1), F(1)], [F(1), 1 + EPS]]), ("nearsing.3", [[F(1), F(2), F(3)], [F(4), F(5), F(6)], [F(7), F(8), 9 + EPS]])]
    C += [("hilbert.%02d" % n, hilbert(n)) for n in range(5, 13)]
    C += [("scale_huge.1x1", [[H6 + 1]]), ("scale_tiny.1x1", [[T6 * F(3, 5)]]),
          ("scale_mixed.diag2", [[H6, F(0)], [F(0), T6]]),
          ("scale_huge.2", [[B4 + 1, B4], [B4, B4 - 3]]), ("scale_tiny.3", [[a * T3 for a in r] for r in rmat(3, 3)]),
          ("scale_mixed.3", [[B3, F(1), T3], [F(1), T3, B3], [T3, B3, F(1)]])]
    for n in (2, 3, 4, 5, 6, 8):
        C.append(("random_q.%02d" % n, rmat(n, n)))
    for n in (12, 16, 24, 32):
        C.append(("random_z.%02d" % n, rmat(n, n, rational=False)))
    return C

def spd_corpus():
    C = [("onebyone.pos", [[F(9, 4)]]), ("onebyone.irr", [[F(2)]])]
    C += [("basic.2", [[F(4), F(2)], [F(2), F(3)]]), ("basic.tridiag5", [[F(2) if i == j else (F(-1) if abs(i - j) == 1 else F(0)) for j in range(5)] for i in range(5)])]
    C += [("hilbert.%02d" % n, hilbert(n)) for n in range(5, 13)]
    C += [("nearsing.2", [[F(1), F(1)], [F(1), 1 + EPS]])]
    C += [("scale_huge.1x1", [[H6]]), ("scale_tiny.1x1", [[T6 * 2]]), ("scale_mixed.diag2", [[H6, F(0)], [F(0), T6]]),
          ("scale_huge.2", [[B4 * 4, B4], [B4, B4]]), ("scale_tiny.3", [[a * T3 for a in r] for r in spd(3)])]
    for n in (2, 3, 4, 6, 8): C.append(("random_q.%02d" % n, spd(n)))
    for n in (12, 16, 24, 32): C.append(("random_z.%02d" % n, spd(n, rational=False)))
    # refusals
    C += [("refuse.zero2", zero(2)), ("refuse.onebyone_neg", [[F(-1)]]), ("refuse.onebyone_zero", [[F(0)]]),
          ("refuse.indefinite2", [[F(1), F(2)], [F(2), F(1)]]), ("refuse.psd_singular3", [[F(1), F(1), F(0)], [F(1), F(1), F(0)], [F(0), F(0), F(1)]]),
          ("refuse.nonsym2", [[F(2), F(1)], [F(0), F(2)]]), ("refuse.neg_def3", [[-a for a in r] for r in spd(3)]),
          ("refuse.nearsing_neg2", [[F(1), F(1)], [F(1), 1 - EPS]])]
    return C

# ---------------------------------------------------------------- per-op emitters
def dims_sq(A): return str(len(A))
def dims_mn(A): return f"{len(A)}x{len(A[0])}"

def row(cid, dims, A, outs): return " ".join([cid, dims] + [q(x) for x in flat(A)] + outs)

def gen_lu():
    rows = []
    for cid, A in square_corpus("lu"):
        perm, L, U = lu_pp(A)
        rows.append(row("lu." + cid, dims_sq(A), A, [str(p) for p in perm] + [q(x) for x in flat(L)] + [q(x) for x in flat(U)]))
    hdr = ["op: lu",
           "canonical: PA = LU, partial pivoting. At step k pivot row p = argmax_{i>=k} |U[i][k]|, ties -> smallest i; rows k,p of U and of L[:, :k] swapped. If the pivot column is all zero, no swap and no elimination (L column k stays e_k). L unit lower triangular, U upper triangular (zero/singular rows allowed). Exact rationals.",
           "perm: n integers, 0-based; perm[i] = index of the row of A placed at row i of PA",
           "columns: case_id dims(n) A[n*n] perm[n] L[n*n] U[n*n]"]
    return hdr, rows

def gen_solve():
    rows = []
    for cid, A in square_corpus("solve"):
        n = len(A); b = [F(1)] * n if cid.startswith("hilbert") else [F(i + 1, 2) - 1 for i in range(n)]
        x = solve(A, b)
        outs = ["ERR", "singular"] if x is None else [q(v) for v in x]
        rows.append(" ".join(["solve." + cid, str(n)] + [q(v) for v in flat(A)] + [q(v) for v in b] + outs))
    hdr = ["op: solve",
           "canonical: unique x with A x = b, exact rationals. b = ones for Hilbert cases, b[i] = (i+1)/2 - 1 otherwise. Any singular A (det = 0) -> refusal row 'ERR singular'.",
           "columns: case_id dims(n) A[n*n] b[n] (x[n] | ERR singular)"]
    return hdr, rows

def gen_logdet():
    rows = []
    for cid, A in square_corpus("logdet"):
        d = det(A)
        if d == 0: outs = ["ERR", "singular"]
        else:
            outs = [str(1 if d > 0 else -1), q(d)]
            for b in LOG_BITS: outs.append(pair(*enclose_log(d, b)))
        rows.append(row("logdet." + cid, dims_sq(A), A, outs))
    hdr = ["op: logdet",
           "canonical: sign = sign(det A) in {1,-1}; det = exact rational determinant; logabsdet = ln|det A| (irrational unless det = +-1). det = 0 -> refusal row 'ERR singular'.",
           "exact form: ln(det) with det the exact rational in the row (sympy: log(Rational(det))).",
           "enclosure: lo_256 hi_256 lo_4096 hi_4096; each pair encloses ln|det|; grid 2^e with e = floor(log2 max(|lo_raw|,|hi_raw|)) - bits + 1; computed with mpmath.iv.log at bits+64 and checked to contain an independent mpmath.iv enclosure at 2*bits+64.",
           "columns: case_id dims(n) A[n*n] (sign det lo256 hi256 lo4096 hi4096 | ERR singular)"]
    return hdr, rows

def gen_cholesky():
    rows = []
    for cid, A in spd_corpus():
        r = ldl(A); n = len(A)
        if isinstance(r, str): outs = ["ERR", r]
        else:
            L, D = r; outs = [q(x) for x in flat(L)] + [q(x) for x in D]
            for i in range(n):
                for j in range(n):
                    outs.append(pair(*enclose_csqrt(L[i][j], D[j])) if j <= i else "0*2^0 0*2^0")
        rows.append(row("cholesky." + cid, dims_sq(A), A, outs))
    hdr = ["op: cholesky",
           "canonical: A = G G^T with G lower triangular and diag(G) > 0 (unique for SPD A). Exact rational companion: A = L D L^T, L unit lower triangular, D diagonal > 0, computed without pivoting; G[i][j] = L[i][j]*sqrt(D[j]) (this is the exact algebraic form).",
           "refusals: 'ERR not_symmetric' if A != A^T; 'ERR not_spd' if some pivot D[j] <= 0 (indefinite, negative, singular PSD, zero).",
           "enclosure: G[n*n] as lo hi pairs, row-major, upper triangle emitted as exact 0*2^0 0*2^0; grid 2^e with e = floor(log2(G^2))//2 - 255, lo/hi from exact integer isqrt; lo == hi iff exactly representable.",
           "columns: case_id dims(n) A[n*n] (L[n*n] D[n] G_lohi[2*n*n] | ERR kind)"]
    return hdr, rows

def gen_qr():
    rows = []
    C = [("onebyone.pos", [[F(3)]]), ("onebyone.neg", [[F(-2, 3)]]), ("basic.2", [[F(3), F(1)], [F(4), F(2)]]),
         ("pythag.3", [[F(2), F(1), F(0)], [F(1), F(2), F(1)], [F(2), F(2), F(3)]]),
         ("tall.3x2", [[F(1), F(1)], [F(1), F(2)], [F(1), F(3)]]), ("tall.6x3", rmat(6, 3)), ("tall.20x8", rmat(20, 8, rational=False)),
         ("nearsing.2", [[F(1), F(1)], [F(1), 1 + EPS]])]
    C += [("hilbert.%02d" % n, hilbert(n)) for n in range(5, 13)]
    C += [("scale_huge.1x1", [[H6]]), ("scale_mixed.diag2", [[H6, F(0)], [F(0), T6]]), ("scale_tiny.3", [[a * T3 for a in r] for r in rmat(3, 3)]),
          ("scale_huge.2", [[B4 + 1, B4], [B4, B4 - 3]])]
    for n in (3, 4, 6): C.append(("random_q.%02d" % n, rmat(n, n)))
    for n in (8, 12, 16): C.append(("random_z.%02d" % n, rmat(n, n, rational=False)))
    C += [("refuse.zero2", zero(2)), ("refuse.onebyone_zero", [[F(0)]]), ("refuse.rankdef3", [[F(1), F(2), F(3)], [F(4), F(5), F(6)], [F(7), F(8), F(9)]]),
          ("refuse.dupcol_4x2", [[F(i), F(i)] for i in range(1, 5)]), ("refuse.wide2x3", [[F(1), F(0), F(0)], [F(0), F(1), F(0)]])]
    for cid, A in C:
        m, n = len(A), len(A[0])
        g = None if m < n else gram_schmidt(A)
        if g is None: outs = ["ERR", "wide" if m < n else "rank_deficient"]
        else:
            Qt, Rt, d = g
            outs = [q(x) for x in flat(Qt)] + [q(x) for x in flat(Rt)] + [q(x) for x in d]
            for i in range(m):
                for j in range(n): outs.append(pair(*enclose_csqrt(Qt[i][j] / d[j], d[j])))
            for i in range(n):
                for j in range(n): outs.append(pair(*enclose_csqrt(Rt[i][j], d[i])))
        rows.append(row("qr." + cid, dims_mn(A), A, outs))
    hdr = ["op: qr",
           "canonical: thin QR, A (m x n, m >= n, full column rank) = Q R, Q m x n with Q^T Q = I, R n x n upper triangular with diag(R) > 0 (unique).",
           "exact rational companion (exact algebraic form): A = Qt Rt with Qt orthogonal columns (Qt^T Qt = diag(d)), Rt unit upper triangular, d[j] = ||Qt[:,j]||^2; Q[i][j] = Qt[i][j]/sqrt(d[j]), R[i][j] = Rt[i][j]*sqrt(d[i]).",
           "refusals: 'ERR rank_deficient' if some d[j] = 0; 'ERR wide' if m < n.",
           "enclosure: Q (m*n) then R (n*n) as lo hi pairs, row-major; grid 2^e with e = floor(log2(x^2))//2 - 255, exact integer isqrt.",
           "columns: case_id dims(mxn) A[m*n] (Qt[m*n] Rt[n*n] d[n] Q_lohi[2*m*n] R_lohi[2*n*n] | ERR kind)"]
    return hdr, rows

def gen_gls():
    rows = []
    def ar1(m, rho): return [[rho ** abs(i - j) for j in range(m)] for i in range(m)]
    def diag(v): return [[v[i] if i == j else F(0) for j in range(len(v))] for i in range(len(v))]
    def design(m, p): return [[F(i + 1) ** k for k in range(p)] for i in range(m)]
    C = [("basic.ols_identity", design(4, 2), ident(4), [F(1), F(3), F(2), F(5)]),
         ("basic.wls_diag", design(5, 2), diag([F(1), F(2), F(1, 2), F(4), F(1)]), [F(2), F(1), F(4), F(3), F(6)]),
         ("ar1.6x2", design(6, 2), ar1(6, F(1, 2)), [rq() for _ in range(6)]),
         ("ar1.10x3", design(10, 3), ar1(10, F(9, 10)), [rq() for _ in range(10)]),
         ("exactfit.3x3", design(3, 3), spd(3), [F(1), F(4), F(9)]),
         ("onebyone.1x1", [[F(2)]], [[F(3)]], [F(5)]),
         ("mean.5x1", [[F(1)]] * 5, spd(5), [rq() for _ in range(5)]),
         ("random.8x3", rmat(8, 3), spd(8), [rq() for _ in range(8)]),
         ("random.16x4", rmat(16, 4, rational=False), spd(16, rational=False), [F(rng.randint(-9, 9)) for _ in range(16)]),
         ("hilbert_cov.6x2", design(6, 2), hilbert(6), [F(1)] * 6),
         ("scale_mixed.4x2", design(4, 2), diag([B3, T3, F(1), B3]), [F(1), F(2), F(3), F(4)]),
         ("refuse.sigma_not_spd", design(3, 2), [[F(1), F(2), F(0)], [F(2), F(1), F(0)], [F(0), F(0), F(1)]], [F(1), F(2), F(3)]),
         ("refuse.sigma_singular", design(3, 2), zero(3), [F(1), F(2), F(3)]),
         ("refuse.rankdef_X", [[F(1), F(2)], [F(2), F(4)], [F(3), F(6)]], ident(3), [F(1), F(2), F(3)])]
    for cid, Xm, S, y in C:
        m, p = len(Xm), len(Xm[0])
        r = ldl(S)
        if isinstance(r, str): outs = ["ERR", "sigma_" + r]
        else:
            SiX = tr([solve(S, col) for col in tr(Xm)])
            Siy = solve(S, y)
            Mx = mm(tr(Xm), SiX)
            if det(Mx) == 0: outs = ["ERR", "rank_deficient"]
            else:
                beta = solve(Mx, [sum((Xm[i][k] * Siy[i] for i in range(m)), F(0)) for k in range(p)])
                cov = tr([solve(Mx, [F(int(i == j)) for i in range(p)]) for j in range(p)])
                res = [y[i] - sum((Xm[i][k] * beta[k] for k in range(p)), F(0)) for i in range(m)]
                rs = solve(S, res)
                rss = sum((a * b for a, b in zip(res, rs)), F(0))
                # normal equations check
                assert all(sum((Xm[i][k] * rs[i] for i in range(m)), F(0)) == 0 for k in range(p))
                outs = [q(v) for v in beta] + [q(v) for v in flat(cov)] + [q(rss)]
        rows.append(" ".join(["gls." + cid, f"{m}x{p}"] + [q(v) for v in flat(Xm)] + [q(v) for v in flat(S)] + [q(v) for v in y] + outs))
    hdr = ["op: gls",
           "canonical: generalized least squares, beta = (X^T S^-1 X)^-1 X^T S^-1 y; cov = (X^T S^-1 X)^-1; rss = r^T S^-1 r with r = y - X beta. All exact rationals. Checked: X^T S^-1 r = 0.",
           "refusals: 'ERR sigma_not_spd' / 'ERR sigma_not_symmetric' (S fails LDL^T, see la_cholesky); 'ERR rank_deficient' if X^T S^-1 X singular.",
           "columns: case_id dims(mxp) X[m*p] S[m*m] y[m] (beta[p] cov[p*p] rss | ERR kind)"]
    return hdr, rows

def gen_eig():
    rows = []
    C = [("onebyone.pos", [[F(7, 3)]]), ("zero.03", zero(3)), ("basic.2", [[F(2), F(1)], [F(1), F(2)]]),
         ("irr.2", [[F(1), F(1)], [F(1), F(0)]]), ("tridiag.5", [[F(2) if i == j else (F(-1) if abs(i - j) == 1 else F(0)) for j in range(5)] for i in range(5)]),
         ("repeated.4", [[F(1)] * 4 for _ in range(4)]), ("rankdef.3", [[F(1), F(2), F(3)], [F(2), F(4), F(6)], [F(3), F(6), F(9)]]),
         ("nearsing.2", [[F(1), F(1)], [F(1), 1 + EPS]]), ("diag_dup.3", [[F(2), F(0), F(0)], [F(0), F(-1, 3), F(0)], [F(0), F(0), F(2)]])]
    C += [("hilbert.%02d" % n, hilbert(n)) for n in range(5, 13)]
    C += [("scale_huge.1x1", [[H6]]), ("scale_mixed.diag2", [[H6, F(0)], [F(0), T6]]),
          ("scale_huge.2", [[B3, F(1)], [F(1), B3]]), ("scale_tiny.2", [[T3, T3 * T3], [T3 * T3, 2 * T3]])]
    for n in (3, 4, 6, 8, 12, 16): C.append(("random_z.%02d" % n, rsym(n)))
    C += [("refuse.nonsym2", [[F(0), F(1)], [F(-1), F(0)]])]
    for cid, A in C:
        n = len(A)
        if not issym(A): outs = ["ERR", "not_symmetric"]
        else:
            coeffs, ev = eig_exact(A)
            outs = [q(c) for c in coeffs] + [f"{e[1]} {pair(e[2], e[3])}" for e in ev]
        rows.append(row("eig." + cid, dims_sq(A), A, outs))
    hdr = ["op: eig",
           "canonical: real symmetric A only; eigenvalues listed ascending with algebraic multiplicity. charpoly = det(xI - A), monic, n+1 exact rational coefficients, highest degree first.",
           "exact form: each eigenvalue is the unique root of the charpoly in [lo, hi] at its ascending index (sympy CRootOf of the irreducible factor); token 'p/q' if the eigenvalue is rational, else 'alg'.",
           "enclosure: lo hi per eigenvalue; grid 2^e, e = floor(log2 |lambda|) - 255; from sympy exact rational root isolation/refinement (Poly.intervals/refine_root, exact QQ) then outward rounding; checked with exact Poly.count_roots(lo, hi) == 1 on the irreducible factor. Zero/dyadic rational eigenvalues have lo == hi.",
           "refusals: 'ERR not_symmetric'.",
           "columns: case_id dims(n) A[n*n] (charpoly[n+1] {exact lo hi}[n] | ERR not_symmetric)"]
    return hdr, rows

OPS = [("lu", gen_lu), ("cholesky", gen_cholesky), ("qr", gen_qr), ("solve", gen_solve), ("gls", gen_gls), ("logdet", gen_logdet), ("eig", gen_eig)]

def render():
    global rng
    files = {}
    for name, fn in OPS:
        rng = random.Random(f"{SEED}:{name}")   # per-op stream: ops are independent
        hdr, rows = fn()
        h = [f"generator: {GEN}", f"sympy: {SYMPY_PIN}", f"mpmath: {MPMATH_PIN}", f"seed: {SEED} (python random.Random('{SEED}:{name}'))", f"rows: {len(rows)}"] + hdr + [
             "numbers: every exact entry is p/q in lowest terms, q > 0, arbitrary-size signed integers; dyadics are m*2^e with m odd (or 0*2^0); fields are single-space separated; no floats.",
             "case_id: <op>.<family>.<variant>; inputs are always listed before outputs; refusal rows end in 'ERR <kind>'."]
        files[f"la_{name}.txt"] = ("".join(f"# {l}\n" for l in h) + "".join(r + "\n" for r in rows)).encode("utf-8")
    sums = "".join(f"{hashlib.sha256(files[k]).hexdigest()}  {k}\n" for k in sorted(files))
    files["SHA256SUMS"] = sums.encode()
    return files

def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--check", action="store_true"); a = ap.parse_args()
    files = render()
    if a.check:
        bad = 0
        for k, v in files.items():
            p = os.path.join(OUT, k)
            cur = open(p, "rb").read() if os.path.exists(p) else b""
            if cur != v:
                bad += 1; print(f"MISMATCH {k}")
                if len(v) < 2_000_000:
                    sys.stdout.writelines(list(difflib.unified_diff(cur.decode(errors="replace").splitlines(True), v.decode().splitlines(True), k, k + " (regen)"))[:40])
        print("check:", "OK" if not bad else f"FAIL ({bad} files)"); sys.exit(1 if bad else 0)
    os.makedirs(OUT, exist_ok=True)
    for k, v in files.items():
        with open(os.path.join(OUT, k), "wb") as f: f.write(v)
        print(f"wrote {k} ({len(v)} bytes)")

if __name__ == "__main__":
    main()
