//! Slice S2 harness rows (`src/bridge.rs`). Owned by slice S2 alone; S0 stub, no tests yet.
//! Goldens: tests/goldens/slices/bridge/ (its own SHA256SUMS and GENERATORS). Mutants: tools/mutants/manifest.d/bridge.txt, ids 300..=399.
//! Golden file headers for this slice are declared here, not in tests/common/goldens.rs.
//!
//! Moved here from TEST_PLAN L0a-4 (2026-10-24): the `Mag` / `ExactNum` comparison sets. S2 writes these tests here:
//! - `cmp_exact_num`: radius exponents outside `i32` compare correctly without building the radius; ties; zero radius;
//!   `Infinity` radius (TEST_PLAN section 5 row, oracle: model M1 extension; PLAN C6).
//! - `cmp_exact_diff`: the exact diff-compare `m1 - m2` against `r1 + r2` that CONTRACT 15.2 and 10.2c need; same corner set.
//! - `from_exact_num`, `from_exact_num_lower`, `to_exact_num` round-trip sets and property P9.
