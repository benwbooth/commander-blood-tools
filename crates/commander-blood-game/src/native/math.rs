//! Native integer mathematics recovered from BLOODPRG.

const WORD_BIT_COUNT: u32 = 16;
const HIGH_BYTE_MASK: u16 = 0xff00;
const EXTREME_HIGH_WORD_THRESHOLD: u16 = 0xfffe;
const LARGE_INITIAL_ESTIMATE: u16 = 0xffff;
const MEDIUM_INITIAL_ESTIMATE: u16 = 0x0fff;
const SMALL_INITIAL_ESTIMATE: u16 = 0x00ff;
const TINY_INITIAL_ESTIMATE: u16 = 0x000f;
const AVERAGE_SHIFT: u32 = 1;

/// Calculate the recovered native approximation to an unsigned square root.
///
/// This is a direct translation of `binary_u32_sqrt` at BLOODPRG file offset
/// `0x002e33`. The high-end return path intentionally differs from
/// [`u32::isqrt`]: the original returns the low input word when the high word
/// is at least `0xfffe`.
pub fn binary_u32_sqrt(value: u32) -> u16 {
    let low_word = value as u16;
    let high_word = (value >> WORD_BIT_COUNT) as u16;

    let mut estimate = if high_word != u16::MIN {
        if high_word & HIGH_BYTE_MASK != u16::MIN {
            if high_word >= EXTREME_HIGH_WORD_THRESHOLD {
                return low_word;
            }
            LARGE_INITIAL_ESTIMATE
        } else {
            MEDIUM_INITIAL_ESTIMATE
        }
    } else {
        if low_word == u16::MIN {
            return low_word;
        }
        if low_word & HIGH_BYTE_MASK != u16::MIN {
            SMALL_INITIAL_ESTIMATE
        } else {
            TINY_INITIAL_ESTIMATE
        }
    };

    loop {
        let quotient = (value / u32::from(estimate)) as u16;
        let candidate = ((u32::from(quotient) + u32::from(estimate)) >> AVERAGE_SHIFT) as u16;
        if candidate >= estimate {
            return candidate;
        }
        estimate = candidate;
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORIGINAL_ORACLE_VECTOR_COUNT: usize = 404;

    #[derive(Deserialize)]
    struct OracleVector {
        value: u32,
        return_value: u16,
    }

    #[test]
    fn matches_all_original_binary_oracle_vectors() {
        let vectors: Vec<OracleVector> = serde_json::from_str(include_str!(
            "../../../../re/tools/oracle_vectors/func_2e33_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORIGINAL_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            assert_eq!(binary_u32_sqrt(vector.value), vector.return_value);
        }
    }
}
