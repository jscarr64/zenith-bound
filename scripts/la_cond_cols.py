"""Per-row condition / tightness columns for the exact LA golds (format 2; Spence's spec from Cassie's review, Harper 2026-10-29).

Importable helper, shared by scripts/gen_la_goldens.py (la_logdet, la_solve) and reusable for the LA3 lstsq golds (Rusty).
Pure exact arithmetic on gmpy2.mpq (no floats, no sympy). Inputs are exact rationals (anything gmpy2.mpq accepts).

    la_cond_cols(A, perm, L, U, scale_row, rref_kind, x=None) -> CondCols   (square: logdet, solve)
        A          n x n exact matrix (list of rows), nonsingular
        perm, L, U exact partial-pivoting factors with P A = L U (perm[i] = row of A placed at row i of P A)
        scale_row  exact rational >= 0: scale_row_ln() for ln outputs (logdet), scale_row_solve(x) for solve,
                   scale_row_lstsq(x) for lstsq (both ||x||_inf)
        rref_kind  "logdet" (trace form) or "solve" (forward-error form; needs x)
    make_cond_cols(cond_row, dim, scale_row, rref) -> CondCols
        the shared tail: k_row from (cond_row, dim), containment_only and reason from k_row. Every op builds its CondCols
        here, so cond_tokens and FORMAT_LINES hold for all of them. LA3 (Rusty) adds la_cond_cols_lstsq next to la_cond_cols,
        with Spence's cond_row = (|| |A^+||A| ||*||x|| + || |(A^TA)^-1||A^T||r| ||)/||x|| and dim = m (k_row uses ceil(log2 m)),
        and returns make_cond_cols(cond_row, m, scale_row_lstsq(x), rref).
    cond_tokens(c) -> the six row tokens, in COLUMNS order, formatted as in FORMAT_LINES.

Definitions (G = P^T |L| |U|, so |A| <= G entrywise; |.| entrywise; gamma_n ~ n * 2^-prec, first order):
    cond_row = || |A^-1| G ||_inf                        exact rational >= 1 (componentwise Skeel condition with LU growth)
    k_row    = 4 + ceil(log2 dim) + ceil(log2 cond_row)  exact integer; dim = n (square ops), m rows (lstsq)
    scale_row: 1 (ln outputs), ||x||_inf (solve, lstsq)
    containment_only, reason: 1 k_row_too_large when prec > k_row + 8 holds for no replay prec in PRECS
                              (k_row + 8 >= max(PRECS)); else 0 tight. Prec-independent: 1 = containment only at every
                              replay prec; 0 = the bound applies at each prec with prec > k_row + 8 (the harness decides per prec).
    rref (diagnostic only, never a gate): the coefficient c with first-order bound c * 2^-prec from |dA| <= gamma_n G:
        logdet: c = n * tr(|A^-1| G)          (d ln|det A| = tr(A^-1 dA))
        solve : c = n * || |A^-1| G |x| ||_inf (dx = -A^-1 dA x)
Replay tightness rule (tests/harness): rad <= 2^(k_row - prec) * max(|v|, scale_row) when prec > k_row + 8, containment only otherwise.
"""
from collections import namedtuple
from gmpy2 import mpq as F

PRECS = (53, 113, 192)
COLUMNS = ("cond_row", "k_row", "scale_row", "containment_only", "reason", "rref")
FORMAT_VERSION = 2
FORMAT_LINES = [
    f"format: {FORMAT_VERSION} (format 1 had no cond columns; readers accept both, keyed on this line)",
    "cond columns: after the value columns of a non-refusal row, in this order: " + " ".join(COLUMNS) + " (refusal rows have none)",
    "cond_row: exact rational p/q = || |A^-1| P^T|L||U| ||_inf (componentwise Skeel condition with growth from the exact L, U of la_lu's pivoting)",
    "k_row: decimal integer = 4 + ceil(log2 n) + ceil(log2 cond_row), exact",
    "scale_row: exact rational p/q; 1/1 for ln outputs (logdet), ||x||_inf for solve",
    f"containment_only reason: 0 tight | 1 k_row_too_large (k_row + 8 >= {max(PRECS)}: no replay prec in {{{', '.join(map(str, PRECS))}}} has prec > k_row + 8). The flag does not depend on prec: 1 means containment only at EVERY replay prec; 0 does not mean tight at every prec, the replay still applies prec > k_row + 8 per prec",
    "rref: exact rational p/q, diagnostic only (never a gate): c with first-order bound c*2^-prec from |dA| <= gamma_n P^T|L||U|, gamma_n ~ n*2^-prec; logdet c = n*tr(|A^-1| P^T|L||U|), solve c = n*|| |A^-1| P^T|L||U| |x| ||_inf",
    "tightness (replay): rad <= 2^(k_row - prec) * max(|v|, scale_row) when prec > k_row + 8, containment only otherwise",
]

CondCols = namedtuple("CondCols", COLUMNS)

def _q(x):
    x = F(x); return f"{x.numerator}/{x.denominator}"

def ceil_log2(x):
    """ceil(log2 x) for exact rational x > 0."""
    x = F(x); assert x > 0
    n, d = x.numerator, x.denominator
    k = n.bit_length() - d.bit_length()          # 2^(k-1) < x < 2^(k+1)
    while F(2) ** k < x: k += 1
    while F(2) ** (k - 1) >= x: k -= 1
    return k

def scale_row_ln(): return F(1)
def scale_row_solve(x): return max((abs(F(v)) for v in x), default=F(0))
def scale_row_lstsq(x):
    """lstsq wrapper (LA3, Rusty): scale_row = ||x||_inf of the exact least-squares solution x."""
    return scale_row_solve(x)

def _inverse(perm, L, U):
    n = len(L); inv = [[F(0)] * n for _ in range(n)]
    for c in range(n):
        y = []
        for i in range(n): y.append(F(int(perm[i] == c)) - sum((F(L[i][j]) * y[j] for j in range(i)), F(0)))
        x = [F(0)] * n
        for i in reversed(range(n)):
            x[i] = (y[i] - sum((F(U[i][j]) * x[j] for j in range(i + 1, n)), F(0))) / F(U[i][i])
        for i in range(n): inv[i][c] = x[i]
    return inv

def la_cond_cols(A, perm, L, U, scale_row, rref_kind, x=None):
    n = len(A); assert n > 0 and all(F(U[i][i]) != 0 for i in range(n)), "la_cond_cols: singular input"
    LU = [[sum((abs(F(L[i][k])) * abs(F(U[k][j])) for k in range(n)), F(0)) for j in range(n)] for i in range(n)]
    G = [None] * n
    for i in range(n): G[perm[i]] = LU[i]
    for i in range(n):
        for j in range(n): assert abs(F(A[i][j])) <= G[i][j]
    Ai = [[abs(v) for v in r] for r in _inverse(perm, L, U)]
    M = [[sum((Ai[i][k] * G[k][j] for k in range(n)), F(0)) for j in range(n)] for i in range(n)]
    cond = max(sum(r, F(0)) for r in M)
    if rref_kind == "logdet": rref = F(n) * sum((M[i][i] for i in range(n)), F(0))
    elif rref_kind == "solve":
        assert x is not None
        rref = F(n) * max(sum((M[i][j] * abs(F(x[j])) for j in range(n)), F(0)) for i in range(n))
    else: raise ValueError(f"la_cond_cols: rref_kind {rref_kind!r}")
    return make_cond_cols(cond, n, scale_row, rref)

def make_cond_cols(cond_row, dim, scale_row, rref):
    """CondCols from an exact cond_row >= 1 and the op dimension dim >= 1 (n for square ops, m for lstsq); see the module doc."""
    cond_row = F(cond_row); assert cond_row >= 1 and dim >= 1
    k_row = 4 + (dim - 1).bit_length() + ceil_log2(cond_row)
    co = int(k_row + 8 >= max(PRECS))
    return CondCols(cond_row, k_row, F(scale_row), co, "k_row_too_large" if co else "tight", F(rref))

def cond_tokens(c):
    return [_q(c.cond_row), str(c.k_row), _q(c.scale_row), str(c.containment_only), c.reason, _q(c.rref)]
