# zenith-bound test plan: wide-range testing, build plan and checklist

**Status: working build plan (2026-10-15).** This is the document the tests get built against. It is not a proposal waiting on sign-off: Jeff's instruction (2026-10-15 10:45 PM CT) is "on all our crates we need wide range testing not just narrow testing... we need to make sure it handles all edges; not sure how we go about testing zenith-bound but we need to be very thorough." Items still open are technical choices for Rusty (section 17); they have a default stated here and the build starts on the default.

- **Written against:** branch `spence/zb-l0-prep` at `581f677` (local worktree `/tmp/zb-testplan`, branch `spence/zb-test-plan`). Another executor keeps adding commits to `spence/zb-l0-prep`; names and counts below were read at `581f677` and must be re-read when the plan is applied.
- **Scope of this document:** planning only. No code changed, nothing pushed. It reads `docs/PLAN.md`, `docs/CONTRACT.md`, `docs/reviews/*`, `tests/mag`, `tests/model`, `tests/goldens/mag_ops.txt`, `scripts/gen_goldens.py`, `tools/rules_scan.py`, `tools/full_gate.sh`, `Cargo.toml`, `clippy.toml`.
- **Levels** use the names in `docs/PLAN.md` Part D: **L0** = integer-only `Mag` plus real balls (split here into **L0a Mag** and **L0b real balls**); **L1** = elementary functions and constants; **L2** = complex, polynomials, series, matrices, then selected specials (L2a to L2d); **L3** = rigorous quadrature; **L4** = validation, adaptive precision, validated workflows.
- **Marks.** `[V]` = I verified it on Pop on 2026-10-15 (command run or file read). `[NV]` = not verified by me: taken from `docs/PLAN.md`, `docs/CONTRACT.md` or from Rusty's input, or a plan/estimate. `[R]` = from Rusty's rounding-core input (Appendix A, folded into the tables). All counts of tests, cases and goldens are **targets**, never results. Nothing in this document has been run through the gate.
- **Contract wins.** Every test cites a `CONTRACT n.m` clause and names its oracle (CONTRACT header rule). Clauses in the "Open clauses" list (O1 to O10) are not normative; a test of an open item is written only after it is decided, except where noted in section 17.

## 1. For Paul: what gets built per level (targets only)

Counts are targets. "Test fns" = `#[test]` functions. "Gate cases" = random/property cases per gate run at fixed seeds. "Deep cases" = per nightly/on-demand run. "Golden rows" = committed mpmath/Fraction golden lines. "Mutants" = hand-written mutants (or cargo-mutants kills) that must each make the suite fail. Today (`581f677`): 50 `#[test]` fns in `tests/mag` and 6,627 golden data lines in `tests/goldens/mag_ops.txt` `[V]` (counted by grep and `wc`, not run).

| Level | What gets built | Test fns | Gate cases | Deep cases | Golden rows | Mutants |
|---|---|---:|---:|---:|---:|---:|
| Shared | `tests/common`: splitmix64 PRNG, exponent-edge and mantissa-pattern generators, replay/shrink, pub-fn coverage script, mutant runner, golden regen-and-diff gate step, rules-scan rule for `-(b-a)` | 25 | n/a | n/a | n/a | 10 |
| L0a Mag | wide-range property sets for all 8 round ops, `cmp`, `from/to_exact_num`, `ulp`, `cmp_exact_*`; exponent-translation properties; constructor/canonical-form checks; widened mpmath goldens | 130 | 300,000 | 50,000,000 | 40,000 | 120 |
| L0b real balls | every L0 Ball op (add/sub/neg/abs/mul/mul_2exp/sqr/powi/div/inv/sqrt/hypot/sum/dot/compare/union/intersect/floor/ceil/sgn/set_prec/is_unique_at/interval/decimal/encode); P1 to P10 for every op x 6 rounding modes x precision schedule; chains; exact-rational enclosure checks | 260 | 200,000 | 20,000,000 | 60,000 | 200 |
| L1 elementary | 21 functions + 4 constants, each: containment over the whole exponent range, exact cases, domains and poles, reduction boundaries, monotonicity edges, both domain policies, overflow/underflow; goldens at 2x precision | 200 | 100,000 | 10,000,000 | 80,000 | 150 |
| L2 | ComplexBall Rect/Disk, BallPoly, BallSeries, BallMat/ComplexBallMat, gamma/lgamma/erf/erfc/J0/J1/zeta; equivalent-expression and chain tests | 220 | 50,000 | 5,000,000 | 40,000 | 120 |
| L3 | `integrate_ball`: analytic and non-analytic integrands, budgets, partial sums, never-narrow-uncertified | 45 | 5,000 | 200,000 | 600 | 30 |
| L4 | adaptive precision, root isolation, verified solve, residuals, eigen-enclosures, ODE; Hilbert n=5..12, singular and multiple-root failure paths | 70 | 5,000 | 200,000 | 1,500 | 50 |

Gate rule for all levels: wide ranges at **fixed seeds** run in the normal gate (fast); deeper runs with more cases (and seeds listed in section 7) run nightly or on demand. Same split as zf's oracle-fast and oracle-deep `[R]`. Effort to build all of this is in section 19 (estimate).

## 2. Starting point (observed at `581f677`)

- `[V]` `src/` holds only `src/lib.rs` (crate doc, `#![forbid(unsafe_code)]`); the `mag` module the tests import (`zenith_bound::mag::Mag`) is not at this head, so the L0 tests are red-first. `Cargo.toml` has an empty `[dependencies]`. I did not run `cargo`.
- `[V]` `tests/mag/` (14 files, 50 `#[test]` fns by grep): add_up, mul_up, div_up, sqrt_up, lower_{mul,div,sqrt,sub}, cmp, corners, golden, model_selftest, ulp, support. `tests/model/` (dyadic, nat, oracle, sets): a pure-integer dyadic model independent of zenith-float. Failing checks return `Err(String)`.
- `[V]` The model refuses (returns `None`, which the checkers turn into a test error) when two nonzero operands' exponents differ by more than `MAX_ALIGN = 4096` (`tests/model/dyadic.rs`). So the present model cannot judge wide-gap cases; section 6 (model task M1) extends it.
- `[V]` Random mantissas come from `Rng` in `tests/model/sets.rs`: xorshift64* with state `seed | 1`, seed `0x5EED`, 5 draws. Operand sets: 12 mantissas (7 fixed incl. `2^63`, `2^63+1`, `2^63+2`, `0xC000...`, `0xAAAA...B`, `u64::MAX-1`, `u64::MAX`, plus 5 random) and 13 mid exponents (`-200..200`), plus an "extreme exponent" set (names `*_extreme_exponent_set_*`). Zero is in every set.
- `[V]` `scripts/gen_goldens.py`: mpmath.libmp directed rounding to 64 bits; 8 ops; operand set = zero plus 11 exponents x 3 mantissas (`2^63`, `2^63+1`, `2^64-1`). It writes `tests/goldens/mag_ops.txt` (6,630 lines incl. 3 comment lines; add_up 1156, mul_up 1156, mul_lower 1012, sub_lower 1156, div_up 1122, div_lower 957, sqrt_up 34, sqrt_lower 34).
- `[V]` **Byte-reproducibility check, 2026-10-15 on Pop:** a scratch copy of the generator under `/tmp/zbgen`, run with `/usr/bin/python3` (mpmath 1.2.1), produced a file `cmp`-identical to the committed `tests/goldens/mag_ops.txt` (sha256 `9d79db0f98b965d0b0fd060fc41ebe9bc9165923510c0b199a72d3deb69c6744`). Nothing under `/home/jsc64/MathWorks` was touched.
- `[V]` **Trap present in the generator:** `from_mpf` returns `None` for a round-down overflow and the writer does `continue`, so no round-down-overflow row is ever written. The decided rule (commit `1faec8c`, tests in `tests/mag/corners.rs`) is that round-down overflow gives `Finite{i64::MAX, u64::MAX}`. The hand-stated corner tests cover it; the golden file does not. Fix is in section 9.
- `[V]` `tools/full_gate.sh TAG` steps: build-debug (`cargo test`), build-release (`cargo test --release`), rules-scan (self-test + `rules_scan.sh`), fmt, clippy `--all-targets -D warnings`, unsafe-scan, no-float-scan, no-panic-scan, no-clone-scan; each under `timeout` (`STEP_TIMEOUT`, default 20m). `tools/rules_scan.py` scans added lines of every `.rs` file under `src tests benches examples` plus `build.rs`, no test-code exemption; allowlist `tools/rules_allowlist.txt` is protected by `.github/CODEOWNERS`.
- `[V]` `Cargo.toml` `[lints.clippy]` denies float_arithmetic, float_cmp, cast_precision_loss, as_conversions, unwrap_used, expect_used, panic, unreachable, todo, unimplemented, indexing_slicing; `clippy.toml` disallows `clone`, `clone_from`, `to_owned`, `cloned` and asm macros. Consequences for test code: no `as` casts (use `try_from`/`from`), no slice indexing (use `get`/iterators), no panics.
- `[V]` Docs inconsistency to fix by whoever owns CONTRACT: at this head `docs/CONTRACT.md` 14.2/14.6 still call round-down overflow and `Infinity` operands of round-down ops "open item O7, not tested", while commit `1faec8c` has tests for them as decided (Paul/Rusty). Not changed here (one-file rule).

## 3. Rules for all test code

Zero `.clone()`/`.to_vec()`/`.cloned()`/`.to_owned()`; no `Copy`/`Clone` derives; no `unwrap`/`expect`/`panic!`/`unreachable!`/`todo!`/`unimplemented!` (a test returns `Err` instead; generators and shrinkers return `Result`); no `unsafe`; no `f32`/`f64` and no hardware float anywhere (including "convenient" test arithmetic); no `#[allow]`; no new dependencies without Rusty's approval; no `as` casts and no indexing (the clippy denies apply to tests); goldens only from mpmath or Arb-style hand derivation, never from zenith-float or zenith-bound output. Python: `/usr/bin/python3` with mpmath 1.2.1 `[V]` (`scripts/requirements.lock`); the box python has no mpmath.

**Never use zf's own result as the oracle for a zf-backed layer** (the self-consistency trap Jeff flagged) `[R]`.

## 4. Oracles, in trust order `[R]`

1. **Independent exact-integer/rational reference written without zf.** `tests/model` is this reference for L0a today `[V]`; it is extended (section 6) for L0b. Rusty's L0 review used such a reference over **212,772 add/sub/mul/div/sqrt edge cases with 0 mismatches** `[R]` (his number; I have not seen that harness). Rusty offers to write the zf-side oracle harness (section 16).
2. **MPFR via the zf mpfr suite**, with rounding mode, precision and exponent range set explicitly in every call `[R]`. MPFR lives in zf's test suite, not in zenith-bound's `Cargo.toml` (no new dependency). In zenith-bound the MPFR-equivalent is **mpmath** (`/usr/bin/python3`, 1.2.1) writing goldens; for transcendentals mpmath at guard bits (`2*prec + 64`, see section 9). Agreement between two oracles is a check, not a proof (PLAN Part E).
3. **Exact rational enclosure checks:** the exact result of the operation on the sampled exact input point, computed in rationals, must lie in `[mid - rad, mid + rad]` `[R]`. Used when no tight oracle value exists (chains, specials, solvers).

Regenerated goldens must byte-match the committed ones `[R]`; the gate step in section 13 enforces it.

## 5. Properties to assert for every op at every level

P1 to P10 are Rusty's `[R]`. Each is a reusable checker in `tests/common/props.rs`; an op's test file calls the ones that apply and lists them in its `COVERS:` header (section 11).

| # | Property | Form of the test | Applies |
|---|---|---|---|
| P1 | **Containment for every rounding mode**, including ball inputs: sample the exact point at the input balls' **edges (`mid-rad`, `mid+rad`) and middle**, plus random interior points; the exact result lies in the output ball | exact-rational check (oracle 3), loop over the 6 valid modes `ToEven, ToOdd, Up, Down, ToZero, FromZero` | every op, every level |
| P2 | `rad` is **never rounded down** (round-up Mag ops only), **never NaN**; `Infinity` is **uninformative and absorbing** (an op with an `Infinity`-radius input gives `Infinity` radius or an error) | Mag-level: compare with oracle 1 (smallest Mag >= exact). Ball-level: output `rad >=` the exact error bound; no NaN is representable in `Mag` (type check) and no NaN midpoint is stored (CONTRACT 6.2) | all |
| P3 | **Widening an input radius never narrows the output radius** (and never loses containment) | metamorphic: for `r1 <= r2` same mids, `rad(f(r1)) <= rad(f(r2))`; inclusion monotonicity `A ⊆ A' => f(A) ⊆ f(A')` | all |
| P4 | **No second rounding:** the output midpoint is exactly zf's single rounded result at `p` | for exact inputs, mid equals oracle-1 correct rounding of the exact result under `rm` (not "close"): bit-equal | L0b and every op that rounds a mid |
| P5 | `rad >= half ulp` for nearest modes, `>= full ulp` for directed modes whenever the mid rounding was inexact; **no wildcard `match` on the mode**; `RoundingMode::None` is **rejected** (returns an error) | assertion on `rad` vs `ulp_mag`/`half_ulp` from the model; `None` returns `Err`. Wildcard ban: a clippy `wildcard_enum_match_arm` deny (or a rules-scan pattern) so a new mode cannot fall through `[NV]`. These are lower bounds, so they hold under either answer to open clause O4 | L0b+, anything with a mid rounding |
| P6 | Exact inputs and exact ops give `rad == 0` **only when `inexact()` is false** (per-op, not inherited). The converse: `inexact()` true gives `rad > 0` | across all modes; pins zf's per-op ternary before anyone relies on it `[R]` | L0b, L1 constants |
| P7 | add/mul **midpoint commutes bit-for-bit, radius symmetric** | `identical(add(a,b), add(b,a))`, same for mul, all modes, ball inputs included | add, mul, hypot, sum order is NOT commutative (sequential index order, PLAN L0) so do not assert it there |
| P8 | `a - a` **contains 0** and `a / a` **contains 1** (dependency may widen, never exclude) | all `a` in the sets incl. balls straddling 0 (`a/a` then returns `Domain`, which is the correct answer) | L0b, L1+ compounds |
| P9 | `from_exact_num` then enclosure **contains the original** | Mag: `from_exact_num(x) >= abs(x)`; Ball: `from_integer/from_rational/decimal parse` enclose the exact value | L0 |
| P10 | **Determinism:** identical bits on x86_64, i686 and thumbv7em `[R]` | byte digests of every golden/grid run compared across targets and across debug/release, 1 vs N threads (PLAN Part J). Note PLAN Part B/J names `thumbv6m-none-eabi`; Rusty names `thumbv7em`. Which target(s) run is open item D8 | all, via grid digests |

**Additional wide-range properties (mine, `[NV]` until built):**

| # | Property | Why |
|---|---|---|
| W1 | **Exponent translation:** with no overflow, underflow or subnormal involved, shifting inputs' exponents by `k` (and `k1`, `k2` for mul) shifts the output mid exponent and the `Mag` exponent by exactly the matching amount, **bit-for-bit** (same mantissas, same radius mantissa) | tests the whole exponent range without materialising `2^(2^31)`; also catches any `i32`/`i64` truncation or wrap |
| W2 | **Mode symmetry:** `Up(f(x))` equals `neg(Down(f(neg x)))` for odd-symmetric ops (add, sub, mul by sign, neg, sqrt domain excluded), `ToEven/ToOdd/ToZero/FromZero` map to themselves under negation; the radius is the same `[R]` (metamorphic aim 6) | catches sign-handling bugs in directed rounding |
| W3 | **Scale by `2^k` commutes** with `mul_2exp` for every op where scaling is exact | `[R]` metamorphic aim 6 |
| W4 | **Precision monotonicity:** raising `prec` never shrinks `rad` of `set_prec`; lowering never loses containment; `refine` (on `Refinable` only) result is contained in the lower-precision result for exact inputs | PLAN Part E, G4 |
| W5 | **`rm` never changes containment**, only tightness: for all 6 modes the output encloses the same exact value, and the balls of different modes overlap | PLAN Part A gap 6 |
| W6 | **Serial vs chain:** a chain of n ops replayed in exact rationals stays enclosed at every step (section 10) | `[R]` aim 7 |

## 6. How "the whole exponent range" is actually reached

Plain random draws at independent exponents give nearly every add a huge exponent gap, and an exact model cannot materialise values like `2^(2^31)`. Three techniques together cover the range:

1. **Exponent-translation (W1)** for mul/div/sqrt/add/sub: check the op near exponent 0 against the exact model (cheap, exact), then check that the same mantissas translated by `k` over the whole range (`k` in the full `i32` range for mids, full `i64` for `Mag`) give the translated result bit-for-bit. Exceptions are exactly the overflow/underflow/subnormal bands, which get their own enumerated sets (edge tables, section 8).
2. **Controlled-gap generators** for add/sub/sum/dot: gap drawn from `{0, ±1, ±2, ±(p-1), ±p, ±(p+1), ±(p+guard-1), ±(p+guard), ±(p+guard+1), random up to the full range}` with guard = 64 (PLAN C6, "exact add only for a bounded gap"), both operand orders, all sign pairs, all 6 modes. Cost: the exact model must work for gaps up to `p+guard+1` (at most 4096+ for the p schedule, which exceeds `MAX_ALIGN = 4096` at p = 4096: raise `MAX_ALIGN` for those cases, see M1).
3. **Gap-aware symbolic model comparison (task M1):** for gaps beyond what can be aligned, the model decides `|result - exact| <= rad` and the "smallest/largest Mag" relations by a **sound bounding argument** (the smaller operand only affects the sticky bit and a one-unit correction), implemented and self-tested in `tests/model` so a wrong model returns an error, not a pass. Until M1 exists, such cases are not asserted (the current model returns `None`, which fails the test, so they cannot silently pass `[V]`).

Mid exponents are zf `Exponent = i32` (full range on 64-bit targets per PLAN K5, CONTRACT 8.3; the 32-bit-target sentence is open O3) and `Mag` exponents are `i64`. Every conversion from `i64`/`i128` to `i32` in a test goes through `try_from`, and each exponent-extreme set contains `EXPONENT_MIN-1` and `EXPONENT_MAX+1` **before** the conversion `[R]`.

## 7. Deterministic generator, seeds, shrinking and replay (no new dependency)

**Generator.** `tests/common/rng.rs`: splitmix64 (one `u64` state, the standard increment `0x9E3779B97F4A7C15` and the two xor-shift-multiply finaliser steps) — about 15 lines, integer-only `[NV: to be written]`. Streams: `stream_seed(suite_id, case_class) = splitmix64(BASE + suite_id * 2^32 + case_class)`, with `BASE` a single constant fixed in `tests/common/seeds.rs` and recorded in this section when written. Each suite lists its seed(s) in one table in that file; the table is the record of fixed seeds. The existing xorshift64* `Rng` (seed `0x5EED`) stays for the already-committed sets so existing random mantissas do not change; whether to retire it later is a minor cleanup.

**Generators (Rusty's aims 1, 2, 4 `[R]`):**
- Exponent draw (mids and `Mag`): **30% within 3 ticks of `EXPONENT_MIN`/`EXPONENT_MAX`** (and `i64::MIN`/`i64::MAX` for `Mag`), the subnormal boundary (`MIN_NORMAL_EXPONENT` ±1, section 8), and powers of two ±1 ulp; remaining draws: about 20% in a band around 0 (±64), about 30% uniform over the whole range, about 10% exact powers of two, about 10% from the controlled-gap rule. The mix is a constant table in `generators.rs`, fixed in the checklist; if a bug class escapes, the table changes by a decision recorded here.
- Mantissa patterns: all-ones, single bit, top bit only (`2^63` for `Mag`), top two bits, alternating `0xAAAA`/`0x5555`, halfway-tie patterns at precision `p` (bit `p` set and all lower bits zero, and one tick either side), `2^63+1`, `u64::MAX-1`, plus uniform random.
- Precision draw: the schedule in section 9 plus uniform random in `1..=4096` and values at `operand precision ± 1`.
- Ball inputs: radius `0`, `1 ulp`, `half ulp`, `huge` (comparable to or larger than `|mid|`), `Infinity`, and random `Mag`; mids and radii at independent exponents (so radius ≫ mid and radius ≪ ulp are both hit).
- Signs: all four sign pairs; both zeros as mids (`+0`, `-0` inputs reach the boundary normalisation of CONTRACT 6.3).
- No `unwrap` in generators: every draw returns `Result` or an `Option` converted to an error.

**Failure report and replay.** Every property failure returns an `Err` whose text is one replay line: `suite=<id> seed=<hex> case=<n> op=<name> prec=<p> rm=<mode> a=<enc> b=<enc>`. `ZB_FUZZ_SEED` (hex) and `ZB_FUZZ_CASES` (count, a count, never wall-clock time, PLAN Part J) override the defaults; `ZB_REPLAY=<seed>:<case>` runs exactly that case. **Shrink:** a deterministic greedy shrinker with a fixed step cap (no randomness, so the shrunk case reproduces): (1) drop radii to 0, then to `1 ulp`; (2) reduce `prec` along the schedule; (3) translate exponents toward 0 by halving (valid because of W1) while the failure persists; (4) replace mantissas by simpler patterns (all-ones, then top-bit-only); (5) for chains, delete steps. The shrunk case is printed as a ready-to-paste enumerated regression entry for a named adversarial set (the B3 ban on single exact-input tests means the entry goes into a **set**, not a lone test; PLAN Part E).

**Cost (estimates, unmeasured `[NV]`).** `Mag` ops are nanoseconds; the cost is the bigint model, roughly microseconds per case at 64 bits and tens to hundreds of microseconds at 4096 bits, and debug builds 10 to 30 times slower than release. Counts in section 1 are therefore sized in two tiers: the gate tier must fit the debug `cargo test` step inside its timeout; deep runs are release only. **Calibration is the first build step** (checklist item S0): time one op family on Pop in debug and release and set the final case counts from the measurement. Memory: the largest set is one op's operand table in memory; chains hold O(n) values. Maintenance cost: any change to the generator's draw order changes every case from that suite, so the draw order is frozen once recorded and changes bump a generator version in `seeds.rs` (goldens are unaffected because they come from Python, not from the Rust generator).

## 8. Edge classes per level and what a test asserts

Columns: **Class** | **A test asserts** | **Oracle** | **Source**. Source tags: `C n.m` CONTRACT clause; `[R]` Rusty; `PLAN` plan part. "Err" means the op returns `BoundError`/`Indeterminate`/`Infinity` radius as the contract says, and the test checks it returned (errors return answers; a panic would abort the test binary).

Mapping of Rusty's list to PLAN levels (mine; Rusty to confirm, item D9): his "L0 (Mag)" is L0a; his "L1" list (signed zero, subnormals, precision limits, ties, cancellation, division, sqrt) is the real-ball arithmetic edge list, PLAN L0b; his "L2-L4" list covers PLAN L1 (functions), L2 (specials, compounds), and L4 (solvers).

### 8.1 L0a: `Mag` (integer-only)

| Class | A test asserts | Oracle | Source |
|---|---|---|---|
| Canonical zero | every `mant == 0` value is the one zero `{exp:0, mant:0}`; all zeros equal; zero early-returns in every op before `leading_zeros` | model, rules in clause | C 2.1, 2.3 `[R]` |
| Non-canonical rejected | the constructor/decoder rejects `mant` in `1..2^63` and `mant == 0` with `exp != 0` | exact | C 2.1 `[R]` |
| `Infinity` | greatest in `Ord`, equal to itself; **absorbing for round-up ops** (`add_up`, `mul_up` incl. `0*Infinity`, `sqrt_up`, `div_up(Infinity, b)`); **zero for round-down ops**; `0/0` and `x/0` give `Infinity` (round-up) | clause text | C 14.5, 14.6, 2.4 `[R]`; round-down results with `Infinity`/zero divisor are decided by `1faec8c` |
| Exponent extremes | operands at `i64::MIN`, `i64::MIN+1`, `i64::MAX-1`, `i64::MAX`; mantissas `2^63`, `2^63+1`, `u64::MAX`; no wrap (checked arithmetic) | model | C 3.1 `[R]` |
| Carry across the exponent | `add_up`/`mul_up` where the mantissa carry makes `2^64` (renormalise: `mant = 2^63`, `exp+1`), incl. at `exp = i64::MAX` (carry overflows to `Infinity`) | model, mpmath goldens | C 1.3, 14.3 `[R]` |
| Round-up overflow / underflow | exponent above `i64::MAX` gives `Infinity`; below `i64::MIN` gives smallest positive `Finite{i64::MIN, 2^63}`, never zero | model | C 3.2, 3.3 |
| Round-down overflow / underflow | overflow gives `Finite{i64::MAX, u64::MAX}` (largest Mag, a valid lower bound); underflow may give zero | model | C 14.2 `[R]` + `1faec8c` |
| `sub_lower` | `a < b` gives zero, `a == b` gives zero, `a > b` gives largest Mag <= exact; huge gaps (`a` huge, `b` tiny: result is `a` minus one unit); near-equal operands (massive cancellation across the full 64 bits) | model | C 14.2 `[R]` |
| `sqrt` | odd and even exponent parity; `mant = 2^63`, `2^63+1`, `u64::MAX`; exact powers of two (even and odd exponent, result exact, no `+1`); perfect squares of 32-bit mantissas; `sqrt_up >= sqrt_lower` and they differ by at most 1 unit | model (squares, exact comparison) | C 14.4 `[R]` |
| Mantissa patterns | top-bit-only (`2^63`), all-ones, `2^63+1`, alternating `0xAAAA`/`0x5555` | model, goldens | `[R]` |
| Shifts | alignment gaps `0, 1, 62, 63, 64, 65` and `>= 128`, `i64`-range gaps: sticky bit and "gap >= 64: larger + 1 ulp" rule | model | C 14.3 `[R]` |
| `Ord` consistency | transitivity, antisymmetry, totality, `Eq` follows `Ord`, `==`/`<`/`<=` agree with `cmp`, over a set containing zero and `Infinity`; sorting a shuffled set is stable under any permutation | exact | C 14.5 (O8) `[R]` |
| Round-up/down bracketing | `down(x) <= exact <= up(x)` and `up` is within one Mag-ulp of `down` for every op and sampled operand pair | model | C 14.1, 14.2 |
| Exponent translation | W1 for `mul_up`, `div_up`, `sqrt_up` across the full `i64` range away from the overflow bands | W1 | section 5 |
| `ulp_mag`, `half_ulp`, `smaller_than_half_ulp` | boundary sets at `mid_exp - prec - 1` for `prec` in the schedule and `mid_exp` near `i64::MIN/MAX`; zero true, `Infinity` false; threshold below `i64::MIN` never rounded up | model | C 1.4, 14.7 |
| `from_exact_num` (+`_lower`), `to_exact_num` | `abs(x)` rounded up/down to 64 bits; carry at all-ones; exponent beyond zf `i32` returns error not wrap; round-trip contains original (P9) | model, mpmath | C 2.3 PLAN C6 |
| `cmp_exact_num`, `cmp_exact_diff` | radius exponents outside `i32` compare correctly without building the radius; ties; zero radius; `Infinity` radius | model (M1 extension) | PLAN C6 |

### 8.2 L0b: real balls (arithmetic, compare, precision, decimal, encode)

| Class | A test asserts | Oracle | Source |
|---|---|---|---|
| Signed zero, both ways | `+0`/`-0` mids as operands in every op, `-0` results under `Down`, `0 * negative`, `x - x` under each mode: output is normalised to `+0`; `identical()` and the encoding see one zero; `-0` input to `sgn`, `floor`, `ceil`, `sqrt`, `div` | exact | C 6.3 `[R]` |
| Zero radius and zero mid | mid 0 radius 0; mid 0 radius > 0; mid > 0 radius 0; all 3 in every op; `rad == 0` only when `inexact()` is false (P6) | exact | C 2.2 `[R]` |
| Subnormals and the boundary | on the zf grid `2^(EXPONENT_MIN - p)`: results **at, just above and just below** the smallest normal, at `MIN_NORMAL_EXPONENT - 1, 0, +1`, all modes; `exponent()` for zero must map to `EXPONENT_MIN` `[R]` (zf's zero reports `exponent() == 0`, a known trap, section 12); underflow to zero is inexact and `rad` covers the missing range | model (subnormal ulp = `2^(EXPONENT_MIN - p)`) | C 14.7, PLAN G3, C6 `[R]` |
| Exponent extremes | `EXPONENT_MIN`, `EXPONENT_MIN-1`, `EXPONENT_MAX`, `EXPONENT_MAX+1` tested **before** converting to `i32`; never wraps; the `Mag` exponent of `exp(mid) - prec - 1` below `i32::MIN` is fine in `Mag`, not fine as a zf number | exact | C 8.3, PLAN K5 `[R]` |
| Overflow | `Inf`, NaN, or `inexact()` with `exponent() == EXPONENT_MAX` is overflow; zf saturates to `max_value(p)` toward zero (P12 behaviour); result is `Overflow`/`Infinity`-radius per O2, never a finite ball that excludes the true value; ± signs; add/mul/div/sqr/powi/`mul_2exp` | exact (symbolic: product exponent > max) | C 3.3, 10.4, O2 `[R]` |
| Underflow | result exponent below `EXPONENT_MIN` rounds per mode; `rad` is not zero and covers the lost value; `mul` of two tiny balls, `div` of tiny by huge | exact | C 10.4 `[R]` |
| Precision limits | `p = 1, 2, 3`, the smallest zf supports, word boundaries **31/32/33, 63/64/65, 127/128/129**, `p` larger than operands, `p` much smaller than operands via `add(&zero, p, rm)`, `p = 4096`; **mixed operand precisions in both orders** | model (exact-bit rounding, any `k`) | C 5, 8.1 `[R]` |
| `prec` conversion | `u32 -> usize` only via `try_from`; `prec = 0` rejected; `prec` above the cap gives `Resource` | exact | C 8.1 |
| Ties | exact halfway for `ToEven` vs `ToOdd`: both parities, both signs; for `Up/Down/ToZero/FromZero` **one tick either side** of the halfway point; known counterexample **`p = 3`, `x = 81/64`: `Up` gives `3/2`, error `15/64 > half ulp`** (so directed `rad` must be a full ulp) | model | C 9.4 (O4) `[R]` |
| `RoundingMode::None` | every op returns an error for `None`; no `match` falls through a wildcard | exact | P5 `[R]` |
| Cancellation | `a - b` close with an **exactly representable result** keeps the minimal `rad` (no spurious growth); **catastrophic cancellation** with different input radii (`rad` is `r + s + eps`, never smaller); `a + (-a)`; an **exactly zero result with nonzero input radii** (mid `+0`, `rad = r + s`); `(1 + 2^-k) - 1` for k across `1..p+guard` | exact | PLAN L0 tests `[R]` |
| Carry into the next binade | mantissa all-ones rounding up in each mode and sign; result exponent increments; `ulp_mag` uses the pre-round exponent (O5) | model | C 9.4 `[R]` |
| Huge gap add/sub | gaps at `p+guard±1`, near `EXPONENT_MAX - EXPONENT_MIN`, mixed signs, all 6 modes; no allocation proportional to the gap (a test completes in bounded time and memory) | W1/M1 | PLAN C6 |
| `mul`/`sqr`/`powi` | straddle-0 ball square gives `[0, (abs(m)+r)^2]`; even `powi` uses the sqr rule; `powi(x, 0)`, `powi(x, 1)`, `powi` with exponent `i32::MAX`/`MIN` (bounded work or `Resource`); `mul_2exp` at exponent extremes; radius terms `abs(m)·s + abs(n)·r + rs + eps` all round up | exact | C 9.3 PLAN L0 |
| Division | exact vs inexact; `1/3` repeating; division by a subnormal; **ball straddling zero gives `Domain`** (both policies, C 4.2) / **`Infinity` rad where the policy says**; `0/0`; `a/a` contains 1; divisor with `abs(m_b) == r_b` (touching exactly); `div` formula `(r_a + abs(q)_up r_b)/(abs(m_b) - r_b)_down + eps`, numerator up, denominator down | exact rational | C 4.1 to 4.4 `[R]` |
| `sqrt`/`hypot` | perfect squares; exactly-half-ulp cases; subnormal; ball straddling zero (`Clip` gives `[0, sqrt(m+r)]`; `Strict` gives `Domain`); entirely negative gives `Domain` under both; `m > r` decided by `cmp_exact_num`; `hypot` never `Domain` | exact | C 10.2 PLAN C6 `[R]` |
| Domain policy | every restricted op returns the same shape under `Strict` and `Clip`; every `DomainPolicy` value is exercised (no wildcard) | exact | C 10.1, 15.1 `[R]` |
| `sum`/`dot` | empty (identity), single element, alternating-sign sums (catastrophic cancellation), huge-gap terms, length 1000+; sequential index order is byte-stable; one rounding | exact rational | PLAN L0 |
| Compare / set ops | `Tri` comparisons on disjoint/touching/overlapping/nested/identical balls and with `Infinity` radius; `contains` at the exact edge (equality counts), edge ± 1 ulp; `overlaps`; `union` (hull rounded outward), `intersect` returns none exactly when disjoint (touching edges overlap); `is_exact` only with proof; `identical` is bitwise | exact | PLAN C5 |
| `floor`/`ceil`/`sgn` | ball inside one integer gives exact ball; straddling an integer gives `Indeterminate`; `sgn` of balls containing 0; `-0` | exact | PLAN L0 |
| `set_prec`, `is_unique_at` | lowering adds the mid rounding error to `rad` only if inexact; raising never shrinks `rad`; `is_unique_at` at ties and at the rounding-cell boundaries for every mode; `rad = 0` unique iff representable; `Infinity` never unique | model (cell boundaries) | C 5, PLAN C6 `[R]` |
| `Interval` conversions | `to_interval` lo rounded Down, hi rounded Up; `to_ball` encloses; round trip only enlarges | exact | PLAN C5 |
| Decimal | parse `0.1`, `1e-400`, `1e400`, long digit strings, `-0`, malformed input (error, no panic); parse encloses exact decimal; print is outward-integer: all printed digits are proven; `[0.9999, 1.0001]` fallback; `0 ± r`; round-trip print->parse contains original | exact (Fraction) | PLAN Part F |
| Encoding | one value one encoding; decoder rejects tags 2, 3, sign 1 on zero, non-canonical `Mag`, truncated input, trailing bytes, `len` that exceeds the input; encode->decode->encode byte-identical; byte flips of any single bit are either rejected or decode to a different canonical value (never accepted as the same ball) | exact | PLAN Part J |
| NaN inputs from zf | both operands NaN of different kinds: the error kind is the contract's, not whichever NaN came first | exact | `[R]` trap, section 12 |
| `inexact()` pin | per-op for add/sub/mul/div/sqrt: a grid over all modes pins that `inexact()` reflects this operation only (exact op on inexact inputs reports exact), before any radius logic depends on it | exact | P6 `[R]` |
| Determinism | grid digests (section 9) identical across debug/release, 1 vs N threads, i686, thumbv7em/thumbv6m, x86_64 | digest compare | P10 `[R]`, PLAN Part J |

### 8.3 L1: elementary functions and constants

Functions: exp, exp2, expm1, log, log2, log10, log1p, sin, cos, tan, asin, acos, atan, atan2, sinh, cosh, tanh, asinh, acosh, atanh, pow (21), constants pi, e, ln2, ln10 (4); sqrt is L0b. Every function gets every row that applies.

| Class | A test asserts | Oracle | Source |
|---|---|---|---|
| Arguments at 0, ±1, `2^-p`, huge | `f` at 0, `±1`, `±2^-p`, `±2^-400`, `±2^30`, `±2^1000`, and at `EXPONENT_MIN/MAX`: contains the true value; overflow (`exp(2^30)`) gives `Overflow`/`Infinity`, never saturation; `exp(-2^30)` underflow: radius covers `[0, tiny]` | mpmath at `2p + 64` bits | PLAN L1 `[R]` |
| Exact cases | `exp(0)`, `log(1)`, `sin(0)`, `atan(0)`, `pow(x, 0)`, `exp2(n)`, `log2(2^n)`, `sqrt` of squares: result mid exact and `rad == 0` only if proven; all 6 modes (the Ziv trap: no spin, no wrong directed neighbour) | exact rational | PLAN P14 |
| Poles and branch points | `tan` near `pi/2 + k pi`; `atan2` at `(0,0)` (`Domain`, both policies), near the negative real axis cut, on the cut; `log` at 0 and on negative; the pole returns `Domain`/`Infinity` radius, never a finite ball | mpmath, rules | PLAN L1 `[R]` |
| Domain edges | `log 0`, `log` negative, `asin(±1)`, `acos(±1)`, `acosh(1)`, `atanh(±1)`, balls touching ±1: `Strict` gives `Domain`, `Clip` encloses over the intersection and gives `Domain` only if empty/single pole | rules + mpmath | C 10.2 `[R]` |
| Argument-reduction boundaries | arguments near multiples of `pi/2` and `ln 2`, at `k pi/2 ± 2^-p`, `sin(10^100)`, `sin(2^1000)`, `exp(k ln2 ± 2^-p)`; reduction constants enclosed at `prec + exponent + guard` bits | mpmath at `2p + exponent + 64` | PLAN L1 `[R]` |
| Results near binade boundaries | argument chosen so the true result is within 2^-p relative of a power of two (both sides), all modes: carry into the next binade handled | mpmath | `[R]` |
| Monotonicity-region edges | extremes of `sin/cos` (near `pi/2 + k pi`), `tanh` saturation, `cosh` minimum at 0, `atan` tails; the `min(r, 2)`-style caps bind exactly at their thresholds | mpmath | `[R]`, PLAN L1 radius rules |
| Cancellation inside functions | `log(1 + 2^-400)`, `log1p`/`expm1` at `±2^-400`, `1 - cos(x)` style: relative radius stays within the stated factor (section 9) | mpmath | PLAN L1 |
| Radius rules | `exp`: `exp_up(m) r exp_up(r) + ulp`; `sin/cos`: `min(r, 2) + ulp`; `ln`, `sqrt`, others by PLAN L1 formulas: each is bounded below by the mean-value bound `sup abs(f') times r` evaluated at sampled points (P1 edge points) and no rule is checked by trusting its own formula | exact/mpmath | PLAN L1 |
| Mid rounding | the mid is a directed or `ToEven` call per the P14 finding: `Down`/`Up` calls bracket; `ToEven` + 1 ulp: the ball contains the truth at hard-to-round arguments (table-maker's-dilemma inputs from mpmath at 4p bits) | mpmath | PLAN P14 |
| Per-op `inexact()` | transcendentals: use the documented bound or the full-ulp rule, never `inexact()` as a per-op fact `[R]` | rules | `[R]` |
| `DomainPolicy` | every restricted function under both policies at each domain edge | rules | C 10 |
| Constants | `pi`, `e`, `ln2`, `ln10` at every precision in the schedule; call order A then B equals B then A, fresh vs warm cache byte-equal; `Refinable::refine` result contained in lower-precision result | mpmath | PLAN Part J |
| Compositions | expression chains in section 10, dependency-problem widening and subdivision; both algebraic forms of one expression enclose the truth | exact/mpmath | `[R]` |
| Determinism | digests as in 8.2 | digest | P10 |

### 8.4 L2: complex, polynomials, series, matrices, specials

| Class | A test asserts | Oracle | Source |
|---|---|---|---|
| Complex Rect/Disk | containment of `f(z)` at sampled points in the disk/rect; conversions only enlarge (`rect->disk->rect` contains the original); zero real or imaginary part; `-0` parts; huge/tiny parts; branch cuts: ball straddling a cut returns an enclosure of the union; principal branch values on both sides of the cut | mpmath complex | PLAN L2a |
| Polynomials | Horner vs power form of the same polynomial both enclose the truth; coefficients that are balls with `Infinity` radius; degree 0, 1, 100+; evaluation at 0, ±1, huge and tiny `x`; root bounds (Cauchy/Fujiwara) contain all roots of generated polynomials with known roots; leading coefficient ball touching 0 returns no finite bound | Fraction exact | PLAN L2b `[R]` |
| Series | order `n = 0, 1, many`; truncation is never dropped (order stated or tail bound carried); mul/compose/inverse/exp/log; inverse of a series whose constant term touches 0 returns an error | Fraction/mpmath | PLAN L2b |
| Matrices | sizes 1, 2, 3, 10; product formula `abs(A) rB + rA (abs(B) + rB)` encloses exact product of sampled exact matrices; identity, zero, rank-deficient; Hilbert; determinant and inverse enclose the exact value; **singular matrix returns an error, not a bogus tiny ball** | Fraction exact | PLAN L2c `[R]` |
| Specials | gamma/lgamma at 0 and negative integers (poles, `Infinity` radius or `Domain`), at `1`, `2`, half-integers, huge `abs(z)` (Stirling) and reflection region; erf/erfc tails (`erfc(30)`), `erf(0)`; J0/J1 at 0, first zeros, huge `x`; **zeta at 0, -1, 1** (pole at 1; **zf 2.0.0 fixed zeta at 0 and -1, use mpmath golds there** `[R]`) and at even negative integers, critical line, large imaginary part | mpmath | PLAN L2d `[R]` |
| Refusals | anything without a proof returns `Indeterminate`/`Infinity` radius | rules | PLAN L2d |
| Compounds | chains of 2, 10, 100, 1000 ops, algebraically equal forms, Horner vs power form, large cancellation chains: see section 10 | exact rational replay | `[R]` |

### 8.5 L3: quadrature

| Class | A test asserts | Oracle | Source |
|---|---|---|---|
| Analytic integrands | `integral` encloses the truth for entire integrands at 5+ precisions; tightness `rad <= 2^-(prec-10) abs(I)` (metric in section 9) | mpmath `quad` at 2p, closed forms | PLAN L3 |
| Non-analytic | `abs(x)` across 0, floor, piecewise, sqrt at an endpoint: wide but valid; **never a narrow uncertified result**; budget exhaustion returns the partial sum with the remaining-piece bound | exact closed forms | PLAN L3 |
| Adversarial | `exp(-x^2)` on a huge interval, `sin(1/x)` (must be wide), near-singular `1/(x^2+eps^2)`, `sin(1000 x)`, endpoints equal (zero-width), reversed endpoints, infinite-radius endpoint balls | mpmath | PLAN L3 |
| Budgets | counts only, never wall-clock; same input gives the same bytes | digest | PLAN Part J |

### 8.6 L4: validated workflows

| Class | A test asserts | Oracle | Source |
|---|---|---|---|
| Adaptive precision | stops at `accuracy_bits >= target` or budget; budget exhaustion returns best ball with `Resource` metadata; target at the precision limit; termination caps respected (counts) | exact | PLAN L4 `[R]` |
| Root isolation | **multiple root** (never tagged `Unique`), clustered roots, no root, root at an interval endpoint, tolerance at the precision limit; tags `Unique`/`Exists`/`Unknown` correct against polynomials with known roots | Fraction exact | PLAN L4 `[R]` |
| Verified linear solve | **ill-conditioned Hilbert n = 5..12 (and 8..12 as the stress range)** contains the exact rational solution or reports failure; **singular matrix returns an error**; `‖I - RA‖ >= 1` returns failure; never a narrow wrong answer | Fraction exact | PLAN L4 `[R]` |
| Residuals / eigen | residual ball contains the true residual; symmetric eigen-enclosures contain exact eigenvalues of rational test matrices; general matrices refuse | exact | PLAN L4 |
| ODE | `y' = -y`, harmonic oscillator, short-horizon Lorenz: enclosure contains the exact/mpmath solution at every step; a-priori enclosure failure returns an error; step-size shrink to the cap returns failure, not a plausible ball | mpmath `odefun` or closed forms | PLAN L4 |
| **Failure rule for all of L2 to L4** | **A failed enclosure returns an error or `Infinity` radius, never a plausible small ball** | assertion on every refusal path | `[R]` |

## 9. Goldens: many precisions, containment, tight and loose checks

**Tools on Pop `[V]` 2026-10-15:** `/usr/bin/python3` 3.12.3 with mpmath 1.2.1 (`mpmath.iv` present, backend `python`, no gmpy2); python-flint is **not** installed; `cargo` 1.99.1; `cargo-fuzz` 0.13.2 is installed; `cargo-mutants` and `cargo-llvm-cov` are **not** installed; `rg` is not installed (use `grep`); 32 cores.

**Precision schedule PS** (bits): `1..16` each, then `20, 24, 31, 32, 33, 48, 53, 63, 64, 65, 96, 113, 127, 128, 129, 192, 256, 257, 384, 512, 1000, 1024, 2048, 4095, 4096` (41 values). It extends the "2 through 4096" request down to 1 because CONTRACT 8.1 allows `prec >= 1` and Rusty lists `p = 1` `[R]`; it includes every word boundary Rusty lists (31/32/33, 63/64/65, 127/128/129). **Fast subset PS-fast** (gate): `1, 2, 3, 4, 8, 24, 31, 32, 33, 53, 63, 64, 65, 127, 128, 129, 256, 1024` (18 values). PS-deep = all 41 plus random values in `1..=4096`.

**What a golden row holds.**
- *L0a Mag rows* (existing format, extended): `OP EA MA EB MB R`, `R` = `ER MR`, `inf`, or (new) the round-down-overflow result `max` (decided value `i64::MAX u64::MAX`). Computed by mpmath.libmp with explicit 64-bit directed rounding.
- *L0b exact-input rows:* `op prec rm  inputs(sign,exp,mant-hex[,rad])  expected_mid  expected_inexact`, computed by mpmath `mpf_add/sub/mul/div/sqrt` with `prec` and the rounding mode **set explicitly** (mpmath has no `ToOdd`/`FromZero` names; those are produced from the exact Fraction by an explicit rounding function in `goldlib.py`, itself self-tested against mpmath on the modes both support, and against the Rust model through P4 `[NV]`).
- *Enclosure rows (L0b with ball inputs, L1+):* the input ball and a **truth interval** `[lo, hi]` for sampled input points (edges, middle, random interior), as exact dyadic values rounded **outward** at `2*prec + 64` bits. The test checks, for each of the 6 rounding modes, that the output ball contains `[lo, hi]` (containment) and applies the tight and loose checks below. The truth does not depend on `rm`, so one row serves all modes.
- *Truth source for transcendentals:* prefer `mpmath.iv` (rigorous interval arithmetic) where it implements the function; otherwise mpmath `mp` at two working precisions (`wp` and `wp + 64`) widened by 2 units at `wp`, and **the row is rejected by the generator unless the two agree to at least `prec + 40` bits**. mpmath's `mp` functions are not documented as correctly rounded in every case, so this is a cross-check, not a proof (open item D2). Exact cases come from Fractions.

**Metric (defined; factors are open items for Rusty, D2).** For an op `f`, input balls `X_i`, sample points `s` (exact dyadic points in the balls), output ball `(mid, rad)`, working precision `prec`:
- *True error at a point:* `err(s) = abs(mid - f(s))`, exact. **Containment:** `err(s) <= rad` for every sampled `s` (this is P1, exact-rational comparison, no tolerance).
- *Floor `U(s)`:* one full ulp of `f(s)`'s binade at `prec` (so that a lucky tiny `err` cannot blow up a ratio), and `U = 2^(EXPONENT_MIN - prec)` for results at or below the subnormal boundary.
- *Reference radius `R_ref`:* `max over sampled s of abs(f(s) - f(m))` plus `U`, where `m` is the input midpoint (the observed spread of the true value across the input ball, plus the unavoidable rounding). For exact inputs this reduces to `err + U`.
- ***Tight check:*** `rad <= tau_op * R_ref`. ***Loose check:*** `rad <= lambda * R_ref`, with `lambda >> tau_op`; it guards against absurdly wide radii that still contain the truth. A result with `rad = Infinity` is permitted only where the contract allows it (domain/overflow), and then the check is skipped and replaced by the refusal assertion.
- *Tracked, not asserted:* the distribution of `rad / R_ref` (min, median, p99, max) per op and precision is written to a report; a ratchet file (`tools/tightness_baseline.txt`, like the clone-count baseline) fails the gate if the maximum for any (op, precision) grows. `rad / rad_arb` against python-flint (PLAN Part E) is added only if python-flint is installed (not on Pop now).
- **Proposed starting factors (placeholders for Rusty to set, D2):** `tau`: add/sub 2, mul 4, div 4, sqrt 4, sqr 4, powi(n) `2n`, sum/dot of n terms `n`; exp/exp2/expm1/log family 8, trig/hyperbolic and inverses 16, pow 32; L2 specials 64; matrices `4n`. `lambda`: `2^10` for L0/L1, `2^16` for L2; L3 uses PLAN's `rad <= 2^-(prec-10) * abs(I)` for entire integrands; L4 per-set. These are guesses to build against; they change only by a recorded decision.

**Generator scripts** (all under `scripts/`, `/usr/bin/python3`, mpmath 1.2.1, `scripts/requirements.lock` unchanged): `gen_goldens.py` (Mag; extended), `gen_ball_goldens.py` (L0b), `gen_fn_goldens.py` (L1), `gen_l2_goldens.py`, `gen_l3_l4_goldens.py`, shared `goldlib.py` (rounding functions, splitmix64 in Python, row formatting). Rules for **byte-reproducibility**:
1. each script asserts `mpmath.__version__ == "1.2.1"` and exits nonzero otherwise; Python's `random` is not used (its algorithms changed across 3.x); `goldlib.splitmix64` mirrors the Rust generator and is version-independent;
2. no iteration over `dict`/`set` order; every list sorted by a documented key; LF line endings; no timestamps, hostnames or paths in the file; the header names the script, generator version and mpmath version only;
3. integers printed in decimal (as the existing file does `[V]`), dyadic values as `sign exp mant`;
4. a `--check DIR` mode writes to a temp dir and `cmp`s against the committed file;
5. `tests/goldens/SHA256SUMS` lists every golden file and the gate checks it; in addition one Rust test requires every golden file to be non-empty with the expected header line, so an emptied file fails even if the sums were edited.
6. **Verified for the existing Mag generator** (section 2): regenerated output is byte-identical.

**Gate step `goldens-regen`** (new in `tools/full_gate.sh`): run each generator with `--check` into a temp dir; fail on any difference or missing file; print the first differing line. Fast tier regenerates files below a size/time bound (target under 2 minutes total, unmeasured); the full set regenerates nightly. **Size policy (D5):** commit golden rows up to about 5 MB per level; beyond that, commit the generator plus per-file SHA-256 and regenerate to compare (still byte-checked, since the hash must match).

**Fixes to the existing Mag generator (build items):** emit the round-down-overflow rows (currently dropped, section 2); widen the operand set (mantissas: all `[V]` patterns in section 7; exponents: add `±(p)`-neighbours, `i64::MIN/MAX` ±3, the `2^63`-boundary exponents, gaps `0,1,62,63,64,65,127,128`); add the `sqrt` odd/even-exponent grid (today 34 rows per sqrt op `[V]`); keep line order stable so diffs stay reviewable.

## 10. Adversarial cases and fuzz harness

**Adversarial sets (each a named, enumerated generator; B3 ban applies).**
- *Cancellation chains:* `((1 + 2^-k) - 1)` for `k` in `1..p+guard`; `a - b - a + b` in every association; `x - x` computed through different paths; `(a + b) - a` with `b` below, at and above half an ulp of `a`; repeated `x <- (x + y) - y` for 1000 steps.
- *Near-overflow:* values within 2^3 of `EXPONENT_MAX` and `i64::MAX` (Mag): `mul`, `sqr`, `powi`, `add` of two near-max values (carry overflow), `div` of near-max by near-min, `mul_2exp`.
- *Near-subnormal:* values within 3 ticks of `EXPONENT_MIN` and the subnormal grid, every mode, `mul` of two values whose product lands across the boundary, division by subnormals.
- *Alternating-sign sums:* `sum` of `(-1)^i * t_i` with `t_i` from harmonic, geometric, and huge-gap sequences; exact rational replay; `dot` with cancellation.
- *Ties at exact half:* operands crafted so the exact result is a tie at `p` for every `p` in PS-fast, all modes, both parities and signs, plus one tick either side `[R]`; includes `p = 3, x = 81/64` (`Up` gives `3/2`).
- *Repeated ops growing radii:* `x <- x*(1+tiny)` 1000 times; `x <- x*x` until `Mag` overflow to `Infinity` (the radius must become `Infinity` or an error, never wrap or saturate); `x <- sqrt(x)` 1000 times toward 1; `x <- 1/x` pairs; precision change inside a chain (`set_prec` down then up).
- *Compounds `[R]`:* chains of **2, 10, 100, 1000** ops. Protocol: a seeded random program over the L0b op set (and L1 later); a sample point is drawn inside every fresh input ball; the program is evaluated in ball arithmetic and, step by step, in exact rationals at the sample point; after every step assert containment (P1). At the end assert (a) `rad >= max over samples of abs(mid - exact)`; (b) `rad` is **at most as large as the interval bound predicts**: the model evaluates the same program in exact interval arithmetic (rationals, no rounding) and the check is `rad <= hull_radius + allowance(n, prec)`, where `allowance` is the model's own accumulation of per-step ulp terms (definition is part of D2); (c) the growth per step is monotone for ops that cannot shrink uncertainty (P3). The same expression written two algebraically equal ways (Horner vs power form, `(a+b)^2` vs `a^2+2ab+b^2`, `a*(b+c)` vs `ab+ac`) must both enclose the truth, and their balls must overlap.
- *Cross-checks against zenith-float exact results (exact vs ball enclosure):* for exact inputs, the exact result from zf's `*_full_prec` ops must lie inside the ball **and must equal the independent model's exact result**. zf's value is never the sole oracle (section 4): a zf/model disagreement is reported as a bug in one of them, not accepted as truth.
- *Seeded known traps:* section 12.

**Fuzz harness (no new dependency).** An ordinary integration-test target (`tests/fuzz_*.rs`, one per level) using `tests/common` (section 7). Properties P1 to P9 and W1 to W6 run over generator draws; counts and seeds come from `seeds.rs` and the `ZB_FUZZ_*` variables. Gate tier: fixed seeds, section 1 gate counts. Deep tier: more cases and the extra seeds, release build only, nightly or on demand. **Fuzz aims `[R]`, mapped:**

| # | Aim | Where it lives |
|---|---|---|
| 1 | exponent-edge-weighted generators (30% within 3 ticks of `EXPONENT_MIN/MAX`, subnormal boundary, powers of two ±1 ulp) | section 7 generators |
| 2 | mantissa patterns (all-ones, single-bit, alternating, top-bit only, halfway-tie patterns at `p`) | section 7 generators |
| 3 | per-op, per-mode, per-`p` grids with **pinned digests** like zf's `add_sub_grid_digest` `[R: zf name not read by me]` | `tests/goldens/grid_digests.txt`: a 64-bit FNV-1a (and a second independent 64-bit fold) over the ordered encoded outputs of each grid, integer-only, no crate; a digest changes only in a commit that bumps the op version (PLAN Part J). Grids test determinism (P10), not correctness |
| 4 | ball inputs with rad 0, 1 ulp, half ulp, huge, `Infinity` | section 7 |
| 5 | differential fuzz vs the integer oracle at `p <= 64` and vs MPFR at `p` up to 4096 | `p <= 64`: `tests/model` (live). Up to 4096: the bigint model for all `p` (slow, deep tier) plus mpmath goldens (gate) plus zf's own mpfr suite (zf repo) |
| 6 | metamorphic: swap commutative operands, negate and flip direction (`Up(x) == neg(Down(-x))`), scale by `2^k` | W1, W2, W3, P7 |
| 7 | long random chains replayed in exact rationals | compounds above |
| 8 | record the failing seed, shrink, no `unwrap` in generators | section 7 |

**Decoder fuzz:** the canonical-encoding decoder gets mutation-style fuzz inside the same harness (flip/insert/delete bytes, fixed seeds; accepts only canonical input; encode(decode(x)) == x when accepted). `cargo-fuzz` (libFuzzer, nightly, `libfuzzer-sys` dependency, installed on Pop `[V]`) is **not** used for ball ops; whether to add it for the decoder only (PLAN Part I mentions it) is a tool/dependency choice for Rusty (D3). Default: no.

## 11. Coverage of every public fn, and proof that tests can fail

**Public-API coverage list (checkable).** `tools/pubfn_coverage.sh` (grep and awk only; no `rg` on Pop `[V]`):
1. lists every `pub fn` / `pub const fn` in `src/**/*.rs` as `Type::name` (the enclosing `impl Type` or module path), every `impl Trait for Type` (e.g. `Ord for Mag`), and every `pub` enum variant constructor path;
2. every test file declares `//! COVERS: Type::name, Type::name, ...`; the script collects them;
3. **fails** if a public fn is in no `COVERS` line, if a `COVERS` name no longer exists (stale), or if the declaring file contains no `#[test]` whose text mentions the name (`grep -w`);
4. an exemption list `tools/pubfn_coverage_exempt.txt` starts empty and sits under the same `CODEOWNERS` protection as `tools/rules_allowlist.txt` `[V]`.
Name coverage is deliberately weak; it only proves a test exists that names the function. The strong check is the mutation matrix below: **a public fn with zero killing mutants in the matrix is reported as uncovered.** Line coverage (`cargo-llvm-cov`, not installed `[V]`, needs `llvm-tools` in the toolchain, which `rust-toolchain.toml` does not list `[V]`) is a tool decision (D4), default off.

**Mutation checks (tests must be able to fail).**
- *Existing:* the L0 "known-wrong stub must fail" check is already done (stated by the requester; I did not see its harness). `tests/mag/model_selftest.rs` `[V]` shows the self-test pattern: the model must reject perturbed goldens (`model_rejects_every_perturbed_mpmath_golden`) and must return errors when it cannot compare (`check_up_reports_an_error_when_the_model_cannot_compare`).
- *Rule extended to every oracle checker:* each checker (`check_up`, `check_down`, the ball enclosure checker, the tightness checker, the golden row parser) gets a self-test that feeds a deliberately wrong result (radius shrunk by one unit, mid off by one ulp, wrong mode, truncated input) and requires rejection.
- ***Runner (default plan, D3): hand-written mutant patches.*** `tools/mutants/NNN-name.patch` (unified diffs against `src/`) plus `tools/mutants/manifest.txt` rows `fn_path  mutant_id  patch  expected_killing_suite`. `tools/mutate.sh` creates a throwaway git worktree under `/tmp` (never a folder in `/home/jsc64/MathWorks`), applies one patch, runs only the named suite (`cargo test --test <suite>`) under `timeout`, and requires a **failing** result; a passing suite is a surviving mutant and the script exits nonzero. No `cfg` switch is added to `src/` (PLAN Part E's `cfg(zb_shrink_radius)` is replaced by the patch `radius-shrink-1ulp`, keeping the library free of test hooks; D3 records the change).
- *Alternative:* `cargo-mutants` over `src/mag/*`, `src/ball/*`, `src/encode.rs` (PLAN Part E: nightly; every survivor killed or documented equivalent). It is a tool, not a crate dependency, but is **not installed** on Pop `[V]` and installation access is unverified (the lockfile notes pip has no SSL on Pop `[V]`); whether to install it is Rusty's call. Hand patches are the baseline either way, because they encode the soundness-relevant mutants a generic tool will not think of.
- **Mutant catalog (seed list):** shrink the radius by one unit/ulp; round-up op rounds down (and vice versa); drop the sticky bit; drop carry renormalisation (`2^64` mantissa); exponent off by one (`+1`, `-1`) in `mul_up`, `sqrt`, `ulp_mag`, `half_ulp`; `<` vs `<=` in `cmp_exact_num`, `smaller_than_half_ulp`, the domain check `m > r`; wrapping instead of `checked_*`; remove the zero early-return; remove the `Infinity` early-return; `0 * Infinity` gives 0; swap `Up`/`Down` branch; treat `None` as `ToZero`; wildcard `_ =>` on the mode; skip `-0` normalisation; half ulp where full ulp is needed for directed modes (the `p = 3, 81/64` case); drop the `r*s` term of mul; drop `eps` when inexact; divisor lower bound rounded **up** instead of down; `|q|_up` replaced by the rounded `q` (the CONTRACT 4.3 bug class); `eff_exp` clamped against `EXPONENT_MIN - 1` (PLAN G3 bug); gap rule threshold `p+guard` off by one; `sub_lower` rounded up; encoder emits `-0`; decoder accepts tag 2/3 or non-canonical `Mag`; `b.sub(a).neg()` instead of `a.sub(b)` (section 12). **Target: every non-equivalent mutant killed; equivalent ones documented with the reasoning in `tools/mutants/EQUIVALENT.txt`.**

## 12. Known traps, each seeded as a regression set `[R]` unless marked

| Trap | Seeded test |
|---|---|
| Round-down overflow dropped from goldens | generator emits `max` rows `[V: present bug in gen_goldens.py]`; test asserts the golden file contains round-down-overflow rows for `mul_lower`, `div_lower` (and `sub_lower` cannot overflow) and that the Rust result equals `Finite{i64::MAX, u64::MAX}` |
| Golden runner second-argument bug | commit `0eec0fe` ("fetch the second operand only for binary ops") appears to fix it by its subject `[V: subject read, diff not read]`; regression: a unary row with a stray second-operand column, and a binary row missing one, both make the runner return an error |
| zf zero result `exponent() == 0` | any code path that reads `exponent()` of a zero is covered by a set with exact-zero results (cancellation, `0 * x`, underflow) at every mode and mid exponent; asserts `ulp_mag`/`MIN_NORMAL_EXPONENT` logic does not use `0` as an exponent of zero (O5) |
| `ExactNum::round(n, rm)` rounds to `n` **fractional bits**, not precision | a set at mids of exponent `-40, 0, +40, +1000` showing that precision rounding and fractional-bit rounding differ; the code under test must use the precision API, and the test fails if `round(n, rm)` is used for `prec` |
| NaN error kinds when both operands are NaN of different kinds | operand pairs (NaN kind A, NaN kind B) in both orders for every op; the returned error is the contract's single kind, identical in both orders |
| `b.sub(&a).neg()` sign-of-zero trap under `Up`/`Down` (**wrong for about 56 percent of pairs**) | (1) a **scan rule** in `tools/rules_scan.py` (with a self-test case in `tools/rules_scan_selftest.sh`) refusing `.sub(...).neg()` and `neg(...sub(...))` forms, i.e. any `a - b` written as `-(b - a)`, in `src` and `tests`; (2) a grid test over all sign/magnitude pairs and every mode asserting `sub(a, b)` equals the single correctly rounded `a - b` and that the negation form is **not** used; (3) mutant `neg-of-reversed-sub`. The 56 percent figure is Rusty's; I did not reproduce it |
| zf overflow ignores directed mode (K3) | P12 set: `Up`/`Down` at overflow for both signs: returns `max_value` or `Overflow` per O2, never a finite lower bound above the true value `[V: PLAN K3 text read]` |
| `inexact` is inherited from inputs (K5) | the `inexact()` pin in 8.2 |
| Directed-rounding `p = 3, x = 81/64` | 8.2 ties row |

## 13. How each test class runs in the gate

Budgets are proposals, unmeasured `[NV]`; item S0 measures first and sets the final numbers. `tools/full_gate.sh` today runs each step under `timeout` (default 20 minutes) `[V]`.

| Class | Gate (every run) | Nightly | On demand / deep |
|---|---|---|---|
| Existing sets, model self-tests | always, debug and release | yes | n/a |
| Edge-class sets (section 8) | always | yes | n/a |
| Property/fuzz at fixed seeds | gate counts (section 1), debug and release, target under 8 min debug, 3 min release | same + extra seeds | deep counts, release, hours |
| Golden checks (read committed goldens) | always | yes | n/a |
| `goldens-regen` (regenerate and `cmp`) | fast subset, target under 2 min | full set | full set |
| Grid digests (P10) | always on x86_64 debug+release | cross-target (i686, wasm32-wasip1, thumbv6m/thumbv7em under qemu; D8) | same |
| Pub-fn coverage script | always (seconds) | yes | n/a |
| Rules-scan incl. new `-(b-a)` rule | always | yes | n/a |
| Mutation matrix | the L0a mutants only until the suite is calibrated; then a rotating 20% | full matrix | full matrix; `cargo-mutants` if adopted |
| Tightness report and ratchet | always (reads, no regen) | regenerated | n/a |
| Cross-check vs zf exact (needs the zf pin) | with the zf pin | yes | n/a |

## 14. Per-level exit criteria (all must hold; evidence goes in the level-closing commit message: gate tag, executed counts, seeds, deep-run log path)

| Level | Exit criteria |
|---|---|
| Shared (S) | S0 calibration done and counts recorded; generators, props, replay/shrink, coverage script, mutant runner, `goldens-regen` step, `-(b-a)` scan rule are in the gate and green; M1 model extension has self-tests that fail on a wrong model |
| L0a Mag | every row of 8.1 has at least one named test citing its clause; all 8 round ops plus `cmp`, conversions, `ulp` helpers satisfy P1 to P3, P9 and W1 at gate seeds with 0 failures and in one deep run at or above the section 1 deep target; goldens regenerate byte-identically and contain round-down-overflow rows; pub-fn coverage clean for `src/mag`; every Mag mutant in the catalog killed (or documented equivalent); extra review of the Mag tests by Paul or Cassie (PLAN L0 review rule) and Rusty's rounding review recorded |
| L0b real balls | every row of 8.2 tested; P1 to P10 and W1 to W6 hold for every op x 6 modes x PS-fast at gate and PS-deep in a deep run, 0 failures; `RoundingMode::None` rejected everywhere; `inexact()` pin test green; tightness report within the (D2) factors with the ratchet file committed; chains of 2/10/100/1000 hold; encoder/decoder fuzz clean; pub-fn coverage and mutation matrix clean for `src/ball`, `src/encode.rs`, `src/decimal.rs`; grid digests identical on the required targets (D8). Needs the zf pin items in section 15 |
| L1 elementary | every row of 8.3 tested for each of the 25 items; exact cases return in bounded work under all modes; both policies at every domain edge; goldens at the PS-fast precisions at gate and all PS in nightly, containment 0 failures, tight/loose within factors; P14 findings exist for every function (L1 does not start without them, PLAN); mutation and coverage clean |
| L2 | rows of 8.4; Rect/Disk conversions only enlarge; singular/ill-conditioned paths return errors; specials goldens including zeta at 0, -1, 1 from mpmath; compounds and equivalent-form checks; coverage and mutation clean |
| L3 | rows of 8.5; budgets are counts; no narrow uncertified result in any adversarial set; coverage and mutation clean |
| L4 | rows of 8.6; the failure rule (error or `Infinity` radius on every failed enclosure) asserted on every refusal path; Hilbert n = 5..12 contain the exact solution or fail; coverage and mutation clean |
| Whole crate (before ship) | all levels closed; full nightly set green twice in a row at different extra seeds; cross-target digest diff clean; no surviving non-equivalent mutant; no open item from section 17 that blocks an assertion |

## 15. What depends on the final zenith-float pin SHA (pending)

`Cargo.toml` has an empty `[dependencies]` at `581f677` `[V]`; the zf pin is not recorded in this repo yet. **Independent of the pin (start now):** the model (`tests/model`), M1, all Python goldens and generators (they use mpmath, not zf), `tests/common`, the coverage script, the mutant runner, the gate step, the Mag tests, the scan rule. **Depends on the pin (written red-first against the contract; run green only when the pin lands):**

| Pin item (PLAN C1) | Tests that wait for it |
|---|---|
| P15 exact-bit rounding | every precision in PS that is not a multiple of 64 (`1..16, 20, 24, 31, 33, 48, 53, 63, 65, 96, 113, 127, 129, ...`), P4 at those `p`, tie sets, `is_unique_at`, `set_prec` lowering |
| P12 directed overflow returns `max_value`, P13 tests | overflow rows in 8.2, the K3 regression set, O2 shape |
| P7/P9 `pre_round_exp`, per-op ternary | P5, P6, the `inexact()` pin, `ulp_mag` inputs (O5) |
| P5 `MIN_NORMAL_EXPONENT`, same range on every target | subnormal-boundary rows, 32-bit cells of P10 (O3, D8) |
| P1 to P3 (`exponent`, `top_u64_mantissa_and_sticky`, `from_mag_parts`) | `from_exact_num`, `to_exact_num`, conversions with zf numbers |
| P11 `Consts::get_exact_prec` | constants call-order and warm/cold-cache byte-equality |
| P14 Ziv-remainder audit + MPFR Up/Down sets | every L1 function's mid-rounding assertions |
| zf 2.0.0 zeta fix (Rusty) | zeta at 0 and -1 uses mpmath golds `[R]` |

When the pin SHA changes: grid digests may change (they are our outputs; PLAN Part J requires the op-version bump rule), goldens and truth do not (independent of zf). The pin SHA is recorded in `tests/common/seeds.rs` next to the generator version so a failing run names both.

## 16. Ownership

| Area | Author | Reviewer |
|---|---|---|
| Rounding sets, ties, directed modes, subnormal and overflow edges, `ulp`/precision tests, tightness factors, goldens that encode rounding | authoring agent | **Rusty** (float/rounding core) |
| `tests/common` (PRNG, generators layout, shrink/replay), `scripts/` plumbing, `tools/pubfn_coverage.sh`, `tools/mutate.sh`, gate steps, encoding/decimal/compare/set-op/sum/dot ordering tests, L2 to L4 control-flow and container tests | authoring agent | **Cassie** (non-float parts) |
| `Mag` tests (trust root of every radius) | authoring agent | Paul or Cassie (PLAN L0 review rule), plus Rusty for rounding semantics |
| Protected files (`tools/rules_scan.py`, `tools/full_gate.sh`, allowlist, baselines, `rust-toolchain.toml`) | whoever changes them | `.github/CODEOWNERS` (currently `@jscarr64` for everything) `[V]`: edits to these files follow that existing review path |
| Pinned digests and golden files | generator output only | the reviewer of the area; never hand-edited |

**Option (D1): Rusty writes the zf-side oracle harness and the exponent-edge/mantissa-pattern generators in zenith-bound's `tests/` directory as local commits** (Rusty's offer). If adopted, the harness goes to `tests/zf_oracle/` and the generators to `tests/common/generators.rs` with the interface in section 7 (so the property suites call one API whichever agent wrote it); the checklist items marked `[Rusty-option]` are then his, and the authoring agent skips them. Assignment is between Paul and Rusty. The schedule in section 19 shows the possible saving separately.

## 17. Open technical decisions (build proceeds on the stated default)

| # | Decision | Owner | Default being built | Blocks |
|---|---|---|---|---|
| D1 | Who writes the zf-side oracle harness and edge/pattern generators (section 16) | Paul, Rusty | authoring agent writes them | S2, S11 |
| D2 | **Metrics and factors:** `tau_op`, `lambda`, `R_ref`, `allowance(n, prec)` for chains, and truth source (`mpmath.iv` vs `mp` cross-check) per op (section 9) | **Rusty** | placeholders in section 9 | tight/loose assertions only; containment tests do not wait |
| D3 | **Tools:** hand-written mutant patches as baseline, `cargo-mutants` addition; `cargo-fuzz` for the decoder only (installed on Pop, needs `libfuzzer-sys`, nightly); replacing PLAN's `cfg(zb_shrink_radius)` with a patch | **Rusty** | patches only, no `cargo-fuzz` | nightly mutation scope |
| D4 | `cargo-llvm-cov` (needs `llvm-tools` toolchain component) | Rusty | off | optional metric only |
| D5 | Golden size policy (commit up to about 5 MB per level, else hash-only) | Rusty | as stated | repo size |
| D6 | Generator mix constants (30% edge etc.) and keeping xorshift64* beside splitmix64 | Rusty | section 7 | none |
| D7 | Final case counts after S0 calibration | authoring agent, Rusty | section 1 targets | gate time |
| D8 | Cross-target set for P10: PLAN names `thumbv6m-none-eabi` (no FPU), Rusty names `thumbv7em`; plus i686, wasm32-wasip1 | Rusty | both thumbv6m and thumbv7em if qemu cells exist, else thumbv6m | P10 only |
| D9 | Confirm the mapping of Rusty's L0/L1/L2-L4 lists to PLAN levels (section 8) | Rusty | as mapped | none |
| D10 | Contract open items that change assertions (below) | Rusty, Paul | defaults below | listed tests |
| D11 | python-flint on Pop (not installed `[V]`) for `rad/rad_arb` tracking | Rusty | not used | tightness tracking only |

D10 detail (CONTRACT open clauses): **O1** result shape (decides whether "uninformative" is `Ok(Indeterminate)`, `rad = Infinity`, or an error: the refusal assertion is written as a helper `is_refusal(&result)` so only one function changes); **O2** overflow reporting (same helper); **O3** 32-bit range (cross-target cells); **O4** half vs full ulp (P5 is written as lower bounds, valid under both); **O5** `pre_round_exp` and where the subnormal clamp lives (subnormal-boundary rows); **O6** default policy (every restricted-op test passes the policy explicitly, so it does not matter); **O7** `Mag::sub_up` and corner operands (decided for round-down by `1faec8c`; `sub_up` tests wait); **O9, O10** API ownership and `half_ulp` signatures (test helpers wrap the calls in one `tests/common/api.rs` so a signature change is one edit).

## 18. Pending inputs

**18.1 Rusty's rounding-core edge list: received 2026-10-15, folded in.** The full text is Appendix A. Where each part went:

| Rusty's part | Folded into |
|---|---|
| ORACLES in trust order | section 4 |
| PROPERTIES P1 to P10 | section 5 |
| L0 (Mag) edge list | 8.1 |
| L1 edge list (real-ball arithmetic) | 8.2 |
| L2-L4 edge list and compounds and solvers | 8.3, 8.4, 8.6, section 10 |
| FUZZING AIMS 1 to 8 | sections 7 and 10 |
| KNOWN TRAPS | section 12 |
| zf-side harness offer | sections 16, 17 (D1), 19 |

**18.2 Cassie's non-float edge list: PENDING.** It slots in like this, with no restructuring: each item becomes (a) rows in the matching table of section 8 tagged `[C]`; (b) a property `P11, P12, ...` in section 5 if it is a rule rather than a case; (c) generator or model additions in sections 6 and 7; (d) a checklist line in section 21 under the owning level; (e) coverage entries for any new pub fn. Expected areas (my guess at where her list will land, not her list): canonical encoding/decoder rejection rules, decimal parse/print, compare/set ops and `Tri`, `sum`/`dot` ordering, `Interval` conversions, API/ownership and error-kind plumbing, `DomainPolicy` plumbing, thread-count determinism, trace format and checker (PLAN Part I), polynomial/series/matrix container logic, L3/L4 budgets and termination.

| Slot | Cassie item | Section/table | Property | Checklist id | Status |
|---|---|---|---|---|---|
| C-1 | (pending) | | | | not received |
| C-2 | (pending) | | | | not received |
| C-3 | (pending) | | | | not received |

## 19. Schedule: effort of building these tests in (ESTIMATES, not measurements)

Jeff discounts AI timeline estimates (PLAN Part G) so read these as sizes with stated assumptions. Unit: **working days for one agent**, ranges, plus review. Nothing here was measured; I have not run the existing suite, so I have no timing basis from this repo.

**Assumptions.**
1. One agent authors the tests. I have no timing data from this repo (the existing work, 50 `#[test]` fns plus a model, a generator and a gate at `581f677`, was counted but not timed), so the ranges are judgment. Wide-range sets cost more per op than narrow ones because each needs a generator, a model path and an adversarial set.
2. Each level's work is split as in the table: test authoring (edge-class sets, section 8), golden generation (scripts, truth, regen gate, section 9), fuzz/property harness (generators, props, shrink, grids, chains, section 7 and 10), mutation checks (mutants, runner, self-tests, section 11).
3. Review is 20% to 30% of the level's effort (Rusty on the rounding core, Cassie on the rest, section 16).
4. zf pin items (section 15) land in time for the pin-dependent tests to go green; otherwise those tests stay red and the level cannot close.
5. 100% of the work is the plan above (all 6 classes in section 13, PS-deep goldens, full mutant catalog). Defect fixing is separate (assumption 7).
6. About 60% to 75% of this effort is **incremental** over what PLAN Part G's L0 to L4 runs already assume (I assume part of the plan's per-level runs is narrow-set test writing; I could not separate it from the plan text).
7. Fixing defects the wide tests find is not in the table; allowance 10% to 20% of gross.

| Level | Test authoring | Golden generation | Fuzz / property harness | Mutation checks | Gross (days) | Review (days) | Total with review |
|---|---|---|---|---|---|---|---|
| Shared | 1–2 | 1–2 | 2–3 | 1–2 | 5–9 | 1–3 | 6–12 |
| L0 (Mag + real balls) | 8–13 | 3–5 | 4–7 | 3–5 | 18–30 | 4–9 | 22–39 |
| L1 | 8–14 | 6–10 | 4–7 | 3–5 | 21–36 | 5–11 | 26–47 |
| L2 | 8–14 | 6–10 | 3–6 | 2–4 | 19–34 | 4–11 | 23–45 |
| L3 | 3–5 | 2–4 | 1–3 | 1–2 | 7–14 | 2–5 | 9–19 |
| L4 | 4–7 | 2–4 | 2–4 | 1–3 | 9–18 | 2–6 | 11–24 |
| **Total** | **32–55** | **20–35** | **16–30** | **11–21** | **79–141** | **18–45** | **97–186** |

(L0 splits roughly 40% L0a Mag / 60% L0b real balls. Row sums and column sums were added by script.)

**Possible saving from Rusty's offer (section 16, D1), estimate:** the zf-side oracle harness and the edge/pattern generators are about 3 to 6 working days of the gross above (generators about 1 to 2 in Shared; the zf-side harness about 2 to 4 spread over L0 and L1 authoring and harness). If he writes them: gross 76–135 instead of 79–141. Review of his commits still costs time. It also adds an independent author for the float-rounding oracle, which matters for the "no self-consistency" rule.

**Impact on the 2 January 2027 target (estimate).**
- Working days left from 16 October 2026 to 2 January 2027: about 52 (weekdays counted by script, excluding 26-27 Nov and 24-25 Dec as assumed holidays, counting Fri 1 Jan; 79 calendar days from today). `[V: computed]`
- Added effort that is truly new (assumption 6): about 58 to 140 working days with review. **Done serially by one agent this exceeds the days left (about 52)**, so the date cannot hold on that basis.
- Done **in parallel** (a test-building agent per level running red-first alongside implementation, the shared infrastructure first), only the tail sits on the critical path: golden regeneration and diffs, mutation runs, review fixes, deep runs and the defects they find. I estimate the tail at 25% to 35% of the incremental effort (about 15 to 49 days) plus the defect allowance (about 8 to 28 days): **about 23 to 77 working days added to the critical path, roughly 5 to 15 weeks**, about 2 to 5 days less with Rusty's option.
- zenith-bound must finish before ship. I have **not** converted the baseline (PLAN Part G, about 201 to 356 runs) into days or checked whether it already has slack, so I cannot say whether 2 January 2027 survives. If it had none, the ship date moves by the amount above.
- **Levers, in order:** (1) start S0 to S9 now (they do not depend on the zf pin); (2) author tests per level in parallel with implementation; (3) take Rusty's offer; (4) if time compresses, hold L0 and L1 at full scope and run L2 to L4 at a **floor**: containment (P1), rounding (P4/P5), the edge tables, PS-fast goldens, the failure rule, and the top 30 catalog mutants per level, with no deep tier. I estimate the floor saves 25% to 35% of L2 to L4 gross (about 10 to 25 days). The floor is a scope choice recorded in the level's exit commit, not a silent cut.

## 20. Risks

- **The model cannot judge wide gaps** (`MAX_ALIGN = 4096` `[V]`): wide-range add/sub cases are unassertable until M1. Mitigation: W1 (translation) for same-gap checks, M1 early (S5), and the current failure mode is an error, not a pass.
- **Wide tests find real defects** (zf directed overflow, `Mag` boundary, exponent wrap). That is the point; it is also schedule risk (assumption 7). Mitigation: defects are logged with the shrunk case and turned into set entries.
- **Test time:** bigint model plus 4096-bit precision plus debug builds can blow the gate budget. Mitigation: S0 calibration, two tiers, deep runs release-only and nightly.
- **Oracle trust:** mpmath is not documented as correctly rounded everywhere; mpmath and zf can agree and both be wrong. Mitigation: `mpmath.iv` and the two-precision agreement rule (section 9), the independent integer model first in the trust order, exact-rational checks, and never using zf as sole oracle.
- **Pin dependency:** the zf pin is pending; pin-dependent tests stay red until it lands. Mitigation: section 15 separates what can proceed.
- **Tightness factors are guesses** until Rusty sets them (D2); a too-tight factor produces false failures. Mitigation: containment tests never wait on factors; factors start loose and ratchet.
- **Contract open items** (O1 to O10) can change assertions. Mitigation: helpers isolate the changing pieces (D10).
- **Generator drift** (changing the draw order silently changes cases). Mitigation: frozen draw order plus a generator version in `seeds.rs`.
- **Test code obeys the rules:** no `as`, no indexing, no unwrap makes test code verbose and slower to write; included in the estimates, not extra.
- **Golden volume** (repo size). Mitigation: D5.
- **Coverage-by-name is weak.** Mitigation: the mutation matrix is the real measure; surviving mutants are tracked.
- **Not verified:** the whole plan is unexecuted; no gate step, script or test described as "to be written" exists yet.

## 21. Build checklist (check items off as they land; each commit names the item id)

**Shared (S)**
- [ ] S0 calibrate: time one op family in debug and release on Pop, set counts (D7)
- [ ] S1 `tests/common/rng.rs` (splitmix64) and `seeds.rs` (BASE, per-suite seeds, generator version, zf pin SHA)
- [ ] S2 `tests/common/generators.rs`: exponent-edge-weighted, mantissa patterns, controlled gaps, ball inputs, precision draws `[Rusty-option]`
- [ ] S3 `tests/common/props.rs`: P1 to P10 and W1 to W6 checkers, `is_refusal` helper, `api.rs` wrapper (D10)
- [ ] S4 failure report line, `ZB_FUZZ_SEED`, `ZB_FUZZ_CASES`, `ZB_REPLAY`, deterministic shrinker
- [ ] S5 model M1: gap-aware sound comparison and `MAX_ALIGN` handling, with self-tests that reject a wrong model
- [ ] S6 `tools/pubfn_coverage.sh`, exemptions file, `COVERS:` headers on existing tests, gate step
- [ ] S7 `tools/mutate.sh`, `tools/mutants/manifest.txt`, `EQUIVALENT.txt`, first 10 mutants
- [ ] S8 `scripts/goldlib.py`, `--check` mode, `tests/goldens/SHA256SUMS`, gate step `goldens-regen`
- [ ] S9 rules-scan rule for `-(b-a)` / `.sub(..).neg()` plus self-test case (protected file path, section 16)
- [ ] S10 clippy `wildcard_enum_match_arm` deny: check the tree stays clean first
- [ ] S11 zf-side oracle harness `[Rusty-option]` (needs the zf pin)
- [ ] S12 tightness report and ratchet file
- [ ] S13 cross-target grid-digest job (D8)

**L0a Mag**
- [ ] L0a-1 extend `gen_goldens.py`: round-down-overflow rows, widened operand set, sqrt parity grid; regenerate, review diff
- [ ] L0a-2 8.1 rows not yet covered (canonical form, constructor rejection, shifts 0/63/64/more, carry at `i64::MAX`, sqrt of 1 / all-ones / powers of two, `Ord` transitivity)
- [ ] L0a-3 W1 translation sets for `mul_up`, `div_up`, `sqrt_up`, `*_lower`
- [ ] L0a-4 `from_exact_num(+_lower)`, `to_exact_num`, `cmp_exact_num`, `cmp_exact_diff` sets and P9
- [ ] L0a-5 wide-range fuzz target `tests/fuzz_mag.rs`, gate and deep counts
- [ ] L0a-6 trap regressions (golden runner second operand, round-down overflow rows)
- [ ] L0a-7 mutants (catalog Mag part), self-tests for each checker
- [ ] L0a-8 exit review: Paul/Cassie, Rusty

**L0b real balls**
- [ ] L0b-1 construction, `-0` normalisation, zero sets, `RoundingMode::None` rejection, `inexact()` pin
- [ ] L0b-2 add/sub/neg/abs incl. huge gaps, cancellation, carry, subnormal and overflow edges, ties
- [ ] L0b-3 mul/mul_2exp/sqr/powi, div/inv, sqrt/hypot (policy variants)
- [ ] L0b-4 sum/dot, compare/contains/overlaps/union/intersect, floor/ceil/sgn
- [ ] L0b-5 `set_prec`, `is_unique_at`, `Interval`, `Refinable`
- [ ] L0b-6 decimal parse/print, encode/decode incl. decoder fuzz
- [ ] L0b-7 `gen_ball_goldens.py` + enclosure rows at PS-fast (gate) and PS (nightly), tight/loose checks
- [ ] L0b-8 properties P1 to P10, W1 to W6 across all ops, modes, PS-fast
- [ ] L0b-9 chains of 2/10/100/1000, equivalent-form checks, zf-exact cross-check (with model agreement)
- [ ] L0b-10 grid digests pinned; cross-target run
- [ ] L0b-11 trap regressions (NaN kinds, `-(b-a)`, `round(n, rm)`, zero `exponent()`, K3)
- [ ] L0b-12 mutants (catalog Ball/encode part), pub-fn coverage clean, exit review

**L1 elementary**
- [ ] L1-1 P14 finding present for each of the 25 items (prerequisite, not a test)
- [ ] L1-2 per-function edge rows of 8.3 (21 functions, 4 constants, both policies)
- [ ] L1-3 `gen_fn_goldens.py`: truth intervals at all PS, exact cases, pole/branch/reduction-boundary arguments, zeta-style mpmath-only rows where zf is suspect
- [ ] L1-4 containment, tight, loose at PS-fast gate / PS nightly
- [ ] L1-5 compositions and subdivision sets, fuzz target `tests/fuzz_l1.rs`
- [ ] L1-6 mutants (radius rules per function), coverage, exit review

**L2**
- [ ] L2-1 Rect/Disk complex incl. cuts and conversions
- [ ] L2-2 BallPoly, BallSeries
- [ ] L2-3 BallMat/ComplexBallMat incl. singular and Hilbert
- [ ] L2-4 specials (gamma, lgamma, erf, erfc, J0, J1, zeta at 0, -1, 1 from mpmath)
- [ ] L2-5 goldens, compounds, fuzz, mutants, coverage, exit review

**L3**
- [ ] L3-1 analytic and non-analytic sets, budgets, adversarial integrands, goldens, fuzz, mutants, coverage, exit review

**L4**
- [ ] L4-1 adaptive precision, root isolation (multiple/clustered), verified solve (Hilbert 5..12, singular), residuals, eigen, ODE sets
- [ ] L4-2 failure-rule assertion on every refusal path, goldens, fuzz, mutants, coverage, exit review

**Whole crate**
- [ ] W-1 two consecutive green nightly full runs at different extra seeds; cross-target diff clean; mutants all killed or documented

## Appendix A. Rusty's rounding-core edge list (as received 2026-10-15, folded into sections 4 to 12)

Attributed to Rusty (review of the float/rounding core). Text kept as received; the nested numbering is his. Items I could not verify (zf test names, the 212,772-case run, the 56 percent figure) are his statements.

**ORACLES in trust order:** (1) an independent exact-integer/rational reference written without zf (his L0 review used one: 212,772 add/sub/mul/div/sqrt edge cases, 0 mismatches); (2) MPFR via the zf mpfr suite (rounding mode, precision and exponent range set explicitly; /usr/bin/python3 mpmath for goldens); (3) exact rational enclosure checks: the exact result in rationals must lie in [mid-rad, mid+rad]. Never use zf's own result as the oracle for zf-backed layers (the self-consistency trap Jeff flagged). Regenerated goldens must match committed ones.

**PROPERTIES for every op at every level:** P1 containment for every rounding mode, including ball inputs (sample the exact point at the ball's edges and middle). P2 rad never rounded down (round-up Mag ops only), never NaN, Infinity is uninformative and absorbing. P3 widening an input radius never narrows the output radius. P4 no second rounding: the output midpoint is exactly zf's single rounded result at p. P5 rad >= half ulp for nearest, >= full ulp for directed, no wildcard match on mode, RoundingMode::None rejected. P6 exact inputs and exact ops give rad == 0 only when inexact() is false. P7 add/mul midpoint commutes bit-for-bit, radius symmetric. P8 a-a contains 0 and a/a contains 1 (dependency may widen, never exclude). P9 from_exact_num then enclosure contains the original. P10 determinism: identical bits on x86_64, i686 and thumbv7em.

**L0 (Mag):** canonical zero (all mant==0 equal); Infinity greatest and absorbing for round-up, zero for round-down; 0/0 and x/0 give Infinity; i64::MAX/MIN exponents and u64::MAX mantissa; carry across the exponent on add/mul; round-up and round-down overflow (round-down gives Finite{i64::MAX,u64::MAX}); sub with a<b, a==b, a>b; sqrt of odd/even exponent, of mant 1, u64::MAX, powers of two; top-bit-only and all-ones mantissas; shifts of 0, 63, 64 and more; Ord consistency (transitivity, antisymmetry, Eq follows Ord); the constructor rejects non-canonical forms.

**L1:** signed zero both ways (+0/-0 operands, -0 results under Down, 0*neg, x-x under each mode); subnormals and the boundary (zf grid 2^(EXPONENT_MIN - p): at, just above and just below the smallest normal; exponent() for zero must map to EXPONENT_MIN); exponent extremes (i32 in zf, i64 in Mag; never wrap; test EXPONENT_MIN-1 and EXPONENT_MAX+1 before converting); overflow (Inf, NaN, or inexact() with exponent()==EXPONENT_MAX is overflow; zf saturates to max_value(p) toward zero); underflow to zero is inexact and the rad must cover the missing range; precision limits (p=1, 2, 3, smallest zf supports, word boundaries 31/32/33, 63/64/65, 127/128/129, p larger than operands, p much smaller via add(&zero,p,rm), mixed operand precisions in both orders); ties (exact halfway for ToEven vs ToOdd, both parities and signs, and for Up/Down/ToZero/FromZero one tick either side; known counterexample p=3, x=81/64, Up gives 3/2, error 15/64 > half ulp); cancellation (a-b close with exact representable result keeps minimal rad; catastrophic cancellation with different input rads; a+(-a); exactly zero result with nonzero input rads); carry into the next binade in each mode; division (exact vs inexact, 1/3 repeating, divide by subnormal, ball straddling zero gives Infinity rad per DomainPolicy, 0/0); sqrt (perfect squares, exactly-half-ulp cases, subnormal, ball straddling zero, negative under Strict and other DomainPolicy values); inexact() is only per-op for add/sub/mul/div/sqrt, so pin it with a test across all modes before relying on it, and for transcendentals use the documented bound or full-ulp rule.

**L2-L4:** argument at 0, +-1, 2^-p, huge, at poles and branch points, near argument-reduction boundaries (multiples of pi/2, ln2), results near binade boundaries, monotonicity region edges, domain edges (ln 0, ln negative, asin(+-1), acosh(1), gamma at 0 and negative integers, zeta at 0, -1, 1; zf 2.0.0 fixed zeta at 0 and -1, so use mpmath golds there). Compounds: chains of 2, 10, 100, 1000 ops (radius grows at most as the interval bound predicts and never below true error); the same expression written two algebraically-equal ways must both enclose; Horner vs power form; large cancellation chains. Solvers (L4): multiple root, ill-conditioned Hilbert n=8..12, singular matrix returns an error not a bogus tiny ball, tolerance at the precision limit, termination caps. A failed enclosure must return an error or Infinity rad, never a plausible small ball.

**FUZZING AIMS:** (1) exponent-edge-weighted generators (30 percent of draws within 3 ticks of EXPONENT_MIN/MAX, the subnormal boundary, powers of two +-1 ulp); (2) mantissa patterns (all-ones, single-bit, alternating, top-bit only, halfway-tie patterns at p); (3) per-op, per-mode, per-p grids with pinned digests like zf's add_sub_grid_digest; (4) ball inputs with rad 0, 1 ulp, half ulp, huge, Infinity; (5) differential fuzz against the integer oracle at p <= 64 and against MPFR at p up to 4096; (6) metamorphic: swap commutative operands, negate and flip the rounding direction (Up on x equals neg of Down on -x), scale by 2^k; (7) long random chains replayed in exact rationals; (8) record the failing seed and shrink, no unwrap in generators. Gate runs wide ranges at fixed seeds (fast), nightly/deep runs wider, like zf's oracle-fast and oracle-deep.

**KNOWN TRAPS to seed:** round-down overflow dropped from goldens; the golden runner second-argument bug; zf zero result exponent()==0; ExactNum::round(n, rm) rounds to n fractional bits not precision; NaN error kinds when both operands are NaN of different kinds; the b.sub(&a).neg() sign-of-zero trap under Up/Down (wrong for about 56 percent of pairs), so any a-b written as -(b-a) must be refused (add a scan rule or test).

**Offer:** Rusty offers to write the zf-side oracle harness and the exponent-edge/mantissa-pattern generators in zenith-bound's tests directory as local commits (recorded as option D1, sections 16, 17 and 19).
