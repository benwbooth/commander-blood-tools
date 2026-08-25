//! Commander Blood's native pseudo-random number generator.

const BYTE_INTERLEAVE_ROUNDS: usize = 8;
const LOWEST_BIT_MASK: u8 = 0x01;
const HIGHEST_BIT_SHIFT: u32 = 7;
const COUNTER_INCREMENT: u8 = 1;
const COUNTER_ROTATION_BITS: u32 = 1;

/// Persistent state owned by `blood_prng_next` at BLOODPRG file offset `0x002de2`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BloodPrng {
    /// Sixteen-bit seed mixed into every result.
    pub seed: u16,
    /// Low mixing byte, rotated and XORed with the call counter.
    pub mix_low: u8,
    /// High mixing byte, decremented by the call counter.
    pub mix_high: u8,
    /// Wrapping call counter.
    pub counter: u8,
}

impl BloodPrng {
    /// Produce the next value using the recovered native routine's exact state updates.
    ///
    /// A zero modulus returns the full mixed value. A nonzero modulus uses the
    /// original repeated-subtraction reduction rather than substituting a
    /// behaviorally different random-number algorithm.
    pub fn next(&mut self, modulus: u16) -> u16 {
        let mut low = self.mix_low;
        let mut high = self.mix_high;
        let mut value = u16::MIN;

        for _ in usize::MIN..BYTE_INTERLEAVE_ROUNDS {
            value = (value << COUNTER_ROTATION_BITS) | u16::from(low & LOWEST_BIT_MASK);
            low >>= COUNTER_ROTATION_BITS;
            value = (value << COUNTER_ROTATION_BITS) | u16::from(high >> HIGHEST_BIT_SHIFT);
            high <<= COUNTER_ROTATION_BITS;
        }

        value ^= self.seed;

        let step = self.counter.wrapping_add(COUNTER_INCREMENT);
        self.counter = step;
        self.mix_high = self.mix_high.wrapping_sub(step);
        self.mix_low ^= step.rotate_left(COUNTER_ROTATION_BITS);

        if modulus != u16::MIN {
            while value >= modulus {
                value = value.wrapping_sub(modulus);
            }
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORIGINAL_ORACLE_VECTOR_COUNT: usize = 300;

    #[derive(Deserialize)]
    struct OracleVector {
        ax_in: u16,
        seed: u16,
        a: u8,
        b: u8,
        counter: u8,
        ax_out: u16,
        a_out: u8,
        b_out: u8,
        counter_out: u8,
    }

    #[test]
    fn matches_all_original_binary_oracle_vectors() {
        let vectors: Vec<OracleVector> = serde_json::from_str(include_str!(
            "../../../../re/tools/oracle_vectors/prng_2de2.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORIGINAL_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut random = BloodPrng {
                seed: vector.seed,
                mix_low: vector.a,
                mix_high: vector.b,
                counter: vector.counter,
            };
            assert_eq!(random.next(vector.ax_in), vector.ax_out);
            assert_eq!(random.mix_low, vector.a_out);
            assert_eq!(random.mix_high, vector.b_out);
            assert_eq!(random.counter, vector.counter_out);
        }
    }
}
