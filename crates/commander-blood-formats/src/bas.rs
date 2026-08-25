//! Lossless typed decoding for conversation and menu `SCRIPT*.BAS` images.
//!
//! BAS images interleave VM instructions with menu tables and selector nodes.
//! Source positions are retained only as serialized-format metadata; consumers
//! operate on interned dictionary words and owned instruction values.

use std::fmt;

use crate::code::{
    decode_script_token, ScriptCodeError, ScriptCodeOffset, ScriptToken, ScriptTokenDecoder,
};
use crate::instruction::{
    decode_script_sequence_request, decode_script_text, decode_script_topic_offer,
    ScriptInstructionError, ScriptSequenceRequest, ScriptText, ScriptTopicOffer,
};
use crate::script::{ScriptDictionary, ScriptWordId};

const MENU_OPCODE: u8 = 0xA3;
const TEXT_OPCODE: u8 = 0xA6;
const TOPIC_OFFER_OPCODE: u8 = 0xA7;
const SEQUENCE_REQUEST_OPCODE: u8 = 0xA8;
const YIELD_OPCODE: u8 = 0xAA;
const SELECTOR_YIELD_OPCODE: u8 = 0xAC;
const SHARED_BIT_STATE_A_OPCODE: u8 = 0xAE;
const SHARED_BIT_STATE_B_OPCODE: u8 = 0xB0;
const SHARED_STATE_A_OPCODE: u8 = 0xB4;
const SHARED_STATE_B_OPCODE: u8 = 0xC0;
const RECORD_CLEAR_OPCODE: u8 = 0xC9;
const RECORD_TRIPLE_OPCODE: u8 = 0xCD;
const END_OPCODE: u8 = 0xFF;
const OPCODE_SIZE: usize = 1;
const WORD_SIZE: usize = 2;
const SELECTOR_NODE_SIZE: usize = WORD_SIZE * 2;
const MINIMUM_MENU_ENTRIES: usize = 1;
const MAXIMUM_MENU_ENTRIES: usize = 128;
const MINIMUM_MENU_WORD_LENGTH: usize = 2;
const MAXIMUM_MENU_WORD_LENGTH: usize = 16;
const MENU_WORD_SEPARATOR: u8 = b' ';

/// Complete losslessly framed BAS program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptBas {
    tokens: Vec<ScriptBasToken>,
}

impl ScriptBas {
    /// Return every BAS token in authored byte order.
    pub fn tokens(&self) -> &[ScriptBasToken] {
        &self.tokens
    }

    /// Re-encode the complete BAS image byte for byte.
    pub fn encode(&self) -> Vec<u8> {
        let capacity = self.tokens.iter().map(|token| token.encoded.len()).sum();
        let mut output = Vec::with_capacity(capacity);
        for token in &self.tokens {
            output.extend_from_slice(&token.encoded);
        }
        output
    }
}

/// One typed BAS structure or VM instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptBasInstruction {
    /// One terminated list of interned menu labels.
    Menu(Box<[ScriptWordId]>),
    /// One active dialogue or subtitle instruction.
    Text(ScriptText),
    /// Yield execution without introducing a selector node.
    Yield,
    /// Yield execution and introduce the selector node that immediately follows.
    SelectorYield,
    /// One selector case and its optional next-node source position.
    SelectorNode {
        /// Interned selector concept.
        selector: ScriptWordId,
        /// Next selector node, or `None` for the authored zero sentinel.
        next: Option<ScriptCodeOffset>,
    },
    /// One optional topic offered to an active presentation.
    TopicOffer(ScriptTopicOffer),
    /// One owned HNM sequence basename request.
    SequenceRequest(ScriptSequenceRequest),
    /// One AEh or B0h shared bit-state operation.
    SharedBitState(ScriptToken),
    /// One B4h or C0h shared state operation.
    SharedState(ScriptToken),
    /// One C9h record-clear operation.
    RecordClear(ScriptToken),
    /// One CDh record-triple operation.
    RecordTriple(ScriptToken),
    /// End one BAS subprogram; later subprograms may follow in the same image.
    End,
}

/// One losslessly framed token in a BAS image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptBasToken {
    source_offset: ScriptCodeOffset,
    instruction: ScriptBasInstruction,
    encoded: Box<[u8]>,
}

impl ScriptBasToken {
    /// Return this token's position in its source image.
    pub const fn source_offset(&self) -> ScriptCodeOffset {
        self.source_offset
    }

    /// Return the decoded BAS meaning.
    pub const fn instruction(&self) -> &ScriptBasInstruction {
        &self.instruction
    }

    /// Return the token's exact authored bytes.
    pub fn encoded_bytes(&self) -> &[u8] {
        &self.encoded
    }

    /// Return the exclusive source position following this token.
    pub fn end_offset(&self) -> ScriptCodeOffset {
        ScriptCodeOffset::new(self.source_offset.index() + self.encoded.len())
    }
}

/// Failure while decoding a BAS image into known structures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptBasError {
    /// A selector node following ACh has fewer than four bytes.
    TruncatedSelectorNode {
        /// Selector-node source position.
        source_offset: ScriptCodeOffset,
    },
    /// A selector node names no word in the companion dictionary.
    InvalidSelectorWord {
        /// Selector-node source position.
        source_offset: ScriptCodeOffset,
        /// Unresolved DIC byte position.
        dictionary_offset: u16,
    },
    /// A selector node points outside its BAS image.
    InvalidSelectorTarget {
        /// Selector-node source position.
        source_offset: ScriptCodeOffset,
        /// Rejected target byte position.
        target: u16,
    },
    /// An A3 menu reaches the end of its image before a zero word.
    UnterminatedMenu {
        /// Menu opcode position.
        source_offset: ScriptCodeOffset,
    },
    /// An A3 menu contains no selectable label.
    EmptyMenu {
        /// Menu opcode position.
        source_offset: ScriptCodeOffset,
    },
    /// An A3 menu contains more labels than the native implementation accepts.
    OversizedMenu {
        /// Menu opcode position.
        source_offset: ScriptCodeOffset,
    },
    /// An A3 menu names no word in the companion dictionary.
    InvalidMenuWord {
        /// Menu opcode position.
        source_offset: ScriptCodeOffset,
        /// Unresolved DIC byte position.
        dictionary_offset: u16,
    },
    /// An A3 label violates the shipped menu-word shape.
    InvalidMenuLabel {
        /// Menu opcode position.
        source_offset: ScriptCodeOffset,
        /// Interned label that failed validation.
        word: ScriptWordId,
    },
    /// An A6 BAS text instruction is not marked active.
    InactiveText {
        /// Text opcode position.
        source_offset: ScriptCodeOffset,
    },
    /// The shared VM token framer rejected an instruction.
    Framing(ScriptCodeError),
    /// A framed instruction failed semantic validation.
    Instruction(ScriptInstructionError),
    /// A byte does not begin any recovered BAS structure.
    UnsupportedByte {
        /// Rejected byte position.
        source_offset: ScriptCodeOffset,
        /// Rejected byte value.
        byte: u8,
    },
}

impl fmt::Display for ScriptBasError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ScriptBasError {}

/// Decode one complete BAS image using its companion interned dictionary.
pub fn decode_script_bas(
    data: &[u8],
    dictionary: &ScriptDictionary,
) -> Result<ScriptBas, ScriptBasError> {
    let mut tokens = Vec::new();
    let mut cursor = usize::MIN;
    let mut selector_follows = false;

    while cursor < data.len() {
        let source_offset = ScriptCodeOffset::new(cursor);
        if selector_follows {
            let end = cursor.saturating_add(SELECTOR_NODE_SIZE);
            if end > data.len() {
                return Err(ScriptBasError::TruncatedSelectorNode { source_offset });
            }
            let selector_offset = read_word(data, cursor);
            let selector = dictionary.resolve_source_offset(selector_offset).ok_or(
                ScriptBasError::InvalidSelectorWord {
                    source_offset,
                    dictionary_offset: selector_offset,
                },
            )?;
            let encoded_next = read_word(data, cursor + WORD_SIZE);
            let next = if encoded_next == u16::MIN {
                None
            } else if usize::from(encoded_next) < data.len() {
                Some(ScriptCodeOffset::new(usize::from(encoded_next)))
            } else {
                return Err(ScriptBasError::InvalidSelectorTarget {
                    source_offset,
                    target: encoded_next,
                });
            };
            tokens.push(ScriptBasToken {
                source_offset,
                instruction: ScriptBasInstruction::SelectorNode { selector, next },
                encoded: Box::from(&data[cursor..end]),
            });
            cursor = end;
            selector_follows = false;
            continue;
        }

        let opcode = data[cursor];
        let (instruction, end) = match opcode {
            MENU_OPCODE => decode_menu(data, source_offset, dictionary)?,
            TEXT_OPCODE => {
                let token = frame_vm_token(data, source_offset)?;
                let end = token.end_offset().index();
                let text =
                    decode_script_text(&token, dictionary).map_err(ScriptBasError::Instruction)?;
                if !text.control.is_active() {
                    return Err(ScriptBasError::InactiveText { source_offset });
                }
                (ScriptBasInstruction::Text(text), end)
            }
            YIELD_OPCODE => (ScriptBasInstruction::Yield, cursor + OPCODE_SIZE),
            SELECTOR_YIELD_OPCODE => {
                selector_follows = true;
                (ScriptBasInstruction::SelectorYield, cursor + OPCODE_SIZE)
            }
            TOPIC_OFFER_OPCODE => {
                let token = frame_vm_token(data, source_offset)?;
                let end = token.end_offset().index();
                let offer = decode_script_topic_offer(&token, dictionary)
                    .map_err(ScriptBasError::Instruction)?;
                (ScriptBasInstruction::TopicOffer(offer), end)
            }
            SEQUENCE_REQUEST_OPCODE => {
                let token = frame_vm_token(data, source_offset)?;
                let end = token.end_offset().index();
                let request =
                    decode_script_sequence_request(&token).map_err(ScriptBasError::Instruction)?;
                (ScriptBasInstruction::SequenceRequest(request), end)
            }
            SHARED_BIT_STATE_A_OPCODE | SHARED_BIT_STATE_B_OPCODE => {
                let token = frame_vm_token(data, source_offset)?;
                let end = token.end_offset().index();
                (ScriptBasInstruction::SharedBitState(token), end)
            }
            SHARED_STATE_A_OPCODE | SHARED_STATE_B_OPCODE => {
                let token = frame_vm_token(data, source_offset)?;
                let end = token.end_offset().index();
                (ScriptBasInstruction::SharedState(token), end)
            }
            RECORD_CLEAR_OPCODE => {
                let token = frame_vm_token(data, source_offset)?;
                let end = token.end_offset().index();
                (ScriptBasInstruction::RecordClear(token), end)
            }
            RECORD_TRIPLE_OPCODE => {
                let token = frame_vm_token(data, source_offset)?;
                let end = token.end_offset().index();
                (ScriptBasInstruction::RecordTriple(token), end)
            }
            END_OPCODE => (ScriptBasInstruction::End, cursor + OPCODE_SIZE),
            byte => {
                return Err(ScriptBasError::UnsupportedByte {
                    source_offset,
                    byte,
                });
            }
        };

        tokens.push(ScriptBasToken {
            source_offset,
            instruction,
            encoded: Box::from(&data[cursor..end]),
        });
        cursor = end;
    }

    Ok(ScriptBas { tokens })
}

fn decode_menu(
    data: &[u8],
    source_offset: ScriptCodeOffset,
    dictionary: &ScriptDictionary,
) -> Result<(ScriptBasInstruction, usize), ScriptBasError> {
    let mut cursor = source_offset.index() + OPCODE_SIZE;
    let mut words = Vec::new();
    while cursor.saturating_add(WORD_SIZE) <= data.len() {
        let dictionary_offset = read_word(data, cursor);
        cursor += WORD_SIZE;
        if dictionary_offset == u16::MIN {
            if words.len() < MINIMUM_MENU_ENTRIES {
                return Err(ScriptBasError::EmptyMenu { source_offset });
            }
            return Ok((ScriptBasInstruction::Menu(words.into_boxed_slice()), cursor));
        }
        if words.len() == MAXIMUM_MENU_ENTRIES {
            return Err(ScriptBasError::OversizedMenu { source_offset });
        }
        let word = dictionary.resolve_source_offset(dictionary_offset).ok_or(
            ScriptBasError::InvalidMenuWord {
                source_offset,
                dictionary_offset,
            },
        )?;
        let label = dictionary
            .word(word)
            .expect("resolved dictionary identities remain valid");
        if !(MINIMUM_MENU_WORD_LENGTH..=MAXIMUM_MENU_WORD_LENGTH).contains(&label.len())
            || label.contains(&MENU_WORD_SEPARATOR)
        {
            return Err(ScriptBasError::InvalidMenuLabel {
                source_offset,
                word,
            });
        }
        words.push(word);
    }
    Err(ScriptBasError::UnterminatedMenu { source_offset })
}

fn frame_vm_token(
    data: &[u8],
    source_offset: ScriptCodeOffset,
) -> Result<ScriptToken, ScriptBasError> {
    decode_script_token(data, source_offset, &mut ScriptTokenDecoder::default())
        .map_err(ScriptBasError::Framing)
}

fn read_word(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        data[offset..offset + WORD_SIZE]
            .try_into()
            .expect("validated BAS word"),
    )
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::script::decode_script_dictionary;

    use super::*;

    const PROFILE_COUNT: usize = 5;
    const EXPECTED_TOPIC_OFFER_COUNT: usize = 19;
    const EXPECTED_BAS_SEQUENCE_REQUEST_COUNT: usize = 3;
    const EXPECTED_YIELD_COUNT: usize = 37;
    const EXPECTED_SELECTOR_YIELD_COUNT: usize = 321;
    const MAXIMUM_SHIPPED_SEQUENCE_BASENAME_LENGTH: usize = 12;

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    #[test]
    fn every_shipped_bas_image_has_complete_typed_framing_and_exact_round_trip() {
        let mut topic_offer_count = usize::MIN;
        let mut sequence_request_count = usize::MIN;
        let mut yield_count = usize::MIN;
        let mut selector_yield_count = usize::MIN;

        for profile in 1..=PROFILE_COUNT {
            let data = std::fs::read(original_asset(&format!("SCRIPT{profile}.BAS"))).unwrap();
            let dictionary_data =
                std::fs::read(original_asset(&format!("SCRIPT{profile}.DIC"))).unwrap();
            let dictionary = decode_script_dictionary(&dictionary_data).unwrap();
            let bas = decode_script_bas(&data, &dictionary).unwrap();
            assert_eq!(bas.encode(), data, "SCRIPT{profile}.BAS");

            for token in bas.tokens() {
                match token.instruction() {
                    ScriptBasInstruction::TopicOffer(offer) => {
                        assert!(offer.topic.is_some());
                        topic_offer_count += 1;
                    }
                    ScriptBasInstruction::SequenceRequest(request) => {
                        assert!(request.basename().ends_with(b".hnm"));
                        assert!(
                            request.basename().len() <= MAXIMUM_SHIPPED_SEQUENCE_BASENAME_LENGTH
                        );
                        sequence_request_count += 1;
                    }
                    ScriptBasInstruction::Yield => yield_count += 1,
                    ScriptBasInstruction::SelectorYield => selector_yield_count += 1,
                    _ => {}
                }
            }
        }

        assert_eq!(topic_offer_count, EXPECTED_TOPIC_OFFER_COUNT);
        assert_eq!(sequence_request_count, EXPECTED_BAS_SEQUENCE_REQUEST_COUNT);
        assert_eq!(yield_count, EXPECTED_YIELD_COUNT);
        assert_eq!(selector_yield_count, EXPECTED_SELECTOR_YIELD_COUNT);
    }
}
