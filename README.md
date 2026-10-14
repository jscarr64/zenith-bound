# zenith-bound

**Pre-release (0.0.0). Nothing is guaranteed yet.** No enclosure, rounding or
determinism property described anywhere in this repository holds until the
release that states it, together with its tests. Do not rely on this crate.

zenith-bound will be a guaranteed-bounds (ball arithmetic) crate built on
[zenith-float](https://github.com/jscarr64/zenith-float). Every result will
either carry a defensible enclosure or say explicitly that the guarantee could
not be established.

- 100% Rust, `#![forbid(unsafe_code)]`, no IEEE `f32`/`f64` in the math.
- The plan lives in [`docs/PLAN.md`](docs/PLAN.md).

## Credit

Portions will be derived from [puremp](https://github.com/KarpelesLab/puremp)
by Karpelès Lab Inc., MIT License. See `NOTICE-puremp`. puremp's `f64` lattice
path is not ported.

## License

Licensed under either of

- Apache License, Version 2.0 (`LICENSE-APACHE`)
- MIT license (`LICENSE-MIT`)

at your option. Files derived from puremp also keep puremp's MIT notice.
