//! splitmix64 (Steele, Lea, Flood): one `u64` state, integer-only, all arithmetic wrapping.
//! The draw order is frozen; changing it changes every case of every suite and bumps `seeds::GENERATOR_VERSION`.

const INCREMENT: u64 = 0x9E37_79B9_7F4A_7C15;
const MUL1: u64 = 0xBF58_476D_1CE4_E5B9;
const MUL2: u64 = 0x94D0_49BB_1331_11EB;

pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub const fn new(seed: u64) -> Self {
        SplitMix64 { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(INCREMENT);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(MUL1);
        z = (z ^ (z >> 27)).wrapping_mul(MUL2);
        z ^ (z >> 31)
    }
}

/// The first output of a generator seeded with `seed` (one-shot mixer, used to derive stream seeds).
pub fn splitmix64(seed: u64) -> u64 {
    SplitMix64::new(seed).next_u64()
}
