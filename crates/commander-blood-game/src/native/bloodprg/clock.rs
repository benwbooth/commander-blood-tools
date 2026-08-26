//! Real-time clock values consumed by script date and hour guards.

use super::packed_bcd_to_binary;

const RAW_CENTURY_1900S_SENTINEL: u8 = 0x13;
const NINETEEN_HUNDREDS_BASE_YEAR: i16 = 1_900;
const TWO_THOUSANDS_BASE_YEAR: i16 = 2_000;

/// Calendar date published to the script runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptClockDate {
    /// Full year selected by the original century rule.
    pub year: i16,
    /// One-based month for valid host input.
    pub month: i16,
    /// One-based day for valid host input.
    pub day: i16,
}

/// Convert a packed-BCD RTC hour to the signed word used by scripts.
///
/// This translates `rtc_time_read` at BLOODPRG routine offset `0x00093B`.
/// The host supplies the RTC field directly instead of invoking BIOS interrupt
/// `1Ah`; nibble arithmetic and signed-byte extension remain exact.
pub fn decode_script_clock_hour(packed_hour: u8) -> i16 {
    i16::from(packed_bcd_to_binary(packed_hour) as i8)
}

/// Convert packed-BCD RTC date fields to the script calendar date.
///
/// This translates `rtc_date_read` at BLOODPRG routine offset `0x000950`.
/// The original treats only packed byte `0x13` (BCD 19) as the 1900s and maps
/// every other century byte to the 2000s. Signed extension of malformed BCD
/// fields is retained because the binary exposes it.
pub fn decode_script_clock_date(
    packed_century: u8,
    packed_year: u8,
    packed_month: u8,
    packed_day: u8,
) -> ScriptClockDate {
    let year_in_century = i16::from(packed_bcd_to_binary(packed_year) as i8);
    let base_year = if packed_century == RAW_CENTURY_1900S_SENTINEL {
        NINETEEN_HUNDREDS_BASE_YEAR
    } else {
        TWO_THOUSANDS_BASE_YEAR
    };
    ScriptClockDate {
        year: base_year.wrapping_add(year_in_century),
        month: i16::from(packed_bcd_to_binary(packed_month) as i8),
        day: i16::from(packed_bcd_to_binary(packed_day) as i8),
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const TIME_ORACLE_VECTOR_COUNT: usize = 7;
    const DATE_ORACLE_VECTOR_COUNT: usize = 6;

    #[derive(Deserialize)]
    struct TimeOracle {
        bcd_hour: u8,
        stored_word: u16,
    }

    #[derive(Deserialize)]
    struct DateOracle {
        bcd: DateFields,
        stored: StoredDate,
    }

    #[derive(Deserialize)]
    struct DateFields {
        century: u8,
        year: u8,
        month: u8,
        day: u8,
    }

    #[derive(Deserialize)]
    struct StoredDate {
        year: i16,
        month: i16,
        day: i16,
    }

    #[test]
    fn hour_conversion_matches_every_original_rtc_vector() {
        let vectors: Vec<TimeOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_093b_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), TIME_ORACLE_VECTOR_COUNT);
        for vector in vectors {
            assert_eq!(
                decode_script_clock_hour(vector.bcd_hour) as u16,
                vector.stored_word
            );
        }
    }

    #[test]
    fn date_conversion_matches_every_original_rtc_vector() {
        let vectors: Vec<DateOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_0950_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), DATE_ORACLE_VECTOR_COUNT);
        for vector in vectors {
            assert_eq!(
                decode_script_clock_date(
                    vector.bcd.century,
                    vector.bcd.year,
                    vector.bcd.month,
                    vector.bcd.day,
                ),
                ScriptClockDate {
                    year: vector.stored.year,
                    month: vector.stored.month,
                    day: vector.stored.day,
                }
            );
        }
    }
}
