//! Lossless textual form for Commander Blood's two VM program images.
//!
//! Each script profile contains a main `.COD` program and a conversation/menu
//! `.BAS` program. `CBVM-ASM` is deliberately lower-level than the eventual
//! reconstructed source language: every source line owns an explicit byte range,
//! so assembling a disassembly must reproduce the input byte for byte. Semantic
//! comments are evidence only and cannot change the assembled result.

use std::collections::HashMap;
use std::fmt::Write as _;

use anyhow::{Result, anyhow, bail};

use crate::vm::{self, VmToken};

const SOURCE_FORMAT: &str = "cbvm-asm-v1";
const BYTES_PER_LINE: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageKind {
    Cod,
    Bas,
}

impl ImageKind {
    pub fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "cod" => Ok(Self::Cod),
            "bas" => Ok(Self::Bas),
            _ => bail!("unknown VM image kind {value:?}; expected cod or bas"),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Cod => "COD",
            Self::Bas => "BAS",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Disassembly {
    pub source: String,
    pub semantic_spans: usize,
    pub semantic_bytes: usize,
    pub raw_bytes: usize,
}

/// Disassemble one complete VM image into the parseable CBVM-ASM format.
///
/// COD uses the recovered opcode descriptor grammar. BAS is not forced through
/// that grammar: only independently recognizable menu tables and text records
/// receive semantic comments, and every other byte remains an explicit raw span.
pub fn disassemble(
    kind: ImageKind,
    image: &[u8],
    dictionary: &HashMap<u16, String>,
) -> Result<Disassembly> {
    let mut output = String::new();
    writeln!(output, "; Commander Blood VM assembly")?;
    writeln!(output, "; format: {SOURCE_FORMAT}")?;
    writeln!(output, "; image: {}", kind.label())?;
    writeln!(output, "; size: 0x{:08X}", image.len())?;
    writeln!(
        output,
        "; operands are authoritative; comments are non-semantic"
    )?;
    writeln!(output)?;

    let (semantic_spans, semantic_bytes, raw_bytes) = match kind {
        ImageKind::Cod => disassemble_cod(&mut output, image, dictionary)?,
        ImageKind::Bas => disassemble_bas(&mut output, image, dictionary)?,
    };
    Ok(Disassembly {
        source: output,
        semantic_spans,
        semantic_bytes,
        raw_bytes,
    })
}

/// Assemble CBVM-ASM back into its exact byte image.
pub fn assemble(source: &str) -> Result<Vec<u8>> {
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
        let (offset_text, directive) = code
            .split_once(':')
            .ok_or_else(|| anyhow!("line {line_number}: expected OFFSET: .byte ..."))?;
        let offset = usize::from_str_radix(offset_text.trim(), 16)
            .map_err(|_| anyhow!("line {line_number}: invalid hexadecimal offset"))?;
        if offset != image.len() {
            bail!(
                "line {line_number}: offset 0x{offset:08X} does not follow 0x{:08X}",
                image.len()
            );
        }

        let byte_text = directive
            .trim()
            .strip_prefix(".byte")
            .ok_or_else(|| anyhow!("line {line_number}: expected .byte directive"))?
            .trim();
        if byte_text.is_empty() {
            bail!("line {line_number}: .byte requires at least one byte");
        }
        for token in byte_text.split_whitespace() {
            if token.len() != 2 {
                bail!("line {line_number}: byte {token:?} must have two hex digits");
            }
            image.push(
                u8::from_str_radix(token, 16)
                    .map_err(|_| anyhow!("line {line_number}: invalid byte {token:?}"))?,
            );
        }
    }

    if !saw_format {
        bail!("missing '; format: {SOURCE_FORMAT}' header");
    }
    Ok(image)
}

fn disassemble_cod(
    output: &mut String,
    image: &[u8],
    dictionary: &HashMap<u16, String>,
) -> Result<(usize, usize, usize)> {
    let tokens = vm::walk(image, 0, image.len());
    let mut cursor = 0usize;
    let mut semantic_spans = 0usize;
    let mut semantic_bytes = 0usize;
    let mut raw_bytes = 0usize;

    for token in &tokens {
        let offset = token.offset();
        if offset < cursor || offset > image.len() {
            bail!("COD token offset 0x{offset:08X} is outside the linear stream");
        }
        if offset > cursor {
            emit_span(output, cursor, &image[cursor..offset], "RAW undecoded gap")?;
            raw_bytes += offset - cursor;
        }

        let Some(encoded) = vm::encode_token(token) else {
            emit_span(output, offset, &image[offset..], "RAW after invalid token")?;
            raw_bytes += image.len() - offset;
            cursor = image.len();
            break;
        };
        let end = offset
            .checked_add(encoded.len())
            .ok_or_else(|| anyhow!("COD token at 0x{offset:08X} overflows"))?;
        if image.get(offset..end) != Some(encoded.as_slice()) {
            bail!("COD token at 0x{offset:08X} does not re-encode exactly");
        }
        emit_span(output, offset, &encoded, &token_comment(token, dictionary))?;
        semantic_spans += 1;
        semantic_bytes += encoded.len();
        cursor = end;
    }

    if cursor < image.len() {
        if image[cursor] == 0xFF {
            emit_span(output, cursor, &image[cursor..cursor + 1], "END")?;
            semantic_spans += 1;
            semantic_bytes += 1;
            cursor += 1;
        }
        if cursor < image.len() {
            emit_span(output, cursor, &image[cursor..], "RAW trailing bytes")?;
            raw_bytes += image.len() - cursor;
        }
    }
    Ok((semantic_spans, semantic_bytes, raw_bytes))
}

fn disassemble_bas(
    output: &mut String,
    image: &[u8],
    dictionary: &HashMap<u16, String>,
) -> Result<(usize, usize, usize)> {
    let mut cursor = 0usize;
    let mut raw_start = 0usize;
    let mut semantic_spans = 0usize;
    let mut semantic_bytes = 0usize;
    let mut raw_bytes = 0usize;

    while cursor < image.len() {
        let recognized = bas_menu_at(image, cursor, dictionary)
            .map(|(end, labels)| (end, format!("MENU {}", labels.join(" | "))))
            .or_else(|| {
                bas_text_at(image, cursor, dictionary)
                    .map(|(end, token)| (end, token_comment(&token, dictionary)))
            });

        if let Some((end, comment)) = recognized {
            if raw_start < cursor {
                emit_span(
                    output,
                    raw_start,
                    &image[raw_start..cursor],
                    "RAW BAS structure",
                )?;
                raw_bytes += cursor - raw_start;
            }
            emit_span(output, cursor, &image[cursor..end], &comment)?;
            semantic_spans += 1;
            semantic_bytes += end - cursor;
            cursor = end;
            raw_start = cursor;
        } else {
            cursor += 1;
        }
    }
    if raw_start < image.len() {
        emit_span(output, raw_start, &image[raw_start..], "RAW BAS structure")?;
        raw_bytes += image.len() - raw_start;
    }
    Ok((semantic_spans, semantic_bytes, raw_bytes))
}

fn bas_menu_at(
    image: &[u8],
    offset: usize,
    dictionary: &HashMap<u16, String>,
) -> Option<(usize, Vec<String>)> {
    if image.get(offset) != Some(&0xA3) {
        return None;
    }
    let mut cursor = offset + 1;
    let mut labels = Vec::new();
    while cursor + 1 < image.len() {
        let word = u16::from_le_bytes([image[cursor], image[cursor + 1]]);
        cursor += 2;
        if word == 0 {
            return (labels.len() >= 2).then_some((cursor, labels));
        }
        let label = dictionary.get(&word)?;
        if !(2..=16).contains(&label.len()) || label.contains(' ') {
            return None;
        }
        labels.push(label.clone());
        if labels.len() > 128 {
            return None;
        }
    }
    None
}

fn bas_text_at(
    image: &[u8],
    offset: usize,
    dictionary: &HashMap<u16, String>,
) -> Option<(usize, VmToken)> {
    if image.get(offset) != Some(&vm::OP_TEXT) {
        return None;
    }
    let token = vm::walk(image, offset, image.len()).into_iter().next()?;
    let VmToken::Text {
        flags_b5,
        word_offsets,
        ..
    } = &token
    else {
        return None;
    };
    if flags_b5 & 0x80 == 0
        || !word_offsets
            .iter()
            .all(|word| *word == 0xFFFF || dictionary.contains_key(word))
    {
        return None;
    }
    let encoded = vm::encode_token(&token)?;
    let end = offset.checked_add(encoded.len())?;
    (image.get(offset..end) == Some(encoded.as_slice())).then_some((end, token))
}

fn token_comment(token: &VmToken, dictionary: &HashMap<u16, String>) -> String {
    match token {
        VmToken::Text {
            line_index,
            voice_selector,
            flags_b4,
            flags_b5,
            word_offsets,
            ..
        } => {
            let spoken = word_offsets
                .iter()
                .take_while(|word| **word != 0xFFFF)
                .filter_map(|word| dictionary.get(word))
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "TEXT line=0x{line_index:04X} voice=0x{voice_selector:02X} flags=0x{flags_b4:02X},0x{flags_b5:02X} {:?}",
                spoken
            )
        }
        VmToken::Actor {
            record_offset,
            related_record_offset,
            inverted,
            ..
        } => format!(
            "ACTOR record=0x{record_offset:04X} related=0x{related_record_offset:04X} inverted={inverted}"
        ),
        VmToken::RecordLink {
            record_offset,
            related_record_offset,
            inverted,
            ..
        } => format!(
            "RECORD_LINK record=0x{record_offset:04X} related=0x{related_record_offset:04X} inverted={inverted}"
        ),
        VmToken::RecordEntry {
            entry_opcode,
            record_offset,
            operand,
            inverted,
            ..
        } => format!(
            "OP_{entry_opcode:02X} RECORD_ENTRY record=0x{record_offset:04X} operand=0x{operand:04X} inverted={inverted}"
        ),
        VmToken::RecordClear { record_offset, .. } => {
            format!("RECORD_CLEAR record=0x{record_offset:04X}")
        }
        VmToken::BitFlag {
            flag_offset,
            bit_index,
            clear,
            ..
        } => format!("BIT_FLAG record=0x{flag_offset:04X} bit={bit_index} clear={clear}"),
        VmToken::RecordState {
            opcode,
            record_offset,
            operand,
            inverted,
            ..
        } => format!(
            "OP_{opcode:02X} RECORD_STATE record=0x{record_offset:04X} operand=0x{operand:04X} inverted={inverted}"
        ),
        VmToken::GlobalWordCompare {
            operator,
            tag,
            value,
            ..
        } => format!(
            "GLOBAL_WORD_COMPARE operator=0x{operator:02X} tag=0x{tag:02X} value=0x{value:04X}"
        ),
        VmToken::GlobalPairCompare {
            operator,
            packed_value,
            reserved,
            ..
        } => format!(
            "GLOBAL_PAIR_COMPARE operator=0x{operator:02X} value=0x{packed_value:04X} reserved=0x{reserved:04X}"
        ),
        VmToken::PairRecord {
            opcode,
            record_offset,
            first_word,
            second_word,
            ..
        } => format!(
            "OP_{opcode:02X} PAIR_RECORD record=0x{record_offset:04X} first=0x{first_word:04X} second=0x{second_word:04X}"
        ),
        VmToken::RecordTriple {
            record_offset,
            first_word,
            second_word,
            inverted,
            ..
        } => format!(
            "RECORD_TRIPLE record=0x{record_offset:04X} first=0x{first_word:04X} second=0x{second_word:04X} inverted={inverted}"
        ),
        VmToken::ScriptProfileRequest {
            operand,
            profile_index,
            ..
        } => format!("RUN_PROFILE operand=0x{operand:02X} profile=0x{profile_index:04X}"),
        VmToken::Op {
            opcode, operands, ..
        } => {
            let suffix = operands
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            format!("OP_{opcode:02X} {suffix}").trim_end().to_string()
        }
        VmToken::Invalid { byte, .. } => format!("INVALID 0x{byte:02X}"),
    }
}

fn emit_span(output: &mut String, offset: usize, bytes: &[u8], comment: &str) -> Result<()> {
    for (chunk_index, chunk) in bytes.chunks(BYTES_PER_LINE).enumerate() {
        let at = offset + chunk_index * BYTES_PER_LINE;
        write!(output, "{at:08X}: .byte")?;
        for byte in chunk {
            write!(output, " {byte:02X}")?;
        }
        if chunk_index == 0 {
            writeln!(output, " ; {comment}")?;
        } else {
            writeln!(output, " ; continuation")?;
        }
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
    fn assembler_rejects_non_contiguous_offsets() {
        let source = "; format: cbvm-asm-v1\n00000001: .byte AA\n";
        assert!(assemble(source).is_err());
    }

    #[test]
    fn every_shipped_vm_image_round_trips_through_text() {
        let Some(root) = game_dir() else { return };
        for script in 1..=5 {
            let dic_raw = std::fs::read(root.join(format!("SCRIPT{script}.DIC"))).unwrap();
            let dictionary = crate::script::parse_dictionary(&dic_raw);
            for (extension, kind) in [("COD", ImageKind::Cod), ("BAS", ImageKind::Bas)] {
                let image =
                    std::fs::read(root.join(format!("SCRIPT{script}.{extension}"))).unwrap();
                let listing = disassemble(kind, &image, &dictionary).unwrap();
                let rebuilt = assemble(&listing.source).unwrap();
                assert_eq!(
                    rebuilt, image,
                    "SCRIPT{script}.{extension} must rebuild byte-exactly"
                );
            }
        }
    }
}
