//! Typed, lossless source language for Commander Blood VM programs.
//!
//! BloodScript IR is the compiler-facing layer above CBVM-ASM. It gives proven
//! token families authoritative typed statements while retaining `OP` and `RAW`
//! escapes for constructs whose high-level structure is not established. The
//! syntax is reconstructed for this project; it is not claimed to be the lost
//! historical source syntax.

use std::collections::HashMap;
use std::fmt::Write as _;

use anyhow::{Result, anyhow, bail};

use crate::vm::{self, VmToken};
use crate::vm_source::{self, ImageKind};

const SOURCE_FORMAT: &str = "bloodscript-ir-v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decompilation {
    pub source: String,
    pub typed_statements: usize,
    pub typed_bytes: usize,
    pub generic_op_statements: usize,
    pub generic_op_bytes: usize,
    pub raw_bytes: usize,
}

pub fn decompile(
    kind: ImageKind,
    image: &[u8],
    dictionary: &HashMap<u16, String>,
) -> Result<Decompilation> {
    let mut source = String::new();
    writeln!(source, "; BloodScript typed VM source")?;
    writeln!(source, "; format: {SOURCE_FORMAT}")?;
    writeln!(
        source,
        "; image: {}",
        match kind {
            ImageKind::Cod => "COD",
            ImageKind::Bas => "BAS",
        }
    )?;
    writeln!(source, "; size: 0x{:08X}", image.len())?;
    writeln!(source)?;

    let (typed_statements, typed_bytes, generic_op_statements, generic_op_bytes, raw_bytes) =
        match kind {
            ImageKind::Cod => decompile_cod(&mut source, image, dictionary)?,
            ImageKind::Bas => decompile_bas(&mut source, image, dictionary)?,
        };
    Ok(Decompilation {
        source,
        typed_statements,
        typed_bytes,
        generic_op_statements,
        generic_op_bytes,
        raw_bytes,
    })
}

pub fn compile(source: &str) -> Result<Vec<u8>> {
    let mut image = Vec::new();
    let mut saw_format = false;

    for (line_index, original_line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = original_line.trim();
        if let Some(value) = trimmed.strip_prefix("; format:") {
            if value.trim() != SOURCE_FORMAT {
                bail!("line {line_number}: unsupported format {:?}", value.trim());
            }
            saw_format = true;
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with(';') {
            continue;
        }

        let code = trimmed
            .split_once(';')
            .map_or(trimmed, |(code, _)| code)
            .trim();
        let (offset_text, statement) = code
            .split_once(':')
            .ok_or_else(|| anyhow!("line {line_number}: expected OFFSET: STATEMENT"))?;
        let offset = parse_hex_usize(offset_text.trim(), line_number, "offset")?;
        if offset != image.len() {
            bail!(
                "line {line_number}: offset 0x{offset:08X} does not follow 0x{:08X}",
                image.len()
            );
        }

        let mut fields = statement.split_whitespace();
        let name = fields
            .next()
            .ok_or_else(|| anyhow!("line {line_number}: missing statement"))?;
        let args: Vec<&str> = fields.collect();
        let encoded = compile_statement(name, &args, line_number)?;
        if encoded.is_empty() {
            bail!("line {line_number}: statement emitted no bytes");
        }
        image.extend_from_slice(&encoded);
    }

    if !saw_format {
        bail!("missing '; format: {SOURCE_FORMAT}' header");
    }
    Ok(image)
}

fn decompile_cod(
    output: &mut String,
    image: &[u8],
    dictionary: &HashMap<u16, String>,
) -> Result<(usize, usize, usize, usize, usize)> {
    let mut cursor = 0usize;
    let mut typed_statements = 0usize;
    let mut typed_bytes = 0usize;
    let mut generic_op_statements = 0usize;
    let mut generic_op_bytes = 0usize;
    let mut raw_bytes = 0usize;

    for token in vm::walk(image, 0, image.len()) {
        let offset = token.offset();
        if offset > cursor {
            emit_raw(output, cursor, &image[cursor..offset], "undecoded gap")?;
            raw_bytes += offset - cursor;
        }
        let Some(encoded) = vm::encode_token(&token) else {
            emit_raw(output, offset, &image[offset..], "invalid token tail")?;
            raw_bytes += image.len() - offset;
            cursor = image.len();
            break;
        };
        let end = offset
            .checked_add(encoded.len())
            .ok_or_else(|| anyhow!("token at 0x{offset:08X} overflows"))?;
        if image.get(offset..end) != Some(encoded.as_slice()) {
            bail!("token at 0x{offset:08X} does not re-encode exactly");
        }
        emit_token(output, &token, dictionary)?;
        typed_statements += 1;
        typed_bytes += encoded.len();
        if matches!(token, VmToken::Op { .. }) {
            generic_op_statements += 1;
            generic_op_bytes += encoded.len();
        }
        cursor = end;
    }

    if cursor < image.len() && image[cursor] == 0xFF {
        writeln!(output, "{cursor:08X}: END")?;
        typed_statements += 1;
        typed_bytes += 1;
        cursor += 1;
    }
    if cursor < image.len() {
        emit_raw(output, cursor, &image[cursor..], "trailing bytes")?;
        raw_bytes += image.len() - cursor;
    }
    Ok((
        typed_statements,
        typed_bytes,
        generic_op_statements,
        generic_op_bytes,
        raw_bytes,
    ))
}

fn decompile_bas(
    output: &mut String,
    image: &[u8],
    dictionary: &HashMap<u16, String>,
) -> Result<(usize, usize, usize, usize, usize)> {
    let mut cursor = 0usize;
    let mut raw_start = 0usize;
    let mut typed_statements = 0usize;
    let mut typed_bytes = 0usize;
    let mut raw_bytes = 0usize;

    while cursor < image.len() {
        if let Some((end, token)) = vm_source::bas_token_at(image, cursor, dictionary) {
            if raw_start < cursor {
                emit_raw(
                    output,
                    raw_start,
                    &image[raw_start..cursor],
                    "BAS structure",
                )?;
                raw_bytes += cursor - raw_start;
            }
            let encoded = token.encode().ok_or_else(|| {
                anyhow!("BAS token at 0x{cursor:08X} cannot be encoded")
            })?;
            if image.get(cursor..end) != Some(encoded.as_slice()) {
                bail!("BAS token at 0x{cursor:08X} does not re-encode exactly");
            }
            match &token {
                vm_source::BasToken::Menu { word_offsets, .. } => {
                    write!(output, "{cursor:08X}: MENU")?;
                    for word in word_offsets {
                        write!(output, " {word:04X}")?;
                    }
                    writeln!(
                        output,
                        " ; {}",
                        word_offsets
                            .iter()
                            .filter_map(|word| dictionary.get(word))
                            .map(String::as_str)
                            .collect::<Vec<_>>()
                            .join(" | ")
                    )?;
                }
                vm_source::BasToken::Text(token) | vm_source::BasToken::Vm(token) => {
                    emit_token(output, token, dictionary)?;
                }
                vm_source::BasToken::Yield { .. } => {
                    writeln!(output, "{cursor:08X}: YIELD ; opcode AA")?;
                }
                vm_source::BasToken::BlockEnd {
                    selector,
                    continuation,
                    ..
                } => {
                    writeln!(
                        output,
                        "{cursor:08X}: BAS_BLOCK_END {selector:04X} {continuation:04X} ; {:?}",
                        dictionary.get(selector).map(String::as_str).unwrap_or("")
                    )?;
                }
                vm_source::BasToken::PresentationRegister { value, .. } => {
                    writeln!(
                        output,
                        "{cursor:08X}: PRESENTATION_REGISTER {value:04X}"
                    )?;
                }
                vm_source::BasToken::End { .. } => {
                    writeln!(output, "{cursor:08X}: END")?;
                }
            }
            typed_statements += 1;
            typed_bytes += encoded.len();
            cursor = end;
            raw_start = cursor;
            continue;
        }
        cursor += 1;
    }

    if raw_start < image.len() {
        emit_raw(output, raw_start, &image[raw_start..], "BAS structure")?;
        raw_bytes += image.len() - raw_start;
    }
    Ok((typed_statements, typed_bytes, 0, 0, raw_bytes))
}

fn emit_token(
    output: &mut String,
    token: &VmToken,
    dictionary: &HashMap<u16, String>,
) -> Result<()> {
    let offset = token.offset();
    write!(output, "{offset:08X}: ")?;
    match token {
        VmToken::Text {
            line_index,
            voice_selector,
            flags_b4,
            flags_b5,
            loop_target,
            control_word,
            word_offsets,
            ..
        } => {
            write!(
                output,
                "TEXT {line_index:04X} {voice_selector:02X} {flags_b4:02X} {flags_b5:02X} {} {}",
                option_word(*loop_target),
                option_word(*control_word)
            )?;
            for word in word_offsets {
                write!(output, " {word:04X}")?;
            }
        }
        VmToken::GuardPush { target, .. } => write!(output, "GUARD_PUSH {target:04X}")?,
        VmToken::GuardPop { .. } => write!(output, "GUARD_POP")?,
        VmToken::ConceptGuard {
            word_offset,
            inverted,
            ..
        } => write!(
            output,
            "CONCEPT_GUARD {word_offset:04X} {}",
            bool_digit(*inverted)
        )?,
        VmToken::Jump { target, .. } => write!(output, "JUMP {target:04X}")?,
        VmToken::StateArray {
            index,
            value: Some(value),
            ..
        } => write!(output, "STATE_ARRAY_SET {index:02X} {value:04X}")?,
        VmToken::StateArray {
            index, value: None, ..
        } => write!(output, "STATE_ARRAY_TEST {index:02X}")?,
        VmToken::ConditionalBlock { flags, target, .. } => {
            write!(output, "CONDITIONAL_BLOCK {flags:02X} {target:04X}")?
        }
        VmToken::LoadString { value, .. } => write!(output, "LOAD_STRING \"{value}\"")?,
        VmToken::PokeByte { address, value, .. } => {
            write!(output, "POKE_BYTE {address:04X} {value:02X}")?
        }
        VmToken::CharacterSlot { slot, name, .. } => {
            write!(output, "CHARACTER_SLOT {slot:02X} \"{name}\"")?
        }
        VmToken::ClearAlternateConcept { .. } => write!(output, "CLEAR_ALTERNATE_CONCEPT")?,
        VmToken::FlagBranch { opcode, .. } => match *opcode {
            vm::OP_COND_BRANCH_PRESENTATION => write!(output, "BRANCH_PRESENTATION")?,
            vm::OP_COND_BRANCH_GAMEFLAG => write!(output, "BRANCH_GAMEFLAG")?,
            vm::OP_COND_BRANCH_FLAG_274F => write!(output, "BRANCH_FLAG_274F")?,
            _ => bail!("unsupported flag-branch opcode {opcode:02X}"),
        },
        VmToken::Actor {
            record_offset,
            related_record_offset,
            inverted,
            ..
        } => write!(
            output,
            "ACTOR {record_offset:04X} {related_record_offset:04X} {}",
            bool_digit(*inverted)
        )?,
        VmToken::RecordLink {
            record_offset,
            related_record_offset,
            inverted,
            ..
        } => write!(
            output,
            "RECORD_LINK {record_offset:04X} {related_record_offset:04X} {}",
            bool_digit(*inverted)
        )?,
        VmToken::RecordEntry {
            entry_opcode,
            record_offset,
            operand,
            inverted,
            ..
        } => write!(
            output,
            "RECORD_ENTRY {entry_opcode:02X} {record_offset:04X} {operand:04X} {}",
            bool_digit(*inverted)
        )?,
        VmToken::RecordClear { record_offset, .. } => {
            write!(output, "RECORD_CLEAR {record_offset:04X}")?
        }
        VmToken::BitFlag {
            flag_offset,
            bit_index,
            clear,
            ..
        } => write!(
            output,
            "BIT_FLAG {flag_offset:04X} {bit_index:02X} {}",
            bool_digit(*clear)
        )?,
        VmToken::SharedState {
            opcode,
            field_offset,
            operator,
            rhs_mode,
            rhs,
            ..
        } => write!(
            output,
            "SHARED_STATE {opcode:02X} {field_offset:04X} {operator:02X} {rhs_mode:02X} {rhs:04X}"
        )?,
        VmToken::SharedBitState {
            opcode,
            field_offset,
            mask,
            inverted,
            ..
        } => write!(
            output,
            "SHARED_BIT_STATE {opcode:02X} {field_offset:04X} {mask:04X} {}",
            bool_digit(*inverted)
        )?,
        VmToken::RecordWildcard {
            opcode,
            record_offset,
            value,
            inverted,
            ..
        } => write!(
            output,
            "RECORD_WILDCARD {opcode:02X} {record_offset:04X} {value:04X} {}",
            bool_digit(*inverted)
        )?,
        VmToken::RecordState {
            opcode,
            record_offset,
            operand,
            inverted,
            ..
        } => write!(
            output,
            "RECORD_STATE {opcode:02X} {record_offset:04X} {operand:04X} {}",
            bool_digit(*inverted)
        )?,
        VmToken::GlobalWordCompare {
            operator,
            tag,
            value,
            ..
        } => write!(
            output,
            "GLOBAL_WORD_COMPARE {operator:02X} {tag:02X} {value:04X}"
        )?,
        VmToken::GlobalPairCompare {
            operator,
            packed_value,
            reserved,
            ..
        } => write!(
            output,
            "GLOBAL_PAIR_COMPARE {operator:02X} {packed_value:04X} {reserved:04X}"
        )?,
        VmToken::PairRecord {
            opcode,
            record_offset,
            first_word,
            second_word,
            ..
        } => write!(
            output,
            "PAIR_RECORD {opcode:02X} {record_offset:04X} {first_word:04X} {second_word:04X}"
        )?,
        VmToken::RecordTriple {
            record_offset,
            first_word,
            second_word,
            inverted,
            ..
        } => write!(
            output,
            "RECORD_TRIPLE {record_offset:04X} {first_word:04X} {second_word:04X} {}",
            bool_digit(*inverted)
        )?,
        VmToken::ScriptProfileRequest { operand, .. } => {
            write!(output, "RUN_PROFILE {operand:02X}")?
        }
        VmToken::Op {
            opcode, operands, ..
        } => {
            write!(output, "OP {opcode:02X}")?;
            for byte in operands {
                write!(output, " {byte:02X}")?;
            }
        }
        VmToken::Invalid { byte, .. } => write!(output, "RAW {byte:02X}")?,
    }
    writeln!(output, " ; {}", vm_source::token_comment(token, dictionary))?;
    Ok(())
}

fn emit_raw(output: &mut String, offset: usize, bytes: &[u8], comment: &str) -> Result<()> {
    write!(output, "{offset:08X}: RAW")?;
    for byte in bytes {
        write!(output, " {byte:02X}")?;
    }
    writeln!(output, " ; {comment}")?;
    Ok(())
}

fn compile_statement(name: &str, args: &[&str], line: usize) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let word = |output: &mut Vec<u8>, value: u16| {
        output.extend_from_slice(&value.to_le_bytes());
    };
    match name {
        "RAW" => return parse_byte_list(args, line, "RAW"),
        "END" => {
            require_count(args, 0, line, name)?;
            output.push(0xFF);
        }
        "OP" => {
            require_min_count(args, 1, line, name)?;
            output.push(parse_byte(args[0], line, "opcode")?);
            for value in &args[1..] {
                output.push(parse_byte(value, line, "operand")?);
            }
        }
        "MENU" => {
            require_min_count(args, 1, line, name)?;
            output.push(0xA3);
            for value in args {
                word(&mut output, parse_word(value, line, "menu word")?);
            }
            word(&mut output, 0);
        }
        "YIELD" => {
            require_count(args, 0, line, name)?;
            output.push(vm::OP_YIELD_A);
        }
        "BAS_BLOCK_END" => {
            require_count(args, 2, line, name)?;
            output.push(vm::OP_YIELD_B);
            word(&mut output, parse_word(args[0], line, "selector")?);
            word(&mut output, parse_word(args[1], line, "continuation")?);
        }
        "PRESENTATION_REGISTER" => {
            require_count(args, 1, line, name)?;
            output.push(0xA7);
            word(&mut output, parse_word(args[0], line, "presentation value")?);
        }
        "GUARD_PUSH" => {
            require_count(args, 1, line, name)?;
            output.push(vm::OP_PUSH);
            word(&mut output, parse_word(args[0], line, "guard target")?);
        }
        "GUARD_POP" => {
            require_count(args, 0, line, name)?;
            output.push(vm::OP_POP);
        }
        "CONCEPT_GUARD" => {
            require_count(args, 2, line, name)?;
            output.push(vm::OP_CONCEPT_GUARD);
            if parse_bool(args[1], line, "inverted")? {
                output.push(vm::OP_POP);
            }
            word(
                &mut output,
                parse_word(args[0], line, "dictionary word offset")?,
            );
        }
        "JUMP" => {
            require_count(args, 1, line, name)?;
            output.push(vm::OP_JUMP);
            word(&mut output, parse_word(args[0], line, "jump target")?);
        }
        "STATE_ARRAY_TEST" => {
            require_count(args, 1, line, name)?;
            output.push(vm::OP_COND_STATE_ARRAY);
            output.push(parse_byte(args[0], line, "state index")?);
        }
        "STATE_ARRAY_SET" => {
            require_count(args, 2, line, name)?;
            output.push(vm::OP_COND_STATE_ARRAY);
            output.push(parse_byte(args[0], line, "state index")?);
            word(&mut output, parse_word(args[1], line, "state value")?);
        }
        "CONDITIONAL_BLOCK" => {
            require_count(args, 2, line, name)?;
            output.push(vm::OP_COND_JUMP);
            output.push(parse_byte(args[0], line, "conditional flags")?);
            word(
                &mut output,
                parse_word(args[1], line, "conditional target")?,
            );
        }
        "LOAD_STRING" => {
            require_count(args, 1, line, name)?;
            output.push(vm::OP_LOAD_STRING);
            output.extend_from_slice(parse_simple_ascii(args[0], line, "string")?.as_bytes());
            output.extend_from_slice(&[0, 0]);
        }
        "POKE_BYTE" => {
            require_count(args, 2, line, name)?;
            output.push(vm::OP_POKE_BYTE);
            output.push(parse_byte(args[1], line, "value")?);
            word(&mut output, parse_word(args[0], line, "address")?);
        }
        "CHARACTER_SLOT" => {
            require_count(args, 2, line, name)?;
            output.push(vm::OP_SET_CHARACTER_SLOT);
            output.push(parse_byte(args[0], line, "slot")?);
            output.extend_from_slice(parse_simple_ascii(args[1], line, "name")?.as_bytes());
            output.extend_from_slice(&[0, 0]);
        }
        "CLEAR_ALTERNATE_CONCEPT" => {
            require_count(args, 0, line, name)?;
            output.push(vm::OP_CLEAR_ALTERNATE_CONCEPT);
        }
        "BRANCH_PRESENTATION" | "BRANCH_GAMEFLAG" | "BRANCH_FLAG_274F" => {
            require_count(args, 0, line, name)?;
            output.push(match name {
                "BRANCH_PRESENTATION" => vm::OP_COND_BRANCH_PRESENTATION,
                "BRANCH_GAMEFLAG" => vm::OP_COND_BRANCH_GAMEFLAG,
                _ => vm::OP_COND_BRANCH_FLAG_274F,
            });
        }
        "TEXT" => {
            require_min_count(args, 6, line, name)?;
            let line_index = parse_word(args[0], line, "line index")?;
            let voice = parse_byte(args[1], line, "voice")?;
            let flags_b4 = parse_byte(args[2], line, "control flags")?;
            let flags_b5 = parse_byte(args[3], line, "display flags")?;
            let loop_target = parse_optional_word(args[4], line, "loop target")?;
            let control_word = parse_optional_word(args[5], line, "control word")?;
            if (flags_b4 & 0x10 != 0) != loop_target.is_some() {
                bail!("line {line}: TEXT loop target disagrees with flag 0x10");
            }
            if (flags_b4 & 0x04 != 0) != control_word.is_some() {
                bail!("line {line}: TEXT control word disagrees with flag 0x04");
            }
            output.push(vm::OP_TEXT);
            word(&mut output, line_index);
            output.push(voice);
            output.push(flags_b4);
            output.push(flags_b5);
            if let Some(value) = loop_target {
                word(&mut output, value);
            }
            if let Some(value) = control_word {
                word(&mut output, value);
            }
            for value in &args[6..] {
                word(&mut output, parse_word(value, line, "dictionary word")?);
            }
            word(&mut output, 0);
        }
        "ACTOR" | "RECORD_LINK" => {
            require_count(args, 3, line, name)?;
            output.push(if name == "ACTOR" { 0xC4 } else { 0xC3 });
            if parse_bool(args[2], line, "inverted")? {
                output.push(0xA1);
            }
            word(&mut output, parse_word(args[0], line, "record")?);
            word(&mut output, parse_word(args[1], line, "related record")?);
        }
        "RECORD_ENTRY" | "RECORD_STATE" => {
            require_count(args, 4, line, name)?;
            output.push(parse_byte(args[0], line, "opcode")?);
            if parse_bool(args[3], line, "inverted")? {
                output.push(0xA1);
            }
            word(&mut output, parse_word(args[1], line, "record")?);
            word(&mut output, parse_word(args[2], line, "operand")?);
        }
        "RECORD_CLEAR" => {
            require_count(args, 1, line, name)?;
            output.push(0xC9);
            word(&mut output, parse_word(args[0], line, "record")?);
        }
        "BIT_FLAG" => {
            require_count(args, 3, line, name)?;
            output.push(vm::OP_BIT_FLAG);
            if parse_bool(args[2], line, "clear")? {
                output.push(0xA1);
            }
            word(&mut output, parse_word(args[0], line, "record")?);
            output.push(parse_byte(args[1], line, "bit index")?);
        }
        "SHARED_STATE" => {
            require_count(args, 5, line, name)?;
            let opcode = parse_byte(args[0], line, "opcode")?;
            if !vm::is_shared_state_opcode(opcode) {
                bail!("line {line}: opcode {opcode:02X} is not a SHARED_STATE opcode");
            }
            output.push(opcode);
            word(&mut output, parse_word(args[1], line, "field offset")?);
            output.push(parse_byte(args[2], line, "operator")?);
            output.push(parse_byte(args[3], line, "RHS mode")?);
            word(&mut output, parse_word(args[4], line, "RHS")?);
        }
        "SHARED_BIT_STATE" => {
            require_count(args, 4, line, name)?;
            let opcode = parse_byte(args[0], line, "opcode")?;
            if !vm::is_shared_bit_state_opcode(opcode) {
                bail!("line {line}: opcode {opcode:02X} is not a SHARED_BIT_STATE opcode");
            }
            output.push(opcode);
            if parse_bool(args[3], line, "inverted")? {
                output.push(vm::OP_POP);
            }
            word(&mut output, parse_word(args[1], line, "field offset")?);
            word(&mut output, parse_word(args[2], line, "mask")?);
        }
        "RECORD_WILDCARD" => {
            require_count(args, 4, line, name)?;
            let opcode = parse_byte(args[0], line, "opcode")?;
            if !vm::is_record_wildcard_opcode(opcode) {
                bail!("line {line}: opcode {opcode:02X} is not a RECORD_WILDCARD opcode");
            }
            let inverted = parse_bool(args[3], line, "inverted")?;
            if inverted && vm::OPCODE_DESC[(opcode - vm::OP_MIN) as usize].1 != 0xFD {
                bail!("line {line}: opcode {opcode:02X} cannot carry an A1 prefix");
            }
            output.push(opcode);
            if inverted {
                output.push(vm::OP_POP);
            }
            word(&mut output, parse_word(args[1], line, "record offset")?);
            word(&mut output, parse_word(args[2], line, "value")?);
        }
        "GLOBAL_WORD_COMPARE" => {
            require_count(args, 3, line, name)?;
            output.push(vm::OP_GLOBAL_WORD_COMPARE);
            output.push(parse_byte(args[0], line, "operator")?);
            output.push(parse_byte(args[1], line, "tag")?);
            word(&mut output, parse_word(args[2], line, "value")?);
        }
        "GLOBAL_PAIR_COMPARE" => {
            require_count(args, 3, line, name)?;
            output.push(vm::OP_GLOBAL_PAIR_COMPARE);
            output.push(parse_byte(args[0], line, "operator")?);
            word(&mut output, parse_word(args[1], line, "packed value")?);
            word(&mut output, parse_word(args[2], line, "reserved")?);
        }
        "PAIR_RECORD" => {
            require_count(args, 4, line, name)?;
            output.push(parse_byte(args[0], line, "opcode")?);
            word(&mut output, parse_word(args[1], line, "record")?);
            word(&mut output, parse_word(args[2], line, "first word")?);
            word(&mut output, parse_word(args[3], line, "second word")?);
        }
        "RECORD_TRIPLE" => {
            require_count(args, 4, line, name)?;
            output.push(vm::OP_RECORD_TRIPLE);
            if parse_bool(args[3], line, "inverted")? {
                output.push(0xA1);
            }
            word(&mut output, parse_word(args[0], line, "record")?);
            word(&mut output, parse_word(args[1], line, "first word")?);
            word(&mut output, parse_word(args[2], line, "second word")?);
        }
        "RUN_PROFILE" => {
            require_count(args, 1, line, name)?;
            output.push(vm::OP_SCRIPT_PROFILE_REQUEST);
            output.push(parse_byte(args[0], line, "profile operand")?);
        }
        _ => bail!("line {line}: unknown statement {name:?}"),
    }
    Ok(output)
}

fn option_word(value: Option<u16>) -> String {
    value.map_or_else(|| "-".to_string(), |word| format!("{word:04X}"))
}

fn bool_digit(value: bool) -> u8 {
    u8::from(value)
}

fn parse_hex_usize(value: &str, line: usize, field: &str) -> Result<usize> {
    usize::from_str_radix(value, 16)
        .map_err(|_| anyhow!("line {line}: invalid hexadecimal {field} {value:?}"))
}

fn parse_word(value: &str, line: usize, field: &str) -> Result<u16> {
    u16::from_str_radix(value, 16)
        .map_err(|_| anyhow!("line {line}: invalid hexadecimal {field} {value:?}"))
}

fn parse_byte(value: &str, line: usize, field: &str) -> Result<u8> {
    if value.len() != 2 {
        bail!("line {line}: {field} {value:?} must have two hex digits");
    }
    u8::from_str_radix(value, 16)
        .map_err(|_| anyhow!("line {line}: invalid hexadecimal {field} {value:?}"))
}

fn parse_simple_ascii<'a>(value: &'a str, line: usize, field: &str) -> Result<&'a str> {
    let Some(value) = value.strip_prefix('"').and_then(|value| value.strip_suffix('"')) else {
        bail!("line {line}: {field} must be a quoted ASCII atom");
    };
    if !value
        .as_bytes()
        .iter()
        .all(|byte| byte.is_ascii_graphic() && !matches!(*byte, b'"' | b'\\'))
    {
        bail!("line {line}: {field} must contain unescaped printable ASCII without spaces");
    }
    Ok(value)
}

fn parse_optional_word(value: &str, line: usize, field: &str) -> Result<Option<u16>> {
    if value == "-" {
        Ok(None)
    } else {
        parse_word(value, line, field).map(Some)
    }
}

fn parse_bool(value: &str, line: usize, field: &str) -> Result<bool> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => bail!("line {line}: {field} must be 0 or 1"),
    }
}

fn parse_byte_list(values: &[&str], line: usize, field: &str) -> Result<Vec<u8>> {
    require_min_count(values, 1, line, field)?;
    values
        .iter()
        .map(|value| parse_byte(value, line, field))
        .collect()
}

fn require_count(values: &[&str], expected: usize, line: usize, name: &str) -> Result<()> {
    if values.len() != expected {
        bail!(
            "line {line}: {name} expects {expected} argument(s), got {}",
            values.len()
        );
    }
    Ok(())
}

fn require_min_count(values: &[&str], minimum: usize, line: usize, name: &str) -> Result<()> {
    if values.len() < minimum {
        bail!(
            "line {line}: {name} expects at least {minimum} argument(s), got {}",
            values.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn game_dir() -> Option<PathBuf> {
        [
            "accuracy/cblood_install/cblood",
            "../accuracy/cblood_install/cblood",
        ]
        .iter()
        .map(Path::new)
        .find(|path| path.join("SCRIPT1.COD").exists())
        .map(Path::to_path_buf)
    }

    #[test]
    fn typed_text_flags_must_match_optional_operands() {
        let source = "; format: bloodscript-ir-v1\n00000000: TEXT 0001 FF 00 80 1234 -\n";
        assert!(compile(source).is_err());
    }

    #[test]
    fn shared_state_families_compile_exact_bytes() {
        let source = concat!(
            "; format: bloodscript-ir-v1\n",
            "00000000: SHARED_STATE C0 1234 F6 C2 5678\n",
            "00000007: SHARED_BIT_STATE AE 2345 00FF 1\n",
            "0000000D: RECORD_WILDCARD AF 4567 FFFF 1\n",
            "00000013: END\n",
        );
        let expected = vec![
            0xC0, 0x34, 0x12, 0xF6, 0xC2, 0x78, 0x56, 0xAE, 0xA1, 0x45, 0x23, 0xFF, 0x00, 0xAF,
            0xA1, 0x67, 0x45, 0xFF, 0xFF, 0xFF,
        ];
        assert_eq!(compile(source).unwrap(), expected);

        let decompiled = decompile(ImageKind::Cod, &expected, &HashMap::new()).unwrap();
        assert_eq!(decompiled.generic_op_statements, 0);
        assert!(
            decompiled
                .source
                .contains("SHARED_STATE C0 1234 F6 C2 5678")
        );
        assert!(
            decompiled
                .source
                .contains("SHARED_BIT_STATE AE 2345 00FF 1")
        );
        assert!(decompiled.source.contains("RECORD_WILDCARD AF 4567 FFFF 1"));
        assert_eq!(compile(&decompiled.source).unwrap(), expected);
    }

    #[test]
    fn record_wildcard_rejects_prefix_for_fixed_length_opcode() {
        let source = "; format: bloodscript-ir-v1\n00000000: RECORD_WILDCARD AD 4567 FFFF 1\n";
        assert!(compile(source).is_err());
    }

    #[test]
    fn control_flow_families_compile_exact_bytes() {
        let source = concat!(
            "; format: bloodscript-ir-v1\n",
            "00000000: GUARD_PUSH 1234\n",
            "00000003: STATE_ARRAY_TEST FE\n",
            "00000005: GUARD_POP\n",
            "00000006: STATE_ARRAY_SET 02 5678\n",
            "0000000A: JUMP 9ABC\n",
            "0000000D: CONDITIONAL_BLOCK 01 DEF0\n",
            "00000011: BRANCH_PRESENTATION\n",
            "00000012: BRANCH_GAMEFLAG\n",
            "00000013: GUARD_POP\n",
            "00000014: END\n",
        );
        let expected = vec![
            0xA0, 0x34, 0x12, 0xA5, 0xFE, 0xA1, 0xA5, 0x02, 0x78, 0x56, 0xA4, 0xBC,
            0x9A, 0xA9, 0x01, 0xF0, 0xDE, 0xCE, 0xD0, 0xA1, 0xFF,
        ];
        assert_eq!(compile(source).unwrap(), expected);

        let decompiled = decompile(ImageKind::Cod, &expected, &HashMap::new()).unwrap();
        assert_eq!(decompiled.generic_op_statements, 0);
        for statement in [
            "GUARD_PUSH 1234",
            "STATE_ARRAY_TEST FE",
            "GUARD_POP",
            "STATE_ARRAY_SET 02 5678",
            "JUMP 9ABC",
            "CONDITIONAL_BLOCK 01 DEF0",
            "BRANCH_PRESENTATION",
            "BRANCH_GAMEFLAG",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert_eq!(compile(&decompiled.source).unwrap(), expected);
    }

    #[test]
    fn residual_opcode_families_compile_exact_bytes() {
        let source = concat!(
            "; format: bloodscript-ir-v1\n",
            "00000000: GUARD_PUSH 1234\n",
            "00000003: CONCEPT_GUARD 0D26 0\n",
            "00000006: CONCEPT_GUARD 0EE8 1\n",
            "0000000A: GUARD_POP\n",
            "0000000B: LOAD_STRING \"fin.hnm\"\n",
            "00000015: POKE_BYTE 1234 56\n",
            "00000019: CHARACTER_SLOT 02 \"scrut\"\n",
            "00000022: CLEAR_ALTERNATE_CONCEPT\n",
            "00000023: BRANCH_FLAG_274F\n",
            "00000024: END\n",
        );
        let expected = vec![
            0xA0, 0x34, 0x12, 0xA3, 0x26, 0x0D, 0xA3, 0xA1, 0xE8, 0x0E, 0xA1, 0xA8,
            b'f', b'i', b'n', b'.', b'h', b'n', b'm', 0, 0, 0xAB, 0x56, 0x34, 0x12, 0xCC,
            0x02, b's', b'c', b'r', b'u', b't', 0, 0, 0xCF, 0xD1, 0xFF,
        ];
        assert_eq!(compile(source).unwrap(), expected);

        let decompiled = decompile(ImageKind::Cod, &expected, &HashMap::new()).unwrap();
        assert_eq!(decompiled.generic_op_statements, 0);
        for statement in [
            "CONCEPT_GUARD 0D26 0",
            "CONCEPT_GUARD 0EE8 1",
            "LOAD_STRING \"fin.hnm\"",
            "POKE_BYTE 1234 56",
            "CHARACTER_SLOT 02 \"scrut\"",
            "CLEAR_ALTERNATE_CONCEPT",
            "BRANCH_FLAG_274F",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert_eq!(compile(&decompiled.source).unwrap(), expected);
    }

    #[test]
    fn bas_structural_families_compile_exact_bytes() {
        let source = concat!(
            "; format: bloodscript-ir-v1\n",
            "00000000: YIELD\n",
            "00000001: BAS_BLOCK_END 1234 0006\n",
            "00000006: MENU 1234\n",
            "0000000B: PRESENTATION_REGISTER 9ABC\n",
            "0000000E: END\n",
        );
        let expected = vec![
            0xAA, 0xAC, 0x34, 0x12, 0x06, 0x00, 0xA3, 0x34, 0x12, 0x00, 0x00, 0xA7,
            0xBC, 0x9A, 0xFF,
        ];
        assert_eq!(compile(source).unwrap(), expected);

        let dictionary = HashMap::from([(0x1234, "topic".to_string())]);
        let decompiled = decompile(ImageKind::Bas, &expected, &dictionary).unwrap();
        assert_eq!(decompiled.raw_bytes, 0);
        for statement in [
            "YIELD",
            "BAS_BLOCK_END 1234 0006",
            "MENU 1234",
            "PRESENTATION_REGISTER 9ABC",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert_eq!(compile(&decompiled.source).unwrap(), expected);
    }

    #[test]
    fn every_shipped_vm_image_round_trips_through_bloodscript() {
        let Some(root) = game_dir() else { return };
        for script in 1..=5 {
            let dic_raw = std::fs::read(root.join(format!("SCRIPT{script}.DIC"))).unwrap();
            let dictionary = crate::script::parse_dictionary(&dic_raw);
            for (extension, kind) in [("COD", ImageKind::Cod), ("BAS", ImageKind::Bas)] {
                let image =
                    std::fs::read(root.join(format!("SCRIPT{script}.{extension}"))).unwrap();
                let source = decompile(kind, &image, &dictionary).unwrap();
                let rebuilt = compile(&source.source).unwrap();
                assert_eq!(
                    source.generic_op_bytes, 0,
                    "SCRIPT{script}.{extension} must have no generic opcodes"
                );
                assert_eq!(
                    source.raw_bytes, 0,
                    "SCRIPT{script}.{extension} must have no raw spans"
                );
                assert_eq!(
                    rebuilt, image,
                    "SCRIPT{script}.{extension} must rebuild byte-exactly"
                );
            }
        }
    }
}
