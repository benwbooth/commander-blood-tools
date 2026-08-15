//! Lossless source and compiler for Commander Blood VM data companions.

use anyhow::{Context, Result, bail};

const DEB_RECORD_BYTES: usize = 20;
const DEB_NAME_BYTES: usize = 16;
const VAR_WORDS_PER_LINE: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataKind {
    Deb,
    Dic,
    Var,
}

impl DataKind {
    pub fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "deb" => Ok(Self::Deb),
            "dic" => Ok(Self::Dic),
            "var" => Ok(Self::Var),
            _ => bail!("unknown VM data kind {value:?}; expected deb, dic, or var"),
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Deb => "DEB",
            Self::Dic => "DIC",
            Self::Var => "VAR",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decompilation {
    pub source: String,
    pub statements: usize,
    pub bytes: usize,
}

pub fn decompile(kind: DataKind, data: &[u8]) -> Result<Decompilation> {
    let mut source = format!("; format: blooddata-v1\n; image: {}\n", kind.extension());
    let statements = match kind {
        DataKind::Deb => decompile_deb(data, &mut source)?,
        DataKind::Dic => decompile_dic(data, &mut source),
        DataKind::Var => decompile_var(data, &mut source),
    };
    Ok(Decompilation {
        source,
        statements,
        bytes: data.len(),
    })
}

pub fn compile(kind: DataKind, source: &str) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut statements = 0usize;
    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line
            .split_once(';')
            .map_or(raw_line, |(body, _)| body)
            .trim();
        if line.is_empty() {
            continue;
        }
        let (offset_text, statement) = line
            .split_once(':')
            .with_context(|| format!("line {line_number}: expected offset and statement"))?;
        let offset = parse_offset(offset_text.trim(), line_number)?;
        if offset != output.len() {
            bail!(
                "line {line_number}: offset 0x{offset:08X} does not match output offset 0x{:08X}",
                output.len()
            );
        }
        let fields: Vec<_> = statement.split_whitespace().collect();
        let Some((directive, arguments)) = fields.split_first() else {
            bail!("line {line_number}: missing statement after offset");
        };
        match kind {
            DataKind::Deb => compile_deb(arguments, &mut output, line_number, directive)?,
            DataKind::Dic => compile_dic(arguments, &mut output, line_number, directive)?,
            DataKind::Var => compile_var(arguments, &mut output, line_number, directive)?,
        }
        statements += 1;
    }
    if statements == 0 {
        bail!("BloodData source contains no statements");
    }
    Ok(output)
}

fn decompile_deb(data: &[u8], source: &mut String) -> Result<usize> {
    if data.len() % DEB_RECORD_BYTES != 0 {
        bail!(
            "DEB image is {} bytes, not a multiple of {DEB_RECORD_BYTES}",
            data.len()
        );
    }
    for (record_index, record) in data.chunks_exact(DEB_RECORD_BYTES).enumerate() {
        let offset = record_index * DEB_RECORD_BYTES;
        let name = &record[..DEB_NAME_BYTES];
        let value = u16::from_le_bytes([record[16], record[17]]);
        let kind = u16::from_le_bytes([record[18], record[19]]);
        source.push_str(&format!(
            "{offset:08X}: SYMBOL {} {value:04X} {kind:04X}{}\n",
            encode_hex(name),
            display_comment(name)
        ));
    }
    Ok(data.len() / DEB_RECORD_BYTES)
}

fn decompile_dic(data: &[u8], source: &mut String) -> usize {
    let mut offset = 0usize;
    let mut statements = 0usize;
    while offset < data.len() {
        let start = offset;
        while offset < data.len() && data[offset] != 0 {
            offset += 1;
        }
        let payload = &data[start..offset];
        if offset < data.len() {
            source.push_str(&format!(
                "{start:08X}: STRING {}{}\n",
                encode_hex_or_dash(payload),
                display_comment(payload)
            ));
            offset += 1;
        } else {
            source.push_str(&format!(
                "{start:08X}: TAIL {}{}\n",
                encode_hex_or_dash(payload),
                display_comment(payload)
            ));
        }
        statements += 1;
    }
    statements
}

fn decompile_var(data: &[u8], source: &mut String) -> usize {
    let even_bytes = data.len() & !1;
    let mut statements = 0usize;
    for offset in (0..even_bytes).step_by(VAR_WORDS_PER_LINE * 2) {
        let end = (offset + VAR_WORDS_PER_LINE * 2).min(even_bytes);
        let words = data[offset..end]
            .chunks_exact(2)
            .map(|word| format!("{:04X}", u16::from_le_bytes([word[0], word[1]])))
            .collect::<Vec<_>>()
            .join(" ");
        source.push_str(&format!("{offset:08X}: WORDS {words}\n"));
        statements += 1;
    }
    if data.len() != even_bytes {
        source.push_str(&format!(
            "{even_bytes:08X}: BYTES {:02X}\n",
            data[even_bytes]
        ));
        statements += 1;
    }
    statements
}

fn compile_deb(
    arguments: &[&str],
    output: &mut Vec<u8>,
    line: usize,
    directive: &str,
) -> Result<()> {
    if !directive.eq_ignore_ascii_case("SYMBOL") {
        bail!("line {line}: DEB source requires SYMBOL statements");
    }
    if arguments.len() != 3 {
        bail!("line {line}: SYMBOL requires name-bytes, value, and kind");
    }
    let name = decode_hex(arguments[0], line)?;
    if name.len() != DEB_NAME_BYTES {
        bail!(
            "line {line}: SYMBOL name field is {} bytes, expected {DEB_NAME_BYTES}",
            name.len()
        );
    }
    let value = parse_word(arguments[1], line, "symbol value")?;
    let kind = parse_word(arguments[2], line, "symbol kind")?;
    output.extend_from_slice(&name);
    output.extend_from_slice(&value.to_le_bytes());
    output.extend_from_slice(&kind.to_le_bytes());
    Ok(())
}

fn compile_dic(
    arguments: &[&str],
    output: &mut Vec<u8>,
    line: usize,
    directive: &str,
) -> Result<()> {
    if arguments.len() != 1 {
        bail!("line {line}: {directive} requires one byte string or '-' for empty");
    }
    let payload = decode_hex_or_dash(arguments[0], line)?;
    output.extend_from_slice(&payload);
    if directive.eq_ignore_ascii_case("STRING") {
        output.push(0);
    } else if !directive.eq_ignore_ascii_case("TAIL") {
        bail!("line {line}: DIC source requires STRING or TAIL statements");
    }
    Ok(())
}

fn compile_var(
    arguments: &[&str],
    output: &mut Vec<u8>,
    line: usize,
    directive: &str,
) -> Result<()> {
    if directive.eq_ignore_ascii_case("WORDS") {
        if arguments.is_empty() {
            bail!("line {line}: WORDS requires at least one value");
        }
        for value in arguments {
            output.extend_from_slice(&parse_word(value, line, "VAR word")?.to_le_bytes());
        }
    } else if directive.eq_ignore_ascii_case("BYTES") {
        if arguments.len() != 1 {
            bail!("line {line}: BYTES requires one hexadecimal byte string");
        }
        output.extend_from_slice(&decode_hex(arguments[0], line)?);
    } else {
        bail!("line {line}: VAR source requires WORDS or BYTES statements");
    }
    Ok(())
}

fn parse_offset(value: &str, line: usize) -> Result<usize> {
    usize::from_str_radix(value, 16)
        .with_context(|| format!("line {line}: invalid hexadecimal offset {value:?}"))
}

fn parse_word(value: &str, line: usize, field: &str) -> Result<u16> {
    u16::from_str_radix(value, 16)
        .with_context(|| format!("line {line}: invalid hexadecimal {field} {value:?}"))
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02X}")).collect()
}

fn encode_hex_or_dash(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        "-".to_string()
    } else {
        encode_hex(bytes)
    }
}

fn decode_hex_or_dash(value: &str, line: usize) -> Result<Vec<u8>> {
    if value == "-" {
        Ok(Vec::new())
    } else {
        decode_hex(value, line)
    }
}

fn decode_hex(value: &str, line: usize) -> Result<Vec<u8>> {
    if value.len() % 2 != 0 {
        bail!("line {line}: hexadecimal byte string has odd length");
    }
    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .with_context(|| format!("line {line}: invalid hexadecimal byte string {value:?}"))
        })
        .collect()
}

fn display_bytes(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    crate::font::cp437_string(&bytes[..end])
        .escape_default()
        .to_string()
}

fn display_comment(bytes: &[u8]) -> String {
    let display = display_bytes(bytes);
    if display.is_empty() {
        String::new()
    } else {
        format!(" ; {display}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(kind: DataKind, bytes: &[u8]) {
        let source = decompile(kind, bytes).unwrap();
        assert_eq!(compile(kind, &source.source).unwrap(), bytes);
    }

    #[test]
    fn deb_preserves_name_padding_and_fields() {
        let mut bytes = [0xA5; DEB_RECORD_BYTES];
        bytes[..4].copy_from_slice(b"test");
        bytes[4] = 0;
        bytes[16..18].copy_from_slice(&0x1234u16.to_le_bytes());
        bytes[18..20].copy_from_slice(&0x0002u16.to_le_bytes());
        round_trip(DataKind::Deb, &bytes);
    }

    #[test]
    fn dictionary_preserves_empty_and_unterminated_entries() {
        round_trip(DataKind::Dic, b"\0hello\0\0tail");
    }

    #[test]
    fn var_preserves_words_and_an_odd_tail() {
        round_trip(DataKind::Var, &[0x34, 0x12, 0xCD, 0xAB, 0xEF]);
    }

    #[test]
    fn compiler_rejects_noncontiguous_offsets() {
        let error = compile(DataKind::Var, "00000002: WORDS 1234\n").unwrap_err();
        assert!(error.to_string().contains("does not match output offset"));
    }
}
