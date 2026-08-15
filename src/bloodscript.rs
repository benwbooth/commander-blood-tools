//! Typed, lossless source language for Commander Blood VM programs.
//!
//! BloodScript IR is the compiler-facing layer above CBVM-ASM. It gives proven
//! token families authoritative typed statements while retaining `OP` and `RAW`
//! escapes for constructs whose high-level structure is not established. The
//! syntax is reconstructed for this project; it is not claimed to be the lost
//! historical source syntax.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;

use anyhow::{Result, anyhow, bail};

use crate::bas_cfg::{BasControlFlow, analyze_bas};
use crate::script::DebSymbol;
use crate::vm::{self, VmToken};
use crate::vm_cfg::{GuardRecovery, GuardRejection, StructuredGuard, analyze_structured_guards};
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
    pub symbolic_labels: usize,
    pub procedures: usize,
    pub structured_guards: usize,
    pub unstructured_guards: usize,
    pub guard_rejection_counts: BTreeMap<String, usize>,
    pub structured_selector_lists: usize,
    pub structured_cases: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct BodyStats {
    typed_statements: usize,
    typed_bytes: usize,
    generic_op_statements: usize,
    generic_op_bytes: usize,
    raw_bytes: usize,
    symbolic_labels: usize,
    procedures: usize,
    structured_guards: usize,
    unstructured_guards: usize,
    guard_rejection_counts: BTreeMap<String, usize>,
    structured_selector_lists: usize,
    structured_cases: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SourceAnnotations {
    directives: BTreeMap<usize, Vec<String>>,
    labels: HashMap<u16, String>,
    procedure_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParsedSourceLine<'a> {
    line_number: usize,
    offset: usize,
    name: &'a str,
    args: Vec<&'a str>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct StructuredAnnotations {
    starts: BTreeMap<usize, StructuredGuard>,
    thens: BTreeMap<usize, usize>,
    ends: BTreeMap<usize, Vec<usize>>,
    rejected: BTreeMap<usize, BTreeSet<GuardRejection>>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct BasStructuredAnnotations {
    starts: BTreeMap<usize, (String, String, u16)>,
    cases: BTreeSet<usize>,
    ends: BTreeMap<usize, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OpenSelectorList<'a> {
    name: &'a str,
    prefix_offset: usize,
    prefix_emitted: bool,
    case_count: usize,
    last_next: u16,
    needs_menu: bool,
    case_terminated: bool,
}

pub fn decompile(
    kind: ImageKind,
    image: &[u8],
    dictionary: &HashMap<u16, String>,
) -> Result<Decompilation> {
    decompile_with_symbols(kind, image, dictionary, &[])
}

pub fn decompile_with_symbols(
    kind: ImageKind,
    image: &[u8],
    dictionary: &HashMap<u16, String>,
    symbols: &[DebSymbol],
) -> Result<Decompilation> {
    decompile_mode(kind, image, dictionary, symbols, false)
}

pub fn decompile_structured_with_symbols(
    kind: ImageKind,
    image: &[u8],
    dictionary: &HashMap<u16, String>,
    symbols: &[DebSymbol],
) -> Result<Decompilation> {
    decompile_mode(kind, image, dictionary, symbols, true)
}

pub fn decompile_structured_bas_with_symbols(
    image: &[u8],
    var: &[u8],
    dictionary: &HashMap<u16, String>,
    symbols: &[DebSymbol],
) -> Result<Decompilation> {
    let graph = analyze_bas("BAS", image, var, dictionary, symbols)?;
    decompile_mode_with_bas_graph(image, dictionary, &graph)
}

fn decompile_mode(
    kind: ImageKind,
    image: &[u8],
    dictionary: &HashMap<u16, String>,
    symbols: &[DebSymbol],
    structured: bool,
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

    let stats = match kind {
        ImageKind::Cod => decompile_cod(&mut source, image, dictionary, symbols, structured)?,
        ImageKind::Bas => decompile_bas(&mut source, image, dictionary, None)?,
    };
    Ok(Decompilation {
        source,
        typed_statements: stats.typed_statements,
        typed_bytes: stats.typed_bytes,
        generic_op_statements: stats.generic_op_statements,
        generic_op_bytes: stats.generic_op_bytes,
        raw_bytes: stats.raw_bytes,
        symbolic_labels: stats.symbolic_labels,
        procedures: stats.procedures,
        structured_guards: stats.structured_guards,
        unstructured_guards: stats.unstructured_guards,
        guard_rejection_counts: stats.guard_rejection_counts,
        structured_selector_lists: stats.structured_selector_lists,
        structured_cases: stats.structured_cases,
    })
}

fn decompile_mode_with_bas_graph(
    image: &[u8],
    dictionary: &HashMap<u16, String>,
    graph: &BasControlFlow,
) -> Result<Decompilation> {
    let mut source = String::new();
    writeln!(source, "; BloodScript typed VM source")?;
    writeln!(source, "; format: {SOURCE_FORMAT}")?;
    writeln!(source, "; image: BAS")?;
    writeln!(source, "; size: 0x{:08X}", image.len())?;
    writeln!(source)?;

    let stats = decompile_bas(&mut source, image, dictionary, Some(graph))?;
    Ok(Decompilation {
        source,
        typed_statements: stats.typed_statements,
        typed_bytes: stats.typed_bytes,
        generic_op_statements: stats.generic_op_statements,
        generic_op_bytes: stats.generic_op_bytes,
        raw_bytes: stats.raw_bytes,
        symbolic_labels: stats.symbolic_labels,
        procedures: stats.procedures,
        structured_guards: stats.structured_guards,
        unstructured_guards: stats.unstructured_guards,
        guard_rejection_counts: stats.guard_rejection_counts,
        structured_selector_lists: stats.structured_selector_lists,
        structured_cases: stats.structured_cases,
    })
}

pub fn compile(source: &str) -> Result<Vec<u8>> {
    let (lines, saw_format) = parse_source_lines(source)?;
    if !saw_format {
        bail!("missing '; format: {SOURCE_FORMAT}' header");
    }

    let mut labels = HashMap::new();
    for line in &lines {
        if !matches!(line.name, "LABEL" | "PROCEDURE") {
            continue;
        }
        require_count(&line.args, 1, line.line_number, line.name)?;
        validate_identifier(line.args[0], line.line_number)?;
        let address = u16::try_from(line.offset).map_err(|_| {
            anyhow!(
                "line {}: label offset 0x{:08X} exceeds the VM address space",
                line.line_number,
                line.offset
            )
        })?;
        if labels.insert(line.args[0], address).is_some() {
            bail!(
                "line {}: duplicate label {:?}",
                line.line_number,
                line.args[0]
            );
        }
    }

    let mut image = Vec::new();
    let mut current_procedure: Option<&str> = None;
    let mut open_whens: Vec<(&str, bool)> = Vec::new();
    let mut open_selector_list: Option<OpenSelectorList<'_>> = None;
    for line in lines {
        if line.offset != image.len() {
            bail!(
                "line {}: offset 0x{:08X} does not follow 0x{:08X}",
                line.line_number,
                line.offset,
                image.len()
            );
        }
        match line.name {
            "LABEL" => {
                require_count(&line.args, 1, line.line_number, line.name)?;
                continue;
            }
            "PROCEDURE" => {
                require_count(&line.args, 1, line.line_number, line.name)?;
                if let Some((target, _)) = open_whens.last() {
                    bail!(
                        "line {}: PROCEDURE reached before END_WHEN {:?}",
                        line.line_number,
                        target
                    );
                }
                if let Some(open) = current_procedure {
                    bail!(
                        "line {}: PROCEDURE {:?} starts before {:?} ends",
                        line.line_number,
                        line.args[0],
                        open
                    );
                }
                current_procedure = Some(line.args[0]);
                continue;
            }
            "END_PROCEDURE" => {
                require_count(&line.args, 1, line.line_number, line.name)?;
                if let Some((target, _)) = open_whens.last() {
                    bail!(
                        "line {}: END_PROCEDURE reached before END_WHEN {:?}",
                        line.line_number,
                        target
                    );
                }
                if current_procedure != Some(line.args[0]) {
                    bail!(
                        "line {}: END_PROCEDURE {:?} does not match {:?}",
                        line.line_number,
                        line.args[0],
                        current_procedure
                    );
                }
                current_procedure = None;
                continue;
            }
            "WHEN" => {
                require_count(&line.args, 1, line.line_number, line.name)?;
                let target =
                    parse_address(line.args[0], &labels, line.line_number, "WHEN target")?;
                if usize::from(target) <= line.offset {
                    bail!("line {}: WHEN target must be forward", line.line_number);
                }
                open_whens.push((line.args[0], false));
            }
            "THEN" => {
                require_count(&line.args, 0, line.line_number, line.name)?;
                let Some((_, saw_then)) = open_whens.last_mut() else {
                    bail!("line {}: THEN without WHEN", line.line_number);
                };
                if *saw_then {
                    bail!("line {}: duplicate THEN", line.line_number);
                }
                *saw_then = true;
            }
            "END_WHEN" => {
                require_count(&line.args, 1, line.line_number, line.name)?;
                let Some((target, saw_then)) = open_whens.pop() else {
                    bail!("line {}: END_WHEN without WHEN", line.line_number);
                };
                if target != line.args[0] {
                    bail!(
                        "line {}: END_WHEN {:?} does not match {:?}",
                        line.line_number,
                        line.args[0],
                        target
                    );
                }
                if !saw_then {
                    bail!("line {}: END_WHEN reached before THEN", line.line_number);
                }
                let target_offset =
                    parse_address(target, &labels, line.line_number, "END_WHEN target")?;
                if usize::from(target_offset) != line.offset {
                    bail!(
                        "line {}: END_WHEN target {:?} resolves to 0x{:04X}, not 0x{:04X}",
                        line.line_number,
                        target,
                        target_offset,
                        line.offset
                    );
                }
                continue;
            }
            "SELECTOR_LIST" => {
                require_count(&line.args, 1, line.line_number, line.name)?;
                validate_identifier(line.args[0], line.line_number)?;
                if let Some(open) = &open_selector_list {
                    bail!(
                        "line {}: SELECTOR_LIST {:?} starts before {:?} ends",
                        line.line_number,
                        line.args[0],
                        open.name
                    );
                }
                if current_procedure.is_some() || !open_whens.is_empty() {
                    bail!(
                        "line {}: SELECTOR_LIST cannot nest in COD structure",
                        line.line_number
                    );
                }
                open_selector_list = Some(OpenSelectorList {
                    name: line.args[0],
                    prefix_offset: line.offset,
                    prefix_emitted: false,
                    case_count: 0,
                    last_next: 0,
                    needs_menu: false,
                    case_terminated: false,
                });
                continue;
            }
            "END_SELECTOR_LIST" => {
                require_count(&line.args, 1, line.line_number, line.name)?;
                let Some(open) = open_selector_list.take() else {
                    bail!(
                        "line {}: END_SELECTOR_LIST without SELECTOR_LIST",
                        line.line_number
                    );
                };
                if open.name != line.args[0] {
                    bail!(
                        "line {}: END_SELECTOR_LIST {:?} does not match {:?}",
                        line.line_number,
                        line.args[0],
                        open.name
                    );
                }
                if open.case_count == 0 || open.last_next != 0 || !open.case_terminated {
                    bail!(
                        "line {}: selector list {:?} is incomplete",
                        line.line_number,
                        open.name
                    );
                }
                if !image
                    .last()
                    .is_some_and(|byte| matches!(*byte, vm::OP_YIELD_A | 0xFF))
                {
                    bail!(
                        "line {}: selector list {:?} does not end in YIELD or END",
                        line.line_number,
                        open.name
                    );
                }
                continue;
            }
            "CASE" => {
                require_count(&line.args, 2, line.line_number, line.name)?;
                let next = parse_address(
                    line.args[1],
                    &labels,
                    line.line_number,
                    "next selector node",
                )?;
                let Some(open) = open_selector_list.as_mut() else {
                    bail!("line {}: CASE outside SELECTOR_LIST", line.line_number);
                };
                if !open.prefix_emitted || image.last() != Some(&vm::OP_YIELD_B) {
                    bail!(
                        "line {}: CASE is not preceded by YIELD_B",
                        line.line_number
                    );
                }
                if open.case_count == 0 {
                    if line.offset != open.prefix_offset + 1 {
                        bail!(
                            "line {}: first CASE must follow the selector-list prefix",
                            line.line_number
                        );
                    }
                } else {
                    if !open.case_terminated {
                        bail!(
                            "line {}: prior CASE body has no YIELD_B",
                            line.line_number
                        );
                    }
                    if usize::from(open.last_next) != line.offset {
                        bail!(
                            "line {}: prior CASE next target is 0x{:04X}, not 0x{:04X}",
                            line.line_number,
                            open.last_next,
                            line.offset
                        );
                    }
                }
                if next != 0 && usize::from(next) <= line.offset {
                    bail!(
                        "line {}: CASE next target must be forward",
                        line.line_number
                    );
                }
                open.case_count += 1;
                open.last_next = next;
                open.needs_menu = true;
                open.case_terminated = false;
            }
            _ => {}
        }

        if line.name != "CASE" {
            if let Some(open) = open_selector_list.as_ref() {
                if open.case_count == 0 {
                    if line.name != "YIELD_B" || open.prefix_emitted {
                        bail!(
                            "line {}: SELECTOR_LIST must begin with exactly one YIELD_B",
                            line.line_number
                        );
                    }
                } else {
                    if open.case_terminated {
                        bail!(
                            "line {}: byte-emitting statement follows a terminated CASE",
                            line.line_number
                        );
                    }
                    if open.needs_menu && line.name != "MENU" {
                        bail!(
                            "line {}: CASE body must begin with MENU",
                            line.line_number
                        );
                    }
                    match line.name {
                        "YIELD_B" if open.last_next == 0 => bail!(
                            "line {}: terminal CASE must end in YIELD or END",
                            line.line_number
                        ),
                        "YIELD" | "END" if open.last_next != 0 => bail!(
                            "line {}: nonterminal CASE must end in YIELD_B",
                            line.line_number
                        ),
                        "SELECTOR_NODE" => bail!(
                            "line {}: use CASE inside SELECTOR_LIST",
                            line.line_number
                        ),
                        _ => {}
                    }
                }
            }
        }
        let encoded = compile_statement(line.name, &line.args, line.line_number, &labels)?;
        if encoded.is_empty() {
            bail!("line {}: statement emitted no bytes", line.line_number);
        }
        image.extend_from_slice(&encoded);
        if line.name != "CASE" {
            if let Some(open) = open_selector_list.as_mut() {
                if open.case_count == 0 {
                    open.prefix_emitted = true;
                } else {
                    if open.needs_menu {
                        open.needs_menu = false;
                    }
                    if matches!(line.name, "YIELD_B" | "YIELD" | "END") {
                        open.case_terminated = true;
                    }
                }
            }
        }
    }
    if let Some(open) = current_procedure {
        bail!("procedure {open:?} has no END_PROCEDURE");
    }
    if let Some((target, _)) = open_whens.last() {
        bail!("WHEN {target:?} has no END_WHEN");
    }
    if let Some(open) = open_selector_list {
        bail!("SELECTOR_LIST {:?} has no END_SELECTOR_LIST", open.name);
    }
    Ok(image)
}

fn parse_source_lines(source: &str) -> Result<(Vec<ParsedSourceLine<'_>>, bool)> {
    let mut lines = Vec::new();
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
        let mut fields = statement.split_whitespace();
        let name = fields
            .next()
            .ok_or_else(|| anyhow!("line {line_number}: missing statement"))?;
        lines.push(ParsedSourceLine {
            line_number,
            offset,
            name,
            args: fields.collect(),
        });
    }
    Ok((lines, saw_format))
}

fn decompile_cod(
    output: &mut String,
    image: &[u8],
    dictionary: &HashMap<u16, String>,
    symbols: &[DebSymbol],
    structured: bool,
) -> Result<BodyStats> {
    let tokens = vm::walk(image, 0, image.len());
    let annotations = cod_annotations(&tokens, image, symbols)?;
    let structured = if structured {
        structured_annotations(analyze_structured_guards("COD", image, symbols)?)
    } else {
        StructuredAnnotations::default()
    };
    let mut cursor = 0usize;
    let mut stats = BodyStats {
        symbolic_labels: annotations.labels.len(),
        procedures: annotations.procedure_count,
        structured_guards: structured.starts.len(),
        unstructured_guards: structured.rejected.len(),
        guard_rejection_counts: guard_rejection_counts(&structured.rejected),
        ..BodyStats::default()
    };

    for token in tokens {
        let offset = token.offset();
        if offset > cursor {
            emit_raw(output, cursor, &image[cursor..offset], "undecoded gap")?;
            stats.raw_bytes += offset - cursor;
        }
        let Some(encoded) = vm::encode_token(&token) else {
            emit_raw(output, offset, &image[offset..], "invalid token tail")?;
            stats.raw_bytes += image.len() - offset;
            cursor = image.len();
            break;
        };
        let end = offset
            .checked_add(encoded.len())
            .ok_or_else(|| anyhow!("token at 0x{offset:08X} overflows"))?;
        if image.get(offset..end) != Some(encoded.as_slice()) {
            bail!("token at 0x{offset:08X} does not re-encode exactly");
        }
        emit_structured_ends(output, offset, &structured, &annotations.labels)?;
        emit_directives(output, offset, &annotations)?;
        if let Some(region) = structured.starts.get(&offset) {
            writeln!(
                output,
                "{offset:08X}: WHEN {} ; GUARD_PUSH target=0x{:04X}",
                address_operand(region.end as u16, &annotations.labels),
                region.end
            )?;
        } else if structured.thens.contains_key(&offset) {
            writeln!(output, "{offset:08X}: THEN ; GUARD_POP")?;
        } else {
            emit_token(
                output,
                &token,
                dictionary,
                &annotations.labels,
                structured.rejected.get(&offset),
            )?;
        }
        stats.typed_statements += 1;
        stats.typed_bytes += encoded.len();
        if matches!(token, VmToken::Op { .. }) {
            stats.generic_op_statements += 1;
            stats.generic_op_bytes += encoded.len();
        }
        cursor = end;
    }

    if cursor < image.len() && image[cursor] == 0xFF {
        emit_structured_ends(output, cursor, &structured, &annotations.labels)?;
        emit_directives(output, cursor, &annotations)?;
        writeln!(output, "{cursor:08X}: END")?;
        stats.typed_statements += 1;
        stats.typed_bytes += 1;
        cursor += 1;
    }
    if cursor < image.len() {
        emit_raw(output, cursor, &image[cursor..], "trailing bytes")?;
        stats.raw_bytes += image.len() - cursor;
    }
    emit_structured_ends(output, image.len(), &structured, &annotations.labels)?;
    emit_directives(output, image.len(), &annotations)?;
    Ok(stats)
}

fn decompile_bas(
    output: &mut String,
    image: &[u8],
    dictionary: &HashMap<u16, String>,
    graph: Option<&BasControlFlow>,
) -> Result<BodyStats> {
    let annotations = bas_annotations(image, dictionary)?;
    let structured = bas_structured_annotations(graph);
    let mut cursor = 0usize;
    let mut raw_start = 0usize;
    let mut stats = BodyStats {
        symbolic_labels: annotations.labels.len(),
        structured_selector_lists: structured.starts.len(),
        structured_cases: structured.cases.len(),
        ..BodyStats::default()
    };

    while cursor < image.len() {
        if let Some((end, token)) = vm_source::bas_token_at(image, cursor, dictionary) {
            if raw_start < cursor {
                emit_raw(
                    output,
                    raw_start,
                    &image[raw_start..cursor],
                    "BAS structure",
                )?;
                stats.raw_bytes += cursor - raw_start;
            }
            let encoded = token.encode().ok_or_else(|| {
                anyhow!("BAS token at 0x{cursor:08X} cannot be encoded")
            })?;
            if image.get(cursor..end) != Some(encoded.as_slice()) {
                bail!("BAS token at 0x{cursor:08X} does not re-encode exactly");
            }
            emit_bas_structured_boundaries(output, cursor, &structured)?;
            emit_directives(output, cursor, &annotations)?;
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
                    emit_token(output, token, dictionary, &HashMap::new(), None)?;
                }
                vm_source::BasToken::Yield { .. } => {
                    writeln!(output, "{cursor:08X}: YIELD ; opcode AA")?;
                }
                vm_source::BasToken::YieldB { .. } => {
                    writeln!(output, "{cursor:08X}: YIELD_B ; opcode AC")?;
                }
                vm_source::BasToken::SelectorNode { selector, next, .. } => {
                    let statement = if structured.cases.contains(&cursor) {
                        "CASE"
                    } else {
                        "SELECTOR_NODE"
                    };
                    writeln!(
                        output,
                        "{cursor:08X}: {statement} {selector:04X} {} ; {:?}",
                        bas_next_operand(*next, &annotations.labels),
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
            stats.typed_statements += 1;
            stats.typed_bytes += encoded.len();
            cursor = end;
            raw_start = cursor;
            continue;
        }
        cursor += 1;
    }

    if raw_start < image.len() {
        emit_raw(output, raw_start, &image[raw_start..], "BAS structure")?;
        stats.raw_bytes += image.len() - raw_start;
    }
    emit_bas_structured_boundaries(output, image.len(), &structured)?;
    Ok(stats)
}

fn cod_annotations(
    tokens: &[VmToken],
    image: &[u8],
    symbols: &[DebSymbol],
) -> Result<SourceAnnotations> {
    let mut annotations = SourceAnnotations::default();
    let mut boundaries: BTreeSet<usize> = tokens.iter().map(VmToken::offset).collect();
    if image.last() == Some(&0xFF) {
        boundaries.insert(image.len() - 1);
    }

    let mut procedures = BTreeMap::new();
    for symbol in symbols.iter().filter(|symbol| symbol.kind == 2) {
        if symbol.offset == 0xFFFF {
            continue;
        }
        if symbol.offset == 0 {
            bail!(
                "DEB kind-2 symbol {:?} has invalid one-based offset zero",
                symbol.name
            );
        }
        let offset = usize::from(symbol.offset - 1);
        if !boundaries.contains(&offset) {
            bail!(
                "DEB kind-2 symbol {:?} encoded as 0x{:04X} does not resolve to a COD token boundary",
                symbol.name,
                symbol.offset
            );
        }
        if procedures.contains_key(&offset) {
            bail!("multiple DEB kind-2 symbols resolve to COD offset 0x{offset:04X}");
        }
        let identifier = format!("proc_{}_{offset:04X}", identifier_component(&symbol.name));
        procedures.insert(offset, (identifier, symbol));
    }

    let mut prior_procedure: Option<String> = None;
    for (&offset, (identifier, symbol)) in &procedures {
        if let Some(prior) = prior_procedure.replace(identifier.clone()) {
            annotations
                .directives
                .entry(offset)
                .or_default()
                .push(format!("END_PROCEDURE {prior}"));
        }
        annotations
            .directives
            .entry(offset)
            .or_default()
            .push(format!(
                "PROCEDURE {identifier} ; DEB kind 2 {:?}, encoded offset 0x{:04X}",
                symbol.name, symbol.offset
            ));
        annotations.labels.insert(offset as u16, identifier.clone());
        annotations.procedure_count += 1;
    }
    if let Some(prior) = prior_procedure {
        annotations
            .directives
            .entry(image.len())
            .or_default()
            .push(format!("END_PROCEDURE {prior}"));
    }

    let targets: BTreeSet<u16> = tokens.iter().filter_map(cod_target).collect();
    for target in targets {
        if !boundaries.contains(&usize::from(target)) {
            bail!("COD target 0x{target:04X} does not resolve to a token boundary");
        }
        if annotations.labels.contains_key(&target) {
            continue;
        }
        let identifier = format!("block_{target:04X}");
        annotations
            .directives
            .entry(usize::from(target))
            .or_default()
            .push(format!("LABEL {identifier}"));
        annotations.labels.insert(target, identifier);
    }
    Ok(annotations)
}

fn cod_target(token: &VmToken) -> Option<u16> {
    match token {
        VmToken::Text {
            loop_target: Some(target),
            ..
        }
        | VmToken::GuardPush { target, .. }
        | VmToken::Jump { target, .. }
        | VmToken::ConditionalBlock { target, .. } => Some(*target),
        _ => None,
    }
}

fn bas_annotations(image: &[u8], dictionary: &HashMap<u16, String>) -> Result<SourceAnnotations> {
    let mut cursor = 0usize;
    let mut nodes = BTreeSet::new();
    let mut next_nodes = BTreeSet::new();
    while cursor < image.len() {
        if let Some((end, token)) = vm_source::bas_token_at(image, cursor, dictionary) {
            if let vm_source::BasToken::SelectorNode { next, .. } = token {
                nodes.insert(cursor);
                if next != 0 {
                    next_nodes.insert(next);
                }
            }
            cursor = end;
        } else {
            cursor += 1;
        }
    }

    let mut annotations = SourceAnnotations::default();
    for target in next_nodes {
        let offset = usize::from(target);
        if !nodes.contains(&offset) {
            bail!(
                "BAS selector next offset 0x{target:04X} does not resolve to a selector node"
            );
        }
        let identifier = format!("selector_{offset:04X}");
        annotations
            .directives
            .entry(offset)
            .or_default()
            .push(format!("LABEL {identifier}"));
        annotations.labels.insert(offset as u16, identifier);
    }
    Ok(annotations)
}

fn bas_structured_annotations(graph: Option<&BasControlFlow>) -> BasStructuredAnnotations {
    let mut annotations = BasStructuredAnnotations::default();
    let Some(graph) = graph else {
        return annotations;
    };
    for list in &graph.lists {
        let entry = &list.entrypoint;
        let name = format!(
            "list_{}_{:04X}",
            identifier_component(&entry.object_name),
            entry.prefix_yield_b
        );
        annotations.starts.insert(
            entry.prefix_yield_b,
            (name.clone(), entry.object_name.clone(), entry.object_offset),
        );
        annotations.ends.insert(list.end_exclusive, name);
        annotations.cases.extend(list.node_offsets.iter().copied());
    }
    annotations
}

fn emit_bas_structured_boundaries(
    output: &mut String,
    offset: usize,
    structured: &BasStructuredAnnotations,
) -> Result<()> {
    if let Some(name) = structured.ends.get(&offset) {
        writeln!(output, "{offset:08X}: END_SELECTOR_LIST {name}")?;
    }
    if let Some((name, object_name, object_offset)) = structured.starts.get(&offset) {
        writeln!(
            output,
            "{offset:08X}: SELECTOR_LIST {name} ; DEB object {:?} at 0x{object_offset:04X}",
            object_name
        )?;
    }
    Ok(())
}

fn emit_directives(
    output: &mut String,
    offset: usize,
    annotations: &SourceAnnotations,
) -> Result<()> {
    if let Some(directives) = annotations.directives.get(&offset) {
        for directive in directives {
            writeln!(output, "{offset:08X}: {directive}")?;
        }
    }
    Ok(())
}

fn structured_annotations(recovery: GuardRecovery) -> StructuredAnnotations {
    let mut annotations = StructuredAnnotations {
        rejected: recovery.rejected,
        ..StructuredAnnotations::default()
    };
    for region in recovery.structured {
        annotations.thens.insert(region.then_offset, region.start);
        annotations
            .ends
            .entry(region.end)
            .or_default()
            .push(region.start);
        annotations.starts.insert(region.start, region);
    }
    for starts in annotations.ends.values_mut() {
        starts.sort_unstable_by(|left, right| right.cmp(left));
    }
    annotations
}

fn guard_rejection_counts(
    rejected: &BTreeMap<usize, BTreeSet<GuardRejection>>,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for reasons in rejected.values() {
        for reason in reasons {
            *counts.entry(reason.as_str().to_string()).or_default() += 1;
        }
    }
    counts
}

fn emit_structured_ends(
    output: &mut String,
    offset: usize,
    structured: &StructuredAnnotations,
    labels: &HashMap<u16, String>,
) -> Result<()> {
    let Some(starts) = structured.ends.get(&offset) else {
        return Ok(());
    };
    for start in starts {
        let region = &structured.starts[start];
        writeln!(
            output,
            "{offset:08X}: END_WHEN {}",
            address_operand(region.end as u16, labels)
        )?;
    }
    Ok(())
}

fn identifier_component(name: &str) -> String {
    let mut output = String::new();
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'_' {
            output.push(char::from(byte));
        } else {
            write!(output, "_{byte:02X}").expect("writing to String cannot fail");
        }
    }
    if output.is_empty() {
        output.push_str("unnamed");
    }
    output
}

fn address_operand(address: u16, labels: &HashMap<u16, String>) -> String {
    labels
        .get(&address)
        .cloned()
        .unwrap_or_else(|| format!("{address:04X}"))
}

fn optional_address_operand(address: Option<u16>, labels: &HashMap<u16, String>) -> String {
    address.map_or_else(
        || "-".to_string(),
        |address| address_operand(address, labels),
    )
}

fn bas_next_operand(target: u16, labels: &HashMap<u16, String>) -> String {
    labels
        .get(&target)
        .cloned()
        .unwrap_or_else(|| format!("{target:04X}"))
}

fn emit_token(
    output: &mut String,
    token: &VmToken,
    dictionary: &HashMap<u16, String>,
    labels: &HashMap<u16, String>,
    guard_rejections: Option<&BTreeSet<GuardRejection>>,
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
                optional_address_operand(*loop_target, labels),
                option_word(*control_word)
            )?;
            for word in word_offsets {
                write!(output, " {word:04X}")?;
            }
        }
        VmToken::GuardPush { target, .. } => {
            write!(output, "GUARD_PUSH {}", address_operand(*target, labels))?
        }
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
        VmToken::Jump { target, .. } => {
            write!(output, "JUMP {}", address_operand(*target, labels))?
        }
        VmToken::StateArray {
            index,
            value: Some(value),
            ..
        } => write!(output, "STATE_ARRAY_SET {index:02X} {value:04X}")?,
        VmToken::StateArray {
            index, value: None, ..
        } => write!(output, "STATE_ARRAY_TEST {index:02X}")?,
        VmToken::ConditionalBlock { flags, target, .. } => write!(
            output,
            "CONDITIONAL_BLOCK {flags:02X} {}",
            address_operand(*target, labels)
        )?,
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
    write!(output, " ; {}", vm_source::token_comment(token, dictionary))?;
    if let Some(reasons) = guard_rejections {
        write!(output, " ; unstructured_guard=")?;
        for (index, reason) in reasons.iter().enumerate() {
            if index != 0 {
                output.push(',');
            }
            output.push_str(reason.as_str());
        }
    }
    output.push('\n');
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

fn compile_statement(
    name: &str,
    args: &[&str],
    line: usize,
    labels: &HashMap<&str, u16>,
) -> Result<Vec<u8>> {
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
        "YIELD_B" => {
            require_count(args, 0, line, name)?;
            output.push(vm::OP_YIELD_B);
        }
        "SELECTOR_NODE" | "CASE" => {
            require_count(args, 2, line, name)?;
            word(&mut output, parse_word(args[0], line, "selector")?);
            word(
                &mut output,
                parse_address(args[1], labels, line, "next selector node")?,
            );
        }
        "PRESENTATION_REGISTER" => {
            require_count(args, 1, line, name)?;
            output.push(0xA7);
            word(&mut output, parse_word(args[0], line, "presentation value")?);
        }
        "GUARD_PUSH" | "WHEN" => {
            require_count(args, 1, line, name)?;
            output.push(vm::OP_PUSH);
            word(
                &mut output,
                parse_address(args[0], labels, line, "guard target")?,
            );
        }
        "GUARD_POP" | "THEN" => {
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
            word(
                &mut output,
                parse_address(args[0], labels, line, "jump target")?,
            );
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
                parse_address(args[1], labels, line, "conditional target")?,
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
            word(
                &mut output,
                parse_address(args[0], labels, line, "address")?,
            );
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
            let loop_target = parse_optional_address(args[4], labels, line, "loop target")?;
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

fn parse_address(
    value: &str,
    labels: &HashMap<&str, u16>,
    line: usize,
    field: &str,
) -> Result<u16> {
    labels
        .get(value)
        .copied()
        .map(Ok)
        .unwrap_or_else(|| parse_word(value, line, field))
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

fn parse_optional_address(
    value: &str,
    labels: &HashMap<&str, u16>,
    line: usize,
    field: &str,
) -> Result<Option<u16>> {
    if value == "-" {
        Ok(None)
    } else {
        parse_address(value, labels, line, field).map(Some)
    }
}

fn validate_identifier(value: &str, line: usize) -> Result<()> {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        bail!("line {line}: identifier cannot be empty");
    };
    if !(first.is_ascii_alphabetic() || first == b'_')
        || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        bail!("line {line}: invalid identifier {value:?}");
    }
    Ok(())
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

    fn has_complete_bundle(path: &Path) -> bool {
        (1..=5).all(|script| {
            ["COD", "BAS", "DIC", "DEB", "VAR"]
                .iter()
                .all(|extension| path.join(format!("SCRIPT{script}.{extension}")).exists())
        })
    }

    fn game_dir() -> Option<PathBuf> {
        if let Some(path) = std::env::var_os("CBLOOD_GAME_DIR").map(PathBuf::from) {
            if has_complete_bundle(&path) {
                return Some(path);
            }
        }
        [
            "accuracy/cblood_install/cblood",
            "../accuracy/cblood_install/cblood",
        ]
        .iter()
        .map(Path::new)
        .find(|path| has_complete_bundle(path))
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
            "00000000: GUARD_PUSH 0005\n",
            "00000003: STATE_ARRAY_TEST FE\n",
            "00000005: GUARD_POP\n",
            "00000006: STATE_ARRAY_SET 02 5678\n",
            "0000000A: JUMP 000D\n",
            "0000000D: CONDITIONAL_BLOCK 01 0011\n",
            "00000011: BRANCH_PRESENTATION\n",
            "00000012: BRANCH_GAMEFLAG\n",
            "00000013: GUARD_POP\n",
            "00000014: END\n",
        );
        let expected = vec![
            0xA0, 0x05, 0x00, 0xA5, 0xFE, 0xA1, 0xA5, 0x02, 0x78, 0x56, 0xA4, 0x0D, 0x00, 0xA9,
            0x01, 0x11, 0x00, 0xCE, 0xD0, 0xA1, 0xFF,
        ];
        assert_eq!(compile(source).unwrap(), expected);

        let decompiled = decompile(ImageKind::Cod, &expected, &HashMap::new()).unwrap();
        assert_eq!(decompiled.generic_op_statements, 0);
        for statement in [
            "GUARD_PUSH block_0005",
            "STATE_ARRAY_TEST FE",
            "GUARD_POP",
            "STATE_ARRAY_SET 02 5678",
            "JUMP block_000D",
            "CONDITIONAL_BLOCK 01 block_0011",
            "BRANCH_PRESENTATION",
            "BRANCH_GAMEFLAG",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert_eq!(compile(&decompiled.source).unwrap(), expected);
    }

    #[test]
    fn structured_guards_compile_to_the_exact_low_level_tokens() {
        let image = vec![
            0xA0, 0x0B, 0x00, // guard -> END
            0xA3, 0x34, 0x12, // condition
            0xA1, // then
            0xA5, 0x02, 0x78, 0x56, // body
            0xFF,
        ];
        let decompiled =
            decompile_structured_with_symbols(ImageKind::Cod, &image, &HashMap::new(), &[])
                .unwrap();
        assert_eq!(decompiled.structured_guards, 1);
        for statement in ["WHEN block_000B", "THEN ; GUARD_POP", "END_WHEN block_000B"] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert_eq!(compile(&decompiled.source).unwrap(), image);

        let malformed = decompiled
            .source
            .replace("END_WHEN block_000B", "END_WHEN wrong");
        assert!(compile(&malformed).is_err());
    }

    #[test]
    fn rejected_structured_guard_keeps_exact_tokens_with_reason() {
        let image = vec![
            0xA4, 0x0A, 0x00, // external jump into the guard body
            0xA0, 0x0E, 0x00, // guard -> END
            0xA3, 0x34, 0x12, // condition
            0xA1, 0xA5, 0x02, 0x78, 0x56, // external-entry target
            0xFF,
        ];
        let decompiled =
            decompile_structured_with_symbols(ImageKind::Cod, &image, &HashMap::new(), &[])
                .unwrap();
        assert_eq!(decompiled.structured_guards, 0);
        assert_eq!(decompiled.unstructured_guards, 1);
        assert_eq!(
            decompiled.guard_rejection_counts.get("external_entry"),
            Some(&1)
        );
        assert!(decompiled.source.contains(
            "GUARD_PUSH block_000E ; GUARD_PUSH target=0x000E ; unstructured_guard=external_entry"
        ));
        assert_eq!(compile(&decompiled.source).unwrap(), image);
    }

    #[test]
    fn symbolic_procedures_and_cod_targets_compile_exact_bytes() {
        let image = vec![vm::OP_COND_JUMP, 0x01, 0x04, 0x00, 0xFF];
        let symbols = vec![DebSymbol {
            name: "entry".to_string(),
            offset: 1,
            kind: 2,
        }];
        let decompiled =
            decompile_with_symbols(ImageKind::Cod, &image, &HashMap::new(), &symbols).unwrap();
        assert_eq!(decompiled.procedures, 1);
        assert_eq!(decompiled.symbolic_labels, 2);
        assert!(
            decompiled
                .source
                .contains("00000000: PROCEDURE proc_entry_0000")
        );
        assert!(
            decompiled
                .source
                .contains("CONDITIONAL_BLOCK 01 block_0004")
        );
        assert!(decompiled.source.contains("00000004: LABEL block_0004"));
        assert!(
            decompiled
                .source
                .contains("00000005: END_PROCEDURE proc_entry_0000")
        );
        assert_eq!(compile(&decompiled.source).unwrap(), image);
    }

    #[test]
    fn symbolic_decompilation_rejects_unaligned_addresses() {
        let bad_cod_target = vec![vm::OP_COND_JUMP, 0x01, 0x02, 0x00, 0xFF];
        assert!(decompile(ImageKind::Cod, &bad_cod_target, &HashMap::new()).is_err());

        let symbols = vec![DebSymbol {
            name: "inside_instruction".to_string(),
            offset: 2,
            kind: 2,
        }];
        let valid_cod = vec![vm::OP_COND_JUMP, 0x01, 0x04, 0x00, 0xFF];
        assert!(
            decompile_with_symbols(ImageKind::Cod, &valid_cod, &HashMap::new(), &symbols).is_err()
        );

        let dictionary = HashMap::from([(0x1234, "topic".to_string())]);
        let bad_bas_next = vec![0xAC, 0x34, 0x12, 0x03, 0x00, 0xFF];
        assert!(decompile(ImageKind::Bas, &bad_bas_next, &dictionary).is_err());
    }

    #[test]
    fn residual_opcode_families_compile_exact_bytes() {
        let source = concat!(
            "; format: bloodscript-ir-v1\n",
            "00000000: GUARD_PUSH 000A\n",
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
            0xA0, 0x0A, 0x00, 0xA3, 0x26, 0x0D, 0xA3, 0xA1, 0xE8, 0x0E, 0xA1, 0xA8, b'f', b'i',
            b'n', b'.', b'h', b'n', b'm', 0, 0, 0xAB, 0x56, 0x34, 0x12, 0xCC, 0x02, b's', b'c',
            b'r', b'u', b't', 0, 0, 0xCF, 0xD1, 0xFF,
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
            "00000001: YIELD_B\n",
            "00000002: SELECTOR_NODE 1234 selector_000C\n",
            "00000006: MENU 1234\n",
            "0000000B: YIELD_B\n",
            "0000000C: LABEL selector_000C\n",
            "0000000C: SELECTOR_NODE 1234 0000\n",
            "00000010: PRESENTATION_REGISTER 9ABC\n",
            "00000013: END\n",
        );
        let expected = vec![
            0xAA, 0xAC, 0x34, 0x12, 0x0C, 0x00, 0xA3, 0x34, 0x12, 0x00, 0x00, 0xAC, 0x34,
            0x12, 0x00, 0x00, 0xA7, 0xBC, 0x9A, 0xFF,
        ];
        assert_eq!(compile(source).unwrap(), expected);

        let dictionary = HashMap::from([(0x1234, "topic".to_string())]);
        let decompiled = decompile(ImageKind::Bas, &expected, &dictionary).unwrap();
        assert_eq!(decompiled.raw_bytes, 0);
        for statement in [
            "YIELD",
            "YIELD_B",
            "SELECTOR_NODE 1234 selector_000C",
            "LABEL selector_000C",
            "MENU 1234",
            "PRESENTATION_REGISTER 9ABC",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert_eq!(compile(&decompiled.source).unwrap(), expected);
    }

    #[test]
    fn structured_selector_lists_compile_to_the_exact_low_level_tokens() {
        let image = vec![
            0xAA,
            0xAC,
            0x34, 0x12, 0x0C, 0x00,
            0xA3, 0x00, 0x20, 0x00, 0x00,
            0xAC,
            0x00, 0x20, 0x00, 0x00,
            0xA3, 0x34, 0x12, 0x00, 0x00,
            0xFF,
        ];
        let mut var = vec![0; 0x1C];
        var[0..2].copy_from_slice(&2u16.to_le_bytes());
        var[0x1A..0x1C].copy_from_slice(&1u16.to_le_bytes());
        let dictionary = HashMap::from([
            (0x1234, "talk".to_string()),
            (0x2000, "leave".to_string()),
        ]);
        let symbols = vec![DebSymbol {
            name: "actor".to_string(),
            offset: 0,
            kind: 1,
        }];

        let decompiled = decompile_structured_bas_with_symbols(
            &image,
            &var,
            &dictionary,
            &symbols,
        )
        .unwrap();
        assert_eq!(decompiled.structured_selector_lists, 1);
        assert_eq!(decompiled.structured_cases, 2);
        for statement in [
            "SELECTOR_LIST list_actor_0001",
            "CASE 1234 selector_000C",
            "CASE 2000 0000",
            "END_SELECTOR_LIST list_actor_0001",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert_eq!(compile(&decompiled.source).unwrap(), image);

        let malformed = decompiled
            .source
            .replace("CASE 1234 selector_000C", "CASE 1234 0000");
        assert!(compile(&malformed).is_err());
    }

    #[test]
    fn every_shipped_bas_structures_into_exact_selector_lists() {
        let Some(root) = game_dir() else { return };
        let expected = [(1, 1, 1), (2, 10, 122), (3, 12, 98), (4, 10, 43), (5, 4, 57)];
        for (script, list_count, case_count) in expected {
            let read = |extension: &str| {
                std::fs::read(root.join(format!("SCRIPT{script}.{extension}"))).unwrap()
            };
            let image = read("BAS");
            let var = read("VAR");
            let dictionary = crate::script::parse_dictionary(&read("DIC"));
            let symbols = crate::script::parse_deb(&read("DEB"));
            let source = decompile_structured_bas_with_symbols(
                &image,
                &var,
                &dictionary,
                &symbols,
            )
            .unwrap();
            assert_eq!(source.structured_selector_lists, list_count);
            assert_eq!(source.structured_cases, case_count);
            assert_eq!(compile(&source.source).unwrap(), image);
        }
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
