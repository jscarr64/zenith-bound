//! COVERS: Mag::add_up, Mag::div_lower, Mag::div_up, Mag::half_ulp, Mag::mul_lower, Mag::mul_up, Mag::smaller_than_half_ulp, Mag::sqrt_lower, Mag::sqrt_up, Mag::sub_lower, Mag::ulp_mag
//! L0a-5 wide-range fuzz of the integer-only `Mag` (TEST_PLAN 8.1, section 7, checklist L0a-5).
//! SCAFFOLD BY SPENCE; HARPER OWNS THIS FILE AND MAY RESTRUCTURE IT (Paul's ruling, 2026-10-16). The Mag property checkers and their
//! self-tests are in tests/harness/fuzz_mag_props.rs and fuzz_mag_props_selftest.rs, which do not depend on how the cases are run here.
//! What is here: the runner (one deterministic stream per property, seed of suite `l0a_mag` mixed with the property index, one SplitMix64
//! per case from `FuzzConfig::seed_of`), the replay line on failure, the case counts, and one `#[test]` per property family.
//! COUNTS: gate default `GATE_CASES` per property (a count, never wall-clock time). `ZB_FUZZ_CASES` (or `ZB_PROP_CASES`) sets the count,
//! `ZB_FUZZ_SEED` the base seed, `ZB_REPLAY=<seed>:<case>` runs exactly one case (run the one property with `cargo test <name>`). The deep
//! run sets `ZB_FUZZ_DEEP=1`, which selects the deep case class of the seeds table (a different stream from the gate), see tools/deep_mag.sh.
//! A failure returns `Err` whose text starts with one replay line (common/replay.rs) and then the reason.

use crate::common::replay::{config_from_env, FuzzConfig, ReplayLine};
use crate::common::rng::{splitmix64, SplitMix64};
use crate::common::seeds::{seed_for, CASE_CLASS_DEEP, CASE_CLASS_GATE};
use crate::fuzz_mag_props::{self as props, Bad, Mono, Ops};

/// Cases per property at the gate tier.
const GATE_CASES: u64 = 1_000_000;
/// Property indices start here, so no stream coincides with the W1 tests (which use indices 0 and up).
const PROP_BASE: u64 = 1000;
/// Counts of the outcome classes need this many cases before the non-vacuity check applies.
const NON_VACUITY_CASES: u64 = 100_000;

/// Counts per outcome class (`props::FIN`, `INF`, `ZERO`, `EXACT` in this order).
pub type Counts = [u64; 4];

fn base_seed(prop_index: u64, deep: bool) -> Result<u64, String> {
    let class = if deep {
        CASE_CLASS_DEEP
    } else {
        CASE_CLASS_GATE
    };
    let s = seed_for("l0a_mag", class)
        .ok_or_else(|| String::from("suite l0a_mag is not in the seeds table"))?;
    Ok(splitmix64(
        s.wrapping_add(PROP_BASE).wrapping_add(prop_index),
    ))
}

/// The configuration of property `prop_index` at `cases` cases: environment (count, seed, replay) over the defaults.
pub fn config_for(prop_index: u64, cases: u64) -> Result<FuzzConfig, String> {
    let deep = std::env::var_os("ZB_FUZZ_DEEP").is_some();
    config_from_env(base_seed(prop_index, deep)?, cases)
}

fn replay_text(cfg: &FuzzConfig, case: u64, b: &Bad) -> String {
    let seed = match &cfg.replay {
        Some(t) => t.seed,
        None => cfg.seed,
    };
    let line = ReplayLine {
        suite: String::from("l0a_mag"),
        seed,
        case,
        op: String::from(b.op),
        prec: b.prec,
        rm: String::from("None"),
        a: String::from(b.a.as_str()),
        b: String::from(b.b.as_str()),
    };
    match line.format_line() {
        Ok(t) => t,
        Err(e) => format!("(replay line not formattable: {})", e),
    }
}

/// Runs the cases of `cfg` of one property; returns the outcome class counts. The first failing case ends the run.
pub fn run_cfg(
    cfg: &FuzzConfig,
    f: &dyn Fn(&mut SplitMix64) -> Result<u8, Bad>,
) -> Result<Counts, String> {
    let mut counts: Counts = [0; 4];
    for case in cfg.case_range()? {
        let mut rng = SplitMix64::new(cfg.seed_of(case));
        match f(&mut rng) {
            Ok(flags) => {
                for (i, slot) in counts.iter_mut().enumerate() {
                    if (flags >> i) & 1 == 1 {
                        *slot += 1;
                    }
                }
            }
            Err(b) => return Err(format!("{} -- {}", replay_text(cfg, case, &b), b.msg)),
        }
    }
    Ok(counts)
}

/// One property at the gate/env case count; `need` lists the outcome classes that must occur (checked at `NON_VACUITY_CASES` and above
/// outside a replay), so a property cannot pass by never reaching overflow, underflow or the exact case.
fn go(
    prop_index: u64,
    label: &str,
    need: &[(u8, &str)],
    f: &dyn Fn(&Ops, &mut SplitMix64) -> Result<u8, Bad>,
) -> Result<(), String> {
    let cfg = config_for(prop_index, GATE_CASES)?;
    let ops = props::real();
    let counts = run_cfg(&cfg, &|rng| f(&ops, rng))?;
    eprintln!(
        "fuzz_mag property {} ({}): cases={} deep={} outcome counts [finite, Infinity, zero, exact] = {:?}",
        prop_index,
        label,
        cfg.cases,
        std::env::var_os("ZB_FUZZ_DEEP").is_some(),
        counts
    );
    if cfg.replay.is_none() && cfg.cases >= NON_VACUITY_CASES {
        for (bit, name) in need {
            let idx =
                usize::try_from(bit.trailing_zeros()).map_err(|_| String::from("bit index"))?;
            let seen = match counts.get(idx) {
                Some(n) => *n,
                None => 0,
            };
            if seen == 0 {
                return Err(format!(
                    "property {}: no case had the outcome class `{}` in {} cases ({:?}), the property is vacuous",
                    prop_index, name, cfg.cases, counts
                ));
            }
        }
    }
    Ok(())
}

const ALL: [(u8, &str); 4] = [
    (props::FIN, "finite"),
    (props::INF, "Infinity"),
    (props::ZERO, "zero"),
    (props::EXACT, "exact"),
];

#[test]
fn fuzz_add_up_is_sound_and_the_smallest_upper_bound() -> Result<(), String> {
    go(0, "add_up", &[ALL[0], ALL[1]], &props::add_up_case)
}

#[test]
fn fuzz_sub_lower_is_zero_iff_not_above_and_the_largest_lower_bound() -> Result<(), String> {
    go(1, "sub_lower", &[ALL[0], ALL[2]], &props::sub_lower_case)
}

#[test]
fn fuzz_mul_up_and_mul_lower_are_tight_and_bracket_each_other() -> Result<(), String> {
    go(
        2,
        "mul_up mul_lower",
        &[ALL[0], ALL[1], ALL[2], ALL[3]],
        &props::mul_case,
    )
}

#[test]
fn fuzz_div_up_and_div_lower_are_tight_and_bracket_each_other() -> Result<(), String> {
    go(
        3,
        "div_up div_lower",
        &[ALL[0], ALL[1], ALL[2], ALL[3]],
        &props::div_case,
    )
}

#[test]
fn fuzz_sqrt_up_and_sqrt_lower_are_tight_and_bracket_each_other() -> Result<(), String> {
    go(
        4,
        "sqrt_up sqrt_lower",
        &[ALL[0], ALL[3]],
        &props::sqrt_case,
    )
}

#[test]
fn fuzz_add_up_mul_up_and_mul_lower_commute() -> Result<(), String> {
    go(
        5,
        "add_up mul_up mul_lower",
        &[ALL[0]],
        &props::commutative_case,
    )
}

#[test]
fn fuzz_add_up_is_monotone() -> Result<(), String> {
    go(6, "add_up", &[ALL[0]], &|ops, rng| {
        props::mono_case(ops, &Mono::AddUp, rng)
    })
}

#[test]
fn fuzz_mul_up_and_mul_lower_are_monotone() -> Result<(), String> {
    go(7, "mul_up mul_lower", &[ALL[0]], &|ops, rng| {
        props::mono_case(ops, &Mono::MulUp, rng)?;
        props::mono_case(ops, &Mono::MulLower, rng)
    })
}

#[test]
fn fuzz_div_up_and_div_lower_are_nondecreasing_in_the_numerator_and_nonincreasing_in_the_divisor(
) -> Result<(), String> {
    go(8, "div_up div_lower", &[ALL[0]], &|ops, rng| {
        props::mono_case(ops, &Mono::DivUp, rng)?;
        props::mono_case(ops, &Mono::DivLower, rng)
    })
}

#[test]
fn fuzz_sqrt_up_and_sqrt_lower_are_monotone() -> Result<(), String> {
    go(9, "sqrt_up sqrt_lower", &[ALL[0]], &|ops, rng| {
        props::mono_case(ops, &Mono::SqrtUp, rng)?;
        props::mono_case(ops, &Mono::SqrtLower, rng)
    })
}

#[test]
fn fuzz_sub_lower_is_nondecreasing_in_the_minuend_and_nonincreasing_in_the_subtrahend(
) -> Result<(), String> {
    go(10, "sub_lower", &[ALL[0]], &|ops, rng| {
        props::mono_case(ops, &Mono::SubLower, rng)
    })
}

#[test]
fn fuzz_identities_of_add_up_sub_lower_mul_up_mul_lower_div_up_div_lower_and_sqrt(
) -> Result<(), String> {
    go(
        11,
        "add_up sub_lower mul_up mul_lower div_up div_lower sqrt_up sqrt_lower",
        &[ALL[0]],
        &props::identity_case,
    )
}

#[test]
fn fuzz_sqrt_up_and_sqrt_lower_are_exact_on_squares_and_odd_powers_of_two() -> Result<(), String> {
    go(
        12,
        "sqrt_up sqrt_lower",
        &[ALL[0], ALL[3]],
        &props::sqrt_exact_case,
    )
}

#[test]
fn fuzz_every_result_is_canonical_and_zero_and_infinity_operands_give_the_specified_value(
) -> Result<(), String> {
    go(
        13,
        "add_up sub_lower mul_up mul_lower div_up div_lower sqrt_up sqrt_lower",
        &[ALL[0], ALL[1], ALL[2]],
        &props::canonical_case,
    )
}

#[test]
fn fuzz_ulp_mag_half_ulp_and_smaller_than_half_ulp_match_the_exact_values() -> Result<(), String> {
    go(
        14,
        "ulp_mag half_ulp smaller_than_half_ulp",
        &[ALL[0], ALL[1], ALL[2], ALL[3]],
        &props::ulp_case,
    )
}
