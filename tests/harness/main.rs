//! The ONE harness integration test target of zenith-bound: shared support (tests/common), its self-tests and, later, the suites built on it.
//! tests/common is included here and nowhere else (see the rule in tests/common/mod.rs).
#[path = "../common/mod.rs"]
mod common;

mod generators_selftest;
mod goldens_selftest;
mod replay_selftest;
mod rng_selftest;
mod seeds_selftest;
mod shrink_selftest;
mod w1_translation;
