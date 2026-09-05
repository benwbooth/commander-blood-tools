//! Offline authored-COD audit, not a gameplay or reachability test.

use std::error::Error;
use std::path::PathBuf;

use commander_blood_formats::code::{ScriptDialect, decode_script_code_for_dialect};
use commander_blood_formats::instruction::{ScriptTextControl, decode_script_text};
use commander_blood_formats::script::decode_script_dictionary;
use serde::Serialize;

const TEXT_OPCODE: u8 = 166;
const TEXT_HEADER_SIZE: usize = 6;
const WORD_SIZE: usize = 2;
const DYNAMIC_INVENTORY_MARKER: u16 = 65534;
const STATE_NUMBER_MARKER: u16 = 1;
const SEQUEL_PROFILE_COUNT: usize = 17;

#[derive(Serialize)]
struct Occurrence {
    token_byte: usize,
    flags: u16,
    word_byte: usize,
    state_byte: Option<u16>,
}

#[derive(Serialize)]
struct DecodeFailure {
    token_byte: usize,
    error: String,
}

#[derive(Serialize)]
struct ProfileAudit {
    profile: usize,
    text_tokens: usize,
    typed_success: usize,
    inventory_markers: Vec<Occurrence>,
    state_numbers: Vec<Occurrence>,
    failures: Vec<DecodeFailure>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("expected loose resource root")?,
    );
    let mut reports = Vec::new();
    for profile in 1..=SEQUEL_PROFILE_COUNT {
        let name = format!("SCRIPT{profile}");
        let bytes = std::fs::read(root.join(format!("{name}.COD")))?;
        let dictionary =
            decode_script_dictionary(&std::fs::read(root.join(format!("{name}.DIC")))?)?;
        let code = decode_script_code_for_dialect(&bytes, ScriptDialect::BigBugBang)?;
        let mut report = ProfileAudit {
            profile,
            text_tokens: 0,
            typed_success: 0,
            inventory_markers: Vec::new(),
            state_numbers: Vec::new(),
            failures: Vec::new(),
        };
        for token in code
            .tokens()
            .iter()
            .filter(|token| token.opcode().byte() == TEXT_OPCODE)
        {
            report.text_tokens += 1;
            let encoded = token.encoded_bytes();
            let control_bytes = encoded.get(4..6).ok_or("truncated A6 control")?;
            let control =
                ScriptTextControl::decode(u16::from_le_bytes([control_bytes[0], control_bytes[1]]));
            let words_start = TEXT_HEADER_SIZE
                + WORD_SIZE * usize::from(control.arms_resume())
                + WORD_SIZE * usize::from(control.uses_record_condition());
            let mut words = encoded
                .get(words_start..)
                .ok_or("truncated A6 word list")?
                .chunks_exact(WORD_SIZE)
                .enumerate();
            while let Some((index, word)) = words.next() {
                let value = u16::from_le_bytes([word[0], word[1]]);
                if value == STATE_NUMBER_MARKER || value == DYNAMIC_INVENTORY_MARKER {
                    let state_byte = if value == STATE_NUMBER_MARKER {
                        let (_, operand) = words.next().ok_or("missing state-number operand")?;
                        Some(u16::from_le_bytes([operand[0], operand[1]]))
                    } else {
                        None
                    };
                    let occurrence = Occurrence {
                        token_byte: token.source_offset().index(),
                        flags: control.bits(),
                        word_byte: words_start + WORD_SIZE * index,
                        state_byte,
                    };
                    if state_byte.is_some() {
                        report.state_numbers.push(occurrence);
                    } else {
                        report.inventory_markers.push(occurrence);
                    }
                }
            }
            match decode_script_text(token, &dictionary) {
                Ok(_) => report.typed_success += 1,
                Err(error) => report.failures.push(DecodeFailure {
                    token_byte: token.source_offset().index(),
                    error: format!("{error:?}"),
                }),
            }
        }
        reports.push(report);
    }
    serde_json::to_writer_pretty(std::io::stdout().lock(), &reports)?;
    Ok(())
}
