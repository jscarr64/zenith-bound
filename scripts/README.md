# scripts

## gen_goldens.py

Generates `tests/goldens/mag_ops.txt`, the mpmath golden rows for the integer-only `Mag` operations (`add_up`, `mul_up`, `mul_lower`, `sub_lower`, `div_up`, `div_lower`, `sqrt_up`, `sqrt_lower`). The rows come from `mpmath.libmp` directed rounding to 64 bits, an oracle that is independent of zenith-float and of the Rust model. Every row is also checked with exact rationals (`fractions.Fraction`) before it is written, when its exponents are close enough to compute; the script prints how many rows were checked and how many were skipped.

Run it with **`/usr/bin/python3`**:

    /usr/bin/python3 scripts/gen_goldens.py

On Pop the `python3` on `PATH` (`/usr/local/bin/python3`) has no mpmath. mpmath 1.2.1 is the Ubuntu package (`python3-mpmath`) and is visible only to `/usr/bin/python3`. The script refuses to run with any other mpmath version. `requirements.lock` records the version; do not `pip install` it (Pop's pip has no TLS).

The output is deterministic (fixed operand sets, a fixed xorshift seed, order-preserving de-duplication), so regenerating must leave `tests/goldens/mag_ops.txt` byte-identical; the full gate regenerates it and fails on any difference.

Operand sets: binary rows use four mantissas and eleven exponents (including `i64::MIN`, `i64::MAX` and their neighbours) plus a wide block of twelve mantissas at the exponent extremes; unary rows use twelve mantissas (the same list as `tests/model/sets.rs`) and perfect or near-perfect squares. Round-down overflow rows expect `Finite{i64::MAX, u64::MAX}`; round-up overflow rows expect `inf`.
