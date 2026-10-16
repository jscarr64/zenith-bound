# scripts

## gen_goldens.py

Generates `tests/goldens/mag_ops.txt`, the mpmath golden rows for the integer-only `Mag` operations (`add_up`, `mul_up`, `mul_lower`, `sub_lower`, `div_up`, `div_lower`, `sqrt_up`, `sqrt_lower`). The rows come from `mpmath.libmp` directed rounding to 64 bits, an oracle that is independent of zenith-float and of the Rust model. Every row is also checked with exact rationals (`fractions.Fraction`) before it is written, when its exponents are close enough to compute; the script prints how many rows were checked and how many were skipped.

Run it with **`/usr/bin/python3`**:

    /usr/bin/python3 scripts/gen_goldens.py

On Pop the `python3` on `PATH` (`/usr/local/bin/python3`) has no mpmath. mpmath 1.2.1 is the Ubuntu package (`python3-mpmath`) and is visible only to `/usr/bin/python3`. The script refuses to run with any other mpmath version. `requirements.lock` records the version; do not `pip install` it (Pop's pip has no TLS).

The output is deterministic (fixed operand sets, a fixed xorshift seed, order-preserving de-duplication), so regenerating must leave `tests/goldens/mag_ops.txt` byte-identical (a gate step that regenerates and diffs it is planned, shared-harness item S8; it does not exist yet).

Operand sets: binary rows use four mantissas and eleven exponents (including `i64::MIN`, `i64::MAX` and their neighbours) plus a wide block of twelve mantissas at the exponent extremes; unary rows use twelve mantissas (the same list as `tests/model/sets.rs`) and perfect or near-perfect squares. Round-down overflow rows expect `Finite{i64::MAX, u64::MAX}`; round-up overflow rows expect `inf`.

## gen_review_goldens.py

Generates `tests/goldens/mag_ops_review.txt`: boundary rows added after the L0a review (sub_lower at exponent gap 191, division with remainder exactly 1 and mb - 1, add_up and sub_lower at the alignment gaps around 64, 128 and 192, mul carry rows at the exponent limits, `ulp_mag` and `half_ulp` at the `i32` exponent limits and large precisions). The oracle is a big-integer script inside the generator (no mpmath, no Rust code), so any `/usr/bin/python3` or `python3` runs it:

    python3 scripts/gen_review_goldens.py > tests/goldens/mag_ops_review.txt

The output is deterministic. `tests/mag/review_rows.rs` runs every row against the crate.
