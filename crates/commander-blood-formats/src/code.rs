//! Lossless framing for compiled BloodScript `SCRIPT*.COD` images.
//!
//! The original token walker combines input indexing with machine-specific
//! pointer arithmetic. This module retains only the authored format behavior:
//! descriptor lookup, query-mode transitions, optional prefixes, and bounded
//! fixed or terminated payloads.

use std::fmt;

const FIRST_OPCODE: u8 = 0xA0;
const END_MARKER: u8 = 0xFF;
const TEXT_OPCODE: u8 = 0xA6;
const OPTIONAL_PREFIX_OPCODE: u8 = 0xA1;
const CONTROL_FLAG: u8 = 0x80;
const ENTER_QUERY_MODE: u8 = 0xFF;
const LEAVE_QUERY_MODE: u8 = 0xFE;
const OPTIONAL_PREFIX: u8 = 0xFD;
const SCAN_OR_OPTIONAL_PREFIX: u8 = 0xFB;
const OPCODE_SIZE: usize = 1;
const WORD_SIZE: usize = 2;
const TEXT_HEADER_SIZE: usize = 5;
const MAXIMUM_FORWARD_TOKEN_LENGTH: usize = i8::MAX as usize + OPCODE_SIZE;
const DESCRIPTOR_COUNT: usize = u8::MAX as usize - FIRST_OPCODE as usize + OPCODE_SIZE;
const LAST_SHARED_OPCODE: u8 = 0xD2;
const LAST_BIG_BUG_BANG_OPCODE: u8 = 0xD7;
const BIG_BUG_BANG_EXTENSION_LENGTHS: [u8; 5] = [9, 5, 3, 5, 1];

/// Native instruction dialect selected by the game, not guessed from operands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScriptDialect {
    /// Commander Blood's original descriptor window, including observed data tails.
    #[default]
    CommanderBlood,
    /// Big Bug Bang's A0-D7 instructions. Unrecovered adjacent-data reads fail closed.
    BigBugBang,
}

impl ScriptDialect {
    fn descriptor(self, opcode: u8) -> Option<(u8, u8)> {
        match self {
            Self::CommanderBlood => {
                Some(OBSERVABLE_DESCRIPTORS[usize::from(opcode - FIRST_OPCODE)])
            }
            Self::BigBugBang if opcode <= LAST_SHARED_OPCODE => {
                Some(OBSERVABLE_DESCRIPTORS[usize::from(opcode - FIRST_OPCODE)])
            }
            Self::BigBugBang if opcode <= LAST_BIG_BUG_BANG_OPCODE => {
                // BLOOD2PG.EXE skip table at file 0x16AEA, D3-D7 pairs.
                let length =
                    BIG_BUG_BANG_EXTENSION_LENGTHS[usize::from(opcode - LAST_SHARED_OPCODE - 1)];
                Some((length, length))
            }
            Self::BigBugBang => None,
        }
    }
}

/// Bytes observed by the original unbounded descriptor lookup for byte values
/// `0xA0..=0xFF`.
///
/// The first 52 pairs are the authored table. Later pairs are adjacent static
/// data that shipped COD images can reach through the original lookup, so they
/// are part of the format behavior even though they were not declared table
/// entries by the original program.
const OBSERVABLE_DESCRIPTORS: [(u8, u8); DESCRIPTOR_COUNT] = [
    (0x03, 0xFF),
    (0x01, 0xFE),
    (0x03, 0x03),
    (0x03, 0xFB),
    (0x03, 0x03),
    (0x04, 0x02),
    (0x00, 0x00),
    (0x03, 0x03),
    (0x00, 0x00),
    (0x04, 0xFF),
    (0x01, 0x01),
    (0x04, 0x04),
    (0x00, 0x00),
    (0x05, 0x05),
    (0x05, 0xFD),
    (0x05, 0xFD),
    (0x05, 0xFD),
    (0x07, 0x07),
    (0x05, 0xFD),
    (0x05, 0xFD),
    (0x07, 0x07),
    (0x07, 0x07),
    (0x07, 0x07),
    (0x04, 0xFD),
    (0x07, 0x07),
    (0x07, 0x07),
    (0x05, 0xFD),
    (0x05, 0xFD),
    (0x05, 0xFD),
    (0x07, 0x07),
    (0x07, 0x07),
    (0x07, 0x07),
    (0x07, 0x07),
    (0x05, 0xFD),
    (0x05, 0xFD),
    (0x05, 0xFD),
    (0x05, 0xFD),
    (0x05, 0xFD),
    (0x05, 0xFD),
    (0x05, 0xFD),
    (0x05, 0xFD),
    (0x03, 0xFD),
    (0x05, 0x05),
    (0x06, 0x06),
    (0x00, 0x00),
    (0x07, 0xFD),
    (0x01, 0x01),
    (0x01, 0x01),
    (0x01, 0x01),
    (0x01, 0x01),
    (0x02, 0x02),
    (0x00, 0x00),
    (0x6D, 0x65),
    (0x6D, 0x6F),
    (0x69, 0x72),
    (0x65, 0x20),
    (0x6C, 0x69),
    (0x62, 0x72),
    (0x65, 0x00),
    (0x00, 0x00),
    (0x46, 0x0A),
    (0x09, 0x00),
    (0x66, 0x69),
    (0x6E, 0x00),
    (0x00, 0x00),
    (0x00, 0x00),
    (0x00, 0x00),
    (0x00, 0x00),
    (0x00, 0x00),
    (0x00, 0x00),
    (0x00, 0x00),
    (0x00, 0x00),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0x27),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0x28),
    (0xFF, 0xFF),
    (0xFF, 0xFF),
    (0xFF, 0x29),
    (0x25, 0xFF),
];

/// Byte position within one decoded COD source image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptCodeOffset(usize);

impl ScriptCodeOffset {
    /// Construct a source position for incremental decoding.
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Return the zero-based byte position in the source image.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Opcode byte known to the original token descriptor lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptOpcode(u8);

impl ScriptOpcode {
    /// Decode an opcode accepted by the original token walker.
    pub const fn decode(byte: u8) -> Option<Self> {
        if byte >= FIRST_OPCODE && byte < END_MARKER {
            Some(Self(byte))
        } else {
            None
        }
    }

    /// Return the encoded opcode byte.
    pub const fn byte(self) -> u8 {
        self.0
    }
}

/// Length-selection mode used while framing COD tokens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScriptDecodingMode {
    /// Ordinary execution token lengths.
    #[default]
    Normal,
    /// Conditional-query token lengths.
    Query,
}

/// Minimal parser state carried between adjacent COD tokens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptTokenDecoder {
    dialect: ScriptDialect,
    mode: ScriptDecodingMode,
    scan_variable_blocks: bool,
}

impl ScriptTokenDecoder {
    /// Start framing an explicitly selected native dialect in normal mode.
    pub const fn new(dialect: ScriptDialect) -> Self {
        Self {
            dialect,
            mode: ScriptDecodingMode::Normal,
            scan_variable_blocks: false,
        }
    }

    /// Return the current descriptor-length mode.
    pub const fn mode(self) -> ScriptDecodingMode {
        self.mode
    }

    /// Select the original block-scan interpretation of scan-or-prefix tokens.
    pub fn set_variable_block_scanning(&mut self, enabled: bool) {
        self.scan_variable_blocks = enabled;
    }
}

/// One losslessly framed COD token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptToken {
    dialect: ScriptDialect,
    source_offset: ScriptCodeOffset,
    opcode: ScriptOpcode,
    mode_before: ScriptDecodingMode,
    mode_after: ScriptDecodingMode,
    encoded: Box<[u8]>,
}

impl ScriptToken {
    /// Return the native dialect that established this token's boundaries.
    pub const fn dialect(&self) -> ScriptDialect {
        self.dialect
    }

    /// Return this token's position in its source image.
    pub const fn source_offset(&self) -> ScriptCodeOffset {
        self.source_offset
    }

    /// Return the token's opcode.
    pub const fn opcode(&self) -> ScriptOpcode {
        self.opcode
    }

    /// Return the parser mode in effect before this token.
    pub const fn mode_before(&self) -> ScriptDecodingMode {
        self.mode_before
    }

    /// Return the parser mode after this token's descriptor control.
    pub const fn mode_after(&self) -> ScriptDecodingMode {
        self.mode_after
    }

    /// Return the token's exact authored bytes, including its opcode.
    pub fn encoded_bytes(&self) -> &[u8] {
        &self.encoded
    }

    /// Return the exclusive source position following this token.
    pub fn end_offset(&self) -> ScriptCodeOffset {
        ScriptCodeOffset(self.source_offset.index() + self.encoded.len())
    }
}

/// Complete losslessly framed COD program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptCode {
    dialect: ScriptDialect,
    tokens: Vec<ScriptToken>,
    end_marker_offset: ScriptCodeOffset,
}

impl ScriptCode {
    /// Return the owning dialect even when the program contains only its end marker.
    pub const fn dialect(&self) -> ScriptDialect {
        self.dialect
    }

    /// Return every token in authored byte order.
    pub fn tokens(&self) -> &[ScriptToken] {
        &self.tokens
    }

    /// Return the position of the final end marker.
    pub const fn end_marker_offset(&self) -> ScriptCodeOffset {
        self.end_marker_offset
    }

    /// Re-encode the complete COD image byte for byte.
    pub fn encode(&self) -> Vec<u8> {
        let token_bytes: usize = self.tokens.iter().map(|token| token.encoded.len()).sum();
        let mut output = Vec::with_capacity(token_bytes + OPCODE_SIZE);
        for token in &self.tokens {
            output.extend_from_slice(&token.encoded);
        }
        output.push(END_MARKER);
        output
    }
}

/// Failure while framing a COD image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptCodeError {
    /// Incremental decoding was requested at or beyond the available image.
    MissingOpcode {
        /// Requested source position.
        source_offset: ScriptCodeOffset,
    },
    /// A byte is outside the descriptor lookup's opcode domain.
    InvalidOpcode {
        /// Source position of the byte.
        source_offset: ScriptCodeOffset,
        /// Rejected byte.
        byte: u8,
    },
    /// The final marker was passed to the token decoder as an ordinary opcode.
    UnexpectedEndMarker {
        /// Marker position.
        source_offset: ScriptCodeOffset,
    },
    /// A descriptor reproduces an original backward cursor movement that is not
    /// valid in the shipped flat source images.
    BackwardDescriptorLength {
        /// Token position.
        source_offset: ScriptCodeOffset,
        /// Opcode selecting the descriptor.
        opcode: ScriptOpcode,
        /// Encoded total length.
        length: u8,
    },
    /// A fixed-size token extends beyond the source image.
    TruncatedToken {
        /// Token position.
        source_offset: ScriptCodeOffset,
        /// Required exclusive end position.
        required_end: usize,
        /// Available source length.
        available: usize,
    },
    /// A byte-granular payload has no zero-word terminator.
    UnterminatedPayload {
        /// First payload byte searched.
        source_offset: ScriptCodeOffset,
    },
    /// A text token's aligned dictionary-word list has no terminator.
    UnterminatedTextWords {
        /// Token position.
        source_offset: ScriptCodeOffset,
    },
    /// A complete COD image contains no final marker.
    MissingEndMarker {
        /// Position where the marker was required.
        source_offset: ScriptCodeOffset,
    },
    /// Authored bytes follow the final marker.
    TrailingCodeData {
        /// Marker position.
        end_marker_offset: ScriptCodeOffset,
        /// Number of bytes following it.
        trailing_bytes: usize,
    },
}

impl fmt::Display for ScriptCodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ScriptCodeError {}

/// Scan a byte-granular payload through its zero word and optional zero pad.
///
/// This is the bounds-checked format equivalent of `vm_token_special`.
pub fn scan_zero_terminated_payload(
    data: &[u8],
    source_offset: ScriptCodeOffset,
) -> Result<ScriptCodeOffset, ScriptCodeError> {
    let mut cursor = source_offset.index();
    while cursor.saturating_add(WORD_SIZE) <= data.len() {
        if data[cursor..cursor + WORD_SIZE] == [u8::MIN; WORD_SIZE] {
            cursor += WORD_SIZE;
            if data.get(cursor) == Some(&u8::MIN) {
                cursor += OPCODE_SIZE;
            }
            return Ok(ScriptCodeOffset(cursor));
        }
        cursor += OPCODE_SIZE;
    }
    Err(ScriptCodeError::UnterminatedPayload { source_offset })
}

/// Frame one token and advance the supplied decoding state.
///
/// This is the bounds-checked format equivalent of `vm_token_advance`.
pub fn decode_script_token(
    data: &[u8],
    source_offset: ScriptCodeOffset,
    decoder: &mut ScriptTokenDecoder,
) -> Result<ScriptToken, ScriptCodeError> {
    let start = source_offset.index();
    let Some(&opcode_byte) = data.get(start) else {
        return Err(ScriptCodeError::MissingOpcode { source_offset });
    };
    if opcode_byte == END_MARKER {
        return Err(ScriptCodeError::UnexpectedEndMarker { source_offset });
    }
    let Some(opcode) = ScriptOpcode::decode(opcode_byte) else {
        return Err(ScriptCodeError::InvalidOpcode {
            source_offset,
            byte: opcode_byte,
        });
    };

    let descriptor =
        decoder
            .dialect
            .descriptor(opcode_byte)
            .ok_or(ScriptCodeError::InvalidOpcode {
                source_offset,
                byte: opcode_byte,
            })?;
    let mode_before = decoder.mode;
    let mut mode_after = mode_before;
    let mut prefix_size = usize::MIN;
    let base_length = if descriptor.1 & CONTROL_FLAG == u8::MIN {
        match mode_before {
            ScriptDecodingMode::Normal => descriptor.0,
            ScriptDecodingMode::Query => descriptor.1,
        }
    } else {
        match descriptor.1 {
            ENTER_QUERY_MODE => mode_after = ScriptDecodingMode::Query,
            LEAVE_QUERY_MODE => mode_after = ScriptDecodingMode::Normal,
            OPTIONAL_PREFIX => {
                prefix_size =
                    usize::from(data.get(start + OPCODE_SIZE) == Some(&OPTIONAL_PREFIX_OPCODE));
            }
            _ if decoder.scan_variable_blocks => {}
            SCAN_OR_OPTIONAL_PREFIX => {
                prefix_size =
                    usize::from(data.get(start + OPCODE_SIZE) == Some(&OPTIONAL_PREFIX_OPCODE));
            }
            _ => {}
        }
        if decoder.scan_variable_blocks
            && !matches!(
                descriptor.1,
                ENTER_QUERY_MODE | LEAVE_QUERY_MODE | OPTIONAL_PREFIX
            )
        {
            u8::MIN
        } else {
            descriptor.0
        }
    };

    let end = if base_length == u8::MIN {
        if opcode_byte == TEXT_OPCODE {
            scan_text_token(data, source_offset)?
        } else {
            scan_zero_terminated_payload(data, ScriptCodeOffset(start + OPCODE_SIZE))?.index()
        }
    } else {
        if usize::from(base_length) > MAXIMUM_FORWARD_TOKEN_LENGTH {
            return Err(ScriptCodeError::BackwardDescriptorLength {
                source_offset,
                opcode,
                length: base_length,
            });
        }
        let required_end = start
            .saturating_add(usize::from(base_length))
            .saturating_add(prefix_size);
        if required_end > data.len() {
            return Err(ScriptCodeError::TruncatedToken {
                source_offset,
                required_end,
                available: data.len(),
            });
        }
        required_end
    };

    decoder.mode = mode_after;
    Ok(ScriptToken {
        dialect: decoder.dialect,
        source_offset,
        opcode,
        mode_before,
        mode_after,
        encoded: Box::from(&data[start..end]),
    })
}

/// Decode a complete COD image ending in exactly one final marker.
pub fn decode_script_code(data: &[u8]) -> Result<ScriptCode, ScriptCodeError> {
    decode_script_code_for_dialect(data, ScriptDialect::CommanderBlood)
}

/// Decode a complete COD image using one game's native instruction boundaries.
pub fn decode_script_code_for_dialect(
    data: &[u8],
    dialect: ScriptDialect,
) -> Result<ScriptCode, ScriptCodeError> {
    let mut decoder = ScriptTokenDecoder::new(dialect);
    let mut tokens = Vec::new();
    let mut cursor = usize::MIN;
    while let Some(&byte) = data.get(cursor) {
        if byte == END_MARKER {
            let trailing_bytes = data.len() - cursor - OPCODE_SIZE;
            if trailing_bytes != usize::MIN {
                return Err(ScriptCodeError::TrailingCodeData {
                    end_marker_offset: ScriptCodeOffset(cursor),
                    trailing_bytes,
                });
            }
            return Ok(ScriptCode {
                dialect,
                tokens,
                end_marker_offset: ScriptCodeOffset(cursor),
            });
        }
        let token = decode_script_token(data, ScriptCodeOffset(cursor), &mut decoder)?;
        cursor = token.end_offset().index();
        tokens.push(token);
    }
    Err(ScriptCodeError::MissingEndMarker {
        source_offset: ScriptCodeOffset(cursor),
    })
}

fn scan_text_token(data: &[u8], source_offset: ScriptCodeOffset) -> Result<usize, ScriptCodeError> {
    let word_list_start = source_offset
        .index()
        .saturating_add(OPCODE_SIZE)
        .saturating_add(TEXT_HEADER_SIZE);
    if word_list_start > data.len() {
        return Err(ScriptCodeError::TruncatedToken {
            source_offset,
            required_end: word_list_start,
            available: data.len(),
        });
    }
    let mut cursor = word_list_start;
    while cursor.saturating_add(WORD_SIZE) <= data.len() {
        if data[cursor..cursor + WORD_SIZE] == [u8::MIN; WORD_SIZE] {
            return Ok(cursor + WORD_SIZE);
        }
        cursor += WORD_SIZE;
    }
    Err(ScriptCodeError::UnterminatedTextWords { source_offset })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use serde::Deserialize;

    use super::*;

    const PROFILE_COUNT: usize = 5;
    const EXPECTED_TOKEN_COUNTS: [usize; PROFILE_COUNT] = [214, 3_271, 3_281, 1_714, 1_869];
    const ORIGINAL_DESCRIPTOR_FILE_OFFSET: usize = 0x14338;
    const PAYLOAD_SCAN_ORACLE_VECTOR_COUNT: usize = 9;
    const TOKEN_ADVANCE_ORACLE_VECTOR_COUNT: usize = 17;

    #[derive(Deserialize)]
    struct PayloadScanOracleVector {
        name: String,
        scan_byte_count: usize,
        extra_byte_consumed: bool,
        start_offset: u16,
        final_offset: u16,
    }

    #[derive(Deserialize)]
    struct TokenAdvanceOracleVector {
        name: String,
        opcode: u8,
        start_offset: u16,
        query_mode_before: u8,
        block_scan_flags: u8,
        query_mode_after: u8,
        final_offset: u16,
    }

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    #[test]
    fn descriptor_window_matches_the_original_binary() {
        let executable = include_bytes!("../../../re/bin/BLOODPRG.EXE");
        for (index, descriptor) in OBSERVABLE_DESCRIPTORS.iter().enumerate() {
            let offset = ORIGINAL_DESCRIPTOR_FILE_OFFSET + index * WORD_SIZE;
            assert_eq!(executable[offset], descriptor.0, "descriptor {index}");
            assert_eq!(executable[offset + 1], descriptor.1, "descriptor {index}");
        }
    }

    #[test]
    fn every_original_code_image_round_trips_exactly() {
        for profile in 1..=PROFILE_COUNT {
            let data = std::fs::read(original_asset(&format!("SCRIPT{profile}.COD"))).unwrap();
            let code = decode_script_code(&data).unwrap();
            assert_eq!(code.tokens().len(), EXPECTED_TOKEN_COUNTS[profile - 1]);
            assert_eq!(code.end_marker_offset().index(), data.len() - OPCODE_SIZE);
            assert_eq!(code.encode(), data);
        }
    }

    #[test]
    fn big_bug_bang_extensions_have_native_widths_in_both_modes() {
        for mode in [ScriptDecodingMode::Normal, ScriptDecodingMode::Query] {
            for (index, length) in BIG_BUG_BANG_EXTENSION_LENGTHS.into_iter().enumerate() {
                let opcode = LAST_SHARED_OPCODE + 1 + index as u8;
                let mut data = vec![OPTIONAL_PREFIX_OPCODE; usize::from(length)];
                data[0] = opcode;
                let mut decoder = ScriptTokenDecoder::new(ScriptDialect::BigBugBang);
                decoder.mode = mode;
                let token =
                    decode_script_token(&data, ScriptCodeOffset::new(0), &mut decoder).unwrap();
                assert_eq!(token.encoded_bytes(), data);
                assert_eq!(token.dialect(), ScriptDialect::BigBugBang);
                assert_eq!(decoder.mode(), mode);
                for truncated in 1..data.len() {
                    assert!(matches!(
                        decode_script_token(
                            &data[..truncated],
                            ScriptCodeOffset::new(0),
                            &mut decoder
                        ),
                        Err(ScriptCodeError::TruncatedToken { .. })
                    ));
                }
            }
        }
    }

    #[test]
    fn sequel_does_not_reuse_commanders_adjacent_descriptor_data() {
        let mut decoder = ScriptTokenDecoder::new(ScriptDialect::BigBugBang);
        for opcode in LAST_BIG_BUG_BANG_OPCODE + 1..END_MARKER {
            assert!(matches!(
                decode_script_token(&[opcode, 0, 0], ScriptCodeOffset::new(0), &mut decoder),
                Err(ScriptCodeError::InvalidOpcode { .. })
            ));
        }
        assert_eq!(
            ScriptTokenDecoder::default(),
            ScriptTokenDecoder::new(ScriptDialect::CommanderBlood)
        );
    }

    #[test]
    #[ignore = "requires the original Big Bug Bang disc extracted under output/big-bug-bang/disc"]
    fn big_bug_bang_original_corpus_round_trips_with_native_descriptors() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../output/big-bug-bang/disc");
        let executable = std::fs::read(root.join("BLOOD2PG.EXE")).unwrap();
        const SEQUEL_DESCRIPTOR_FILE_OFFSET: usize = 0x16AEA;
        for opcode in FIRST_OPCODE..=LAST_BIG_BUG_BANG_OPCODE {
            let pair = ScriptDialect::BigBugBang.descriptor(opcode).unwrap();
            let offset =
                SEQUEL_DESCRIPTOR_FILE_OFFSET + usize::from(opcode - FIRST_OPCODE) * WORD_SIZE;
            assert_eq!(
                &executable[offset..offset + WORD_SIZE],
                &[pair.0, pair.1],
                "opcode {opcode:02x}"
            );
        }
        for profile in 1..=17 {
            let data = std::fs::read(root.join(format!("SCRIPT{profile}.COD"))).unwrap();
            let code = decode_script_code_for_dialect(&data, ScriptDialect::BigBugBang)
                .unwrap_or_else(|error| panic!("SCRIPT{profile}: {error}"));
            assert_eq!(code.encode(), data, "SCRIPT{profile}");
            eprintln!(
                "SCRIPT{profile}: {} tokens, {} bytes",
                code.tokens().len(),
                data.len()
            );
        }
    }

    #[test]
    fn mode_prefix_and_terminated_forms_match_the_recovered_decoder() {
        let mut decoder = ScriptTokenDecoder::default();
        let enter_query =
            decode_script_token(&[0xA0, 0, 0], ScriptCodeOffset::new(0), &mut decoder).unwrap();
        assert_eq!(enter_query.encoded_bytes().len(), 3);
        assert_eq!(decoder.mode(), ScriptDecodingMode::Query);

        let query_length =
            decode_script_token(&[0xA5, 0], ScriptCodeOffset::new(0), &mut decoder).unwrap();
        assert_eq!(query_length.encoded_bytes().len(), 2);

        let leave_query =
            decode_script_token(&[0xA1], ScriptCodeOffset::new(0), &mut decoder).unwrap();
        assert_eq!(leave_query.encoded_bytes().len(), 1);
        assert_eq!(decoder.mode(), ScriptDecodingMode::Normal);

        let prefixed = decode_script_token(
            &[0xAE, OPTIONAL_PREFIX_OPCODE, 1, 2, 3, 4],
            ScriptCodeOffset::new(0),
            &mut decoder,
        )
        .unwrap();
        assert_eq!(prefixed.encoded_bytes().len(), 6);

        decoder.set_variable_block_scanning(true);
        let scanned =
            decode_script_token(&[0xA3, 0, 0], ScriptCodeOffset::new(0), &mut decoder).unwrap();
        assert_eq!(scanned.encoded_bytes().len(), 3);
        decoder.set_variable_block_scanning(false);

        let text = decode_script_token(
            &[TEXT_OPCODE, 1, 2, 3, 4, 5, 6, 7, 0, 0],
            ScriptCodeOffset::new(0),
            &mut decoder,
        )
        .unwrap();
        assert_eq!(text.encoded_bytes().len(), 10);

        decoder.mode = ScriptDecodingMode::Query;
        for opcode in [0xA8, 0xAC, 0xDD] {
            let token = decode_script_token(
                &[opcode, 0, 0, OPTIONAL_PREFIX_OPCODE],
                ScriptCodeOffset::new(0),
                &mut decoder,
            )
            .unwrap();
            assert_eq!(token.encoded_bytes().len(), 3, "opcode {opcode:#04X}");
        }
        decoder.mode = ScriptDecodingMode::Normal;
        let adjacent_data_descriptor = decode_script_token(
            &[0xE4, 0, 0, OPTIONAL_PREFIX_OPCODE],
            ScriptCodeOffset::new(0),
            &mut decoder,
        )
        .unwrap();
        assert_eq!(adjacent_data_descriptor.encoded_bytes().len(), 3);
    }

    #[test]
    fn terminated_payload_scanning_matches_every_natural_native_vector() {
        let vectors: Vec<PayloadScanOracleVector> = serde_json::from_str(include_str!(
            "../../../re/tools/oracle_vectors/func_6293_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), PAYLOAD_SCAN_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut encoded = vec![u8::MAX; vector.scan_byte_count];
            encoded.extend_from_slice(&[u8::MIN, u8::MIN]);
            encoded.push(if vector.extra_byte_consumed {
                u8::MIN
            } else {
                u8::MAX
            });
            let end = scan_zero_terminated_payload(&encoded, ScriptCodeOffset::new(0)).unwrap();
            let expected_length = vector.final_offset.wrapping_sub(vector.start_offset) as usize;
            assert_eq!(end.index(), expected_length, "{}", vector.name);
        }
    }

    #[test]
    fn token_advance_matches_every_natural_native_vector() {
        let vectors: Vec<TokenAdvanceOracleVector> = serde_json::from_str(include_str!(
            "../../../re/tools/oracle_vectors/func_62b6_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), TOKEN_ADVANCE_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut decoder = ScriptTokenDecoder {
                dialect: ScriptDialect::CommanderBlood,
                mode: decoding_mode(vector.query_mode_before),
                scan_variable_blocks: vector.block_scan_flags != u8::MIN,
            };
            let encoded = token_advance_fixture(&vector.name);
            let result = decode_script_token(encoded, ScriptCodeOffset::new(0), &mut decoder);

            if vector.name == "ff_length_sign_extends_after_decrement" {
                assert!(matches!(
                    result,
                    Err(ScriptCodeError::BackwardDescriptorLength {
                        length: u8::MAX,
                        ..
                    })
                ));
                continue;
            }

            let token = result.unwrap_or_else(|error| panic!("{}: {error}", vector.name));
            let expected_length = vector.final_offset.wrapping_sub(vector.start_offset) as usize;
            assert_eq!(token.opcode().byte(), vector.opcode, "{}", vector.name);
            assert_eq!(
                token.encoded_bytes().len(),
                expected_length,
                "{}",
                vector.name
            );
            assert_eq!(
                decoder.mode(),
                decoding_mode(vector.query_mode_after),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn malformed_code_images_are_rejected_without_cursor_emulation() {
        assert_eq!(
            decode_script_code(&[]).unwrap_err(),
            ScriptCodeError::MissingEndMarker {
                source_offset: ScriptCodeOffset::new(0),
            }
        );
        assert_eq!(
            decode_script_code(&[END_MARKER, 0]).unwrap_err(),
            ScriptCodeError::TrailingCodeData {
                end_marker_offset: ScriptCodeOffset::new(0),
                trailing_bytes: 1,
            }
        );
        assert_eq!(
            decode_script_token(
                &[0xA2],
                ScriptCodeOffset::new(0),
                &mut ScriptTokenDecoder::default(),
            )
            .unwrap_err(),
            ScriptCodeError::TruncatedToken {
                source_offset: ScriptCodeOffset::new(0),
                required_end: 3,
                available: 1,
            }
        );
        assert_eq!(
            decode_script_token(
                &[0xE8],
                ScriptCodeOffset::new(0),
                &mut ScriptTokenDecoder::default(),
            )
            .unwrap_err(),
            ScriptCodeError::BackwardDescriptorLength {
                source_offset: ScriptCodeOffset::new(0),
                opcode: ScriptOpcode::decode(0xE8).unwrap(),
                length: u8::MAX,
            }
        );
    }

    fn decoding_mode(value: u8) -> ScriptDecodingMode {
        match value {
            0 => ScriptDecodingMode::Normal,
            1 => ScriptDecodingMode::Query,
            _ => panic!("invalid oracle decoding mode {value}"),
        }
    }

    fn token_advance_fixture(name: &str) -> &'static [u8] {
        match name {
            "mode_zero_fixed_length" | "fixed_length_wraps_at_segment_end" => &[0xA2, 0x11, 0x22],
            "mode_one_selects_second_length" => &[0xA5, 0x11, 0x22, 0x33],
            "a0_sentinel_sets_mode" => &[0xA0, 0x11, 0x22],
            "a1_sentinel_clears_mode" => &[0xA1],
            "fd_sentinel_without_prefix" => &[0xAE, 0x22, 0x33, 0x44, 0x55],
            "fd_sentinel_consumes_a1_prefix" => &[0xAE, 0xA1, 0x22, 0x33, 0x44, 0x55],
            "fb_sentinel_without_prefix" => &[0xA3, 0x22, 0x33],
            "fb_sentinel_consumes_a1_prefix" => &[0xA3, 0xA1, 0x22, 0x33],
            "fb_sentinel_scans_when_block_flag_is_set" => &[0xA3, 0x12, 0x34, 0, 0, 0x7E],
            "zero_length_token_stops_after_zero_word" => &[0xA8, 0, 0, 0x7E],
            "zero_length_token_consumes_optional_zero" => &[0xAC, 0, 0, 0, 0x7E],
            "a6_uses_header_and_word_list_layout" => &[0xA6, 1, 2, 3, 4, 5, 0, 0],
            "a6_word_list_wraps_at_segment_end" => &[0xA6, 1, 2, 3, 4, 5, 0x34, 0x12, 0, 0],
            "mode_one_out_of_table_tail_zero_length" => &[0xDD, 0x44, 0, 0, 0x7E],
            "e4_reads_zero_descriptor_past_real_entries" => &[0xE4, 0, 0, 0x7E],
            "ff_length_sign_extends_after_decrement" => &[0xE8],
            unknown => panic!("unmapped token-advance oracle vector {unknown}"),
        }
    }
}
