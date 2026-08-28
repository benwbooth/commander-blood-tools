//! Number conversion routines shared by startup, diagnostics, and presentation code.

use std::fmt::Write;

const BCD_NIBBLE_MASK: u8 = 15;
const BCD_HIGH_NIBBLE_SHIFT: u32 = 4;
const DECIMAL_RADIX: u16 = 10;
const ASCII_ZERO: u8 = b'0';
const SIGN_PREFIX_LENGTH: usize = 1;
const INVALID_DECIMAL_VALUE: i16 = 0;

/// Byte width of the numeric field in a BLOODPRG audio startup option.
pub const STARTUP_AUDIO_NUMBER_LENGTH: usize = 3;

/// Convert both nibbles of a packed-BCD byte to a binary value.
///
/// This preserves `bcd_to_binary` at BLOODPRG file offset `0x000986`, including
/// its arithmetic treatment of nibble values above nine.
pub fn packed_bcd_to_binary(value: u8) -> u8 {
    let low_digit = value & BCD_NIBBLE_MASK;
    let high_digit = value >> BCD_HIGH_NIBBLE_SHIFT;
    high_digit
        .wrapping_mul(DECIMAL_RADIX as u8)
        .wrapping_add(low_digit)
}

/// Append a signed 16-bit value in decimal notation to an owned Rust string.
///
/// This is the flat-memory form of `decimal_append_i16` at BLOODPRG file offset
/// `0x0024b2`. Rust string length replaces the original trailing NUL and caller
/// destination cursor.
pub fn append_decimal_i16(destination: &mut String, value: i16) {
    write!(destination, "{value}").expect("writing to a String cannot fail");
}

/// Append a signed 32-bit value in decimal notation to an owned Rust string.
///
/// This is the flat-memory form of `decimal_append_i32` at BLOODPRG file offset
/// `0x0024eb`. It retains the original minimum-negative-value behavior without
/// exposing the routine's shared scratch buffer.
pub fn append_decimal_i32(destination: &mut String, value: i32) {
    write!(destination, "{value}").expect("writing to a String cannot fail");
}

/// Parse the signed decimal field from a Commander Blood audio startup option.
///
/// This translates `ascii_digit_parse` at BLOODPRG file offset `0x002612`.
/// Static call-graph analysis proves its sole caller supplies exactly these
/// three bytes. Leading whitespace is rejected, a sign is accepted, and parsing
/// stops at the first non-digit.
pub fn parse_startup_audio_number(text: &[u8; STARTUP_AUDIO_NUMBER_LENGTH]) -> i16 {
    let Some(first) = text.first().copied() else {
        return INVALID_DECIMAL_VALUE;
    };
    let (negative, digits) = match first {
        b'+' => (false, &text[SIGN_PREFIX_LENGTH..]),
        b'-' => (true, &text[SIGN_PREFIX_LENGTH..]),
        ASCII_ZERO..=b'9' => (false, text.as_slice()),
        _ => return INVALID_DECIMAL_VALUE,
    };

    let mut magnitude = u16::MIN;
    for digit in digits.iter().copied().take_while(u8::is_ascii_digit) {
        magnitude = magnitude
            .wrapping_mul(DECIMAL_RADIX)
            .wrapping_add(u16::from(digit - ASCII_ZERO));
    }

    if negative {
        u16::MIN.wrapping_sub(magnitude) as i16
    } else {
        magnitude as i16
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const BCD_ORACLE_VECTOR_COUNT: usize = 256;
    const DECIMAL_ORACLE_VECTOR_COUNT: usize = 8;
    const STARTUP_AUDIO_GENERIC_VECTOR_COUNT: usize = 200;

    #[derive(Deserialize)]
    struct BcdOracleVector {
        packed_bcd: u8,
        binary: u8,
    }

    #[derive(Deserialize)]
    struct DecimalOracleVector {
        value: i64,
        output_bytes: Vec<u8>,
    }

    #[derive(Deserialize)]
    struct GenericRegistersOut {
        eax: u32,
    }

    #[derive(Deserialize)]
    struct StartupAudioGenericVector {
        mem_in: Vec<(usize, u8)>,
        regs_out: GenericRegistersOut,
    }

    #[test]
    fn packed_bcd_matches_every_original_byte_input() {
        let vectors: Vec<BcdOracleVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_0986_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), BCD_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            assert_eq!(packed_bcd_to_binary(vector.packed_bcd), vector.binary);
        }
    }

    #[test]
    fn signed_decimal_formatters_match_original_output_bytes() {
        let i16_vectors: Vec<DecimalOracleVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_24b2_natural.json"
        ))
        .unwrap();
        let i32_vectors: Vec<DecimalOracleVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_24eb_natural.json"
        ))
        .unwrap();
        assert_eq!(i16_vectors.len(), DECIMAL_ORACLE_VECTOR_COUNT);
        assert_eq!(i32_vectors.len(), DECIMAL_ORACLE_VECTOR_COUNT);

        for vector in i16_vectors {
            let mut output = String::new();
            append_decimal_i16(&mut output, i16::try_from(vector.value).unwrap());
            assert_eq!(output.as_bytes(), original_payload(&vector.output_bytes));
        }
        for vector in i32_vectors {
            let mut output = String::new();
            append_decimal_i32(&mut output, i32::try_from(vector.value).unwrap());
            assert_eq!(output.as_bytes(), original_payload(&vector.output_bytes));
        }
    }

    #[test]
    fn decimal_formatters_append_to_existing_text() {
        let mut output = String::from("value=");
        append_decimal_i16(&mut output, i16::MIN);
        output.push(',');
        append_decimal_i32(&mut output, i32::MAX);
        assert_eq!(output, "value=-32768,2147483647");
    }

    #[test]
    fn startup_audio_parser_matches_its_recovered_call_domain() {
        let cases: &[(&[u8; STARTUP_AUDIO_NUMBER_LENGTH], i16)] = &[
            (b"\0\0\0", 0),
            (b"x00", 0),
            (b"+\0\0", 0),
            (b"-\0\0", 0),
            (b"0\0\0", 0),
            (b"222", 222),
            (b"+12", 12),
            (b"-12", -12),
            (b"123", 123),
            (b"12x", 12),
            (b" 12", 0),
            (b"\x80\x31\0", 0),
        ];

        for (input, expected) in cases {
            assert_eq!(parse_startup_audio_number(input), *expected);
        }
    }

    #[test]
    fn startup_audio_parser_matches_every_generic_native_vector() {
        let vectors: Vec<StartupAudioGenericVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_2612_generic.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), STARTUP_AUDIO_GENERIC_VECTOR_COUNT);

        for vector in vectors {
            let mut input = [u8::MIN; STARTUP_AUDIO_NUMBER_LENGTH];
            if let Some((_, first_byte)) = vector.mem_in.first() {
                input[0] = *first_byte;
            }
            assert_eq!(
                u32::from(parse_startup_audio_number(&input) as u16),
                vector.regs_out.eax & u32::from(u16::MAX)
            );
        }
    }

    fn original_payload(bytes: &[u8]) -> &[u8] {
        let (&terminator, payload) = bytes.split_last().unwrap();
        assert_eq!(terminator, u8::MIN);
        payload
    }
}
