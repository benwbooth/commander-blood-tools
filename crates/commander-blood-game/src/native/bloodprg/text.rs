//! Byte-string operations recovered from BLOODPRG.

use std::ffi::CStr;

/// Compare two NUL-terminated byte strings for exact equality.
///
/// This is the typed equivalent of `string_compare` at BLOODPRG file offset
/// `0x0025a4`. `CStr` validates termination up front, so the runtime needs no
/// cursor wrapping or unbounded byte reads.
pub fn nul_terminated_bytes_equal(left: &CStr, right: &CStr) -> bool {
    left.to_bytes() == right.to_bytes()
}

/// Return the payload length of a NUL-terminated byte string.
///
/// This translates `bloodprg_strlen` at BLOODPRG file offset `0x002665`.
/// `CStr` makes the original bounded probing loop unnecessary while retaining
/// the byte count consumed by every valid game string.
pub fn nul_terminated_byte_len(text: &CStr) -> usize {
    text.to_bytes().len()
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const STRING_COMPARE_ORACLE_VECTOR_COUNT: usize = 10;
    const STRING_LENGTH_ORACLE_VECTOR_COUNT: usize = 9;

    #[derive(Deserialize)]
    struct EqualityOracleVector {
        name: String,
        matched_carry: bool,
    }

    #[derive(Deserialize)]
    struct LengthOracleVector {
        return_length: usize,
    }

    #[test]
    fn equality_matches_every_original_semantic_case() {
        let vectors: Vec<EqualityOracleVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_25a4_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), STRING_COMPARE_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let (left, right) = equality_case(&vector.name);
            assert_eq!(
                nul_terminated_bytes_equal(
                    CStr::from_bytes_with_nul(left).unwrap(),
                    CStr::from_bytes_with_nul(right).unwrap(),
                ),
                vector.matched_carry,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn length_matches_every_original_result() {
        let vectors: Vec<LengthOracleVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_2665_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), STRING_LENGTH_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut bytes = vec![b'x'; vector.return_length];
            bytes.push(u8::MIN);
            let text = CStr::from_bytes_with_nul(&bytes).unwrap();
            assert_eq!(nul_terminated_byte_len(text), vector.return_length);
        }
    }

    fn equality_case(name: &str) -> (&'static [u8], &'static [u8]) {
        match name {
            "empty" => (b"\0", b"\0"),
            "equal_ascii" => (b"BLOOD\0", b"BLOOD\0"),
            "first_mismatch" => (b"BLOOD\0", b"blood\0"),
            "middle_mismatch" => (b"BLOOD\0", b"BL00D\0"),
            "left_prefix" => (b"CB\0", b"CBLOOD\0"),
            "right_prefix" => (b"CBLOOD\0", b"CB\0"),
            "high_bytes" => (b"\x80\xff\0", b"\x80\xff\0"),
            "left_offset_wrap" | "right_offset_wrap" | "descending_left" => (b"OK\0", b"OK\0"),
            _ => panic!("unknown oracle case {name}"),
        }
    }
}
