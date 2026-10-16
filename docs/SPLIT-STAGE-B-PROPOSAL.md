# Proposal: freeze zenith-float signatures only for the functions L1 v1 calls (split of Stage B)

**Note (2026-10-16 8:26 AM CT, Jeff via Paul: full scope, no cuts, 2 Jan 2027 is not a constraint): this split is kept only where it improves quality or parallelism, not to cut corners. Dates in section 7 are informational.**

Status: **PROPOSAL, not decided.** Nothing here is a ruling. Written 2026-10-16 (CT) on branch `spence/zb-stageb-docs`, docs only.
This is lever 4 of `docs/REPLAN-2026-10-16.md` section 6 ("Split Stage B"); it follows the staged pin that Paul ruled (DECIDED 2026-10-16, Paul under Jeff's delegation; PLAN C1 and the L0/L1 gates now say so).

**Marks.** **[F: file:line]** = read from a file or git. **[I]** = my inference, not in any file. **[A]** = assumption (section 7). **[D]** = derived by arithmetic from tagged inputs. Dates are CT. zenith-float (zf) was read only, through `git show` on `/home/jsc64/MathWorks/zenith-float` (branch `wip/option-a` @ `fe5c15d`, local, 2026-10-16 07:56; `main` @ `abb2d5a` = 1.0.12). The checkout itself sits on `cursor/oracle-step1-2-0-0-79c6` and was not touched. Line numbers in zf are `zenith-float-num/src/ext.rs` at `fe5c15d`.

## 1. What is proposed

Today Stage B (REPLAN section 1) is: P11, P14, and Option A (consuming `self`, Rule R) "for the functions L1 calls", after which "signatures stop moving". The proposal is to make that list **explicit, closed and first**: ship (or at least freeze) exactly the functions in section 2 as **Stage B1**, and leave every other zf function to a later **Stage B2** whose signatures zenith-bound does not depend on. L1 starts when B1 lands, not when all of Option A lands.

Two meanings of "freeze" must not be mixed up [I]:
1. **Signature freeze** (a decision plus a landed API shape): zenith-bound can write call sites once.
2. **Implementation landed** (Option A layer 1a rewritten, P14 audit written and MPFR sets green): needed for a green L1 gate.
Section 5 says which of the two can come early, function by function.

## 2. The exact function list (derived from PLAN L1)

Source: PLAN L1 line 342 ("Functions: sqrt (from L0), exp, exp2, expm1, log, log2, log10, log1p, sin, cos, tan, asin, acos, atan, atan2, sinh, cosh, tanh, asinh, acosh, atanh, pow; constants pi, e, ln2, ln10") [F: PLAN:342]; radius rules PLAN:343-349; P14 scope PLAN:195 (the same list "and the constants"; sqrt is not in the P14 list) [F]; reduction text PLAN:350.

Rule used to build the list: a zf function is on it if an L1 v1 kernel needs it for a **midpoint** or for an **enclosed constant**. Radius arithmetic in `Mag` (div_up, sub_lower, Mag sqrt, `min(r, 2)`) is zenith-bound code and needs no zf function [F: PLAN:344-348, Mag has `add cmp div lower mul round sqrt ulp`, REPLAN section 2]. Signatures are from `wip/option-a` @ `fe5c15d` [F]; all single-argument functions are `(&self, p: usize, rm: RoundingMode, cc: &mut Consts) -> ExactNum` unless noted.

| # | zf function (today's signature) | Called by L1 kernel(s) [F unless [I]] | Stage today | Option A / P14 / P11 effect on signature or accuracy contract | Freeze early? |
|---|---|---|---|---|---|
| 1 | `sqrt(&self, p, rm)` (:2406, no `cc`; "correctly-rounded", exactly `p` bits, per-op `inexact()`) | `sqrt`; also `asin/acos/asinh/acosh` only through Mag sqrt (radius), not zf [I] | B (Option A; still `&self`, extract §0) | Option A: receiver becomes `self` [I: Rule R shape; not yet written for sqrt]. P14: not in P14 scope (PLAN:195). W1 value fixes for sqrt are Stage A. Accuracy contract unchanged [I] | **Yes, signature** (smallest: no `cc`, no transcendental Ziv loop [I]); still goes through the shared `round_exact`/`exact_operand` wrapper (macro `gen_wrapper_exact_rm`, :2190) [F], so it depends on the X-family decision (section 6, Q1) |
| 2 | `exp` (macro `gen_wrapper_ovf_rm_cc`, :2244) | `exp` (mid and `exp_up(m)` in the radius, PLAN:344) | B | Option A: `self` [I]. P14: finding needed (Rigorous / Fixable / Not rigorous, PLAN:195); if Not rigorous the contract becomes `ToEven` plus a proven ≤ 1 ulp bound (accuracy contract changes, signature not) [F: PLAN:195, 349] | Signature: after Q1. Implementation: after layer 1a |
| 3 | `exp2` (same macro) | `exp2` (same pattern as exp, PLAN:348); `exp2(n)` exact case | B | as #2; exact-case trap applies (`2^n`, PLAN:195) | as #2 |
| 4 | `expm1(&self, p, rm, cc)` (:1347, hand written) | `expm1` | B | as #2; exact case `expm1(0)` | as #2 |
| 5 | `sinh` (`gen_wrapper_ovf_rm_cc`) | `sinh` | B | as #2 | as #2 |
| 6 | `cosh` (same macro) | `cosh` | B | as #2 | as #2 |
| 7 | `ln` (macro `gen_wrapper_log`, :2269; named `ln`; PLAN calls it `log`/ln) | `log` (= natural log, PLAN:346) | B | as #2; exact case `ln(1)=0` | as #2 |
| 8 | `log2` (`gen_wrapper_log`) | `log2`; `log2(2^n)` exact case | B | as #2 | as #2 |
| 9 | `log10` (`gen_wrapper_log`) | `log10` | B | as #2 | as #2 |
| 10 | `log1p(&self, p, rm, cc)` (:1323) | `log1p` | B | as #2 | as #2 |
| 11 | `sin` (macro `gen_wrapper_arg_rm_cc`, :2209) | `sin`; `sin(0)=0` exact case | B | as #2 | as #2 |
| 12 | `cos` (same) | `cos` | B | as #2 | as #2 |
| 13 | `tan` (same) | `tan` (mid; pole test uses π bounds, not zf `tan`, CONTRACT 10.2d) | B | as #2 | as #2 |
| 14 | `asin`, 15 `acos` (same macro) | `asin`, `acos` | B | as #2 | as #2 |
| 16 | `atan(&self, p, rm, cc)` (:1209) | `atan` (radius `min(r, π)` also uses constant π) | B | as #2; exact case `atan(0)=0` | as #2 |
| 17 | `atan2(&self, x: &Self, p, rm, cc)` (:1228) | `atan2` (quadrant logic and the (0,0) `Domain` are zenith-bound code) | B (W1 value fixes for atan2 are Stage A; atan2 commits at 06:50 and 07:27 today) | Option A: `self` first, `x` borrowed (Rule R, two operands) [I]. P14: finding needed. Two-operand, so more call-site shapes | Signature: after Q1; Rule R already fixes the shape of the second operand (CONTRACT 15.1) |
| 18 | `tanh(&self, p, rm, cc)` (:1372) | `tanh` | B | as #2 | as #2 |
| 19 | `asinh`, 20 `acosh`, 21 `atanh` (`gen_wrapper_arg_rm_cc`) | `asinh`, `acosh`, `atanh` | B | as #2 | as #2 |
| 22 | `pow(&self, n: &Self, p, rm, cc)` (:761) | `pow` with a non-integer or ball exponent. Integer exponent is `powi` of L0b, built from Ball mul/sqr, not zf `powi` [I] | B | Option A: Rule R `self`, `n` borrowed [I]. P14: finding plus exact-case trap `pow(x, 0)` (PLAN:195). zf `pow(0,0)` returns exactly 1, inexact false, on `wip/option-a` and 1.0.12 [F: `/tmp/zf-logs/zf-pow-zero-zero.md`] | as #17 |
| 23-26 | `Consts::pi`, `e`, `ln_2`, `ln_10` (`&mut self, p, rm) -> ExactNum`, `ops/consts/mod.rs:213-240`) and the new `Consts::get_exact_prec` (does not exist yet) | constants `pi, e, ln2, ln10`; π also for `atan` radius, `tan` pole test, sin/cos/tan argument reduction; ln2 also for exp-family reduction and for the `1/ln2`, `1/ln10` scaling of log2/log10 (PLAN:348, 350) | B (P11) | **P11 adds a method.** The existing getters may also change: option-a-evidence §3.6 decision 5 (borrowed `&ExactNum` second operand vs rebuild from retained integer series state) is **still open** and "a `get_exact_prec` signature should follow that decision" [F: `zf-pin-items-extract.md` P11 row] | **Blocked by Jeff's decision 5**; cheap once decided. `pi` is the only one cheap enough to recompute (18-110 µs) [F: evidence §3.6] |

Count: 22 functions (#1-22) plus 4 constants (#23-26) [D]. Not in the list (no L1 v1 kernel calls them, per PLAN:342-350): `fma`, `hypot`, `sin_cos`, `sinh_cosh`, `exp10`, `powi`, `powsi`, `cbrt`, `nth_root`, `reciprocal`, `sqr`, `log(x, base)`, `rem`, `fused_*`, and every special function (gamma, erf, Bessel, zeta, ...). Notes:
- **`hypot`** is an L0b midpoint op, not L1 (PLAN:302, REPLAN 3b). It is `(&self, other: &Self, p, rm)` (:1287) and still `&self`. If the split is adopted it must be put on the B1 list or explicitly left to B2; that is Rusty's call (Q3) [I]. I did not count it in the 22.
- **`ln` vs `log`:** PLAN names the L1 function `log` and writes `ln` in the radius rule. zf `log(&self, n, p, rm, cc)` (:904) is the two-argument base form; zf `ln` is the natural log. I read PLAN's `log` as natural log [I]; if L1 also wants `log(x, base)` it joins the list.
- **`sin_cos` / `sinh_cosh`** exist (:2578, :2646) and could serve sin+cos pairs; PLAN does not ask for them, so they are left off [I].
- **Midpoint evaluation takes the operand more than once** [F: PLAN:349 "the mid is evaluated with directed `Down`/`Up` calls and the gap is added"; the radius rules also evaluate `f` or a bound of `f'` at `m`] [I: so each L1 kernel calls its zf function at least twice on the same midpoint]. With consuming `self` (Rule R) that needs two owned values of `m` or one call that returns both bounds. This is the same obstacle as Ziv shape C/D in `/tmp/zf-logs/ziv-loop-shapes.md` §3 seen from the caller's side. It is the main thing the signature freeze must settle (Q2).

## 3. Which parts of Stage B stay deferred (Stage B2)

From `/tmp/zf-logs/option-a-evidence.md` §4.1 and `zf-pin-items-extract.md` §0 [F]: Option A is ~150 public methods, layers 0a, 0b, 1b, **1a (ExactNum core, "the pivot, blocks everything above")**, 2 arrays/dsp, 3 poly/numerics, 4 ball, 5 complex, 6 specials, 7 macro (`expr!`/`cexpr!` emit `.clone()`), 8 compare crate, then removal of `impl Clone`/`exact_copy*`/`exact_operand`. Done on `wip/option-a`: neg, abs, frexp, ldexp/scalb, int/fract/ceil/floor, round, add/sub/mul/div (Rule R), sqr, layer 0a clone removals. **Not done: layer 1a and layers 2-8.**
- **B1 (proposed):** decision on the X family and on constants (Q1, decision 5); Option A for the 26 entries above; P11; P14 for #2-22 (21 kernels, including pow and atan2) and the four constants (everything PLAN:195 lists; sqrt is outside P14).
- **B2 (deferred, not needed by L1 v1):** Option A for all other methods (specials, complex, arrays, polynomials, ball, macro, compare), `impl Clone` removal and the extended clone-scan gate, any audit of zf specials (REPLAN 3c says none is planned; L2d needs it or its own proofs), `hypot`/`fma`/`cbrt`/`nth_root`/`reciprocal` signatures unless Rusty puts them on B1. B2 dates stay as the Option A estimate (REPLAN section 1: ~Nov 5 / ~Dec 3 with release prep / ~Jan 14 for zf itself).
- **Not deferred by this proposal and not relieved by it:** Stage A (P1-P3, W1 fixes, P9/P12/P15). P5's named constant, P8 and P13 are unplaced (REPLAN section 7).

## 4. Risk each way

**If we do not split (current plan):** L1 and everything that needs L1 (L2a complex, L2d, L3, expression evaluation in L4) wait for all of Stage B. Pessimistic Stage B is **Jan 21, 19 days after the 2 Jan 2027 ship date** [F: REPLAN §1, §5]; the REPLAN verdict there is "not achievable" unless L1 stops waiting.

**If we split:**
1. *The "subset" is mostly the pivot.* The L1 wrappers sit in macros (`gen_wrapper_ovf_rm_cc`, `_log`, `_arg_rm_cc`, and `_exact_rm` for sqrt) that all call the shared `round_exact`/`exact_operand` retry machinery [F: ext.rs:2173-2300]; layer 1a is "the long pole" [F: evidence §4.3]. A B1 that contains exp/ln/sin/... is therefore most of layer 1a, plus only the **non-L1** leaf work deferred (layers 2-8, specials). The saving is real but smaller than the function count suggests [I]; section 7 assumes 50-70% of Stage B remains in B1.
2. *P14 order.* A written audit of kernels that layer 1a will rewrite is "likely to be invalidated"; the extract says "do the audit after 1a" [F]. So within B1 the order is fixed: decisions, then 1a for the 26, then P14. Splitting does not let P14 start early.
3. *Two pins, two SHAs.* Pin changes can change grid digests (TEST_PLAN 15 end) [F]; B1 and B2 means up to three pin SHAs (A, B1, B2) before the final one. Accumath pins zenith-float and zenith-bound exactly (PLAN:208) [F], so zenith-bound's last pin must equal a published zf release; B2 must not alter a B1 signature [I].
4. *Signature surprise from deferred layers.* A later layer (e.g. 7, the macros, or 5, complex) could want a different shape for a B1 function. Mitigation: B1 freezes only what Rule R and CONTRACT 15.1 already say (first operand by value, second borrowed) [F: CONTRACT 15.1, O9]; surprises would be Rusty's to flag [I].
5. *L1 stays red on real kernels until B1's P14 lands.* Code and radius rules can be written earlier against a stub or mpmath mids (REPLAN lever 2), but the green gate still needs P14 (TEST_PLAN 15: P14 gates "every L1 function's mid-rounding assertions") [F].
6. *Midpoint-twice problem (section 2, Q2).* If the frozen shape consumes `m` and zenith-bound has no copy, L1 either needs a bracketing API or a second owned midpoint built through `as_raw_parts()`/`from_raw_parts()` (exists, validated) [F]; whether that counts as a forbidden copy under zenith-bound's zero-copy rule (CONTRACT O18-C is stated for `Mag` only) is undecided [I].

## 5. Can the freeze be done early?

| Item | Signature freeze early? | Implementation early? | Why |
|---|---|---|---|
| Constants (#23-26) | After Jeff's decision 5; then yes | After the decision, small (P11 is "NOT STARTED", `git log -S get_exact_prec` empty [F]) | Decision 5 is open [F] |
| `sqrt` (#1) | Yes after Q1 [I] | Possibly early; its W1 value fixes are Stage A [F] | No `cc`, no transcendental retry loop [I] |
| exp/log/trig/hyperbolic families (#2-16, 18-21) | One decision covers all of them [I]: they share three macro shapes (`gen_wrapper_ovf_rm_cc`, `gen_wrapper_log`, `gen_wrapper_arg_rm_cc`) plus a few hand-written functions | **No: needs layer 1a** [F: evidence §4.3] | Same `round_exact` core; P14 follows 1a |
| `atan2`, `pow` (#17, 22) | With the same decision; Rule R already fixes the second operand [F] | After 1a | Two-operand; atan2 internals changed at 06:50 and 07:27 on 2026-10-16 [F: git log], so they are still moving |
| Whole list | **Yes at the decision level, if Rusty can state the shape for the X family and constants without implementing 1a** [I] | Only after 1a for those 26 | See risk 1 |

The useful early deliverable is therefore a **signed-off signature table plus a compile-checked stub** of the 26 entries (zenith-bound can build L1 against it), with the implementation following; I did not find this offered anywhere on disk [I].

## 6. What Rusty must confirm

- **Q1 (X family).** Which shape do the L1-called wrappers get: consuming `self` with an input-reconstruction contract, or `&ExactNum` kernels counted as ref forms (evidence §2.2 shape X, "needs a decision before layer 1a")? Without this no signature in section 2 is frozen. Same for constants (decision 5 (a)/(b), Jeff).
- **Q2 (midpoint twice).** L1 evaluates `f(m)` in `Down` and `Up` (PLAN:349) and bounds `f'` at `m`. What is the sanctioned way for a by-value `self` API: a paired directed call returning both bounds, an owned second midpoint via raw parts, or something else? If a new bracketing function is wanted it changes the list in section 2.
- **Q3 (list membership).** Is the 26-entry list right, including: `hypot` (L0b) on B1 or B2; `ln` for PLAN's `log`; `sin_cos`/`sinh_cosh` not needed; zf `powi` not used.
- **Q4 (1a coupling).** Can the 26 be converted without finishing layer 1a for the rest of the library (shared `round_exact`, `exact_operand`)? What part of layer 1a is in B1? This sets the fraction used in section 7.
- **Q5 (P14 scope and order).** P14 on exactly PLAN:195's kernels, after 1a, with per-kernel findings and the ToEven ≤ 1 ulp fallback; no written finding exists today [F: extract P14 row]. Which kernels does Rusty already expect to be Not rigorous (an accuracy-contract change on a frozen signature)?
- **Q6 (call-site count).** How many L1 call sites change when Option A lands (REPLAN assumption A5 asks for exactly this).
- **Q7 (versioning).** Is B1 a published zf release, a git pin, or a tag; and may B2 change nothing in B1's public items (needed for the Accumath exact pins, PLAN:208).
- **Q8 (state of Stage A).** `wip/option-a` already has P1 `leading_bit_exponent` (`23ce0c6`), P2 `top_u64_mantissa_and_sticky` (`2f0c807`), P3 `from_mag_parts`/`pow2` (`e876fe9`), 07:51-07:54 today [F: git log]; REPLAN (written earlier) calls them not started. Confirm Stage A's remaining items and a gate-green SHA.

## 7. Schedule effect (assumption, same units as REPLAN)

REPLAN units: calendar days; Stage dates from Paul's ruling; layer durations in days per run; "serial" and "parallel" lanes [F: REPLAN §4]. The effect of a split is **not on disk**; the following is an assumption chain.

- **A8 [A].** Stage B work is the interval from Stage A to Stage B in REPLAN's table (19 / 33 / 68 days for optimistic / likely / pessimistic [D]). The B1 share of that interval is **f = 0.5 / 0.6 / 0.7** (the pivot, 1a, is in B1; layers 2-8 and specials are not). This is a guess; Q4 and Q6 would replace it. B1 date = Stage A date + f × interval [D].
- **A9 [A].** After B1 lands the L1 blocked tail is unchanged from REPLAN (L1 ends about 3 days after Stage B in the likely table) and L0b and L1 durations are unchanged; the done date moves day for day with the B date when Stage B is the binding constraint, but never earlier than the L0b + L1 chain from Stage A.

| Pin case | Stage A | Stage B (REPLAN) | **Stage B1 [D, A8]** | Days earlier [D] | Effect on "zenith-bound done" (likely days/run) [D, A9] |
|---|---|---|---|---|---|
| optimistic | Oct 24 | Nov 12 | Nov 3 | 9 | at most 9 days earlier (L0b + L1 chain from Oct 24 limits it): about Dec 3 serial / Nov 16 parallel (was Dec 12 / Nov 25) |
| likely | Oct 31 | Dec 3 | Nov 20 | 13 | Dec 21 serial / **Dec 3 parallel** (was Jan 3 / Dec 16) |
| pessimistic | Nov 14 | Jan 21 | **Jan 1** | 20 | Feb 1 serial / **Jan 14 parallel** (was Feb 21 / Feb 3) |

Reading [D]: under these assumptions the split helps the likely case (parallel plan done Dec 3 against the 2 Jan ship date, margin about a month before Accumath integration) and shortens the pessimistic case by 20 days, but **the pessimistic B1 date (Jan 1) is still the day before the ship date**, so zenith-bound still cannot be done by 2 Jan in the pessimistic pin case; the earliest credible done date there moves from about Feb 3 to about Jan 14 (parallel). If f were 1.0 (no saving) every row reverts to REPLAN. Excluded as in REPLAN A7: review turnaround, gate queue, zf defects found by wide tests, Accumath integration, Part G items I and J.

## 8. Recommendation (conditional)

Adopt the split **only if** Rusty can answer Q1, Q2 and Q4 in the sense that the 26-entry list is closed under the shared 1a machinery and f is at or below about 0.6. Otherwise the saving is small and the cost (three pin SHAs) is real. Whether or not it is adopted, ask for the signed-off signature table and stub first (section 5): it is cheap, lets L1 code start, and is a prerequisite either way. This is a proposal for Paul and Jeff to decide; nothing here changes a gate.
