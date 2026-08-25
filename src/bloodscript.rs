//! Typed, lossless source language for Commander Blood VM programs.
//!
//! BloodScript 8 is the canonical editable layer for all five VM resource
//! bundles. Every shipped token and companion-data field has authoritative
//! typed syntax; canonical profiles reject unresolved opcode, byte, address, or
//! state-field fallbacks. The syntax is reconstructed for this project and is
//! not claimed to be the lost historical source spelling.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;

use anyhow::{Result, anyhow, bail};

use crate::bas_cfg::{BasControlFlow, analyze_bas};
use crate::script::DebSymbol;
use crate::vm::{self, VmToken};
use crate::vm_cfg::{GuardRecovery, GuardRejection, StructuredGuard, analyze_structured_guards};
use crate::vm_source::{self, ImageKind};

const SOURCE_FORMAT: &str = "bloodscript-program-v1";
const READABLE_SOURCE_FORMAT: &str = "bloodscript-v2";
const LEGACY_SOURCE_FORMAT: &str = "bloodscript-ir-v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceFormat {
    LegacyOffsets,
    Readable,
}

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
    pub object_aliases: usize,
    pub object_alias_uses: usize,
    pub dictionary_offsets: usize,
    pub dictionary_uses: usize,
    pub field_aliases: usize,
    pub field_alias_uses: usize,
    pub structured_selector_lists: usize,
    pub structured_cases: usize,
}

/// A compiled VM program plus the layout symbols needed by the unified profile
/// compiler to rebuild the companion DEB directory without hard-coded offsets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Compilation {
    pub image: Vec<u8>,
    pub label_offsets: HashMap<String, u16>,
    pub procedure_offsets: HashMap<String, u16>,
    /// First selector-node offset for each structured BAS selector list.
    pub selector_offsets: HashMap<String, u16>,
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
    object_aliases: usize,
    object_alias_uses: usize,
    dictionary_offsets: usize,
    dictionary_uses: usize,
    field_aliases: usize,
    field_alias_uses: usize,
    structured_selector_lists: usize,
    structured_cases: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SourceAnnotations {
    directives: BTreeMap<usize, Vec<String>>,
    labels: HashMap<u16, String>,
    procedure_labels: HashMap<u16, String>,
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
    elses: BTreeMap<usize, usize>,
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
struct ObjectAlias {
    identifier: String,
    source_name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProvenStatement {
    InventoryTransfer,
    Navigate,
    BringAboard,
    TravelThrough,
    PositionAssignment,
    BloodLink { target: u16 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DictionaryAlias {
    value: String,
}

struct DictionaryOperandFormatter<'a> {
    aliases: &'a BTreeMap<u16, DictionaryAlias>,
    canonical_offsets: BTreeMap<String, u16>,
}

struct DictionaryPhraseLexicon {
    canonical_offsets: HashMap<String, u16>,
    by_first: HashMap<char, Vec<(String, u16)>>,
}

impl DictionaryPhraseLexicon {
    fn new(dictionary: &HashMap<u16, String>) -> Self {
        let mut canonical_offsets = HashMap::new();
        for (offset, text) in dictionary {
            if text.is_empty() || text.contains(char::is_whitespace) || text.contains('|') {
                continue;
            }
            canonical_offsets
                .entry(text.clone())
                .and_modify(|canonical: &mut u16| *canonical = (*canonical).min(*offset))
                .or_insert(*offset);
        }
        let mut by_first: HashMap<char, Vec<(String, u16)>> = HashMap::new();
        for (text, offset) in &canonical_offsets {
            if let Some(first) = text.chars().next() {
                by_first
                    .entry(first)
                    .or_default()
                    .push((text.clone(), *offset));
            }
        }
        for candidates in by_first.values_mut() {
            candidates.sort_by(|left, right| {
                right
                    .0
                    .len()
                    .cmp(&left.0.len())
                    .then_with(|| left.0.cmp(&right.0))
            });
        }
        Self {
            canonical_offsets,
            by_first,
        }
    }

    fn render_exact(&self, offsets: &[u16], dictionary: &HashMap<u16, String>) -> Option<String> {
        let texts = offsets
            .iter()
            .map(|offset| dictionary.get(offset).cloned())
            .collect::<Option<Vec<_>>>()?;
        if texts
            .iter()
            .any(|text| text.is_empty() || text.contains(char::is_whitespace) || text.contains('|'))
        {
            return None;
        }
        let mut forced_boundaries = vec![false; texts.len()];
        for index in 1..texts.len() {
            if !phrase_needs_space(phrase_ends_open(&texts[index - 1]), &texts[index]) {
                forced_boundaries[index] = true;
            }
        }
        for index in 1..forced_boundaries.len() {
            if !forced_boundaries[index] {
                continue;
            }
            forced_boundaries[index] = false;
            let candidate = render_phrase_texts(&texts, &forced_boundaries);
            if self.tokenize(&candidate).as_deref() != Some(offsets) {
                forced_boundaries[index] = true;
            }
        }
        Some(render_phrase_texts(&texts, &forced_boundaries))
    }

    fn tokenize(&self, phrase: &str) -> Option<Vec<u16>> {
        let mut memo = HashMap::new();
        let solutions = self.tokenize_from(phrase, 0, false, true, &mut memo);
        solutions.first().cloned()
    }

    fn tokenize_from(
        &self,
        phrase: &str,
        position: usize,
        prior_ends_open: bool,
        first: bool,
        memo: &mut HashMap<(usize, bool, bool), Vec<Vec<u16>>>,
    ) -> Vec<Vec<u16>> {
        let key = (position, prior_ends_open, first);
        if let Some(cached) = memo.get(&key) {
            return cached.clone();
        }
        if position == phrase.len() {
            return vec![Vec::new()];
        }

        let separator = (!first)
            .then(|| phrase.as_bytes().get(position).copied())
            .flatten()
            .filter(|byte| matches!(byte, b' ' | b'|'));
        let start = position + usize::from(matches!(separator, Some(b' ') | Some(b'|')));
        let Some(first_character) = phrase.get(start..).and_then(|tail| tail.chars().next()) else {
            return Vec::new();
        };
        let mut solutions = Vec::new();
        if let Some(candidates) = self.by_first.get(&first_character) {
            for (text, offset) in candidates {
                let separator_matches = first
                    || if phrase_needs_space(prior_ends_open, text) {
                        separator == Some(b' ')
                    } else {
                        matches!(separator, None | Some(b'|'))
                    };
                if !separator_matches || !phrase[start..].starts_with(text) {
                    continue;
                }
                let next = start + text.len();
                for mut suffix in
                    self.tokenize_from(phrase, next, phrase_ends_open(text), false, memo)
                {
                    suffix.insert(0, *offset);
                    if !solutions.contains(&suffix) {
                        solutions.push(suffix);
                    }
                    if !solutions.is_empty() {
                        memo.insert(key, solutions.clone());
                        return solutions;
                    }
                }
            }
        }
        memo.insert(key, solutions.clone());
        solutions
    }
}

/// Make every no-space token boundary in modern `say` phrases explicit. This
/// lets the profile compiler reconstruct DIC without consulting a companion
/// dictionary while preserving normal spaces in the displayed text.
pub(crate) fn make_phrase_boundaries_explicit(
    source: &str,
    dictionary: &HashMap<u16, String>,
) -> Result<String> {
    let lexicon = DictionaryPhraseLexicon::new(dictionary);
    let mut output = String::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("say ") {
            writeln!(output, "{line}")?;
            continue;
        }
        let colon = line
            .find(" : ")
            .ok_or_else(|| anyhow!("say statement has no phrase separator"))?;
        let phrase_start = colon + 3;
        let phrase_end = quoted_json_end(&line[phrase_start..])? + phrase_start;
        let phrase: String = serde_json::from_str(&line[phrase_start..phrase_end])?;
        let offsets = lexicon
            .tokenize(&phrase)
            .ok_or_else(|| anyhow!("say phrase has no exact dictionary tokenization"))?;
        let texts = offsets
            .iter()
            .map(|offset| {
                dictionary
                    .get(offset)
                    .cloned()
                    .ok_or_else(|| anyhow!("say phrase references an unknown dictionary word"))
            })
            .collect::<Result<Vec<_>>>()?;
        let explicit = render_phrase_texts(&texts, &vec![true; texts.len()]);
        let quoted = serde_json::to_string(&explicit)?;
        writeln!(
            output,
            "{}{}{}",
            &line[..phrase_start],
            quoted,
            &line[phrase_end..]
        )?;
    }
    Ok(output.trim_end_matches('\n').to_string())
}

fn quoted_json_end(value: &str) -> Result<usize> {
    if !value.starts_with('"') {
        bail!("expected quoted phrase");
    }
    let mut escaped = false;
    for (index, character) in value.char_indices().skip(1) {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Ok(index + 1);
        }
    }
    bail!("unterminated quoted phrase")
}

fn render_phrase_texts(texts: &[String], forced_boundaries: &[bool]) -> String {
    let mut phrase = String::new();
    for (index, text) in texts.iter().enumerate() {
        if index != 0 {
            if phrase_needs_space(phrase_ends_open(&texts[index - 1]), text) {
                phrase.push(' ');
            } else if forced_boundaries[index] {
                phrase.push('|');
            }
        }
        phrase.push_str(text);
    }
    phrase
}

fn phrase_needs_space(prior_ends_open: bool, text: &str) -> bool {
    !prior_ends_open
        && !text.chars().next().is_some_and(|character| {
            matches!(
                character,
                ',' | '.' | '!' | '?' | ';' | ':' | '%' | ')' | ']' | '}'
            )
        })
}

fn phrase_ends_open(text: &str) -> bool {
    text.chars()
        .next_back()
        .is_some_and(|character| matches!(character, '(' | '[' | '{'))
}

impl<'a> DictionaryOperandFormatter<'a> {
    fn new(aliases: &'a BTreeMap<u16, DictionaryAlias>, dictionary: &HashMap<u16, String>) -> Self {
        let mut canonical_offsets = BTreeMap::new();
        for (offset, value) in dictionary {
            canonical_offsets
                .entry(value.clone())
                .and_modify(|canonical: &mut u16| *canonical = (*canonical).min(*offset))
                .or_insert(*offset);
        }
        Self {
            aliases,
            canonical_offsets,
        }
    }

    fn operand(&mut self, value: u16) -> String {
        let Some(alias) = self.aliases.get(&value) else {
            return format!("{value:04X}");
        };
        let quoted = serde_json::to_string(&alias.value).expect("serializing a String cannot fail");
        if self.canonical_offsets.get(&alias.value) == Some(&value) {
            quoted
        } else {
            format!("{quoted}@{value:04X}")
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FieldAlias {
    identifier: String,
    owner_offset: u16,
    owner_name: String,
    kind: u16,
    selectors: Vec<u8>,
    field_offset: u16,
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct OpenWhen<'a> {
    false_target: &'a str,
    end_target: Option<&'a str>,
    saw_then: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ModernBlock {
    Procedure(String),
    ProcedureCondition {
        name: String,
        end_target: Option<String>,
        target_emitted: bool,
    },
    ProcedureBody {
        name: String,
        end_target: Option<String>,
        target_emitted: bool,
    },
    WhenCondition {
        false_target: String,
        end_target: String,
    },
    WhenBody {
        false_target: String,
        end_target: String,
    },
    WhenElse {
        end_target: String,
    },
    Selector {
        name: String,
        pending_case_label: Option<String>,
    },
    Case {
        continues: bool,
    },
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
    decompile_mode(kind, image, dictionary, symbols, false, None)
}

pub fn decompile_structured_with_symbols(
    kind: ImageKind,
    image: &[u8],
    dictionary: &HashMap<u16, String>,
    symbols: &[DebSymbol],
) -> Result<Decompilation> {
    decompile_mode(kind, image, dictionary, symbols, true, None)
}

pub fn decompile_structured_cod_with_symbols(
    image: &[u8],
    var: &[u8],
    dictionary: &HashMap<u16, String>,
    symbols: &[DebSymbol],
) -> Result<Decompilation> {
    decompile_mode(ImageKind::Cod, image, dictionary, symbols, true, Some(var))
}

pub fn decompile_structured_bas_with_symbols(
    image: &[u8],
    var: &[u8],
    dictionary: &HashMap<u16, String>,
    symbols: &[DebSymbol],
) -> Result<Decompilation> {
    let graph = analyze_bas("BAS", image, var, dictionary, symbols)?;
    decompile_mode_with_bas_graph(image, var, dictionary, symbols, &graph)
}

fn decompile_mode(
    kind: ImageKind,
    image: &[u8],
    dictionary: &HashMap<u16, String>,
    symbols: &[DebSymbol],
    structured: bool,
    var: Option<&[u8]>,
) -> Result<Decompilation> {
    let mut source = String::new();
    writeln!(source, "; BloodScript typed VM source")?;
    writeln!(source, "; format: {READABLE_SOURCE_FORMAT}")?;
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
        ImageKind::Cod => decompile_cod(&mut source, image, dictionary, symbols, structured, var)?,
        ImageKind::Bas => decompile_bas(&mut source, image, dictionary, &[], None, None)?,
    };
    let source = format_modern_source(&source, dictionary)?;
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
        object_aliases: stats.object_aliases,
        object_alias_uses: stats.object_alias_uses,
        dictionary_offsets: stats.dictionary_offsets,
        dictionary_uses: stats.dictionary_uses,
        field_aliases: stats.field_aliases,
        field_alias_uses: stats.field_alias_uses,
        structured_selector_lists: stats.structured_selector_lists,
        structured_cases: stats.structured_cases,
    })
}

fn decompile_mode_with_bas_graph(
    image: &[u8],
    var: &[u8],
    dictionary: &HashMap<u16, String>,
    symbols: &[DebSymbol],
    graph: &BasControlFlow,
) -> Result<Decompilation> {
    let mut source = String::new();
    writeln!(source, "; BloodScript typed VM source")?;
    writeln!(source, "; format: {READABLE_SOURCE_FORMAT}")?;
    writeln!(source, "; image: BAS")?;
    writeln!(source, "; size: 0x{:08X}", image.len())?;
    writeln!(source)?;

    let stats = decompile_bas(
        &mut source,
        image,
        dictionary,
        symbols,
        Some(var),
        Some(graph),
    )?;
    let source = format_modern_source(&source, dictionary)?;
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
        object_aliases: stats.object_aliases,
        object_alias_uses: stats.object_alias_uses,
        dictionary_offsets: stats.dictionary_offsets,
        dictionary_uses: stats.dictionary_uses,
        field_aliases: stats.field_aliases,
        field_alias_uses: stats.field_alias_uses,
        structured_selector_lists: stats.structured_selector_lists,
        structured_cases: stats.structured_cases,
    })
}

pub fn compile(source: &str) -> Result<Vec<u8>> {
    compile_with_dictionary(source, &HashMap::new())
}

pub fn compile_with_dictionary(source: &str, dictionary: &HashMap<u16, String>) -> Result<Vec<u8>> {
    Ok(compile_with_layout(source, dictionary)?.image)
}

pub fn compile_with_layout(source: &str, dictionary: &HashMap<u16, String>) -> Result<Compilation> {
    let normalized_source = normalize_modern_source(source, dictionary)?;
    let source = normalized_source.as_deref().unwrap_or(source);
    let (mut lines, format) = parse_source_lines(source)?;
    let mut objects = HashMap::new();
    let mut dictionary_words = HashMap::new();
    let mut label_names = HashMap::new();
    let mut procedure_names = HashSet::new();
    for line in &lines {
        match line.name {
            "OBJECT" => {
                require_count(&line.args, 2, line.line_number, line.name)?;
                validate_identifier(line.args[0], line.line_number)?;
                let address = parse_word(line.args[1], line.line_number, "object offset")?;
                if objects.insert(line.args[0], address).is_some() {
                    bail!(
                        "line {}: duplicate object {:?}",
                        line.line_number,
                        line.args[0]
                    );
                }
            }
            "DIC_WORD" => {
                require_count(&line.args, 2, line.line_number, line.name)?;
                validate_identifier(line.args[0], line.line_number)?;
                let address = parse_word(line.args[1], line.line_number, "dictionary offset")?;
                if dictionary_words.insert(line.args[0], address).is_some() {
                    bail!(
                        "line {}: duplicate dictionary word {:?}",
                        line.line_number,
                        line.args[0]
                    );
                }
            }
            "LABEL" | "PROCEDURE" => {
                require_count(&line.args, 1, line.line_number, line.name)?;
                validate_identifier(line.args[0], line.line_number)?;
                if label_names.insert(line.args[0], 0).is_some() {
                    bail!(
                        "line {}: duplicate label {:?}",
                        line.line_number,
                        line.args[0]
                    );
                }
                if line.name == "PROCEDURE" {
                    procedure_names.insert(line.args[0]);
                }
            }
            _ => {}
        }
    }
    let interned_dictionary_words = collect_interned_dictionary_words(&lines, dictionary)?;
    let mut fields = HashMap::new();
    for line in &lines {
        if line.name != "FIELD" {
            continue;
        }
        require_count(&line.args, 3, line.line_number, line.name)?;
        validate_field_identifier(line.args[0], line.line_number)?;
        let owner = objects.get(line.args[1]).copied().ok_or_else(|| {
            anyhow!(
                "line {}: field owner {:?} is not a declared object",
                line.line_number,
                line.args[1]
            )
        })?;
        let field_offset = parse_word(line.args[2], line.line_number, "field offset")?;
        let address = owner.wrapping_add(field_offset);
        if fields.insert(line.args[0], address).is_some() {
            bail!(
                "line {}: duplicate field {:?}",
                line.line_number,
                line.args[0]
            );
        }
    }
    let mut var_addresses = objects.clone();
    for (name, address) in fields {
        if var_addresses.insert(name, address).is_some() {
            bail!("field {name:?} conflicts with an object declaration");
        }
    }

    if format == SourceFormat::Readable {
        let mut offset = 0usize;
        for line in &mut lines {
            line.offset = offset;
            if is_zero_byte_statement(line.name) {
                continue;
            }
            let encoded = compile_statement(
                line.name,
                &line.args,
                line.line_number,
                &label_names,
                &procedure_names,
                &var_addresses,
                &dictionary_words,
                &interned_dictionary_words,
            )?;
            if encoded.is_empty() {
                bail!("line {}: statement emitted no bytes", line.line_number);
            }
            offset = offset.checked_add(encoded.len()).ok_or_else(|| {
                anyhow!("line {}: compiled image size overflows", line.line_number)
            })?;
        }
    }

    let mut labels = HashMap::new();
    for line in &lines {
        if !matches!(line.name, "LABEL" | "PROCEDURE") {
            continue;
        }
        let address = u16::try_from(line.offset).map_err(|_| {
            anyhow!(
                "line {}: label offset 0x{:08X} exceeds the VM address space",
                line.line_number,
                line.offset
            )
        })?;
        labels.insert(line.args[0], address);
    }

    let mut image = Vec::new();
    let selector_offsets = lines
        .iter()
        .filter(|line| line.name == "SELECTOR_LIST")
        .map(|line| {
            Ok((
                line.args[0].to_string(),
                u16::try_from(line.offset).map_err(|_| {
                    anyhow!("line {}: selector root exceeds 64 KiB", line.line_number)
                })?,
            ))
        })
        .collect::<Result<HashMap<_, _>>>()?;
    let mut current_procedure: Option<&str> = None;
    let mut procedure_condition_open = false;
    let mut open_whens: Vec<OpenWhen<'_>> = Vec::new();
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
            "OBJECT" | "DIC_WORD" | "FIELD" => continue,
            "LABEL" => {
                require_count(&line.args, 1, line.line_number, line.name)?;
                continue;
            }
            "PROCEDURE" => {
                require_count(&line.args, 1, line.line_number, line.name)?;
                if let Some(open) = open_whens.last() {
                    bail!(
                        "line {}: PROCEDURE reached before END_WHEN {:?}",
                        line.line_number,
                        open.false_target
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
                if let Some(open) = open_whens.last() {
                    bail!(
                        "line {}: END_PROCEDURE reached before END_WHEN {:?}",
                        line.line_number,
                        open.false_target
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
                procedure_condition_open = false;
                current_procedure = None;
                continue;
            }
            "CONDITIONAL_BLOCK" if current_procedure.is_some() => {
                if procedure_condition_open {
                    bail!(
                        "line {}: duplicate procedure condition block",
                        line.line_number
                    );
                }
                procedure_condition_open = true;
            }
            "WHEN" => {
                require_count(&line.args, 1, line.line_number, line.name)?;
                let target = parse_address(line.args[0], &labels, line.line_number, "WHEN target")?;
                if usize::from(target) <= line.offset {
                    bail!("line {}: WHEN target must be forward", line.line_number);
                }
                open_whens.push(OpenWhen {
                    false_target: line.args[0],
                    end_target: None,
                    saw_then: false,
                });
            }
            "THEN" => {
                require_count(&line.args, 0, line.line_number, line.name)?;
                if let Some(open) = open_whens.last_mut() {
                    if open.saw_then {
                        bail!("line {}: duplicate THEN", line.line_number);
                    }
                    open.saw_then = true;
                } else if procedure_condition_open {
                    procedure_condition_open = false;
                } else {
                    bail!("line {}: THEN without WHEN", line.line_number);
                }
            }
            "ELSE" => {
                require_count(&line.args, 2, line.line_number, line.name)?;
                let Some(open) = open_whens.last_mut() else {
                    bail!("line {}: ELSE without WHEN", line.line_number);
                };
                if !open.saw_then {
                    bail!("line {}: ELSE reached before THEN", line.line_number);
                }
                if open.end_target.is_some() {
                    bail!("line {}: duplicate ELSE", line.line_number);
                }
                if open.false_target != line.args[0] {
                    bail!(
                        "line {}: ELSE false target {:?} does not match {:?}",
                        line.line_number,
                        line.args[0],
                        open.false_target
                    );
                }
                let false_offset =
                    parse_address(line.args[0], &labels, line.line_number, "ELSE false target")?;
                if usize::from(false_offset) != line.offset + 3 {
                    bail!(
                        "line {}: ELSE false target {:?} must follow its jump",
                        line.line_number,
                        line.args[0]
                    );
                }
                let end_offset =
                    parse_address(line.args[1], &labels, line.line_number, "ELSE end target")?;
                if usize::from(end_offset) <= usize::from(false_offset) {
                    bail!("line {}: ELSE end target must be forward", line.line_number);
                }
                open.end_target = Some(line.args[1]);
            }
            "END_WHEN" => {
                require_count(&line.args, 1, line.line_number, line.name)?;
                let Some(open) = open_whens.pop() else {
                    bail!("line {}: END_WHEN without WHEN", line.line_number);
                };
                let expected_target = open.end_target.unwrap_or(open.false_target);
                if expected_target != line.args[0] {
                    bail!(
                        "line {}: END_WHEN {:?} does not match {:?}",
                        line.line_number,
                        line.args[0],
                        expected_target
                    );
                }
                if !open.saw_then {
                    bail!("line {}: END_WHEN reached before THEN", line.line_number);
                }
                let target_offset = parse_address(
                    expected_target,
                    &labels,
                    line.line_number,
                    "END_WHEN target",
                )?;
                if usize::from(target_offset) != line.offset {
                    bail!(
                        "line {}: END_WHEN target {:?} resolves to 0x{:04X}, not 0x{:04X}",
                        line.line_number,
                        expected_target,
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
                    bail!("line {}: CASE is not preceded by YIELD_B", line.line_number);
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
                        bail!("line {}: prior CASE body has no YIELD_B", line.line_number);
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
                        bail!("line {}: CASE body must begin with MENU", line.line_number);
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
                        "SELECTOR_NODE" => {
                            bail!("line {}: use CASE inside SELECTOR_LIST", line.line_number)
                        }
                        _ => {}
                    }
                }
            }
        }
        let encoded = compile_statement(
            line.name,
            &line.args,
            line.line_number,
            &labels,
            &procedure_names,
            &var_addresses,
            &dictionary_words,
            &interned_dictionary_words,
        )?;
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
    if let Some(open) = open_whens.last() {
        bail!("WHEN {:?} has no END_WHEN", open.false_target);
    }
    if let Some(open) = open_selector_list {
        bail!("SELECTOR_LIST {:?} has no END_SELECTOR_LIST", open.name);
    }
    let procedure_offsets = procedure_names
        .into_iter()
        .map(|name| {
            let offset = labels
                .get(name)
                .copied()
                .expect("every declared procedure is included in the label layout");
            (name.to_string(), offset)
        })
        .collect();
    let label_offsets = labels
        .into_iter()
        .map(|(name, offset)| (name.to_string(), offset))
        .collect();
    Ok(Compilation {
        image,
        label_offsets,
        procedure_offsets,
        selector_offsets,
    })
}

fn parse_source_lines(source: &str) -> Result<(Vec<ParsedSourceLine<'_>>, SourceFormat)> {
    let mut format = None;
    for (line_index, original_line) in source.lines().enumerate() {
        let trimmed = original_line.trim();
        let Some(value) = trimmed.strip_prefix("; format:") else {
            continue;
        };
        let parsed = match value.trim() {
            READABLE_SOURCE_FORMAT => SourceFormat::Readable,
            LEGACY_SOURCE_FORMAT => SourceFormat::LegacyOffsets,
            value => bail!("line {}: unsupported format {value:?}", line_index + 1),
        };
        if format.replace(parsed).is_some() {
            bail!("line {}: duplicate format header", line_index + 1);
        }
    }
    let format =
        format.ok_or_else(|| anyhow!("missing '; format: {READABLE_SOURCE_FORMAT}' header"))?;
    let mut lines = Vec::new();
    for (line_index, original_line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = original_line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') {
            continue;
        }
        let code = code_before_comment(trimmed, line_number)?.trim();
        if code.is_empty() {
            continue;
        }
        let (offset, statement) = match format {
            SourceFormat::LegacyOffsets => {
                let (offset_text, statement) = code
                    .split_once(':')
                    .ok_or_else(|| anyhow!("line {line_number}: expected OFFSET: STATEMENT"))?;
                (
                    parse_hex_usize(offset_text.trim(), line_number, "offset")?,
                    statement.trim(),
                )
            }
            SourceFormat::Readable => (0, code),
        };
        let fields = split_source_fields(statement, line_number)?;
        let name = fields
            .first()
            .copied()
            .ok_or_else(|| anyhow!("line {line_number}: missing statement"))?;

        lines.push(ParsedSourceLine {
            line_number,
            offset,
            name,
            args: fields[1..].to_vec(),
        });
    }
    Ok((lines, format))
}

fn is_zero_byte_statement(name: &str) -> bool {
    matches!(
        name,
        "OBJECT"
            | "DIC_WORD"
            | "FIELD"
            | "LABEL"
            | "PROCEDURE"
            | "END_PROCEDURE"
            | "END_WHEN"
            | "SELECTOR_LIST"
            | "END_SELECTOR_LIST"
    )
}

fn code_before_comment(line: &str, line_number: usize) -> Result<&str> {
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in line.bytes().enumerate() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else if byte == b'"' {
            quoted = true;
        } else if byte == b';' {
            return Ok(&line[..index]);
        }
    }
    if quoted {
        bail!("line {line_number}: unterminated quoted string");
    }
    Ok(line)
}

fn split_source_fields(statement: &str, line_number: usize) -> Result<Vec<&str>> {
    let mut fields = Vec::new();
    let mut start = None;
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in statement.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        if character == '"' {
            quoted = true;
            start.get_or_insert(index);
        } else if character.is_whitespace() {
            if let Some(start) = start.take() {
                fields.push(&statement[start..index]);
            }
        } else {
            start.get_or_insert(index);
        }
    }
    if quoted {
        bail!("line {line_number}: unterminated quoted string");
    }
    if let Some(start) = start {
        fields.push(&statement[start..]);
    }
    Ok(fields)
}

fn normalize_modern_source(
    source: &str,
    dictionary: &HashMap<u16, String>,
) -> Result<Option<String>> {
    let modern_header = format!("// format: {SOURCE_FORMAT}");
    let legacy_comment_header = format!("; format: {SOURCE_FORMAT}");
    if !source.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == modern_header || trimmed == legacy_comment_header
    }) {
        return Ok(None);
    }

    let lexicon = DictionaryPhraseLexicon::new(dictionary);
    let mut normalized = String::new();
    let mut blocks = Vec::new();
    let mut next_control_block = 0usize;
    writeln!(normalized, "; format: {READABLE_SOURCE_FORMAT}")?;
    for (line_index, original_line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = original_line.trim();
        if trimmed.is_empty() {
            normalized.push('\n');
            continue;
        }
        if trimmed == modern_header || trimmed == legacy_comment_header {
            continue;
        }
        if let Some(comment) = trimmed.strip_prefix("//") {
            writeln!(normalized, ";{}", comment)?;
            continue;
        }
        if trimmed.starts_with(';') {
            writeln!(normalized, "{trimmed}")?;
            continue;
        }

        let code = modern_code_before_comment(trimmed, line_number)?.trim();
        if code.is_empty() {
            continue;
        }
        let Some(statement) = normalize_modern_braced_statement(
            code,
            line_number,
            &lexicon,
            &mut blocks,
            &mut next_control_block,
        )?
        else {
            continue;
        };
        write!(normalized, "{statement}")?;
        if let Some(comment) = modern_comment_after_code(trimmed, line_number)?
            .map(str::trim)
            .filter(|comment| !comment.is_empty())
        {
            write!(normalized, " ; {comment}")?;
        }
        normalized.push('\n');
    }
    if let Some(block) = blocks.last() {
        bail!("unclosed BloodScript block {block:?}");
    }
    Ok(Some(normalized))
}

fn normalize_modern_braced_statement(
    code: &str,
    line_number: usize,
    lexicon: &DictionaryPhraseLexicon,
    blocks: &mut Vec<ModernBlock>,
    next_control_block: &mut usize,
) -> Result<Option<String>> {
    if code == "} then {" {
        let Some(block) = blocks.pop() else {
            bail!("line {line_number}: '}} then {{' does not close a condition block");
        };
        match block {
            ModernBlock::ProcedureCondition {
                name,
                end_target,
                target_emitted,
            } => {
                blocks.push(ModernBlock::ProcedureBody {
                    name,
                    end_target,
                    target_emitted,
                });
            }
            ModernBlock::WhenCondition {
                false_target,
                end_target,
            } => {
                blocks.push(ModernBlock::WhenBody {
                    false_target,
                    end_target,
                });
            }
            _ => bail!("line {line_number}: '}} then {{' does not close a condition block"),
        }
        return Ok(Some("THEN".to_string()));
    }
    if code == "} else {" {
        let Some(ModernBlock::WhenBody {
            false_target,
            end_target,
        }) = blocks.pop()
        else {
            bail!("line {line_number}: '}} else {{' does not follow a when body");
        };
        blocks.push(ModernBlock::WhenElse {
            end_target: end_target.clone(),
        });
        return Ok(Some(format!(
            "ELSE {false_target} {end_target}\nLABEL {false_target}"
        )));
    }
    if code == "}" {
        let Some(block) = blocks.pop() else {
            bail!("line {line_number}: unmatched closing brace");
        };
        return match block {
            ModernBlock::Procedure(name) => Ok(Some(format!("END_PROCEDURE {name}"))),
            ModernBlock::ProcedureBody {
                name,
                end_target,
                target_emitted,
            }
            | ModernBlock::ProcedureCondition {
                name,
                end_target,
                target_emitted,
            } => {
                let derived_target = end_target
                    .filter(|_| !target_emitted)
                    .map(|target| format!("LABEL {target}\n"))
                    .unwrap_or_default();
                Ok(Some(format!("{derived_target}END_PROCEDURE {name}")))
            }
            ModernBlock::WhenBody { false_target, .. } => Ok(Some(format!(
                "LABEL {false_target}\nEND_WHEN {false_target}"
            ))),
            ModernBlock::WhenElse { end_target } => {
                Ok(Some(format!("LABEL {end_target}\nEND_WHEN {end_target}")))
            }
            ModernBlock::Selector {
                name,
                pending_case_label,
            } => {
                if pending_case_label.is_some() {
                    bail!("line {line_number}: final selector case cannot continue");
                }
                Ok(Some(format!("END_SELECTOR_LIST {name}")))
            }
            ModernBlock::Case { continues } => Ok(continues.then(|| "YIELD_B".to_string())),
            ModernBlock::WhenCondition { .. } => {
                bail!("line {line_number}: when closes without a following then block")
            }
        };
    }

    if code == "halt" {
        let statement = normalize_modern_statement(code, line_number, lexicon)?;
        for block in blocks.iter_mut().rev() {
            let (end_target, target_emitted) = match block {
                ModernBlock::ProcedureCondition {
                    end_target,
                    target_emitted,
                    ..
                }
                | ModernBlock::ProcedureBody {
                    end_target,
                    target_emitted,
                    ..
                } => (end_target, target_emitted),
                _ => continue,
            };
            if let Some(target) = end_target.as_ref().filter(|_| !*target_emitted) {
                *target_emitted = true;
                return Ok(Some(format!("LABEL {target}\n{statement}")));
            }
            break;
        }
        return Ok(Some(statement));
    }

    if let Some(opener) = code.strip_suffix('{') {
        let opener = opener.trim_end();
        let fields = split_source_fields(opener, line_number)?;
        let command = fields
            .first()
            .map(|name| name.to_ascii_lowercase())
            .ok_or_else(|| anyhow!("line {line_number}: missing block opener"))?;
        let (block, statement) = match command.as_str() {
            "proc" if fields.len() == 2 => (
                ModernBlock::Procedure(fields[1].to_string()),
                normalize_modern_statement(opener, line_number, lexicon)?,
            ),
            "proc" if fields.len() == 3 => {
                validate_identifier(fields[1], line_number)?;
                let flags = match fields[2] {
                    "enabled" => "01",
                    "disabled" => "00",
                    state => bail!(
                        "line {line_number}: procedure state must be enabled or disabled, found {state:?}"
                    ),
                };
                let id = *next_control_block;
                *next_control_block += 1;
                let end_target = format!("__proc_{id}_end");
                (
                    ModernBlock::ProcedureCondition {
                        name: fields[1].to_string(),
                        end_target: Some(end_target.clone()),
                        target_emitted: false,
                    },
                    format!(
                        "PROCEDURE {}\nCONDITIONAL_BLOCK {flags} {end_target}",
                        fields[1]
                    ),
                )
            }
            "proc" if fields.len() == 5 && fields[3] == "until" => {
                validate_identifier(fields[1], line_number)?;
                let flags = match fields[2] {
                    "enabled" => "01",
                    "disabled" => "00",
                    state => bail!(
                        "line {line_number}: procedure state must be enabled or disabled, found {state:?}"
                    ),
                };
                (
                    ModernBlock::ProcedureCondition {
                        name: fields[1].to_string(),
                        end_target: None,
                        target_emitted: false,
                    },
                    format!(
                        "PROCEDURE {}\nCONDITIONAL_BLOCK {flags} {}",
                        fields[1],
                        modern_operand_to_canonical(fields[4], line_number)?
                    ),
                )
            }
            "when" if fields.len() == 1 => {
                let id = *next_control_block;
                *next_control_block += 1;
                let false_target = format!("__when_{id}_false");
                let end_target = format!("__when_{id}_end");
                (
                    ModernBlock::WhenCondition {
                        false_target: false_target.clone(),
                        end_target,
                    },
                    format!("WHEN {false_target}"),
                )
            }
            "when" if fields.len() == 2 => {
                let id = *next_control_block;
                *next_control_block += 1;
                (
                    ModernBlock::WhenCondition {
                        false_target: fields[1].to_string(),
                        end_target: format!("__when_{id}_end"),
                    },
                    normalize_modern_statement(opener, line_number, lexicon)?,
                )
            }
            "selector" if fields.len() == 2 => (
                ModernBlock::Selector {
                    name: fields[1].to_string(),
                    pending_case_label: None,
                },
                format!(
                    "{}\nYIELD_B",
                    normalize_modern_statement(opener, line_number, lexicon)?
                ),
            ),
            "case" if fields.len() == 2 || (fields.len() == 3 && fields[2] == "continues") => {
                let selector = blocks
                    .iter_mut()
                    .rev()
                    .find_map(|block| match block {
                        ModernBlock::Selector {
                            pending_case_label, ..
                        } => Some(pending_case_label),
                        _ => None,
                    })
                    .ok_or_else(|| anyhow!("line {line_number}: case outside selector block"))?;
                let prior_label = selector
                    .take()
                    .map(|label| format!("LABEL {label}\n"))
                    .unwrap_or_default();
                let target = if fields.len() == 3 {
                    let id = *next_control_block;
                    *next_control_block += 1;
                    let label = format!("__selector_case_{id}");
                    *selector = Some(label.clone());
                    label
                } else {
                    "0".to_string()
                };
                (
                    ModernBlock::Case {
                        continues: fields.len() == 3,
                    },
                    format!(
                        "{prior_label}CASE {} {target}",
                        modern_operand_to_canonical(fields[1], line_number)?
                    ),
                )
            }
            _ => bail!("line {line_number}: unsupported BloodScript block opener {opener:?}"),
        };
        blocks.push(block);
        return Ok(Some(statement));
    }
    Ok(Some(normalize_modern_statement(
        code,
        line_number,
        lexicon,
    )?))
}

fn modern_code_before_comment(line: &str, line_number: usize) -> Result<&str> {
    modern_comment_parts(line, line_number).map(|(code, _)| code)
}

fn modern_comment_after_code(line: &str, line_number: usize) -> Result<Option<&str>> {
    modern_comment_parts(line, line_number).map(|(_, comment)| comment)
}

fn modern_comment_parts(line: &str, line_number: usize) -> Result<(&str, Option<&str>)> {
    let mut quoted = false;
    let mut escaped = false;
    let bytes = line.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else if byte == b'"' {
            quoted = true;
        } else if byte == b'/' && bytes.get(index + 1) == Some(&b'/') {
            return Ok((&line[..index], Some(&line[index + 2..])));
        }
        index += 1;
    }
    if quoted {
        bail!("line {line_number}: unterminated quoted string");
    }
    Ok((line, None))
}

fn normalize_modern_statement(
    code: &str,
    line_number: usize,
    lexicon: &DictionaryPhraseLexicon,
) -> Result<String> {
    if let Some(label) = code
        .strip_suffix(':')
        .filter(|label| !label.bytes().any(|byte| byte.is_ascii_whitespace()))
    {
        let label = label.trim();
        validate_identifier(label, line_number)?;
        return Ok(format!("LABEL {label}"));
    }

    let fields = split_source_fields(code, line_number)?;
    let Some(name) = fields.first().copied() else {
        bail!("line {line_number}: missing statement");
    };
    let command = name.to_ascii_lowercase();

    match command.as_str() {
        "object" => {
            if fields.len() != 4 || fields[2] != "=" {
                bail!("line {line_number}: expected 'object NAME = 0xOFFSET'");
            }
            Ok(format!(
                "OBJECT {} {}",
                fields[1],
                modern_operand_to_canonical(fields[3], line_number)?
            ))
        }
        "field" => {
            if fields.len() != 6 || fields[2] != "=" || fields[4] != "+" {
                bail!("line {line_number}: expected 'field NAME = OWNER + 0xOFFSET'");
            }
            Ok(format!(
                "FIELD {} {} {}",
                fields[1],
                fields[3],
                modern_operand_to_canonical(fields[5], line_number)?
            ))
        }
        "proc" => normalize_modern_fixed("PROCEDURE", &fields[1..], 1, line_number),
        "when" => normalize_modern_fixed("WHEN", &fields[1..], 1, line_number),
        "then" => normalize_modern_fixed("GUARD_POP", &fields[1..], 0, line_number),
        "selector" => normalize_modern_fixed("SELECTOR_LIST", &fields[1..], 1, line_number),
        "activation" => {
            if fields.len() != 4 || fields[2] != "until" {
                bail!("line {line_number}: expected 'activation enabled|disabled until TARGET'");
            }
            let flags = match fields[1] {
                "enabled" => "01",
                "disabled" => "00",
                state => bail!(
                    "line {line_number}: activation state must be enabled or disabled, found {state:?}"
                ),
            };
            Ok(format!(
                "CONDITIONAL_BLOCK {flags} {}",
                modern_operand_to_canonical(fields[3], line_number)?
            ))
        }
        "end" => {
            if fields.len() == 3 && fields[1] == "presentation" {
                validate_identifier(fields[2], line_number)?;
                return Ok(format!("RECORD_CLEAR {}.action", fields[2]));
            }
            if fields.len() != 3 {
                bail!("line {line_number}: expected 'end proc|when|selector|presentation NAME'");
            }
            let canonical = match fields[1].to_ascii_lowercase().as_str() {
                "proc" => "END_PROCEDURE",
                "when" => "END_WHEN",
                "selector" => "END_SELECTOR_LIST",
                _ => {
                    bail!("line {line_number}: expected 'end proc', 'end when', or 'end selector'")
                }
            };
            Ok(format!("{canonical} {}", fields[2]))
        }
        "case" => {
            bail!("line {line_number}: case must be used as a selector block opener")
        }
        "say" => normalize_modern_say(&fields, line_number, lexicon),
        "text" | "text_tokens" => normalize_modern_text(&fields, line_number),
        "queue" => {
            if fields.len() != 3 || fields[1] != "presentation" {
                bail!("line {line_number}: expected 'queue presentation ACTOR'");
            }
            validate_identifier(fields[2], line_number)?;
            Ok(format!("RECORD_LINK {}.action blood 0", fields[2]))
        }
        "transfer" => {
            if fields.len() != 6 || fields[2] != "from" || fields[4] != "to" {
                bail!("line {line_number}: expected 'transfer ITEM from SOURCE to DESTINATION'");
            }
            validate_identifier(fields[1], line_number)?;
            validate_identifier(fields[3], line_number)?;
            validate_identifier(fields[5], line_number)?;
            let source = transfer_holder_to_object(fields[3]);
            let destination = transfer_holder_to_object(fields[5]);
            Ok(format!("TRANSFER {} {source} {destination}", fields[1],))
        }
        "navigate" => {
            if fields.len() != 3 || fields[1] != "to" {
                bail!("line {line_number}: expected 'navigate to DESTINATION'");
            }
            validate_identifier(fields[2], line_number)?;
            Ok(format!("NAVIGATE {}", fields[2]))
        }
        "bring" => {
            if fields.len() != 3 || fields[2] != "aboard" {
                bail!("line {line_number}: expected 'bring CHARACTER aboard'");
            }
            validate_identifier(fields[1], line_number)?;
            Ok(format!("BRING_ABOARD {}", fields[1]))
        }
        "offer" => {
            if fields.len() != 3 || fields[1] != "topic" {
                bail!("line {line_number}: expected 'offer topic WORD'");
            }
            Ok(format!(
                "OFFER_TOPIC {}",
                modern_operand_to_canonical(fields[2], line_number)?
            ))
        }
        "request" => {
            if fields.len() != 3 || fields[1] != "sequence" {
                bail!("line {line_number}: expected 'request sequence \"NAME.hnm\"'");
            }
            if !is_hnm_sequence_atom(fields[2]) {
                bail!(
                    "line {line_number}: sequence name must be a quoted .hnm basename of at most 20 printable ASCII bytes"
                );
            }
            Ok(format!(
                "LOAD_STRING {}",
                modern_operand_to_canonical(fields[2], line_number)?
            ))
        }
        "run" => {
            if fields.len() != 3 || fields[1] != "profile" {
                bail!("line {line_number}: expected 'run profile SCRIPT1..SCRIPT5'");
            }
            let profile = fields[2]
                .strip_prefix("SCRIPT")
                .and_then(|value| value.parse::<u8>().ok())
                .filter(|value| (1..=5).contains(value))
                .ok_or_else(|| anyhow!("line {line_number}: invalid profile {:?}", fields[2]))?;
            Ok(format!("RUN_PROFILE {profile:02X}"))
        }
        "during" => {
            if fields.len() != 2 {
                bail!("line {line_number}: expected 'during bridge|travel|contact'");
            }
            Ok(match fields[1] {
                "bridge" => "BRANCH_PRESENTATION".to_string(),
                "travel" => "BRANCH_GAMEFLAG".to_string(),
                "contact" => "BRANCH_FLAG_274F".to_string(),
                context => bail!(
                    "line {line_number}: scene context must be bridge, travel, or contact, found {context:?}"
                ),
            })
        }
        "check" => normalize_modern_marked_bit_expression(&fields, true, line_number),
        "mark" => normalize_modern_marked_bit_expression(&fields, false, line_number),
        "require" => {
            if fields.len() == 4 && fields[1] == "travel" && fields[2] == "through" {
                validate_identifier(fields[3], line_number)?;
                return Ok(format!("REQUIRE_TRAVEL_THROUGH {}", fields[3]));
            }
            if let Some(statement) =
                normalize_modern_timer_expression(&fields[1..], true, line_number)?
            {
                return Ok(statement);
            }
            if fields.len() == 4 && fields[2] == "in" && fields[3].ends_with(".known_objects") {
                validate_identifier(fields[1], line_number)?;
                validate_field_identifier(fields[3], line_number)?;
                return Ok(format!("BLOOD_LINK {} {} 0", fields[3], fields[1]));
            }
            if fields.len() == 5
                && fields[2] == "not"
                && fields[3] == "in"
                && fields[4].ends_with(".known_objects")
            {
                validate_identifier(fields[1], line_number)?;
                validate_field_identifier(fields[4], line_number)?;
                return Ok(format!("BLOOD_LINK {} {} 1", fields[4], fields[1]));
            }
            if let Some(statement) =
                normalize_modern_presentation_expression(&fields[1..], line_number)?
            {
                return Ok(statement);
            }
            if let Some(statement) = normalize_modern_choice_expression(&fields[1..], line_number)?
            {
                return Ok(statement);
            }
            if let Some(statement) = normalize_modern_clock_expression(&fields[1..], line_number)? {
                return Ok(statement);
            }
            if let Some(statement) =
                normalize_modern_bit_expression(&fields[1..], true, line_number)?
            {
                return Ok(statement);
            }
            if let Some(statement) =
                normalize_modern_record_expression(&fields[1..], true, line_number)?
            {
                return Ok(statement);
            }
            normalize_modern_shared_expression(&fields[1..], true, line_number)
        }
        _ => {
            if let Some(statement) =
                normalize_modern_sequence_slot_assignment(&fields, line_number)?
            {
                return Ok(statement);
            }
            if fields.first() == Some(&"choice") {
                if fields.as_slice() == ["choice", "=", "none"] {
                    return Ok("CLEAR_ALTERNATE_CONCEPT".to_string());
                }
                bail!("line {line_number}: expected 'choice = none'");
            }
            if fields.len() == 3 && fields[1] == "=" {
                if let Some(procedure) = fields[0].strip_suffix(".enabled") {
                    validate_identifier(procedure, line_number)?;
                    let enabled = match fields[2] {
                        "true" => "1",
                        "false" => "0",
                        value => bail!(
                            "line {line_number}: procedure enabled state must be true or false, found {value:?}"
                        ),
                    };
                    return Ok(format!("SET_PROCEDURE_ENABLED {procedure} {enabled}"));
                }
            }
            if fields.len() == 3
                && matches!(fields[1], "+=" | "-=")
                && fields[0].ends_with(".known_objects")
            {
                validate_field_identifier(fields[0], line_number)?;
                validate_identifier(fields[2], line_number)?;
                return Ok(format!(
                    "BLOOD_LINK {} {} {}",
                    fields[0],
                    fields[2],
                    if fields[1] == "-=" { 1 } else { 0 }
                ));
            }
            if fields.len() >= 3 && fields[0].ends_with(".position") && fields[1] == "=" {
                validate_field_identifier(fields[0], line_number)?;
                let tuple = fields[2..].join(" ");
                let inner = tuple
                    .strip_prefix('(')
                    .and_then(|value| value.strip_suffix(')'))
                    .ok_or_else(|| {
                        anyhow!("line {line_number}: position assignment requires '(X, Y)'")
                    })?;
                let (x, y) = inner.split_once(',').ok_or_else(|| {
                    anyhow!("line {line_number}: position assignment requires '(X, Y)'")
                })?;
                if y.contains(',') {
                    bail!("line {line_number}: position assignment requires exactly two values");
                }
                return Ok(format!(
                    "POSITION {} {} {}",
                    fields[0],
                    decimal_word_to_canonical(x.trim(), line_number, "X coordinate")?,
                    decimal_word_to_canonical(y.trim(), line_number, "Y coordinate")?
                ));
            }
            if let Some(statement) = normalize_modern_timer_expression(&fields, false, line_number)?
            {
                return Ok(statement);
            }
            if let Some(statement) = normalize_modern_bit_expression(&fields, false, line_number)? {
                return Ok(statement);
            }
            if let Some(statement) =
                normalize_modern_record_expression(&fields, false, line_number)?
            {
                return Ok(statement);
            }
            if fields.len() == 3 && matches!(fields[1], "=" | "+=" | "-=") {
                return normalize_modern_shared_expression(&fields, false, line_number);
            }
            let canonical_name = if command == "halt" {
                "END".to_string()
            } else {
                command.to_ascii_uppercase()
            };
            let args = fields[1..]
                .iter()
                .map(|value| modern_operand_to_canonical(value, line_number))
                .collect::<Result<Vec<_>>>()?;
            if args.is_empty() {
                Ok(canonical_name)
            } else {
                Ok(format!("{canonical_name} {}", args.join(" ")))
            }
        }
    }
}

fn normalize_modern_sequence_slot_assignment(
    fields: &[&str],
    line_number: usize,
) -> Result<Option<String>> {
    let Some(slot_text) = fields
        .first()
        .and_then(|value| value.strip_prefix("sequence_slots["))
        .and_then(|value| value.strip_suffix(']'))
    else {
        return Ok(None);
    };
    if fields.len() != 3 || fields[1] != "=" {
        bail!("line {line_number}: expected 'sequence_slots[1..6] = \"NAME\"'");
    }
    let slot = slot_text.parse::<u8>().map_err(|_| {
        anyhow!("line {line_number}: sequence slot {slot_text:?} must be a decimal index")
    })?;
    if !(1..=6).contains(&slot) {
        bail!("line {line_number}: sequence slot must be in 1..6, found {slot}");
    }
    let name = parse_simple_ascii(fields[2], line_number, "sequence name")?;
    if name.len() > 15 {
        bail!(
            "line {line_number}: sequence name is {} bytes; a 16-byte native slot allows at most 15 plus NUL",
            name.len()
        );
    }
    Ok(Some(format!("CHARACTER_SLOT {slot:02X} {}", fields[2])))
}

fn normalize_modern_presentation_expression(
    fields: &[&str],
    line_number: usize,
) -> Result<Option<String>> {
    if fields.first() != Some(&"presentation") {
        return Ok(None);
    }
    if fields.len() != 3 {
        bail!("line {line_number}: expected 'require presentation ==|!= ACTOR'");
    }
    let inverted = match fields[1] {
        "==" => "0",
        "!=" => "1",
        operator => bail!(
            "line {line_number}: presentation comparison requires == or !=, found {operator:?}"
        ),
    };
    validate_identifier(fields[2], line_number)?;
    Ok(Some(format!("ACTOR {}.action blood {inverted}", fields[2])))
}

fn transfer_holder_to_object(value: &str) -> &str {
    if value == "aboard" { "blood" } else { value }
}

fn parse_modern_u16(value: &str, line_number: usize, field: &str) -> Result<u16> {
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u16::from_str_radix(hex, 16)
            .map_err(|_| anyhow!("line {line_number}: invalid hexadecimal {field} {value:?}"))
    } else {
        value
            .parse::<u16>()
            .map_err(|_| anyhow!("line {line_number}: invalid decimal {field} {value:?}"))
    }
}

fn parse_timer_index(value: &str, line_number: usize) -> Result<Option<u8>> {
    let Some(index) = value.strip_prefix("timer[") else {
        return Ok(None);
    };
    let index = index
        .strip_suffix(']')
        .ok_or_else(|| anyhow!("line {line_number}: timer reference must end with ']'"))?;
    let index = parse_modern_u16(index, line_number, "timer index")?;
    if index >= 0x1E {
        bail!("line {line_number}: timer index {index} is outside the ISR-managed range 0..29");
    }
    Ok(Some(index as u8))
}

fn normalize_modern_timer_expression(
    fields: &[&str],
    query: bool,
    line_number: usize,
) -> Result<Option<String>> {
    let Some(target) = fields.first() else {
        return Ok(None);
    };
    let Some(index) = parse_timer_index(target, line_number)? else {
        return Ok(None);
    };
    if fields.len() != 3 || !matches!(fields[1], "=" | "==") {
        bail!(
            "line {line_number}: expected 'require timer[{index}] == 0' or 'timer[{index}] = VALUE'"
        );
    }
    if query {
        if fields[1] != "==" || fields[2] != "0" {
            bail!("line {line_number}: timer conditions can only require expiry with '== 0'");
        }
        return Ok(Some(format!("STATE_ARRAY_TEST {index:02X}")));
    }
    if fields[1] != "=" {
        bail!("line {line_number}: timer updates require '='");
    }
    let value = if fields[2] == "disabled" {
        0xFFFF
    } else {
        let value = parse_modern_u16(fields[2], line_number, "timer value")?;
        if value > i16::MAX as u16 {
            bail!(
                "line {line_number}: negative-class timer values do not count down; use \
                 'disabled' for 0xFFFF or state_array_set for an exact raw value"
            );
        }
        value
    };
    Ok(Some(format!("STATE_ARRAY_SET {index:02X} {value:04X}")))
}

fn normalize_modern_choice_expression(
    fields: &[&str],
    line_number: usize,
) -> Result<Option<String>> {
    if fields.first() != Some(&"choice") {
        return Ok(None);
    }
    if fields.len() != 3 {
        bail!("line {line_number}: expected 'require choice ==|!= WORD'");
    }
    let inverted = match fields[1] {
        "==" => false,
        "!=" => true,
        operator => bail!(
            "line {line_number}: choice comparison operator must be '==' or '!=', found {operator:?}"
        ),
    };
    Ok(Some(format!(
        "CONCEPT_GUARD {} {}",
        modern_operand_to_canonical(fields[2], line_number)?,
        bool_digit(inverted)
    )))
}

fn normalize_modern_clock_expression(
    fields: &[&str],
    line_number: usize,
) -> Result<Option<String>> {
    if fields.first() == Some(&"clock.hour") {
        if fields.len() != 3 {
            bail!("line {line_number}: expected 'require clock.hour OP HOUR'");
        }
        let operator = modern_rtc_operator(fields[1], line_number)?;
        let hour = fields[2]
            .parse::<u16>()
            .map_err(|_| anyhow!("line {line_number}: clock hour must be decimal"))?;
        if hour > 23 {
            bail!("line {line_number}: clock hour {hour} is outside 0..23");
        }
        return Ok(Some(format!(
            "GLOBAL_WORD_COMPARE {operator} C1 {hour:04X}"
        )));
    }
    if fields.first() != Some(&"annual_date") {
        return Ok(None);
    }
    if fields.len() != 3 {
        bail!("line {line_number}: expected 'require annual_date OP YYYY-MM-DD'");
    }
    let operator = modern_rtc_operator(fields[1], line_number)?;
    let mut parts = fields[2].split('-');
    let year = parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| anyhow!("line {line_number}: date year must be decimal u16"))?;
    let month = parts
        .next()
        .and_then(|value| value.parse::<u8>().ok())
        .ok_or_else(|| anyhow!("line {line_number}: date month must be decimal"))?;
    let day = parts
        .next()
        .and_then(|value| value.parse::<u8>().ok())
        .ok_or_else(|| anyhow!("line {line_number}: date day must be decimal"))?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        bail!("line {line_number}: invalid calendar date {:?}", fields[2]);
    }
    let month_day = u16::from(month) << 8 | u16::from(day);
    Ok(Some(format!(
        "GLOBAL_PAIR_COMPARE {operator} {month_day:04X} {year:04X}"
    )))
}

fn modern_rtc_operator(operator: &str, line_number: usize) -> Result<&'static str> {
    match operator {
        "<" => Ok("F1"),
        ">" => Ok("F2"),
        "==" => Ok("F5"),
        _ => bail!("line {line_number}: RTC comparison operator must be '<', '>', or '=='"),
    }
}

fn normalize_modern_bit_expression(
    fields: &[&str],
    query: bool,
    line_number: usize,
) -> Result<Option<String>> {
    let opcode = "AE";
    let (target, inverted) = if query {
        if fields.len() != 1 {
            return Ok(None);
        }
        let (inverted, target) = fields[0]
            .strip_prefix('!')
            .map_or((false, fields[0]), |target| (true, target));
        (target, inverted)
    } else {
        if fields.len() != 3 || fields[1] != "=" {
            return Ok(None);
        }
        let inverted = match fields[2] {
            "true" => false,
            "false" => true,
            _ => return Ok(None),
        };
        (fields[0], inverted)
    };
    let Some((field, mask)) = modern_bit_target_to_canonical(target, line_number)? else {
        return Ok(None);
    };
    Ok(Some(format!(
        "SHARED_BIT_STATE {opcode} {field} {mask} {}",
        bool_digit(inverted)
    )))
}

fn normalize_modern_marked_bit_expression(
    fields: &[&str],
    query: bool,
    line_number: usize,
) -> Result<String> {
    let connector = if query { "is" } else { "as" };
    if fields.len() != 4 || fields[2] != connector {
        bail!(
            "line {line_number}: expected '{} TARGET {connector} STATE'",
            if query { "check" } else { "mark" }
        );
    }
    let (target, inverted) = match fields[3] {
        "active" => (format!("{}.active", fields[1]), false),
        "inactive" => (format!("{}.active", fields[1]), true),
        "known" => (format!("{}.known", fields[1]), false),
        "unknown" => (format!("{}.known", fields[1]), true),
        "portable" => (format!("{}.portable", fields[1]), false),
        "not_portable" => (format!("{}.portable", fields[1]), true),
        "set" if fields[1].starts_with("bits(") => (fields[1].to_string(), false),
        "clear" if fields[1].starts_with("bits(") => (fields[1].to_string(), true),
        state => bail!(
            "line {line_number}: bit state must be active, inactive, known, unknown, portable, not_portable, set, or clear; found {state:?}"
        ),
    };
    let (field, mask) = modern_bit_target_to_canonical(&target, line_number)?
        .ok_or_else(|| anyhow!("line {line_number}: {target:?} is not a boolean state field"))?;
    Ok(format!(
        "SHARED_BIT_STATE B0 {field} {mask} {}",
        bool_digit(inverted)
    ))
}

fn modern_bit_target_to_canonical(
    value: &str,
    line_number: usize,
) -> Result<Option<(String, String)>> {
    for (suffix, mask) in [
        (".active", "0001"),
        (".known", "0002"),
        (".portable", "0020"),
    ] {
        if let Some(owner) = value.strip_suffix(suffix) {
            return Ok(Some((
                modern_operand_to_canonical(&format!("{owner}.flags"), line_number)?,
                mask.to_string(),
            )));
        }
    }
    let Some(inner) = value
        .strip_prefix("bits(")
        .and_then(|value| value.strip_suffix(')'))
    else {
        return Ok(None);
    };
    let Some((field, mask)) = inner.split_once(',') else {
        bail!("line {line_number}: bits(FIELD,MASK) requires two operands");
    };
    Ok(Some((
        modern_operand_to_canonical(field, line_number)?,
        modern_operand_to_canonical(mask, line_number)?,
    )))
}

fn normalize_modern_record_expression(
    fields: &[&str],
    query: bool,
    line_number: usize,
) -> Result<Option<String>> {
    if fields.len() != 3 {
        return Ok(None);
    }
    let Some((record, topic)) = modern_record_target_to_canonical(fields[0], line_number)? else {
        return Ok(None);
    };
    let inverted = match (query, fields[1]) {
        (true, "==") => false,
        (true, "!=") => true,
        (false, "=") => false,
        _ => return Ok(None),
    };
    if query && topic {
        bail!("line {line_number}: actor topics are published, not queried, by shipped bytecode");
    }
    let opcode = if topic { "BC" } else { "AF" };
    let value = if fields[2] == "aboard" {
        "FFFF".to_string()
    } else {
        modern_operand_to_canonical(fields[2], line_number)?
    };
    Ok(Some(format!(
        "RECORD_WILDCARD {opcode} {record} {value} {}",
        bool_digit(inverted)
    )))
}

fn modern_record_target_to_canonical(
    value: &str,
    line_number: usize,
) -> Result<Option<(String, bool)>> {
    if value.ends_with(".topic") {
        return Ok(Some((
            modern_operand_to_canonical(value, line_number)?,
            true,
        )));
    }
    if let Some(inner) = bracketed_operand(value, "topic") {
        return Ok(Some((
            modern_operand_to_canonical(inner, line_number)?,
            true,
        )));
    }
    if value.ends_with(".current_location") || value.ends_with(".holder") {
        return Ok(Some((
            modern_operand_to_canonical(value, line_number)?,
            false,
        )));
    }
    Ok(bracketed_operand(value, "record")
        .map(|inner| modern_operand_to_canonical(inner, line_number).map(|inner| (inner, false)))
        .transpose()?)
}

fn normalize_modern_shared_expression(
    fields: &[&str],
    query: bool,
    line_number: usize,
) -> Result<String> {
    if fields.len() != 3 {
        bail!(
            "line {line_number}: expected {}LEFT OPERATOR RIGHT",
            if query { "'require " } else { "" }
        );
    }
    let (opcode, left) = modern_shared_target_to_canonical(fields[0], line_number)?;
    let operator = match (query, fields[1]) {
        (true, "!=") => "F0",
        (true, "<") => "F1",
        (true, ">") => "F2",
        (true, "<=") => "F3",
        (true, ">=") => "F4",
        (true, "==") => "F5",
        (false, "=") => "F5",
        (false, "+=") => "F6",
        (false, "-=") => "F7",
        _ => bail!(
            "line {line_number}: operator {:?} is not valid for a {} expression",
            fields[1],
            if query { "requirement" } else { "state update" }
        ),
    };
    let (rhs_mode, rhs) = modern_shared_rhs_to_canonical(fields[2], line_number)?;
    Ok(format!(
        "SHARED_STATE {opcode} {left} {operator} {rhs_mode} {rhs}"
    ))
}

fn modern_shared_target_to_canonical(
    value: &str,
    line_number: usize,
) -> Result<(&'static str, String)> {
    if let Some(inner) = bracketed_operand(value, "state") {
        return Ok(("C0", modern_operand_to_canonical(inner, line_number)?));
    }
    if bracketed_operand(value, "globals").is_some() {
        return Ok(("C0", value.to_string()));
    }
    if value.starts_with("globals.") {
        return Ok(("C0", modern_operand_to_canonical(value, line_number)?));
    }
    if value.ends_with(".aggressiveness") {
        return Ok(("B4", modern_operand_to_canonical(value, line_number)?));
    }
    if value.ends_with(".encounter_count") {
        return Ok(("BF", modern_operand_to_canonical(value, line_number)?));
    }
    if let Some(inner) = bracketed_operand(value, "aggressiveness") {
        return Ok(("B4", modern_operand_to_canonical(inner, line_number)?));
    }
    if let Some(inner) = bracketed_operand(value, "encounter_count") {
        return Ok(("BF", modern_operand_to_canonical(inner, line_number)?));
    }
    bail!(
        "line {line_number}: shared-state target must be state[ADDRESS], OBJECT.aggressiveness, or OBJECT.encounter_count"
    )
}

fn modern_shared_rhs_to_canonical(
    value: &str,
    line_number: usize,
) -> Result<(&'static str, String)> {
    if let Some(inner) = bracketed_operand(value, "state") {
        return Ok(("C0", modern_operand_to_canonical(inner, line_number)?));
    }
    if bracketed_operand(value, "globals").is_some() {
        return Ok(("C0", value.to_string()));
    }
    if value.contains('.') {
        return Ok(("C0", modern_operand_to_canonical(value, line_number)?));
    }
    Ok((
        "C1",
        decimal_word_to_canonical(value, line_number, "state value")?,
    ))
}

fn decimal_word_to_canonical(value: &str, line_number: usize, field: &str) -> Result<String> {
    Ok(format!(
        "{:04X}",
        parse_modern_u16(value, line_number, field)?
    ))
}

fn bracketed_operand<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    value
        .strip_prefix(prefix)
        .and_then(|value| value.strip_prefix('['))
        .and_then(|value| value.strip_suffix(']'))
        .filter(|value| !value.is_empty())
}

fn normalize_modern_fixed(
    canonical_name: &str,
    args: &[&str],
    expected: usize,
    line_number: usize,
) -> Result<String> {
    if args.len() != expected {
        bail!(
            "line {line_number}: {canonical_name} expects {expected} argument(s), got {}",
            args.len()
        );
    }
    if args.is_empty() {
        Ok(canonical_name.to_string())
    } else {
        Ok(format!("{canonical_name} {}", args.join(" ")))
    }
}

fn normalize_modern_text(fields: &[&str], line_number: usize) -> Result<String> {
    if fields.len() < 8 {
        bail!(
            "line {line_number}: text expects OBJECT, five named controls, ':', and optional words"
        );
    }
    let expected_names = ["voice", "flags", "display", "loop", "control"];
    let mut controls = Vec::with_capacity(expected_names.len());
    for (field, expected_name) in fields[2..7].iter().zip(expected_names) {
        let Some((name, value)) = field.split_once('=') else {
            bail!("line {line_number}: expected {expected_name}=VALUE in text statement");
        };
        if name != expected_name {
            bail!("line {line_number}: expected {expected_name}=VALUE, found {name}=VALUE");
        }
        controls.push(modern_operand_to_canonical(value, line_number)?);
    }
    if fields[7] != ":" {
        bail!("line {line_number}: expected ':' before text words");
    }
    let mut args = vec![modern_operand_to_canonical(fields[1], line_number)?];
    args.extend(controls);
    args.extend(
        fields[8..]
            .iter()
            .map(|value| modern_operand_to_canonical(value, line_number))
            .collect::<Result<Vec<_>>>()?,
    );
    Ok(format!("TEXT {}", args.join(" ")))
}

fn normalize_modern_say(
    fields: &[&str],
    line_number: usize,
    lexicon: &DictionaryPhraseLexicon,
) -> Result<String> {
    if fields.len() < 5 {
        bail!(
            "line {line_number}: say expects OBJECT, presentation=LINE, ':', and a quoted phrase"
        );
    }
    let separator = fields
        .iter()
        .position(|field| *field == ":")
        .ok_or_else(|| anyhow!("line {line_number}: expected ':' before dialogue phrase"))?;
    if separator + 1 >= fields.len() {
        bail!("line {line_number}: say has no quoted phrase");
    }

    let mut presentation = None;
    let mut flags_b4 = 0u8;
    let mut flags_b5 = crate::vm::TEXT_ACTIVE_DISPLAY_FLAG;
    let mut loop_target = None;
    let mut control_word = None;
    let mut recent_choice_requirement: Option<&str> = None;
    let mut modifier = 2usize;
    while modifier < separator {
        let field = fields[modifier];
        if let Some(value) = field.strip_prefix("presentation=") {
            let line_id = value.parse::<i16>().map_err(|_| {
                anyhow!("line {line_number}: presentation line must be a decimal integer")
            })?;
            let selector = line_id
                .checked_sub(crate::vm::DLG_LINE_ID_BIAS)
                .ok_or_else(|| {
                    anyhow!("line {line_number}: presentation line is outside the selector range")
                })?;
            if !(-128..=127).contains(&selector) {
                bail!("line {line_number}: presentation line is outside the selector range");
            }
            presentation = Some(selector as i8 as u8);
        } else if field == "chatter" {
            flags_b4 |= 0x20;
        } else if field == "repeatable" {
            flags_b4 |= crate::vm::TEXT_PRESERVE_ACTIVE_FLAG;
        } else if field == "chance=20%" {
            flags_b4 |= 0x02;
        } else if field == "if_not_shown" {
            let value = fields
                .get(modifier + 1)
                .and_then(|field| field.strip_prefix("skip_next="))
                .ok_or_else(|| {
                    anyhow!("line {line_number}: expected 'if_not_shown skip_next=COUNT'")
                })?;
            let count = value
                .parse::<u8>()
                .map_err(|_| anyhow!("line {line_number}: skip count must be a decimal integer"))?;
            if !(1..=8).contains(&count) {
                bail!("line {line_number}: skip count must be between 1 and 8");
            }
            flags_b4 |= crate::vm::TEXT_CONDITIONAL_SKIP_FLAG;
            flags_b5 |= (count - 1) << 4;
            modifier += 1;
        } else if let Some(value) = field.strip_prefix("resume_at=") {
            loop_target = Some(modern_operand_to_canonical(value, line_number)?);
            flags_b4 |= crate::vm::TEXT_LOOP_TARGET_FLAG;
        } else if field == "when" {
            if fields.get(modifier + 1) == Some(&"aggressiveness")
                && fields.get(modifier + 2) == Some(&"==")
            {
                let value = fields.get(modifier + 3).ok_or_else(|| {
                    anyhow!("line {line_number}: conversation-progress condition has no value")
                })?;
                let value = value.parse::<u16>().map_err(|_| {
                    anyhow!("line {line_number}: conversation-progress value must be decimal")
                })?;
                flags_b4 |= crate::vm::TEXT_EXTRA_CONTROL_WORD_FLAG;
                // b5 bits 1..3 select field 3; bit 0 selects equality.
                flags_b5 |= 0x05;
                control_word = Some(format!("{value:04X}"));
                modifier += 3;
            } else if fields.get(modifier + 1) == Some(&"last_8_choices")
                && fields.get(modifier + 2) == Some(&"count")
                && fields.get(modifier + 4) == Some(&">=")
            {
                let choice = fields.get(modifier + 3).ok_or_else(|| {
                    anyhow!("line {line_number}: recent-choice condition has no choice")
                })?;
                let count = fields
                    .get(modifier + 5)
                    .ok_or_else(|| {
                        anyhow!("line {line_number}: recent-choice condition has no count")
                    })?
                    .parse::<u8>()
                    .map_err(|_| {
                        anyhow!("line {line_number}: recent-choice count must be decimal")
                    })?;
                if !(1..=7).contains(&count) {
                    bail!("line {line_number}: recent-choice count must be between 1 and 7");
                }
                flags_b4 |= 0x40;
                flags_b5 |= count;
                recent_choice_requirement = Some(choice);
                modifier += 5;
            } else {
                bail!(
                    "line {line_number}: expected 'when aggressiveness == VALUE' or \
                     'when last_8_choices count WORD >= COUNT'"
                );
            }
        } else {
            bail!("line {line_number}: unknown say modifier {field:?}");
        }
        modifier += 1;
    }
    let presentation = presentation
        .ok_or_else(|| anyhow!("line {line_number}: say is missing presentation=LINE"))?;

    let phrase: String = serde_json::from_str(fields[separator + 1])
        .map_err(|_| anyhow!("line {line_number}: dialogue phrase must be a quoted string"))?;
    let word_offsets = lexicon.tokenize(&phrase).ok_or_else(|| {
        anyhow!(
            "line {line_number}: dialogue phrase does not have one exact companion-dictionary tokenization"
        )
    })?;

    let choices = fields.get(separator + 2..).unwrap_or_default();
    if !choices.is_empty() {
        if choices[0] != "choices" || choices.len() == 1 {
            bail!("line {line_number}: expected 'choices WORD...' after dialogue phrase");
        }
        if loop_target.is_none() {
            flags_b4 |= 0x40;
        }
    }
    if let Some(required_choice) = recent_choice_requirement {
        if choices != ["choices", required_choice] {
            bail!(
                "line {line_number}: recent-choice condition must name the line's sole trailing choice"
            );
        }
    }

    let mut args = vec![
        modern_operand_to_canonical(fields[1], line_number)?,
        format!("{presentation:02X}"),
        format!("{flags_b4:02X}"),
        format!("{flags_b5:02X}"),
        loop_target.unwrap_or_else(|| "-".to_string()),
        control_word.unwrap_or_else(|| "-".to_string()),
    ];
    args.extend(word_offsets.iter().map(|offset| format!("{offset:04X}")));
    if !choices.is_empty() {
        args.push("FFFF".to_string());
        args.extend(
            choices[1..]
                .iter()
                .map(|value| modern_operand_to_canonical(value, line_number))
                .collect::<Result<Vec<_>>>()?,
        );
    }
    Ok(format!("TEXT {}", args.join(" ")))
}

fn modern_operand_to_canonical(value: &str, line_number: usize) -> Result<String> {
    let value = value.strip_suffix(',').unwrap_or(value);
    match value {
        "none" => return Ok("-".to_string()),
        "true" => return Ok("1".to_string()),
        "false" => return Ok("0".to_string()),
        _ => {}
    }
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        if hex.is_empty() || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("line {line_number}: invalid hexadecimal operand {value:?}");
        }
        return Ok(hex.to_ascii_uppercase());
    }
    if value.starts_with('"') {
        if let Some(index) = value.rfind("@0x").or_else(|| value.rfind("@0X")) {
            let (quoted, suffix) = value.split_at(index);
            let hex = &suffix[3..];
            if hex.is_empty() || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                bail!("line {line_number}: invalid dictionary offset {suffix:?}");
            }
            return Ok(format!("{quoted}@{}", hex.to_ascii_uppercase()));
        }
    }
    Ok(value.to_string())
}

fn canonical_operand_to_modern(value: &str) -> String {
    if value == "-" {
        return "none".to_string();
    }
    if value.starts_with('"') {
        if let Some(index) = value.rfind('@') {
            let (quoted, suffix) = value.split_at(index);
            let hex = &suffix[1..];
            if !hex.is_empty() && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return format!("{quoted}@0x{hex}");
            }
        }
        return value.to_string();
    }
    if looks_like_canonical_hex(value) {
        return format!("0x{value}");
    }
    value.to_string()
}

fn is_hnm_sequence_atom(value: &str) -> bool {
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return false;
    };
    value.len() <= 20
        && value.len() > 4
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_graphic() && !matches!(*byte, b'"' | b'\\' | b'/'))
        && value[value.len() - 4..].eq_ignore_ascii_case(".hnm")
}

fn looks_like_canonical_hex(value: &str) -> bool {
    matches!(value.len(), 2 | 4 | 8)
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && value
            .bytes()
            .any(|byte| byte.is_ascii_digit() || byte.is_ascii_uppercase())
}

fn modern_statement(
    statement: &str,
    line_number: usize,
    dictionary: &HashMap<u16, String>,
    lexicon: &DictionaryPhraseLexicon,
    query_mode: bool,
) -> Result<String> {
    let fields = split_source_fields(statement, line_number)?;
    let name = fields
        .first()
        .copied()
        .ok_or_else(|| anyhow!("line {line_number}: missing statement"))?;
    let args = &fields[1..];
    match name {
        "OBJECT" => Ok(format!(
            "object {} = {}",
            args[0],
            canonical_operand_to_modern(args[1])
        )),
        "FIELD" => Ok(format!(
            "field {} = {} + {}",
            args[0],
            args[1],
            canonical_operand_to_modern(args[2])
        )),
        "LABEL" => Ok(format!("{}:", args[0])),
        "PROCEDURE" => Ok(format!("proc {} {{", args[0])),
        "END_PROCEDURE" => Ok("}".to_string()),
        "WHEN" => Ok("when {".to_string()),
        "THEN" => Ok("} then {".to_string()),
        "ELSE" => Ok("} else {".to_string()),
        "END_WHEN" => Ok("}".to_string()),
        "SELECTOR_LIST" => Ok(format!("selector {} {{", args[0])),
        "END_SELECTOR_LIST" => Ok("}".to_string()),
        "CASE" => {
            let continuation = if args[1] == "0000" || args[1] == "0" {
                ""
            } else {
                " continues"
            };
            Ok(format!(
                "case {}{continuation} {{",
                canonical_operand_to_modern(args[0])
            ))
        }
        "TEXT" => {
            if args.len() < 6 {
                bail!("line {line_number}: malformed generated TEXT statement");
            }
            let raw_words = &args[6..];
            let separator = raw_words.iter().position(|value| *value == "FFFF");
            let phrase_words = separator.map_or(raw_words, |index| &raw_words[..index]);
            let exact_offsets = phrase_words
                .iter()
                .map(|value| canonical_dictionary_operand_offset(value, lexicon))
                .collect::<Option<Vec<_>>>();
            let phrase = exact_offsets
                .as_ref()
                .and_then(|offsets| lexicon.render_exact(offsets, dictionary));
            let phrase_is_exact = phrase
                .as_ref()
                .is_some_and(|phrase| lexicon.tokenize(phrase).as_ref() == exact_offsets.as_ref());
            let words = raw_words
                .iter()
                .map(|value| canonical_operand_to_modern(value))
                .collect::<Vec<_>>()
                .join(" ");
            let command = if phrase_is_exact {
                "say"
            } else {
                "text_tokens"
            };
            let selector = u8::from_str_radix(args[1], 16).map_err(|_| {
                anyhow!("line {line_number}: invalid generated presentation selector")
            })?;
            let flags_b4 = u8::from_str_radix(args[2], 16)
                .map_err(|_| anyhow!("line {line_number}: invalid generated text controls"))?;
            let flags_b5 = u8::from_str_radix(args[3], 16)
                .map_err(|_| anyhow!("line {line_number}: invalid generated text state"))?;
            if flags_b5 & crate::vm::TEXT_ACTIVE_DISPLAY_FLAG == 0 {
                bail!("line {line_number}: shipped dialogue line is not active");
            }
            let mut modifiers = vec![format!(
                "presentation={}",
                crate::vm::dlg_line_id_for_selector(selector)
            )];
            if flags_b4 & 0x20 != 0 {
                modifiers.push("chatter".to_string());
            }
            if flags_b4 & crate::vm::TEXT_PRESERVE_ACTIVE_FLAG != 0 {
                modifiers.push("repeatable".to_string());
            }
            if flags_b4 & 0x02 != 0 {
                modifiers.push("chance=20%".to_string());
            }
            if flags_b4 & crate::vm::TEXT_CONDITIONAL_SKIP_FLAG != 0 {
                let count = ((flags_b5 >> 4) & 0x07) + 1;
                modifiers.push(format!("if_not_shown skip_next={count}"));
            }
            if flags_b4 & crate::vm::TEXT_LOOP_TARGET_FLAG != 0 {
                modifiers.push(format!(
                    "resume_at={}",
                    canonical_operand_to_modern(args[4])
                ));
            } else if args[4] != "-" {
                bail!("line {line_number}: dialogue has a resume target without its flag");
            }
            if flags_b4 & crate::vm::TEXT_EXTRA_CONTROL_WORD_FLAG != 0 {
                if flags_b5 != 0x85 {
                    bail!(
                        "line {line_number}: dialogue predicate flags have no established source form"
                    );
                }
                let value = u16::from_str_radix(args[5], 16).map_err(|_| {
                    anyhow!("line {line_number}: invalid conversation-progress value")
                })?;
                modifiers.push(format!("when aggressiveness == {value}"));
            } else if args[5] != "-" {
                bail!("line {line_number}: dialogue has a predicate word without its flag");
            }
            if flags_b4 & 0x40 != 0 {
                let history = flags_b5 & 0x07;
                if history != 0 {
                    let choice = separator
                        .and_then(|separator| raw_words.get(separator + 1..))
                        .filter(|choices| choices.len() == 1)
                        .and_then(|choices| choices.first())
                        .ok_or_else(|| {
                            anyhow!(
                                "line {line_number}: recent-choice predicate requires exactly one trailing choice"
                            )
                        })?;
                    modifiers.push(format!(
                        "when last_8_choices count {} >= {history}",
                        canonical_operand_to_modern(choice)
                    ));
                }
            }
            let known_b4 = crate::vm::TEXT_PRESERVE_ACTIVE_FLAG
                | 0x02
                | crate::vm::TEXT_EXTRA_CONTROL_WORD_FLAG
                | crate::vm::TEXT_CONDITIONAL_SKIP_FLAG
                | crate::vm::TEXT_LOOP_TARGET_FLAG
                | 0x20
                | 0x40;
            if flags_b4 & !known_b4 != 0 {
                bail!("line {line_number}: dialogue controls contain unknown bits");
            }
            let payload_mask = if flags_b4 & crate::vm::TEXT_CONDITIONAL_SKIP_FLAG != 0 {
                0x70
            } else if flags_b4 & 0x40 != 0 {
                0x07
            } else if flags_b4 & crate::vm::TEXT_EXTRA_CONTROL_WORD_FLAG != 0 {
                0x07
            } else {
                0
            };
            if flags_b5 & !(crate::vm::TEXT_ACTIVE_DISPLAY_FLAG | payload_mask) != 0 {
                bail!("line {line_number}: dialogue state contains unknown bits");
            }
            let mut result = format!(
                "{command} {} {} :",
                canonical_operand_to_modern(args[0]),
                modifiers.join(" ")
            );
            if let Some(phrase) = phrase.filter(|_| phrase_is_exact) {
                result.push(' ');
                result.push_str(&serde_json::to_string(&phrase)?);
                if let Some(separator) = separator {
                    result.push_str(" choices");
                    for value in &raw_words[separator + 1..] {
                        result.push(' ');
                        result.push_str(&canonical_operand_to_modern(value));
                    }
                }
            } else if !words.is_empty() {
                result.push(' ');
                result.push_str(&words);
            }
            Ok(result)
        }
        "SHARED_STATE" => modern_shared_state(args, query_mode, line_number),
        "SHARED_BIT_STATE" => modern_shared_bit_state(args, query_mode, line_number),
        "RECORD_WILDCARD" => modern_record_wildcard(args, query_mode, line_number),
        "GLOBAL_WORD_COMPARE" => modern_rtc_hour_compare(args, line_number),
        "GLOBAL_PAIR_COMPARE" => modern_rtc_date_compare(args, line_number),
        "CONCEPT_GUARD" => {
            if args.len() != 2 || !matches!(args[1], "0" | "1") {
                bail!("line {line_number}: malformed generated CONCEPT_GUARD statement");
            }
            Ok(format!(
                "require choice {} {}",
                if args[1] == "1" { "!=" } else { "==" },
                canonical_operand_to_modern(args[0])
            ))
        }
        "CLEAR_ALTERNATE_CONCEPT" => {
            if !args.is_empty() {
                bail!("line {line_number}: malformed generated CLEAR_ALTERNATE_CONCEPT statement");
            }
            Ok("choice = none".to_string())
        }
        "ACTOR"
            if query_mode
                && args.len() == 3
                && args[1] == "blood"
                && matches!(args[2], "0" | "1")
                && args[0].ends_with(".action") =>
        {
            let actor = args[0]
                .strip_suffix(".action")
                .expect("suffix checked above");
            Ok(format!(
                "require presentation {} {actor}",
                if args[2] == "1" { "!=" } else { "==" }
            ))
        }
        "RECORD_LINK"
            if !query_mode
                && args.len() == 3
                && args[1] == "blood"
                && args[2] == "0"
                && args[0].ends_with(".action") =>
        {
            let actor = args[0]
                .strip_suffix(".action")
                .expect("suffix checked above");
            Ok(format!("queue presentation {actor}"))
        }
        "RECORD_CLEAR" if args.len() == 1 && args[0].ends_with(".action") => {
            let actor = args[0]
                .strip_suffix(".action")
                .expect("suffix checked above");
            Ok(format!("end presentation {actor}"))
        }
        "TRANSFER" => {
            if args.len() != 3 {
                bail!("line {line_number}: malformed generated TRANSFER statement");
            }
            for value in args {
                validate_identifier(value, line_number)?;
            }
            Ok(format!(
                "transfer {} from {} to {}",
                args[0],
                if args[1] == "blood" {
                    "aboard"
                } else {
                    args[1]
                },
                if args[2] == "blood" {
                    "aboard"
                } else {
                    args[2]
                }
            ))
        }
        "NAVIGATE" => {
            if args.len() != 1 {
                bail!("line {line_number}: malformed generated NAVIGATE statement");
            }
            validate_identifier(args[0], line_number)?;
            Ok(format!("navigate to {}", args[0]))
        }
        "BRING_ABOARD" => {
            if args.len() != 1 {
                bail!("line {line_number}: malformed generated BRING_ABOARD statement");
            }
            validate_identifier(args[0], line_number)?;
            Ok(format!("bring {} aboard", args[0]))
        }
        "REQUIRE_TRAVEL_THROUGH" => {
            if args.len() != 1 {
                bail!("line {line_number}: malformed generated REQUIRE_TRAVEL_THROUGH statement");
            }
            validate_identifier(args[0], line_number)?;
            Ok(format!("require travel through {}", args[0]))
        }
        "BLOOD_LINK" => {
            if args.len() != 3 || !matches!(args[2], "0" | "1") {
                bail!("line {line_number}: malformed generated BLOOD_LINK statement");
            }
            validate_field_identifier(args[0], line_number)?;
            validate_identifier(args[1], line_number)?;
            if query_mode {
                Ok(format!(
                    "require {}{} in {}",
                    args[1],
                    if args[2] == "1" { " not" } else { "" },
                    args[0]
                ))
            } else {
                Ok(format!(
                    "{} {} {}",
                    args[0],
                    if args[2] == "1" { "-=" } else { "+=" },
                    args[1]
                ))
            }
        }
        "POSITION" => {
            if args.len() != 3 {
                bail!("line {line_number}: malformed generated POSITION statement");
            }
            validate_field_identifier(args[0], line_number)?;
            Ok(format!(
                "{} = ({}, {})",
                args[0],
                canonical_word_to_decimal(args[1])?,
                canonical_word_to_decimal(args[2])?
            ))
        }
        "OFFER_TOPIC" => {
            if args.len() != 1 {
                bail!("line {line_number}: malformed generated OFFER_TOPIC statement");
            }
            Ok(format!(
                "offer topic {}",
                canonical_operand_to_modern(args[0])
            ))
        }
        "STATE_ARRAY_TEST" => modern_state_array_statement(false, args, line_number),
        "STATE_ARRAY_SET" => modern_state_array_statement(true, args, line_number),
        "LOAD_STRING" => {
            if args.len() != 1 {
                bail!("line {line_number}: malformed generated LOAD_STRING statement");
            }
            if is_hnm_sequence_atom(args[0]) {
                Ok(format!(
                    "request sequence {}",
                    canonical_operand_to_modern(args[0])
                ))
            } else {
                Ok(format!(
                    "load_string {}",
                    canonical_operand_to_modern(args[0])
                ))
            }
        }
        "CONDITIONAL_BLOCK" if matches!(args.first().copied(), Some("00" | "01")) => {
            if args.len() != 2 {
                bail!("line {line_number}: malformed generated CONDITIONAL_BLOCK statement");
            }
            Ok(format!(
                "activation {} until {}",
                if args[0] == "01" {
                    "enabled"
                } else {
                    "disabled"
                },
                canonical_operand_to_modern(args[1])
            ))
        }
        "SET_PROCEDURE_ENABLED" => {
            if args.len() != 2 || !matches!(args[1], "0" | "1") {
                bail!("line {line_number}: malformed generated SET_PROCEDURE_ENABLED statement");
            }
            Ok(format!(
                "{}.enabled = {}",
                args[0],
                if args[1] == "1" { "true" } else { "false" }
            ))
        }
        "CHARACTER_SLOT" => {
            if args.len() != 2 {
                bail!("line {line_number}: malformed generated CHARACTER_SLOT statement");
            }
            let slot = parse_byte(args[0], line_number, "sequence slot")?;
            let name = parse_simple_ascii(args[1], line_number, "sequence name")?;
            if (1..=6).contains(&slot) && name.len() <= 15 {
                Ok(format!("sequence_slots[{slot}] = {}", args[1]))
            } else {
                Ok(format!(
                    "character_slot {} {}",
                    canonical_operand_to_modern(args[0]),
                    canonical_operand_to_modern(args[1])
                ))
            }
        }
        "BRANCH_PRESENTATION" if args.is_empty() => Ok("during bridge".to_string()),
        "BRANCH_GAMEFLAG" if args.is_empty() => Ok("during travel".to_string()),
        "BRANCH_FLAG_274F" if args.is_empty() => Ok("during contact".to_string()),
        "RUN_PROFILE" if args.len() == 1 => {
            let profile = u8::from_str_radix(args[0], 16)
                .map_err(|_| anyhow!("line {line_number}: invalid generated profile number"))?;
            Ok(format!("run profile SCRIPT{profile}"))
        }
        "GUARD_POP" if args.is_empty() => Ok("then".to_string()),
        "END" => Ok("halt".to_string()),
        _ => {
            let args = args
                .iter()
                .map(|value| canonical_operand_to_modern(value))
                .collect::<Vec<_>>();
            let command = name.to_ascii_lowercase();
            if args.is_empty() {
                Ok(command)
            } else {
                Ok(format!("{command} {}", args.join(" ")))
            }
        }
    }
}

fn modern_state_array_statement(set: bool, args: &[&str], line_number: usize) -> Result<String> {
    let expected = if set { 2 } else { 1 };
    if args.len() != expected {
        bail!("line {line_number}: malformed generated state-array statement");
    }
    let index = u8::from_str_radix(args[0], 16)
        .map_err(|_| anyhow!("line {line_number}: invalid generated state-array index"))?;
    if index >= 0x1E {
        return Ok(if set {
            format!(
                "state_array_set {} {}",
                canonical_operand_to_modern(args[0]),
                canonical_operand_to_modern(args[1])
            )
        } else {
            format!("state_array_test {}", canonical_operand_to_modern(args[0]))
        });
    }
    if !set {
        return Ok(format!("require timer[{index}] == 0"));
    }
    let value = u16::from_str_radix(args[1], 16)
        .map_err(|_| anyhow!("line {line_number}: invalid generated state-array value"))?;
    if value == 0xFFFF {
        Ok(format!("timer[{index}] = disabled"))
    } else if value <= i16::MAX as u16 {
        Ok(format!("timer[{index}] = {value}"))
    } else {
        Ok(format!(
            "state_array_set {} {}",
            canonical_operand_to_modern(args[0]),
            canonical_operand_to_modern(args[1])
        ))
    }
}

fn modern_rtc_comparison_operator(operator: &str) -> Option<&'static str> {
    match operator {
        "F1" => Some("<"),
        "F2" => Some(">"),
        "F5" => Some("=="),
        _ => None,
    }
}

fn modern_rtc_hour_compare(args: &[&str], line_number: usize) -> Result<String> {
    if args.len() != 3 {
        bail!("line {line_number}: malformed generated GLOBAL_WORD_COMPARE statement");
    }
    let Some(operator) = modern_rtc_comparison_operator(args[0]) else {
        return Ok(format!(
            "global_word_compare {} {} {}",
            canonical_operand_to_modern(args[0]),
            canonical_operand_to_modern(args[1]),
            canonical_operand_to_modern(args[2])
        ));
    };
    if args[1] != "C1" {
        return Ok(format!(
            "global_word_compare {} {} {}",
            canonical_operand_to_modern(args[0]),
            canonical_operand_to_modern(args[1]),
            canonical_operand_to_modern(args[2])
        ));
    }
    let hour = u16::from_str_radix(args[2], 16)
        .map_err(|_| anyhow!("line {line_number}: invalid generated RTC hour"))?;
    if hour > 23 {
        return Ok(format!(
            "global_word_compare {} {} {}",
            canonical_operand_to_modern(args[0]),
            canonical_operand_to_modern(args[1]),
            canonical_operand_to_modern(args[2])
        ));
    }
    Ok(format!("require clock.hour {operator} {hour}"))
}

fn modern_rtc_date_compare(args: &[&str], line_number: usize) -> Result<String> {
    if args.len() != 3 {
        bail!("line {line_number}: malformed generated GLOBAL_PAIR_COMPARE statement");
    }
    let Some(operator) = modern_rtc_comparison_operator(args[0]) else {
        return Ok(format!(
            "global_pair_compare {} {} {}",
            canonical_operand_to_modern(args[0]),
            canonical_operand_to_modern(args[1]),
            canonical_operand_to_modern(args[2])
        ));
    };
    let month_day = u16::from_str_radix(args[1], 16)
        .map_err(|_| anyhow!("line {line_number}: invalid generated RTC month/day"))?;
    let year = u16::from_str_radix(args[2], 16)
        .map_err(|_| anyhow!("line {line_number}: invalid generated RTC year"))?;
    let month = (month_day >> 8) as u8;
    let day = month_day as u8;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Ok(format!(
            "global_pair_compare {} {} {}",
            canonical_operand_to_modern(args[0]),
            canonical_operand_to_modern(args[1]),
            canonical_operand_to_modern(args[2])
        ));
    }
    Ok(format!(
        "require annual_date {operator} {year:04}-{month:02}-{day:02}"
    ))
}

fn modern_shared_bit_state(args: &[&str], query: bool, line_number: usize) -> Result<String> {
    if args.len() != 4 {
        bail!("line {line_number}: malformed generated SHARED_BIT_STATE statement");
    }
    match args[0] {
        "AE" | "B0" => {}
        opcode => bail!(
            "line {line_number}: shared-bit opcode 0x{opcode} has no established shipped meaning"
        ),
    }
    let field = canonical_operand_to_modern(args[1]);
    let target = match (field.strip_suffix(".flags"), args[2]) {
        (Some(owner), "0001") => format!("{owner}.active"),
        (Some(owner), "0002") => format!("{owner}.known"),
        (Some(owner), "0020") => format!("{owner}.portable"),
        _ => format!("bits({},{})", field, canonical_operand_to_modern(args[2])),
    };
    let inverted = args[3] == "1";
    if !matches!(args[3], "0" | "1") {
        bail!("line {line_number}: shared-bit inversion must be 0 or 1");
    }
    if args[0] == "B0" {
        let (owner, state) = if let Some(owner) = target.strip_suffix(".active") {
            (owner, if inverted { "inactive" } else { "active" })
        } else if let Some(owner) = target.strip_suffix(".known") {
            (owner, if inverted { "unknown" } else { "known" })
        } else if let Some(owner) = target.strip_suffix(".portable") {
            (owner, if inverted { "not_portable" } else { "portable" })
        } else {
            (target.as_str(), if inverted { "clear" } else { "set" })
        };
        return Ok(if query {
            format!("check {owner} is {state}")
        } else {
            format!("mark {owner} as {state}")
        });
    }
    Ok(if query {
        format!("require {}{target}", if inverted { "!" } else { "" })
    } else {
        format!("{target} = {}", if inverted { "false" } else { "true" })
    })
}

fn modern_record_wildcard(args: &[&str], query: bool, line_number: usize) -> Result<String> {
    if args.len() != 4 {
        bail!("line {line_number}: malformed generated RECORD_WILDCARD statement");
    }
    let field = canonical_operand_to_modern(args[1]);
    let left = match args[0] {
        "AF" if field.ends_with(".current_location") || field.ends_with(".holder") => field,
        "BC" if field.ends_with(".topic") => field,
        "BC" => format!("topic[{field}]"),
        _ => format!("record[{field}]"),
    };
    let inverted = args[3] == "1";
    if !matches!(args[3], "0" | "1") {
        bail!("line {line_number}: record comparison inversion must be 0 or 1");
    }
    match args[0] {
        "AF" => {
            let right = if args[2] == "FFFF" {
                "aboard".to_string()
            } else {
                canonical_operand_to_modern(args[2])
            };
            Ok(if query || inverted {
                format!(
                    "require {left} {} {right}",
                    if inverted { "!=" } else { "==" }
                )
            } else {
                format!("{left} = {right}")
            })
        }
        "BC" if !query && !inverted => {
            Ok(format!("{left} = {}", canonical_operand_to_modern(args[2])))
        }
        "BC" => bail!("line {line_number}: shipped 0xBC publishes a topic only in update mode"),
        opcode => bail!(
            "line {line_number}: record-wildcard opcode 0x{opcode} has no established shipped source meaning"
        ),
    }
}

fn modern_shared_state(args: &[&str], query: bool, line_number: usize) -> Result<String> {
    if args.len() != 5 {
        bail!("line {line_number}: malformed generated SHARED_STATE statement");
    }
    let left = match args[0] {
        "B4" => modern_typed_shared_target(args[1], ".aggressiveness", "aggressiveness"),
        "BF" => modern_typed_shared_target(args[1], ".encounter_count", "encounter_count"),
        "C0" => format!("state[{}]", canonical_operand_to_modern(args[1])),
        opcode => bail!(
            "line {line_number}: shared-state opcode 0x{opcode} has not been assigned source semantics"
        ),
    };
    let query = query || matches!(args[2], "F0" | "F1" | "F2" | "F3" | "F4");
    let operator = match (query, args[2]) {
        (true, "F0") => "!=",
        (true, "F1") => "<",
        (true, "F2") => ">",
        (true, "F3") => "<=",
        (true, "F4") => ">=",
        (true, "F5") => "==",
        (false, "F5") => "=",
        (false, "F6") => "+=",
        (false, "F7") => "-=",
        (_, operator) => bail!(
            "line {line_number}: shared-state operator 0x{operator} is invalid in {} mode",
            if query { "query" } else { "update" }
        ),
    };
    let right = match args[3] {
        "C0" => format!("state[{}]", canonical_operand_to_modern(args[4])),
        "C1" => canonical_word_to_decimal(args[4])?,
        mode => bail!(
            "line {line_number}: shared-state RHS mode 0x{mode} has not been assigned source semantics"
        ),
    };
    Ok(format!(
        "{}{left} {operator} {right}",
        if query { "require " } else { "" }
    ))
}

fn canonical_word_to_decimal(value: &str) -> Result<String> {
    Ok(u16::from_str_radix(value, 16)
        .map_err(|_| anyhow!("invalid generated word {value:?}"))?
        .to_string())
}

fn modern_typed_shared_target(value: &str, suffix: &str, fallback: &str) -> String {
    let value = canonical_operand_to_modern(value);
    if value.ends_with(suffix) {
        value
    } else {
        format!("{fallback}[{value}]")
    }
}

fn canonical_dictionary_operand_offset(
    value: &str,
    lexicon: &DictionaryPhraseLexicon,
) -> Option<u16> {
    if let Some((quoted, offset)) = value.rsplit_once('@') {
        let _: String = serde_json::from_str(quoted).ok()?;
        return u16::from_str_radix(offset, 16).ok();
    }
    if value.starts_with('"') {
        let text: String = serde_json::from_str(value).ok()?;
        return lexicon.canonical_offsets.get(&text).copied();
    }
    u16::from_str_radix(value, 16).ok()
}

#[derive(Debug, Default)]
struct ModernProcedureLayout {
    labels: HashMap<String, usize>,
    natural_targets: HashMap<String, usize>,
}

fn modern_procedure_layout(source: &str) -> Result<ModernProcedureLayout> {
    let mut layout = ModernProcedureLayout::default();
    let mut procedures = Vec::new();
    let mut halts = Vec::new();

    for (line_index, original_line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = original_line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') {
            continue;
        }
        let code = code_before_comment(trimmed, line_number)?.trim();
        let Some((offset_text, statement)) = code.split_once(':') else {
            continue;
        };
        let offset = parse_hex_usize(offset_text.trim(), line_number, "generated offset")?;
        let fields = split_source_fields(statement.trim(), line_number)?;
        let Some(name) = fields.first().copied() else {
            continue;
        };
        match name {
            "LABEL" | "PROCEDURE" if fields.len() == 2 => {
                layout.labels.insert(fields[1].to_string(), offset);
                if name == "PROCEDURE" {
                    procedures.push((fields[1].to_string(), offset));
                }
            }
            "END" => halts.push(offset),
            _ => {}
        }
    }

    for (index, (name, _)) in procedures.iter().enumerate() {
        let target = procedures
            .get(index + 1)
            .map(|(_, offset)| *offset)
            .or_else(|| halts.last().copied());
        if let Some(target) = target {
            layout.natural_targets.insert(name.clone(), target);
        }
    }
    Ok(layout)
}

fn generated_address_offset(value: &str, labels: &HashMap<String, usize>) -> Option<usize> {
    labels
        .get(value)
        .copied()
        .or_else(|| usize::from_str_radix(value, 16).ok())
}

fn format_modern_source(source: &str, dictionary: &HashMap<u16, String>) -> Result<String> {
    let lexicon = DictionaryPhraseLexicon::new(dictionary);
    let procedure_layout = modern_procedure_layout(source)?;
    let mut output = String::new();
    let mut indent = 0usize;
    let mut selector_list_open = false;
    let mut selector_case_open = false;
    let mut query_mode = false;
    let mut pending_procedure = None;
    let mut procedure_condition_open = false;

    for (line_index, original_line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = original_line.trim();
        if trimmed.is_empty() {
            output.push('\n');
            continue;
        }
        if trimmed == "; BloodScript typed VM source" || trimmed.starts_with("; size:") {
            continue;
        }
        if trimmed == format!("; format: {READABLE_SOURCE_FORMAT}") {
            writeln!(output, "// format: {SOURCE_FORMAT}")?;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("; image:") {
            writeln!(output, "// image:{}", value)?;
            continue;
        }
        if trimmed.starts_with(';') {
            writeln!(output, "//{}", &trimmed[1..])?;
            continue;
        }

        let code = code_before_comment(trimmed, line_number)?.trim();
        let statement = code
            .split_once(':')
            .map_or(code, |(_, statement)| statement)
            .trim();
        let fields = split_source_fields(statement, line_number)?;
        let name = fields
            .first()
            .copied()
            .ok_or_else(|| anyhow!("line {line_number}: missing statement"))?;

        if name == "LABEL"
            && selector_case_open
            && fields
                .get(1)
                .is_some_and(|label| label.starts_with("selector_"))
        {
            continue;
        }
        if name == "YIELD_B" && selector_list_open {
            continue;
        }

        if name == "PROCEDURE" {
            if pending_procedure.replace(fields[1].to_string()).is_some() {
                bail!("line {line_number}: consecutive procedure declarations");
            }
            continue;
        }
        if let Some(procedure) = pending_procedure.take() {
            if !output.is_empty() && !output.ends_with("\n\n") {
                output.push('\n');
            }
            if name == "CONDITIONAL_BLOCK" && fields.len() == 3 && matches!(fields[1], "00" | "01")
            {
                let state = if fields[1] == "01" {
                    "enabled"
                } else {
                    "disabled"
                };
                let target = generated_address_offset(fields[2], &procedure_layout.labels);
                if target == procedure_layout.natural_targets.get(&procedure).copied() {
                    writeln!(output, "proc {procedure} {state} {{")?;
                } else {
                    writeln!(
                        output,
                        "proc {procedure} {state} until {} {{",
                        canonical_operand_to_modern(fields[2])
                    )?;
                }
                indent += 1;
                procedure_condition_open = true;
                query_mode = true;
                continue;
            }
            writeln!(output, "proc {procedure} {{")?;
            indent += 1;
        }

        if name == "SELECTOR_LIST" && !output.is_empty() && !output.ends_with("\n\n") {
            output.push('\n');
        }

        match name {
            "END_PROCEDURE" | "THEN" | "ELSE" | "END_WHEN" => {
                indent = indent.saturating_sub(1);
            }
            "GUARD_POP" if procedure_condition_open => {
                indent = indent.saturating_sub(1);
            }
            "CASE" => {
                if selector_case_open {
                    indent = indent.saturating_sub(1);
                    writeln!(output, "{}}}", "    ".repeat(indent))?;
                    selector_case_open = false;
                }
            }
            "END_SELECTOR_LIST" => {
                if selector_case_open {
                    indent = indent.saturating_sub(1);
                    writeln!(output, "{}}}", "    ".repeat(indent))?;
                    selector_case_open = false;
                }
                indent = indent.saturating_sub(1);
                selector_list_open = false;
            }
            _ => {}
        }

        let rendered = if name == "GUARD_POP" && procedure_condition_open {
            "} then {".to_string()
        } else {
            modern_statement(statement, line_number, dictionary, &lexicon, query_mode)?
        };
        write!(output, "{}{rendered}", "    ".repeat(indent))?;
        let useful_comment = if name == "FIELD" || name == "RAW" {
            comment_after_code(trimmed, line_number)?.map(str::trim)
        } else {
            trimmed
                .find("unstructured_guard=")
                .map(|start| trimmed[start..].trim())
        };
        if let Some(comment) = useful_comment.filter(|comment| !comment.is_empty()) {
            write!(output, " // {comment}")?;
        }
        output.push('\n');

        match name {
            "WHEN" | "THEN" | "ELSE" | "SELECTOR_LIST" => {
                if name == "SELECTOR_LIST" {
                    selector_list_open = true;
                }
                indent += 1;
            }
            "GUARD_POP" if procedure_condition_open => {
                indent += 1;
                procedure_condition_open = false;
            }
            "CASE" => {
                indent += 1;
                selector_case_open = true;
            }
            _ => {}
        }
        match name {
            "WHEN" | "GUARD_PUSH" | "CONDITIONAL_BLOCK" => query_mode = true,
            "THEN" | "ELSE" | "GUARD_POP" => query_mode = false,
            _ => {}
        }
        if matches!(name, "END_PROCEDURE" | "END_SELECTOR_LIST") {
            output.push('\n');
        }
    }
    while output.ends_with("\n\n") {
        output.pop();
    }
    if let Some(procedure) = pending_procedure {
        bail!("procedure {procedure:?} has no body");
    }
    Ok(remove_unreferenced_flow_labels(&output))
}

fn remove_unreferenced_flow_labels(source: &str) -> String {
    let removable = source
        .lines()
        .filter_map(|line| line.trim().strip_suffix(':'))
        .filter(|label| {
            label.contains("_branch_")
                || label.contains("_jump_target_")
                || label.contains("_dialogue_resume_")
        })
        .filter(|label| identifier_occurrences(source, label) == 1)
        .collect::<HashSet<_>>();
    let mut output = source
        .lines()
        .filter(|line| {
            !line
                .trim()
                .strip_suffix(':')
                .is_some_and(|label| removable.contains(label))
        })
        .collect::<Vec<_>>()
        .join("\n");
    if source.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn identifier_occurrences(source: &str, identifier: &str) -> usize {
    source
        .match_indices(identifier)
        .filter(|(start, _)| {
            let before = source[..*start].chars().next_back();
            let after = source[*start + identifier.len()..].chars().next();
            !before.is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
                && !after
                    .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
        })
        .count()
}

fn comment_after_code(line: &str, line_number: usize) -> Result<Option<&str>> {
    let code = code_before_comment(line, line_number)?;
    Ok(line.get(code.len() + usize::from(code.len() < line.len())..))
}

fn decompile_cod(
    output: &mut String,
    image: &[u8],
    dictionary: &HashMap<u16, String>,
    symbols: &[DebSymbol],
    structured_source: bool,
    var: Option<&[u8]>,
) -> Result<BodyStats> {
    let tokens = vm::walk(image, 0, image.len());
    let annotations = cod_annotations(&tokens, image, symbols)?;
    let structured = if structured_source {
        structured_annotations(analyze_structured_guards("COD", image, symbols)?)
    } else {
        StructuredAnnotations::default()
    };
    let mut field_aliases = if structured_source {
        var.map(|var| field_aliases(tokens.iter().flat_map(object_operand_values), symbols, var))
            .unwrap_or_default()
    } else {
        BTreeMap::new()
    };
    let mut object_aliases = if structured_source {
        cod_object_aliases(&tokens, symbols)
    } else {
        BTreeMap::new()
    };
    add_field_owner_objects(&mut object_aliases, &field_aliases);
    let proven_statements = var
        .map(|var| proven_statement_offsets(&tokens, symbols, var, &field_aliases))
        .unwrap_or_default();
    add_proven_statement_objects(&mut object_aliases, &tokens, &proven_statements, symbols);
    simplify_alias_identifiers(&mut object_aliases, &mut field_aliases);
    let dictionary_aliases = dictionary_aliases(
        tokens.iter().flat_map(dictionary_operand_values),
        dictionary,
    );
    let mut dictionary_operands = DictionaryOperandFormatter::new(&dictionary_aliases, dictionary);
    emit_object_declarations(output, &object_aliases)?;
    emit_field_declarations(output, &field_aliases, &object_aliases)?;
    let mut cursor = 0usize;
    let mut stats = BodyStats {
        symbolic_labels: annotations.labels.len(),
        procedures: annotations.procedure_count,
        structured_guards: structured.starts.len(),
        unstructured_guards: structured.rejected.len(),
        guard_rejection_counts: guard_rejection_counts(&structured.rejected),
        object_aliases: object_aliases.len(),
        object_alias_uses: tokens
            .iter()
            .flat_map(|token| {
                semantic_object_operand_values(token, proven_statements.get(&token.offset()))
            })
            .filter(|value| object_aliases.contains_key(value))
            .count(),
        dictionary_offsets: dictionary_aliases.len(),
        dictionary_uses: tokens
            .iter()
            .flat_map(dictionary_operand_values)
            .filter(|value| dictionary_aliases.contains_key(value))
            .count(),
        field_aliases: field_aliases.len(),
        field_alias_uses: tokens
            .iter()
            .flat_map(object_operand_values)
            .filter(|value| field_aliases.contains_key(value))
            .count(),
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
            let false_target = region.else_offset.unwrap_or(region.end);
            writeln!(
                output,
                "{offset:08X}: WHEN {} ; GUARD_PUSH target=0x{:04X}",
                address_operand(false_target as u16, &annotations.labels),
                false_target
            )?;
        } else if structured.thens.contains_key(&offset) {
            writeln!(output, "{offset:08X}: THEN ; GUARD_POP")?;
        } else if let Some(start) = structured.elses.get(&offset) {
            let region = &structured.starts[start];
            let false_target = region
                .else_offset
                .expect("ELSE annotation requires an else offset");
            writeln!(
                output,
                "{offset:08X}: ELSE {} {} ; JUMP target=0x{:04X}",
                address_operand(false_target as u16, &annotations.labels),
                address_operand(region.end as u16, &annotations.labels),
                region.end
            )?;
        } else {
            emit_token(
                output,
                &token,
                dictionary,
                &annotations.labels,
                &annotations.procedure_labels,
                &object_aliases,
                &field_aliases,
                &mut dictionary_operands,
                structured.rejected.get(&offset),
                proven_statements.get(&offset).copied(),
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
    symbols: &[DebSymbol],
    var: Option<&[u8]>,
    graph: Option<&BasControlFlow>,
) -> Result<BodyStats> {
    let vm_tokens = bas_vm_tokens(image, dictionary);
    let mut field_aliases = var
        .map(|var| {
            field_aliases(
                vm_tokens.iter().flat_map(object_operand_values),
                symbols,
                var,
            )
        })
        .unwrap_or_default();
    let mut object_aliases = if graph.is_some() {
        cod_object_aliases(&vm_tokens, symbols)
    } else {
        BTreeMap::new()
    };
    add_field_owner_objects(&mut object_aliases, &field_aliases);
    let proven_statements = var
        .map(|var| proven_statement_offsets(&vm_tokens, symbols, var, &field_aliases))
        .unwrap_or_default();
    add_proven_statement_objects(&mut object_aliases, &vm_tokens, &proven_statements, symbols);
    simplify_alias_identifiers(&mut object_aliases, &mut field_aliases);
    let dictionary_values = bas_dictionary_operand_values(image, dictionary);
    let dictionary_aliases = dictionary_aliases(dictionary_values.iter().copied(), dictionary);
    let mut dictionary_operands = DictionaryOperandFormatter::new(&dictionary_aliases, dictionary);
    emit_object_declarations(output, &object_aliases)?;
    emit_field_declarations(output, &field_aliases, &object_aliases)?;
    let annotations = bas_annotations(image, dictionary)?;
    let structured = bas_structured_annotations(graph);
    let mut cursor = 0usize;
    let mut raw_start = 0usize;
    let mut stats = BodyStats {
        symbolic_labels: annotations.labels.len(),
        structured_selector_lists: structured.starts.len(),
        structured_cases: structured.cases.len(),
        dictionary_offsets: dictionary_aliases.len(),
        dictionary_uses: dictionary_values
            .iter()
            .filter(|value| dictionary_aliases.contains_key(value))
            .count(),
        object_aliases: object_aliases.len(),
        object_alias_uses: vm_tokens
            .iter()
            .flat_map(|token| {
                semantic_object_operand_values(token, proven_statements.get(&token.offset()))
            })
            .filter(|value| object_aliases.contains_key(value))
            .count(),
        field_aliases: field_aliases.len(),
        field_alias_uses: vm_tokens
            .iter()
            .flat_map(object_operand_values)
            .filter(|value| field_aliases.contains_key(value))
            .count(),
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
            let encoded = token
                .encode()
                .ok_or_else(|| anyhow!("BAS token at 0x{cursor:08X} cannot be encoded"))?;
            if image.get(cursor..end) != Some(encoded.as_slice()) {
                bail!("BAS token at 0x{cursor:08X} does not re-encode exactly");
            }
            emit_bas_structured_boundaries(output, cursor, &structured)?;
            emit_directives(output, cursor, &annotations)?;
            match &token {
                vm_source::BasToken::Menu { word_offsets, .. } => {
                    write!(output, "{cursor:08X}: MENU")?;
                    for word in word_offsets {
                        write!(output, " {}", dictionary_operands.operand(*word))?;
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
                    emit_token(
                        output,
                        token,
                        dictionary,
                        &HashMap::new(),
                        &HashMap::new(),
                        &object_aliases,
                        &field_aliases,
                        &mut dictionary_operands,
                        None,
                        proven_statements.get(&token.offset()).copied(),
                    )?;
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
                        "{cursor:08X}: {statement} {} {} ; {:?}",
                        dictionary_operands.operand(*selector),
                        bas_next_operand(*next, &annotations.labels),
                        dictionary.get(selector).map(String::as_str).unwrap_or("")
                    )?;
                }
                vm_source::BasToken::PresentationRegister { value, .. } => {
                    if dictionary.contains_key(value) {
                        writeln!(
                            output,
                            "{cursor:08X}: OFFER_TOPIC {}",
                            dictionary_operands.operand(*value)
                        )?;
                    } else {
                        writeln!(output, "{cursor:08X}: PRESENTATION_REGISTER {value:04X}")?;
                    }
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
    let mut used_label_names = HashSet::new();
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
        let identifier = unique_identifier(
            identifier_component(&symbol.name),
            offset as u16,
            &mut used_label_names,
        );
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
        annotations
            .procedure_labels
            .insert(offset as u16, identifier.clone());
        annotations.procedure_count += 1;
    }
    if let Some(prior) = prior_procedure {
        annotations
            .directives
            .entry(image.len())
            .or_default()
            .push(format!("END_PROCEDURE {prior}"));
    }

    for symbol in symbols.iter().filter(|symbol| symbol.kind == 4) {
        let target = symbol.offset;
        if !boundaries.contains(&usize::from(target)) {
            bail!(
                "DEB kind-4 symbol {:?} at 0x{target:04X} does not resolve to a COD token boundary",
                symbol.name
            );
        }
        if annotations.labels.contains_key(&target) {
            continue;
        }
        let base = identifier_component(&symbol.name);
        let base = if used_label_names.contains(&base) {
            format!("{base}_label")
        } else {
            base
        };
        let identifier = unique_identifier(base, target, &mut used_label_names);
        annotations
            .directives
            .entry(usize::from(target))
            .or_default()
            .push(format!("LABEL {identifier}"));
        annotations.labels.insert(target, identifier);
    }

    let mut target_roles: BTreeMap<u16, &'static str> = BTreeMap::new();
    for token in tokens {
        let Some((target, role)) = cod_target_role(token) else {
            continue;
        };
        target_roles
            .entry(target)
            .and_modify(|current| {
                if flow_role_priority(role) > flow_role_priority(*current) {
                    *current = role;
                }
            })
            .or_insert(role);
    }
    let mut role_counts: HashMap<(String, &'static str), usize> = HashMap::new();
    for (target, role) in target_roles {
        if !boundaries.contains(&usize::from(target)) {
            bail!("COD target 0x{target:04X} does not resolve to a token boundary");
        }
        if annotations.labels.contains_key(&target) {
            continue;
        }
        let owner = procedures
            .range(..=usize::from(target))
            .next_back()
            .map(|(_, (identifier, _))| identifier.clone())
            .unwrap_or_else(|| "script".to_string());
        let count = role_counts.entry((owner.clone(), role)).or_default();
        *count += 1;
        let identifier = unique_identifier(
            format!("{owner}_{role}_{count}"),
            target,
            &mut used_label_names,
        );
        annotations
            .directives
            .entry(usize::from(target))
            .or_default()
            .push(format!("LABEL {identifier}"));
        annotations.labels.insert(target, identifier);
    }
    Ok(annotations)
}

fn cod_target_role(token: &VmToken) -> Option<(u16, &'static str)> {
    match token {
        VmToken::Text {
            loop_target: Some(target),
            ..
        } => Some((*target, "dialogue_resume")),
        VmToken::Jump { target, .. } => Some((*target, "jump_target")),
        VmToken::GuardPush { target, .. } | VmToken::ConditionalBlock { target, .. } => {
            Some((*target, "branch"))
        }
        _ => None,
    }
}

fn flow_role_priority(role: &str) -> u8 {
    match role {
        "dialogue_resume" => 3,
        "jump_target" => 2,
        _ => 1,
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
            bail!("BAS selector next offset 0x{target:04X} does not resolve to a selector node");
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
    let mut used_names = HashSet::new();
    for list in &graph.lists {
        let entry = &list.entrypoint;
        let name = unique_identifier(
            format!("{}_choices", identifier_component(&entry.object_name)),
            entry.prefix_yield_b as u16,
            &mut used_names,
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
        if let Some(else_jump) = region.else_jump {
            annotations.elses.insert(else_jump, region.start);
        }
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

fn cod_object_aliases(tokens: &[VmToken], symbols: &[DebSymbol]) -> BTreeMap<u16, ObjectAlias> {
    let referenced: BTreeSet<u16> = tokens.iter().flat_map(cod_object_operand_values).collect();

    let mut aliases = BTreeMap::new();
    for symbol in symbols
        .iter()
        .filter(|symbol| symbol.kind == 1 && referenced.contains(&symbol.offset))
    {
        aliases.entry(symbol.offset).or_insert_with(|| ObjectAlias {
            identifier: format!(
                "object_{}_{:04X}",
                identifier_component(&symbol.name),
                symbol.offset
            ),
            source_name: symbol.name.clone(),
        });
    }
    aliases
}

fn field_aliases(
    values: impl IntoIterator<Item = u16>,
    symbols: &[DebSymbol],
    var: &[u8],
) -> BTreeMap<u16, FieldAlias> {
    let referenced: BTreeSet<u16> = values.into_iter().collect();
    let objects: BTreeMap<u16, &DebSymbol> = symbols
        .iter()
        .filter(|symbol| symbol.kind == 1)
        .map(|symbol| (symbol.offset, symbol))
        .collect();
    let object_bases: BTreeSet<u16> = objects.keys().copied().collect();
    let mut candidates: BTreeMap<u16, Vec<FieldAlias>> = BTreeMap::new();

    for (&owner_offset, symbol) in &objects {
        let owner = usize::from(owner_offset);
        let Some(kind_bytes) = var.get(owner..owner + 2) else {
            continue;
        };
        let kind = u16::from_le_bytes([kind_bytes[0], kind_bytes[1]]);
        if kind == 0 {
            continue;
        }
        let column = kind.trailing_zeros() as usize;
        if column >= 16 {
            continue;
        }
        let mut selectors_by_offset: BTreeMap<u16, Vec<u8>> = BTreeMap::new();
        for (selector, row) in vm::FIELD_OFFSETS.iter().enumerate() {
            let field_offset = u16::from(row[column]);
            if field_offset != 0 {
                selectors_by_offset
                    .entry(field_offset)
                    .or_default()
                    .push(selector as u8);
            }
        }
        for (field_offset, selectors) in selectors_by_offset {
            let address = owner_offset.wrapping_add(field_offset);
            if !referenced.contains(&address) || object_bases.contains(&address) {
                continue;
            }
            let selector_component = selectors
                .iter()
                .map(|selector| format!("{selector:02X}"))
                .collect::<Vec<_>>()
                .join("_");
            candidates.entry(address).or_default().push(FieldAlias {
                identifier: format!(
                    "field_{}_{owner_offset:04X}_s{selector_component}_{address:04X}",
                    identifier_component(&symbol.name)
                ),
                owner_offset,
                owner_name: symbol.name.clone(),
                kind,
                selectors,
                field_offset,
            });
        }
    }

    candidates
        .into_iter()
        .filter_map(|(address, mut candidates)| {
            (candidates.len() == 1).then(|| (address, candidates.remove(0)))
        })
        .collect()
}

fn add_field_owner_objects(
    aliases: &mut BTreeMap<u16, ObjectAlias>,
    fields: &BTreeMap<u16, FieldAlias>,
) {
    for field in fields.values() {
        aliases
            .entry(field.owner_offset)
            .or_insert_with(|| ObjectAlias {
                identifier: format!(
                    "object_{}_{:04X}",
                    identifier_component(&field.owner_name),
                    field.owner_offset
                ),
                source_name: field.owner_name.clone(),
            });
    }
}

fn simplify_alias_identifiers(
    objects: &mut BTreeMap<u16, ObjectAlias>,
    fields: &mut BTreeMap<u16, FieldAlias>,
) {
    let mut used = HashSet::new();
    for (&offset, alias) in objects.iter_mut() {
        let base = identifier_component(&alias.source_name);
        alias.identifier = unique_identifier(base, offset, &mut used);
    }
    for (&address, alias) in fields.iter_mut() {
        let owner = objects
            .get(&alias.owner_offset)
            .map(|owner| owner.identifier.as_str())
            .unwrap_or("object");
        let field = semantic_field_component(alias).map_or_else(
            || {
                let selectors = alias
                    .selectors
                    .iter()
                    .map(|selector| format!("{selector:02X}"))
                    .collect::<Vec<_>>()
                    .join("_");
                format!("s{selectors}")
            },
            str::to_string,
        );
        alias.identifier = unique_identifier(format!("{owner}.{field}"), address, &mut used);
    }
}

fn semantic_field_component(alias: &FieldAlias) -> Option<&'static str> {
    match (alias.kind, alias.selectors.as_slice()) {
        (_, [0x00]) => Some("flags"),
        (_, [0x13]) => Some("action"),
        (0x0002, [0x03]) => Some("aggressiveness"),
        (0x0002, [0x08]) => Some("encounter_count"),
        (0x0002, [0x05]) => Some("known_objects"),
        (0x0010, [0x0B]) => Some("position"),
        (0x0002 | 0x0010 | 0x0200, [0x11]) => Some("current_location"),
        (0x0400, [0x11]) => Some("holder"),
        (0x0002, [0x0F]) => Some("topic"),
        _ => None,
    }
}

fn unique_identifier(base: String, _offset: u16, used: &mut HashSet<String>) -> String {
    if used.insert(base.clone()) {
        return base;
    }
    for occurrence in 2usize.. {
        let candidate = format!("{base}_{occurrence}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("an unbounded numeric suffix must produce a unique identifier")
}

fn dictionary_aliases(
    values: impl IntoIterator<Item = u16>,
    dictionary: &HashMap<u16, String>,
) -> BTreeMap<u16, DictionaryAlias> {
    values
        .into_iter()
        .filter_map(|offset| {
            dictionary.get(&offset).map(|value| {
                (
                    offset,
                    DictionaryAlias {
                        value: value.clone(),
                    },
                )
            })
        })
        .collect()
}

fn dictionary_operand_values(token: &VmToken) -> Vec<u16> {
    match token {
        VmToken::Text { word_offsets, .. } => word_offsets.clone(),
        VmToken::ConceptGuard { word_offset, .. } => vec![*word_offset],
        VmToken::RecordWildcard {
            opcode: 0xBC,
            value,
            ..
        } => vec![*value],
        _ => Vec::new(),
    }
}

fn cod_object_operand_values(token: &VmToken) -> Vec<u16> {
    let mut values = object_operand_values(token);
    if let VmToken::RecordWildcard { opcode, value, .. } = token
        && *opcode != 0xBC
        && *value != 0xFFFF
    {
        values.push(*value);
    }
    values
}

fn bas_vm_tokens(image: &[u8], dictionary: &HashMap<u16, String>) -> Vec<VmToken> {
    let mut tokens = Vec::new();
    let mut cursor = 0usize;
    while cursor < image.len() {
        if let Some((end, token)) = vm_source::bas_token_at(image, cursor, dictionary) {
            match token {
                vm_source::BasToken::Text(token) | vm_source::BasToken::Vm(token) => {
                    tokens.push(token);
                }
                _ => {}
            }
            cursor = end;
        } else {
            cursor += 1;
        }
    }
    tokens
}

fn bas_dictionary_operand_values(image: &[u8], dictionary: &HashMap<u16, String>) -> Vec<u16> {
    let mut values = Vec::new();
    let mut cursor = 0usize;
    while cursor < image.len() {
        if let Some((end, token)) = vm_source::bas_token_at(image, cursor, dictionary) {
            match token {
                vm_source::BasToken::Menu { word_offsets, .. } => values.extend(word_offsets),
                vm_source::BasToken::Text(token) | vm_source::BasToken::Vm(token) => {
                    values.extend(dictionary_operand_values(&token));
                }
                vm_source::BasToken::SelectorNode { selector, .. } => values.push(selector),
                vm_source::BasToken::PresentationRegister { value, .. } => values.push(value),
                _ => {}
            }
            cursor = end;
        } else {
            cursor += 1;
        }
    }
    values
}

/// Dictionary offsets in the order the two shipped program streams first
/// expose them to the compiler. COD precedes BAS, matching profile source order.
pub(crate) fn dictionary_operand_order(
    cod: &[u8],
    bas: &[u8],
    dictionary: &HashMap<u16, String>,
) -> Vec<u16> {
    let mut values = vm::walk(cod, 0, cod.len())
        .iter()
        .flat_map(dictionary_operand_values)
        .collect::<Vec<_>>();
    values.extend(bas_dictionary_operand_values(bas, dictionary));
    values
}

fn object_operand_values(token: &VmToken) -> Vec<u16> {
    match token {
        VmToken::Text { line_index, .. } => vec![*line_index],
        VmToken::Actor {
            record_offset,
            related_record_offset,
            ..
        }
        | VmToken::RecordLink {
            record_offset,
            related_record_offset,
            ..
        } => vec![*record_offset, *related_record_offset],
        VmToken::RecordEntry {
            entry_opcode,
            record_offset,
            operand,
            ..
        } if *entry_opcode != vm::OP_RECORD_ENTRY_MAX => vec![*record_offset, *operand],
        VmToken::RecordEntry { record_offset, .. }
        | VmToken::RecordClear { record_offset, .. }
        | VmToken::RecordWildcard { record_offset, .. }
        | VmToken::RecordState { record_offset, .. }
        | VmToken::PairRecord { record_offset, .. } => vec![*record_offset],
        VmToken::RecordTriple {
            record_offset,
            first_word,
            second_word,
            ..
        } => vec![*record_offset, *first_word, *second_word],
        VmToken::BitFlag { flag_offset, .. } => vec![*flag_offset],
        VmToken::SharedState {
            field_offset,
            rhs_mode,
            rhs,
            ..
        } if matches!(*rhs_mode, 0xC0 | 0xC2) => vec![*field_offset, *rhs],
        VmToken::SharedState { field_offset, .. }
        | VmToken::SharedBitState { field_offset, .. } => vec![*field_offset],
        _ => Vec::new(),
    }
}

fn semantic_object_operand_values(
    token: &VmToken,
    statement: Option<&ProvenStatement>,
) -> Vec<u16> {
    let mut values = cod_object_operand_values(token);
    if statement.is_some()
        && let VmToken::RecordState { operand, .. } = token
        && !values.contains(operand)
    {
        values.push(*operand);
    }
    if let Some(ProvenStatement::BloodLink { target }) = statement
        && !values.contains(target)
    {
        values.push(*target);
    }
    values
}

fn add_proven_statement_objects(
    aliases: &mut BTreeMap<u16, ObjectAlias>,
    tokens: &[VmToken],
    statements: &BTreeMap<usize, ProvenStatement>,
    symbols: &[DebSymbol],
) {
    let referenced = tokens
        .iter()
        .flat_map(|token| semantic_object_operand_values(token, statements.get(&token.offset())))
        .collect::<BTreeSet<_>>();
    for symbol in symbols
        .iter()
        .filter(|symbol| symbol.kind == 1 && referenced.contains(&symbol.offset))
    {
        aliases.entry(symbol.offset).or_insert_with(|| ObjectAlias {
            identifier: format!(
                "object_{}_{:04X}",
                identifier_component(&symbol.name),
                symbol.offset
            ),
            source_name: symbol.name.clone(),
        });
    }
}

fn proven_statement_offsets(
    tokens: &[VmToken],
    symbols: &[DebSymbol],
    var: &[u8],
    fields: &BTreeMap<u16, FieldAlias>,
) -> BTreeMap<usize, ProvenStatement> {
    let objects: BTreeMap<u16, &DebSymbol> = symbols
        .iter()
        .filter(|symbol| symbol.kind == 1)
        .map(|symbol| (symbol.offset, symbol))
        .collect();
    let var_kind = |offset: u16| {
        let offset = usize::from(offset);
        var.get(offset..offset + 2)
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
    };
    let action_owner_matches = |record_offset: &u16, name: &str, kind: u16| {
        fields.get(record_offset).is_some_and(|field| {
            field.selectors.as_slice() == [0x13]
                && field.kind == kind
                && objects
                    .get(&field.owner_offset)
                    .is_some_and(|symbol| symbol.name.eq_ignore_ascii_case(name))
        })
    };
    let object_kind_matches =
        |offset: &u16, kind: u16| objects.contains_key(offset) && var_kind(*offset) == Some(kind);
    let is_holder = |offset: u16| {
        objects.get(&offset).is_some_and(|symbol| {
            var_kind(offset) == Some(0x0002) || symbol.name.eq_ignore_ascii_case("blood")
        })
    };

    let mut query_mode = false;
    let mut proven = BTreeMap::new();
    for token in tokens {
        match token {
            VmToken::GuardPush { .. } | VmToken::ConditionalBlock { .. } => query_mode = true,
            VmToken::GuardPop { .. } => query_mode = false,
            VmToken::RecordState {
                offset,
                opcode: vm::OP_RECORD_STATE_MIN,
                record_offset,
                operand,
                inverted: false,
                ..
            } if !query_mode
                && action_owner_matches(record_offset, "orxx", 0x0200)
                && object_kind_matches(operand, 0x0080) =>
            {
                proven.insert(*offset, ProvenStatement::Navigate);
            }
            VmToken::RecordState {
                offset,
                opcode: vm::OP_RECORD_STATE_MAX,
                record_offset,
                operand,
                inverted: false,
                ..
            } if !query_mode
                && action_owner_matches(record_offset, "blood", 0x0001)
                && object_kind_matches(operand, 0x0002) =>
            {
                proven.insert(*offset, ProvenStatement::BringAboard);
            }
            VmToken::RecordEntry {
                offset,
                entry_opcode: 0xC6,
                record_offset,
                operand,
                inverted: false,
                ..
            } if query_mode
                && action_owner_matches(record_offset, "arche", 0x0010)
                && object_kind_matches(operand, 0x0100) =>
            {
                proven.insert(*offset, ProvenStatement::TravelThrough);
            }
            VmToken::PairRecord {
                offset,
                opcode: 0xBD,
                record_offset,
                ..
            } if !query_mode
                && fields.get(record_offset).is_some_and(|field| {
                    field.kind == 0x0010 && field.selectors.as_slice() == [0x0B]
                }) =>
            {
                proven.insert(*offset, ProvenStatement::PositionAssignment);
            }
            VmToken::BitFlag {
                offset,
                flag_offset,
                bit_index: 2,
                ..
            } if fields.get(flag_offset).is_some_and(|field| {
                field.kind == 0x0002 && field.selectors.as_slice() == [0x05]
            }) && symbols.get(2).is_some_and(|symbol| {
                symbol.kind == 1
                    && symbol.name.eq_ignore_ascii_case("blood")
                    && var_kind(symbol.offset) == Some(0x0001)
            }) =>
            {
                proven.insert(
                    *offset,
                    ProvenStatement::BloodLink {
                        target: symbols[2].offset,
                    },
                );
            }
            VmToken::RecordTriple {
                offset,
                record_offset,
                first_word,
                second_word,
                inverted: false,
                ..
            } if !query_mode => {
                let source = fields.get(record_offset);
                let source_is_holder = source.is_some_and(|field| {
                    field.selectors.as_slice() == [0x13] && is_holder(field.owner_offset)
                });
                if source_is_holder
                    && objects.contains_key(first_word)
                    && var_kind(*first_word) == Some(0x0400)
                    && is_holder(*second_word)
                {
                    proven.insert(*offset, ProvenStatement::InventoryTransfer);
                }
            }
            _ => {}
        }
    }
    proven
}

fn emit_object_declarations(
    output: &mut String,
    aliases: &BTreeMap<u16, ObjectAlias>,
) -> Result<()> {
    for (offset, alias) in aliases {
        writeln!(
            output,
            "00000000: OBJECT {} {offset:04X} ; DEB kind 1 {:?}",
            alias.identifier, alias.source_name
        )?;
    }
    if !aliases.is_empty() {
        output.push('\n');
    }
    Ok(())
}

fn emit_field_declarations(
    output: &mut String,
    aliases: &BTreeMap<u16, FieldAlias>,
    object_aliases: &BTreeMap<u16, ObjectAlias>,
) -> Result<()> {
    for alias in aliases.values() {
        let owner = object_aliases.get(&alias.owner_offset).ok_or_else(|| {
            anyhow!(
                "field {:?} has no object declaration at 0x{:04X}",
                alias.identifier,
                alias.owner_offset
            )
        })?;
        let selectors = alias
            .selectors
            .iter()
            .map(|selector| format!("{selector:02X}"))
            .collect::<Vec<_>>()
            .join(",");
        writeln!(
            output,
            "00000000: FIELD {} {} {:04X} ; VAR kind 0x{:04X}, selector(s) {}",
            alias.identifier, owner.identifier, alias.field_offset, alias.kind, selectors
        )?;
    }
    if !aliases.is_empty() {
        output.push('\n');
    }
    Ok(())
}

fn object_operand(
    value: u16,
    object_aliases: &BTreeMap<u16, ObjectAlias>,
    field_aliases: &BTreeMap<u16, FieldAlias>,
) -> String {
    field_aliases
        .get(&value)
        .map(|alias| alias.identifier.clone())
        .or_else(|| {
            object_aliases
                .get(&value)
                .map(|alias| alias.identifier.clone())
        })
        .unwrap_or_else(|| format!("{value:04X}"))
}

fn record_value_operand(value: u16, object_aliases: &BTreeMap<u16, ObjectAlias>) -> String {
    object_aliases
        .get(&value)
        .map(|alias| alias.identifier.clone())
        .unwrap_or_else(|| format!("{value:04X}"))
}

fn emit_token(
    output: &mut String,
    token: &VmToken,
    dictionary: &HashMap<u16, String>,
    labels: &HashMap<u16, String>,
    procedure_labels: &HashMap<u16, String>,
    object_aliases: &BTreeMap<u16, ObjectAlias>,
    field_aliases: &BTreeMap<u16, FieldAlias>,
    dictionary_operands: &mut DictionaryOperandFormatter<'_>,
    guard_rejections: Option<&BTreeSet<GuardRejection>>,
    proven_statement: Option<ProvenStatement>,
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
                "TEXT {} {voice_selector:02X} {flags_b4:02X} {flags_b5:02X} {} {}",
                object_operand(*line_index, object_aliases, field_aliases),
                optional_address_operand(*loop_target, labels),
                option_word(*control_word)
            )?;
            for word in word_offsets {
                write!(output, " {}", dictionary_operands.operand(*word))?;
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
            "CONCEPT_GUARD {} {}",
            dictionary_operands.operand(*word_offset),
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
            let procedure = if matches!(*value, 0 | 1) {
                address
                    .checked_sub(1)
                    .and_then(|start| procedure_labels.get(&start))
            } else {
                None
            };
            if let Some(procedure) = procedure {
                write!(
                    output,
                    "SET_PROCEDURE_ENABLED {procedure} {}",
                    bool_digit(*value != 0)
                )?;
            } else {
                write!(output, "POKE_BYTE {address:04X} {value:02X}")?;
            }
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
            "ACTOR {} {} {}",
            object_operand(*record_offset, object_aliases, field_aliases),
            object_operand(*related_record_offset, object_aliases, field_aliases),
            bool_digit(*inverted)
        )?,
        VmToken::RecordLink {
            record_offset,
            related_record_offset,
            inverted,
            ..
        } => write!(
            output,
            "RECORD_LINK {} {} {}",
            object_operand(*record_offset, object_aliases, field_aliases),
            object_operand(*related_record_offset, object_aliases, field_aliases),
            bool_digit(*inverted)
        )?,
        VmToken::RecordEntry {
            entry_opcode,
            record_offset,
            operand,
            inverted,
            ..
        } => {
            if proven_statement == Some(ProvenStatement::TravelThrough) {
                write!(
                    output,
                    "REQUIRE_TRAVEL_THROUGH {}",
                    object_operand(*operand, object_aliases, field_aliases)
                )?;
            } else {
                write!(
                    output,
                    "RECORD_ENTRY {entry_opcode:02X} {} {} {}",
                    object_operand(*record_offset, object_aliases, field_aliases),
                    if *entry_opcode == vm::OP_RECORD_ENTRY_MAX {
                        format!("{operand:04X}")
                    } else {
                        object_operand(*operand, object_aliases, field_aliases)
                    },
                    bool_digit(*inverted)
                )?;
            }
        }
        VmToken::RecordClear { record_offset, .. } => write!(
            output,
            "RECORD_CLEAR {}",
            object_operand(*record_offset, object_aliases, field_aliases)
        )?,
        VmToken::BitFlag {
            flag_offset,
            bit_index,
            clear,
            ..
        } => {
            if let Some(ProvenStatement::BloodLink { target }) = proven_statement {
                write!(
                    output,
                    "BLOOD_LINK {} {} {}",
                    object_operand(*flag_offset, object_aliases, field_aliases),
                    object_operand(target, object_aliases, field_aliases),
                    bool_digit(*clear)
                )?;
            } else {
                write!(
                    output,
                    "BIT_FLAG {} {bit_index:02X} {}",
                    object_operand(*flag_offset, object_aliases, field_aliases),
                    bool_digit(*clear)
                )?;
            }
        }
        VmToken::SharedState {
            opcode,
            field_offset,
            operator,
            rhs_mode,
            rhs,
            ..
        } => {
            let rhs = if matches!(*rhs_mode, 0xC0 | 0xC2) {
                object_operand(*rhs, object_aliases, field_aliases)
            } else {
                format!("{rhs:04X}")
            };
            write!(
                output,
                "SHARED_STATE {opcode:02X} {} {operator:02X} {rhs_mode:02X} {rhs}",
                object_operand(*field_offset, object_aliases, field_aliases)
            )?
        }
        VmToken::SharedBitState {
            opcode,
            field_offset,
            mask,
            inverted,
            ..
        } => write!(
            output,
            "SHARED_BIT_STATE {opcode:02X} {} {mask:04X} {}",
            object_operand(*field_offset, object_aliases, field_aliases),
            bool_digit(*inverted)
        )?,
        VmToken::RecordWildcard {
            opcode,
            record_offset,
            value,
            inverted,
            ..
        } => {
            let value = if *opcode == 0xBC {
                dictionary_operands.operand(*value)
            } else {
                record_value_operand(*value, object_aliases)
            };
            write!(
                output,
                "RECORD_WILDCARD {opcode:02X} {} {value} {}",
                object_operand(*record_offset, object_aliases, field_aliases),
                bool_digit(*inverted)
            )?
        }
        VmToken::RecordState {
            opcode,
            record_offset,
            operand,
            inverted,
            ..
        } => match proven_statement {
            Some(ProvenStatement::Navigate) => write!(
                output,
                "NAVIGATE {}",
                object_operand(*operand, object_aliases, field_aliases)
            )?,
            Some(ProvenStatement::BringAboard) => write!(
                output,
                "BRING_ABOARD {}",
                object_operand(*operand, object_aliases, field_aliases)
            )?,
            _ => write!(
                output,
                "RECORD_STATE {opcode:02X} {} {operand:04X} {}",
                object_operand(*record_offset, object_aliases, field_aliases),
                bool_digit(*inverted)
            )?,
        },
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
            encoded_year,
            ..
        } => write!(
            output,
            "GLOBAL_PAIR_COMPARE {operator:02X} {packed_value:04X} {encoded_year:04X}"
        )?,
        VmToken::PairRecord {
            opcode,
            record_offset,
            first_word,
            second_word,
            ..
        } => {
            if proven_statement == Some(ProvenStatement::PositionAssignment) {
                write!(
                    output,
                    "POSITION {} {first_word:04X} {second_word:04X}",
                    object_operand(*record_offset, object_aliases, field_aliases)
                )?;
            } else {
                write!(
                    output,
                    "PAIR_RECORD {opcode:02X} {} {first_word:04X} {second_word:04X}",
                    object_operand(*record_offset, object_aliases, field_aliases)
                )?;
            }
        }
        VmToken::RecordTriple {
            record_offset,
            first_word,
            second_word,
            inverted,
            ..
        } => {
            if proven_statement == Some(ProvenStatement::InventoryTransfer) {
                let action = object_operand(*record_offset, object_aliases, field_aliases);
                let source = action
                    .strip_suffix(".action")
                    .expect("proven transfer has an action field");
                write!(
                    output,
                    "TRANSFER {} {} {}",
                    object_operand(*first_word, object_aliases, field_aliases),
                    source,
                    object_operand(*second_word, object_aliases, field_aliases),
                )?;
            } else {
                write!(
                    output,
                    "RECORD_TRIPLE {} {} {} {}",
                    object_operand(*record_offset, object_aliases, field_aliases),
                    object_operand(*first_word, object_aliases, field_aliases),
                    object_operand(*second_word, object_aliases, field_aliases),
                    bool_digit(*inverted)
                )?;
            }
        }
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
    procedures: &HashSet<&str>,
    objects: &HashMap<&str, u16>,
    dictionary_words: &HashMap<&str, u16>,
    interned_dictionary_words: &HashMap<String, Option<u16>>,
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
                word(
                    &mut output,
                    parse_dictionary_address(
                        value,
                        dictionary_words,
                        interned_dictionary_words,
                        line,
                        "menu word",
                    )?,
                );
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
            word(
                &mut output,
                parse_dictionary_address(
                    args[0],
                    dictionary_words,
                    interned_dictionary_words,
                    line,
                    "selector",
                )?,
            );
            word(
                &mut output,
                parse_address(args[1], labels, line, "next selector node")?,
            );
        }
        "OFFER_TOPIC" => {
            require_count(args, 1, line, name)?;
            output.push(0xA7);
            word(
                &mut output,
                parse_dictionary_address(
                    args[0],
                    dictionary_words,
                    interned_dictionary_words,
                    line,
                    "offered topic",
                )?,
            );
        }
        "PRESENTATION_REGISTER" => {
            require_count(args, 1, line, name)?;
            output.push(0xA7);
            word(
                &mut output,
                parse_word(args[0], line, "presentation value")?,
            );
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
        "ELSE" => {
            require_count(args, 2, line, name)?;
            output.push(vm::OP_JUMP);
            word(
                &mut output,
                parse_address(args[1], labels, line, "else end target")?,
            );
        }
        "CONCEPT_GUARD" => {
            require_count(args, 2, line, name)?;
            output.push(vm::OP_CONCEPT_GUARD);
            if parse_bool(args[1], line, "inverted")? {
                output.push(vm::OP_POP);
            }
            word(
                &mut output,
                parse_dictionary_address(
                    args[0],
                    dictionary_words,
                    interned_dictionary_words,
                    line,
                    "dictionary word offset",
                )?,
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
        "SET_PROCEDURE_ENABLED" => {
            require_count(args, 2, line, name)?;
            if !procedures.contains(args[0]) {
                bail!(
                    "line {line}: procedure {:?} is not declared with proc",
                    args[0]
                );
            }
            let procedure = parse_address(args[0], labels, line, "procedure")?;
            let address = procedure.checked_add(1).ok_or_else(|| {
                anyhow!(
                    "line {line}: procedure {:?} has no addressable enable byte",
                    args[0]
                )
            })?;
            output.push(vm::OP_POKE_BYTE);
            output.push(u8::from(parse_bool(args[1], line, "enabled")?));
            word(&mut output, address);
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
            let line_index = parse_object_address(args[0], objects, line, "line index")?;
            let voice = parse_byte(args[1], line, "voice")?;
            let flags_b4 = parse_byte(args[2], line, "control flags")?;
            let flags_b5 = parse_byte(args[3], line, "display flags")?;
            let loop_target = parse_optional_address(args[4], labels, line, "resume target")?;
            let control_word = parse_optional_word(args[5], line, "control word")?;
            if (flags_b4 & 0x10 != 0) != loop_target.is_some() {
                bail!("line {line}: TEXT resume target disagrees with flag 0x10");
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
                word(
                    &mut output,
                    parse_dictionary_address(
                        value,
                        dictionary_words,
                        interned_dictionary_words,
                        line,
                        "dictionary word",
                    )?,
                );
            }
            word(&mut output, 0);
        }
        "ACTOR" | "RECORD_LINK" => {
            require_count(args, 3, line, name)?;
            output.push(if name == "ACTOR" { 0xC4 } else { 0xC3 });
            if parse_bool(args[2], line, "inverted")? {
                output.push(0xA1);
            }
            word(
                &mut output,
                parse_object_address(args[0], objects, line, "record")?,
            );
            word(
                &mut output,
                parse_object_address(args[1], objects, line, "related record")?,
            );
        }
        "NAVIGATE" => {
            require_count(args, 1, line, name)?;
            output.push(vm::OP_RECORD_STATE_MIN);
            word(
                &mut output,
                parse_object_address("orxx.action", objects, line, "orxx action field")?,
            );
            word(
                &mut output,
                parse_object_address(args[0], objects, line, "navigation destination")?,
            );
        }
        "BRING_ABOARD" => {
            require_count(args, 1, line, name)?;
            output.push(vm::OP_RECORD_STATE_MAX);
            word(
                &mut output,
                parse_object_address("blood.action", objects, line, "blood action field")?,
            );
            word(
                &mut output,
                parse_object_address(args[0], objects, line, "character")?,
            );
        }
        "REQUIRE_TRAVEL_THROUGH" => {
            require_count(args, 1, line, name)?;
            output.push(0xC6);
            word(
                &mut output,
                parse_object_address("arche.action", objects, line, "arche action field")?,
            );
            word(
                &mut output,
                parse_object_address(args[0], objects, line, "black hole")?,
            );
        }
        "BLOOD_LINK" => {
            require_count(args, 3, line, name)?;
            if !args[1].eq_ignore_ascii_case("blood") {
                bail!(
                    "line {line}: the recovered object-link syntax currently proves only the built-in blood target"
                );
            }
            let blood = parse_object_address(args[1], objects, line, "blood object")?;
            if blood != 0x0028 {
                bail!("line {line}: built-in blood must be the invariant VAR object at 0x0028");
            }
            output.push(vm::OP_BIT_FLAG);
            if parse_bool(args[2], line, "remove link")? {
                output.push(vm::OP_POP);
            }
            word(
                &mut output,
                parse_object_address(args[0], objects, line, "object-link field")?,
            );
            output.push(2);
        }
        "POSITION" => {
            require_count(args, 3, line, name)?;
            output.push(0xBD);
            word(
                &mut output,
                parse_object_address(args[0], objects, line, "position field")?,
            );
            word(&mut output, parse_word(args[1], line, "position x")?);
            word(&mut output, parse_word(args[2], line, "position y")?);
        }
        "RECORD_ENTRY" => {
            require_count(args, 4, line, name)?;
            let opcode = parse_byte(args[0], line, "opcode")?;
            output.push(opcode);
            if parse_bool(args[3], line, "inverted")? {
                output.push(0xA1);
            }
            word(
                &mut output,
                parse_object_address(args[1], objects, line, "record")?,
            );
            word(
                &mut output,
                if opcode == vm::OP_RECORD_ENTRY_MAX {
                    parse_word(args[2], line, "operand")?
                } else {
                    parse_object_address(args[2], objects, line, "related record")?
                },
            );
        }
        "RECORD_STATE" => {
            require_count(args, 4, line, name)?;
            output.push(parse_byte(args[0], line, "opcode")?);
            if parse_bool(args[3], line, "inverted")? {
                output.push(0xA1);
            }
            word(
                &mut output,
                parse_object_address(args[1], objects, line, "record")?,
            );
            word(&mut output, parse_word(args[2], line, "operand")?);
        }
        "RECORD_CLEAR" => {
            require_count(args, 1, line, name)?;
            output.push(0xC9);
            word(
                &mut output,
                parse_object_address(args[0], objects, line, "record")?,
            );
        }
        "BIT_FLAG" => {
            require_count(args, 3, line, name)?;
            output.push(vm::OP_BIT_FLAG);
            if parse_bool(args[2], line, "clear")? {
                output.push(0xA1);
            }
            word(
                &mut output,
                parse_object_address(args[0], objects, line, "record")?,
            );
            output.push(parse_byte(args[1], line, "bit index")?);
        }
        "SHARED_STATE" => {
            require_count(args, 5, line, name)?;
            let opcode = parse_byte(args[0], line, "opcode")?;
            if !vm::is_shared_state_opcode(opcode) {
                bail!("line {line}: opcode {opcode:02X} is not a SHARED_STATE opcode");
            }
            output.push(opcode);
            word(
                &mut output,
                parse_object_address(args[1], objects, line, "field offset")?,
            );
            output.push(parse_byte(args[2], line, "operator")?);
            let rhs_mode = parse_byte(args[3], line, "RHS mode")?;
            output.push(rhs_mode);
            word(
                &mut output,
                if matches!(rhs_mode, 0xC0 | 0xC2) {
                    parse_object_address(args[4], objects, line, "RHS field")?
                } else {
                    parse_word(args[4], line, "RHS value")?
                },
            );
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
            word(
                &mut output,
                parse_object_address(args[1], objects, line, "field offset")?,
            );
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
            word(
                &mut output,
                parse_object_address(args[1], objects, line, "record offset")?,
            );
            word(
                &mut output,
                if opcode == 0xBC {
                    parse_dictionary_address(
                        args[2],
                        dictionary_words,
                        interned_dictionary_words,
                        line,
                        "topic",
                    )?
                } else {
                    parse_object_address(args[2], objects, line, "record value")?
                },
            );
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
            word(&mut output, parse_word(args[2], line, "encoded year")?);
        }
        "PAIR_RECORD" => {
            require_count(args, 4, line, name)?;
            output.push(parse_byte(args[0], line, "opcode")?);
            word(
                &mut output,
                parse_object_address(args[1], objects, line, "record")?,
            );
            word(&mut output, parse_word(args[2], line, "first word")?);
            word(&mut output, parse_word(args[3], line, "second word")?);
        }
        "RECORD_TRIPLE" => {
            require_count(args, 4, line, name)?;
            output.push(vm::OP_RECORD_TRIPLE);
            if parse_bool(args[3], line, "inverted")? {
                output.push(0xA1);
            }
            word(
                &mut output,
                parse_object_address(args[0], objects, line, "record")?,
            );
            word(
                &mut output,
                parse_object_address(args[1], objects, line, "transferred object")?,
            );
            word(
                &mut output,
                parse_object_address(args[2], objects, line, "destination object")?,
            );
        }
        "TRANSFER" => {
            require_count(args, 3, line, name)?;
            output.push(vm::OP_RECORD_TRIPLE);
            let action = format!("{}.action", args[1]);
            word(
                &mut output,
                parse_object_address(&action, objects, line, "source action field")?,
            );
            word(
                &mut output,
                parse_object_address(args[0], objects, line, "transferred object")?,
            );
            word(
                &mut output,
                parse_object_address(args[2], objects, line, "destination object")?,
            );
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

fn parse_object_address(
    value: &str,
    objects: &HashMap<&str, u16>,
    line: usize,
    field: &str,
) -> Result<u16> {
    if let Some(index) = bracketed_operand(value, "globals") {
        let base = objects
            .get("globals")
            .copied()
            .ok_or_else(|| anyhow!("line {line}: globals block is not declared"))?;
        let index = index
            .parse::<u16>()
            .map_err(|_| anyhow!("line {line}: global index must be a decimal integer"))?;
        return base
            .checked_add(
                index
                    .checked_mul(2)
                    .ok_or_else(|| anyhow!("line {line}: global index exceeds the state image"))?,
            )
            .ok_or_else(|| anyhow!("line {line}: global index exceeds the state image"));
    }
    objects
        .get(value)
        .copied()
        .map(Ok)
        .unwrap_or_else(|| parse_word(value, line, field))
}

fn collect_interned_dictionary_words(
    lines: &[ParsedSourceLine<'_>],
    dictionary: &HashMap<u16, String>,
) -> Result<HashMap<String, Option<u16>>> {
    let mut words = HashMap::new();
    for (offset, text) in dictionary {
        words
            .entry(text.clone())
            .and_modify(|canonical: &mut Option<u16>| {
                *canonical = Some(canonical.unwrap_or(*offset).min(*offset));
            })
            .or_insert(Some(*offset));
    }
    let companion_words: HashSet<String> = words.keys().cloned().collect();
    for line in lines {
        for value in &line.args {
            let Some((text, offset)) = parse_inline_dictionary_address(value, line.line_number)?
            else {
                continue;
            };
            if companion_words.contains(&text) {
                continue;
            }
            if let Some(existing) = words.get_mut(&text) {
                if matches!(*existing, Some(previous) if previous != offset) {
                    *existing = None;
                }
            } else {
                words.insert(text, Some(offset));
            }
        }
    }
    Ok(words)
}

fn parse_inline_dictionary_address(value: &str, line: usize) -> Result<Option<(String, u16)>> {
    let Some((text, address)) = value.rsplit_once('@') else {
        return Ok(None);
    };
    if !text.starts_with('"') {
        return Ok(None);
    }
    let text: String = serde_json::from_str(text)
        .map_err(|_| anyhow!("line {line}: invalid inline dictionary text {text:?}"))?;
    let address = parse_word(address, line, "inline dictionary offset")?;
    Ok(Some((text, address)))
}

fn parse_dictionary_literal(value: &str, line: usize) -> Result<Option<String>> {
    if !value.starts_with('"') {
        return Ok(None);
    }
    serde_json::from_str(value)
        .map(Some)
        .map_err(|_| anyhow!("line {line}: invalid interned dictionary text {value:?}"))
}

fn parse_dictionary_address(
    value: &str,
    dictionary_words: &HashMap<&str, u16>,
    interned_dictionary_words: &HashMap<String, Option<u16>>,
    line: usize,
    field: &str,
) -> Result<u16> {
    if let Some(address) = dictionary_words.get(value).copied() {
        return Ok(address);
    }
    if let Some((_, address)) = parse_inline_dictionary_address(value, line)? {
        return Ok(address);
    }
    if let Some(text) = parse_dictionary_literal(value, line)? {
        return match interned_dictionary_words.get(&text) {
            Some(Some(address)) => Ok(*address),
            Some(None) => bail!(
                "line {line}: interned dictionary text {text:?} has multiple offsets; use an explicit @offset"
            ),
            None => bail!(
                "line {line}: interned dictionary text {text:?} has no companion dictionary entry or explicit @offset in this source"
            ),
        };
    }
    parse_word(value, line, field)
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
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
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

fn validate_field_identifier(value: &str, line: usize) -> Result<()> {
    if let Some((owner, field)) = value.split_once('.') {
        if field.contains('.') {
            bail!("line {line}: invalid field identifier {value:?}");
        }
        validate_identifier(owner, line)?;
        validate_identifier(field, line)
    } else {
        validate_identifier(value, line)
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
    fn readable_source_derives_layout_without_line_addresses() {
        let source = concat!(
            "; format: bloodscript-v2\n",
            "PROCEDURE entry\n",
            "    LOAD_STRING \"a;b.hnm\"\n",
            "    JUMP done\n",
            "    LABEL done\n",
            "    END\n",
            "END_PROCEDURE entry\n",
        );
        assert_eq!(
            compile(source).unwrap(),
            vec![
                vm::OP_LOAD_STRING,
                b'a',
                b';',
                b'b',
                b'.',
                b'h',
                b'n',
                b'm',
                0,
                0,
                vm::OP_JUMP,
                0x0D,
                0x00,
                0xFF,
            ]
        );
    }

    #[test]
    fn shared_state_families_compile_exact_bytes() {
        let source = concat!(
            "; format: bloodscript-ir-v1\n",
            "00000000: SHARED_STATE C0 1234 F6 C0 5678\n",
            "00000007: SHARED_BIT_STATE AE 2345 00FF 1\n",
            "0000000D: RECORD_WILDCARD AF 4567 FFFF 1\n",
            "00000013: END\n",
        );
        let expected = vec![
            0xC0, 0x34, 0x12, 0xF6, 0xC0, 0x78, 0x56, 0xAE, 0xA1, 0x45, 0x23, 0xFF, 0x00, 0xAF,
            0xA1, 0x67, 0x45, 0xFF, 0xFF, 0xFF,
        ];
        assert_eq!(compile(source).unwrap(), expected);

        let decompiled = decompile(ImageKind::Cod, &expected, &HashMap::new()).unwrap();
        assert_eq!(decompiled.generic_op_statements, 0);
        assert!(decompiled.source.contains("state[0x1234] += state[0x5678]"));
        assert!(decompiled.source.contains("bits(0x2345,0x00FF) = false"));
        assert!(
            decompiled
                .source
                .contains("require record[0x4567] != aboard")
        );
        assert_eq!(compile(&decompiled.source).unwrap(), expected);
    }

    #[test]
    fn rtc_conditions_use_readable_clock_expressions_and_preserve_year_bytes() {
        let image = vec![
            vm::OP_GLOBAL_WORD_COMPARE,
            0xF1,
            0xC1,
            0x08,
            0x00,
            vm::OP_GLOBAL_PAIR_COMPARE,
            0xF5,
            0x19,
            0x0C,
            0xCA,
            0x07,
            0xFF,
        ];
        let decompiled = decompile(ImageKind::Cod, &image, &HashMap::new()).unwrap();
        assert!(decompiled.source.contains("require clock.hour < 8"));
        assert!(
            decompiled
                .source
                .contains("require annual_date == 1994-12-25")
        );
        assert_eq!(compile(&decompiled.source).unwrap(), image);

        let edited = concat!(
            "// format: bloodscript-program-v1\n",
            "require clock.hour > 21\n",
            "require annual_date == 1995-01-01\n",
            "halt\n",
        );
        assert_eq!(
            compile(edited).unwrap(),
            vec![
                0xCA, 0xF2, 0xC1, 0x15, 0x00, 0xCB, 0xF5, 0x01, 0x01, 0xCB, 0x07, 0xFF
            ]
        );
    }

    #[test]
    fn hnm_loads_use_sequence_request_syntax() {
        let image = vec![
            vm::OP_LOAD_STRING,
            b'f',
            b'i',
            b'n',
            b'.',
            b'h',
            b'n',
            b'm',
            0,
            0,
            0xFF,
        ];
        let decompiled = decompile(ImageKind::Cod, &image, &HashMap::new()).unwrap();
        assert!(decompiled.source.contains("request sequence \"fin.hnm\""));
        assert_eq!(compile(&decompiled.source).unwrap(), image);

        let edited = concat!(
            "// format: bloodscript-program-v1\n",
            "request sequence \"scene.hnm\"\n",
            "halt\n",
        );
        assert_eq!(
            compile(edited).unwrap(),
            vec![
                vm::OP_LOAD_STRING,
                b's',
                b'c',
                b'e',
                b'n',
                b'e',
                b'.',
                b'h',
                b'n',
                b'm',
                0,
                0,
                0xFF,
            ]
        );

        for invalid in [
            "request sequence scene.hnm",
            "request sequence \"sq/scene.hnm\"",
            "request sequence \"scene.dat\"",
            "request sequence \"12345678901234567.hnm\"",
        ] {
            let source = format!("// format: bloodscript-program-v1\n{invalid}\nhalt\n");
            assert!(compile(&source).is_err(), "accepted {invalid}");
        }

        let fallback = vec![vm::OP_LOAD_STRING, b'n', b'o', b't', b'e', 0, 0, 0xFF];
        let decompiled = decompile(ImageKind::Cod, &fallback, &HashMap::new()).unwrap();
        assert!(decompiled.source.contains("load_string \"note\""));
        assert_eq!(compile(&decompiled.source).unwrap(), fallback);
    }

    #[test]
    fn presentation_pairs_use_actor_level_syntax() {
        let image = vec![
            vm::OP_COND_JUMP,
            0x01,
            0x12,
            0x00,
            vm::OP_ACTOR,
            0x3A,
            0x01,
            0x28,
            0x00,
            vm::OP_POP,
            vm::OP_RECORD_LINK,
            0x3A,
            0x01,
            0x28,
            0x00,
            vm::OP_RECORD_CLEAR,
            0x3A,
            0x01,
            0xFF,
        ];
        let mut var = vec![0; 0x140];
        var[0x28..0x2A].copy_from_slice(&1u16.to_le_bytes());
        var[0x100..0x102].copy_from_slice(&2u16.to_le_bytes());
        let symbols = vec![
            DebSymbol {
                name: "blood".to_string(),
                offset: 0x0028,
                kind: 1,
            },
            DebSymbol {
                name: "Beauregard".to_string(),
                offset: 0x0100,
                kind: 1,
            },
            DebSymbol {
                name: "entry".to_string(),
                offset: 1,
                kind: 2,
            },
        ];
        let decompiled =
            decompile_structured_cod_with_symbols(&image, &var, &HashMap::new(), &symbols).unwrap();
        for statement in [
            "field Beauregard.action = Beauregard + 0x003A",
            "require presentation == Beauregard",
            "queue presentation Beauregard",
            "end presentation Beauregard",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        for low_level in ["actor ", "record_link ", "record_clear ", ".s13"] {
            assert!(
                !decompiled.source.contains(low_level),
                "retained {low_level}"
            );
        }
        assert_eq!(compile(&decompiled.source).unwrap(), image);

        let inverted = decompiled
            .source
            .replace("presentation == Beauregard", "presentation != Beauregard");
        let expected = [vm::OP_COND_JUMP, 0x01, 0x13, 0x00]
            .into_iter()
            .chain([vm::OP_ACTOR, vm::OP_POP])
            .chain(image[5..].iter().copied())
            .collect::<Vec<_>>();
        assert_eq!(compile(&inverted).unwrap(), expected);
    }

    #[test]
    fn record_triple_transfers_use_inventory_syntax() {
        let image = vec![
            vm::OP_RECORD_TRIPLE,
            0x3A,
            0x01,
            0x00,
            0x02,
            0x28,
            0x00,
            vm::OP_RECORD_TRIPLE,
            0x30,
            0x00,
            0x00,
            0x02,
            0x00,
            0x01,
            0xFF,
        ];
        let mut var = vec![0; 0x300];
        var[0x28..0x2A].copy_from_slice(&1u16.to_le_bytes());
        var[0x100..0x102].copy_from_slice(&2u16.to_le_bytes());
        var[0x200..0x202].copy_from_slice(&0x0400u16.to_le_bytes());
        let symbols = vec![
            DebSymbol {
                name: "blood".to_string(),
                offset: 0x0028,
                kind: 1,
            },
            DebSymbol {
                name: "Bug_Deluxe".to_string(),
                offset: 0x0100,
                kind: 1,
            },
            DebSymbol {
                name: "perfume".to_string(),
                offset: 0x0200,
                kind: 1,
            },
        ];
        let decompiled =
            decompile_structured_cod_with_symbols(&image, &var, &HashMap::new(), &symbols).unwrap();
        for statement in [
            "object blood = 0x0028",
            "object Bug_Deluxe = 0x0100",
            "object perfume = 0x0200",
            "field blood.action = blood + 0x0008",
            "field Bug_Deluxe.action = Bug_Deluxe + 0x003A",
            "transfer perfume from Bug_Deluxe to aboard",
            "transfer perfume from aboard to Bug_Deluxe",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert!(!decompiled.source.contains("record_triple"));
        assert_eq!(compile(&decompiled.source).unwrap(), image);

        let query_image = vec![
            vm::OP_PUSH,
            0x0B,
            0x00,
            vm::OP_RECORD_TRIPLE,
            0x3A,
            0x01,
            0x00,
            0x02,
            0x28,
            0x00,
            vm::OP_POP,
            0xFF,
        ];
        let query =
            decompile_structured_cod_with_symbols(&query_image, &var, &HashMap::new(), &symbols)
                .unwrap();
        assert!(
            query
                .source
                .contains("record_triple Bug_Deluxe.action perfume blood 0")
        );
        assert!(!query.source.contains("transfer perfume"));
        assert_eq!(compile(&query.source).unwrap(), query_image);

        let mut wrong_kind_var = var.clone();
        wrong_kind_var[0x200..0x202].copy_from_slice(&2u16.to_le_bytes());
        let wrong_kind = decompile_structured_cod_with_symbols(
            &image,
            &wrong_kind_var,
            &HashMap::new(),
            &symbols,
        )
        .unwrap();
        assert!(
            wrong_kind
                .source
                .contains("record_triple Bug_Deluxe.action perfume blood 0")
        );
        assert!(!wrong_kind.source.contains("transfer perfume"));
        assert_eq!(compile(&wrong_kind.source).unwrap(), image);

        for invalid in [
            "transfer perfume Bug_Deluxe to aboard",
            "transfer perfume from Bug_Deluxe aboard",
            "transfer 0x0200 from Bug_Deluxe to aboard",
        ] {
            let source = format!("// format: {SOURCE_FORMAT}\n{invalid}\nhalt\n");
            assert!(compile(&source).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn navigation_boarding_and_black_hole_actions_use_domain_syntax() {
        let image = vec![
            vm::OP_RECORD_STATE_MIN,
            0x0A,
            0x02,
            0x00,
            0x03,
            vm::OP_RECORD_STATE_MAX,
            0x30,
            0x00,
            0x00,
            0x01,
            vm::OP_PUSH,
            0x13,
            0x00,
            0xC6,
            0x1C,
            0x04,
            0x00,
            0x05,
            vm::OP_POP,
            0xFF,
        ];
        let mut var = vec![0; 0x600];
        for (offset, kind) in [
            (0x0028, 0x0001u16),
            (0x0100, 0x0002),
            (0x0200, 0x0200),
            (0x0300, 0x0080),
            (0x0400, 0x0010),
            (0x0500, 0x0100),
        ] {
            var[offset..offset + 2].copy_from_slice(&kind.to_le_bytes());
        }
        let symbols = [
            ("blood", 0x0028),
            ("Bronko", 0x0100),
            ("orxx", 0x0200),
            ("observatory", 0x0300),
            ("arche", 0x0400),
            ("Oddland", 0x0500),
        ]
        .into_iter()
        .map(|(name, offset)| DebSymbol {
            name: name.to_string(),
            offset,
            kind: 1,
        })
        .collect::<Vec<_>>();
        let decompiled =
            decompile_structured_cod_with_symbols(&image, &var, &HashMap::new(), &symbols).unwrap();
        for statement in [
            "navigate to observatory",
            "bring Bronko aboard",
            "require travel through Oddland",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        for low_level in ["record_state", "record_entry"] {
            assert!(
                !decompiled.source.contains(low_level),
                "retained {low_level}"
            );
        }
        assert_eq!(compile(&decompiled.source).unwrap(), image);

        let query_image = [
            vm::OP_PUSH,
            0x09,
            0x00,
            vm::OP_RECORD_STATE_MIN,
            0x0A,
            0x02,
            0x00,
            0x03,
            vm::OP_POP,
            0xFF,
        ];
        let query =
            decompile_structured_cod_with_symbols(&query_image, &var, &HashMap::new(), &symbols)
                .unwrap();
        assert!(
            query
                .source
                .contains("record_state 0xC1 orxx.action 0x0300 0")
        );
        assert!(!query.source.contains("navigate to"));
        assert_eq!(compile(&query.source).unwrap(), query_image);

        let mut wrong_kind_var = var.clone();
        wrong_kind_var[0x300..0x302].copy_from_slice(&0x0008u16.to_le_bytes());
        let wrong_kind = decompile_structured_cod_with_symbols(
            &image,
            &wrong_kind_var,
            &HashMap::new(),
            &symbols,
        )
        .unwrap();
        assert!(
            wrong_kind
                .source
                .contains("record_state 0xC1 orxx.action 0x0300 0")
        );
        assert!(!wrong_kind.source.contains("navigate to"));
        assert_eq!(compile(&wrong_kind.source).unwrap(), image);

        let mut wrong_character_kind_var = var.clone();
        wrong_character_kind_var[0x100..0x102].copy_from_slice(&0x0400u16.to_le_bytes());
        let wrong_character_kind = decompile_structured_cod_with_symbols(
            &image,
            &wrong_character_kind_var,
            &HashMap::new(),
            &symbols,
        )
        .unwrap();
        assert!(
            wrong_character_kind
                .source
                .contains("record_state 0xC2 blood.action 0x0100 0")
        );
        assert!(!wrong_character_kind.source.contains("bring Bronko aboard"));
        assert_eq!(compile(&wrong_character_kind.source).unwrap(), image);

        let update_c6 = [0xC6, 0x1C, 0x04, 0x00, 0x05, 0xFF];
        let update =
            decompile_structured_cod_with_symbols(&update_c6, &var, &HashMap::new(), &symbols)
                .unwrap();
        assert!(
            update
                .source
                .contains("record_entry 0xC6 arche.action Oddland 0")
        );
        assert!(!update.source.contains("require travel through"));
        assert_eq!(compile(&update.source).unwrap(), update_c6);

        let mut wrong_hole_kind_var = var.clone();
        wrong_hole_kind_var[0x500..0x502].copy_from_slice(&0x0008u16.to_le_bytes());
        let wrong_hole_kind = decompile_structured_cod_with_symbols(
            &image,
            &wrong_hole_kind_var,
            &HashMap::new(),
            &symbols,
        )
        .unwrap();
        assert!(
            wrong_hole_kind
                .source
                .contains("record_entry 0xC6 arche.action Oddland 0")
        );
        assert!(!wrong_hole_kind.source.contains("require travel through"));
        assert_eq!(compile(&wrong_hole_kind.source).unwrap(), image);

        for invalid in [
            "navigate observatory",
            "bring aboard Bronko",
            "travel Oddland",
            "travel to Oddland",
            "require travel Oddland",
        ] {
            let source = format!("// format: {SOURCE_FORMAT}\n{invalid}\nhalt\n");
            assert!(compile(&source).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn position_and_blood_links_use_typed_object_syntax() {
        let image = vec![
            0xBD,
            0x18,
            0x01,
            0x0A,
            0x00,
            0x64,
            0x00, // Kraner position
            vm::OP_BIT_FLAG,
            0x1E,
            0x02,
            0x02, // Bug_Deluxe links to blood
            0xFF,
        ];
        let mut var = vec![0; 0x300];
        for (offset, kind) in [(0x0028, 0x0001u16), (0x0100, 0x0010), (0x0200, 0x0002)] {
            var[offset..offset + 2].copy_from_slice(&kind.to_le_bytes());
        }
        let symbols = [
            ("baby1", 0x0000),
            ("baby", 0x0014),
            ("blood", 0x0028),
            ("Kraner", 0x0100),
            ("Bug_Deluxe", 0x0200),
        ]
        .into_iter()
        .map(|(name, offset)| DebSymbol {
            name: name.to_string(),
            offset,
            kind: 1,
        })
        .collect::<Vec<_>>();

        let decompiled =
            decompile_structured_cod_with_symbols(&image, &var, &HashMap::new(), &symbols).unwrap();
        for statement in [
            "object blood = 0x0028",
            "field Kraner.position = Kraner + 0x0018",
            "field Bug_Deluxe.known_objects = Bug_Deluxe + 0x001E",
            "Kraner.position = (10, 100)",
            "Bug_Deluxe.known_objects += blood",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        for low_level in ["pair_record", "bit_flag", ".s05", ".s0B"] {
            assert!(
                !decompiled.source.contains(low_level),
                "retained {low_level}"
            );
        }
        assert_eq!(compile(&decompiled.source).unwrap(), image);

        let edited = decompiled
            .source
            .replace("(10, 100)", "(100, 10)")
            .replace("known_objects += blood", "known_objects -= blood");
        let expected = vec![
            0xBD,
            0x18,
            0x01,
            0x64,
            0x00,
            0x0A,
            0x00,
            vm::OP_BIT_FLAG,
            vm::OP_POP,
            0x1E,
            0x02,
            0x02,
            0xFF,
        ];
        assert_eq!(compile(&edited).unwrap(), expected);

        let query_image = vec![
            vm::OP_PUSH,
            0x08,
            0x00,
            vm::OP_BIT_FLAG,
            0x1E,
            0x02,
            0x02,
            vm::OP_POP,
            0xFF,
        ];
        let query =
            decompile_structured_cod_with_symbols(&query_image, &var, &HashMap::new(), &symbols)
                .unwrap();
        assert!(
            query
                .source
                .contains("require blood in Bug_Deluxe.known_objects")
        );
        assert_eq!(compile(&query.source).unwrap(), query_image);

        let query_position = vec![
            vm::OP_PUSH,
            0x0B,
            0x00,
            0xBD,
            0x18,
            0x01,
            0x0A,
            0x00,
            0x64,
            0x00,
            vm::OP_POP,
            0xFF,
        ];
        let query =
            decompile_structured_cod_with_symbols(&query_position, &var, &HashMap::new(), &symbols)
                .unwrap();
        assert!(
            query
                .source
                .contains("pair_record 0xBD Kraner.position 0x000A 0x0064")
        );
        assert!(!query.source.contains("Kraner.position = ("));
        assert_eq!(compile(&query.source).unwrap(), query_position);

        let wrong_pair_opcode = [0xB8, 0x18, 0x01, 0x0A, 0x00, 0x64, 0x00, 0xFF];
        let fallback = decompile_structured_cod_with_symbols(
            &wrong_pair_opcode,
            &var,
            &HashMap::new(),
            &symbols,
        )
        .unwrap();
        assert!(fallback.source.contains("pair_record 0xB8 Kraner.position"));
        assert_eq!(compile(&fallback.source).unwrap(), wrong_pair_opcode);

        let wrong_link_index = [vm::OP_BIT_FLAG, 0x1E, 0x02, 0x03, 0xFF];
        let fallback = decompile_structured_cod_with_symbols(
            &wrong_link_index,
            &var,
            &HashMap::new(),
            &symbols,
        )
        .unwrap();
        assert!(
            fallback
                .source
                .contains("bit_flag Bug_Deluxe.known_objects 0x03 0")
        );
        assert!(!fallback.source.contains("known_objects +="));
        assert_eq!(compile(&fallback.source).unwrap(), wrong_link_index);

        let mut wrong_symbols = symbols.clone();
        wrong_symbols[2].name = "not_blood".to_string();
        let fallback = decompile_structured_cod_with_symbols(
            &image[7..],
            &var,
            &HashMap::new(),
            &wrong_symbols,
        )
        .unwrap();
        assert!(
            fallback
                .source
                .contains("bit_flag Bug_Deluxe.known_objects 0x02 0")
        );
        assert!(!fallback.source.contains("known_objects +="));
        assert_eq!(compile(&fallback.source).unwrap(), &image[7..]);

        for invalid in [
            "Kraner.position = 0x000A, 0x0064",
            "Kraner.position = (0x000A)",
            "Bug_Deluxe.known_objects += Kraner",
            "require blood inside Bug_Deluxe.known_objects",
        ] {
            let source = format!("// format: {SOURCE_FORMAT}\n{invalid}\nhalt\n");
            assert!(compile(&source).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn shipped_positions_and_blood_links_match_their_resource_domains() {
        let Some(root) = game_dir() else { return };
        let mut link_statements = 0;
        let mut position_statements = 0;

        for script in 1..=5 {
            let read = |extension: &str| {
                std::fs::read(root.join(format!("SCRIPT{script}.{extension}"))).unwrap()
            };
            let image = read("COD");
            let var = read("VAR");
            let dictionary = crate::script::parse_dictionary(&read("DIC"));
            let symbols = crate::script::parse_deb(&read("DEB"));

            assert_eq!(symbols[2].name, "blood");
            assert_eq!(symbols[2].offset, 0x0028);
            assert_eq!(symbols[2].kind, 1);
            assert_eq!(u16::from_le_bytes([var[0x28], var[0x29]]), 1);

            for symbol in symbols.iter().filter(|symbol| symbol.kind == 1) {
                let base = usize::from(symbol.offset);
                assert!(base + 2 <= var.len());
                if u16::from_le_bytes([var[base], var[base + 1]]) == 2 {
                    assert!(base + 0x36 <= var.len());
                    assert!(
                        var[base + 0x1E..base + 0x36].iter().all(|&byte| byte == 0),
                        "SCRIPT{script} {} has a nonempty initial link set",
                        symbol.name
                    );
                }
            }

            let decompiled =
                decompile_structured_cod_with_symbols(&image, &var, &dictionary, &symbols).unwrap();
            link_statements += decompiled.source.matches(".known_objects += blood").count();
            position_statements += decompiled.source.matches(".position = (").count();
            assert!(!decompiled.source.contains("pair_record "));
            assert!(!decompiled.source.contains("bit_flag "));
            assert_eq!(
                compile_with_dictionary(&decompiled.source, &dictionary).unwrap(),
                image
            );
        }

        assert_eq!(link_statements, 3);
        assert_eq!(position_statements, 2);
    }

    #[test]
    fn semantic_flags_and_topics_preserve_query_mode_and_encoding() {
        let image = vec![
            0xA9, 0x01, 0x0F, 0x00, // conditional block -> query mode
            0xB0, 0x02, 0x01, 0x02, 0x00, // alternate encoding: planet.known
            0xA1, // return to update mode
            0xBC, 0x46, 0x02, 0x34, 0x12, // actor.topic = "secrets"
            0xFF,
        ];
        let mut var = vec![0; 0x280];
        var[0x100..0x102].copy_from_slice(&8u16.to_le_bytes());
        var[0x200..0x202].copy_from_slice(&2u16.to_le_bytes());
        let symbols = vec![
            DebSymbol {
                name: "planet".to_string(),
                offset: 0x0100,
                kind: 1,
            },
            DebSymbol {
                name: "actor".to_string(),
                offset: 0x0200,
                kind: 1,
            },
        ];
        let dictionary = HashMap::from([(0x1234, "secrets".to_string())]);
        let decompiled =
            decompile_structured_cod_with_symbols(&image, &var, &dictionary, &symbols).unwrap();
        for statement in [
            "field planet.flags = planet + 0x0002",
            "field actor.topic = actor + 0x0046",
            "check planet is known",
            "actor.topic = \"secrets\"",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert_eq!(
            compile_with_dictionary(&decompiled.source, &dictionary).unwrap(),
            image
        );
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
            "guard_push script_branch_1",
            "state_array_test 0xFE",
            "then",
            "timer[2] = 22136",
            "jump script_jump_target_1",
            "activation enabled until script_branch_2",
            "during bridge",
            "during travel",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert_eq!(compile(&decompiled.source).unwrap(), expected);
    }

    #[test]
    fn countdown_timers_and_offered_topics_use_domain_syntax() {
        let cod = vec![
            vm::OP_COND_STATE_ARRAY,
            0x03,
            0x0A,
            0x00,
            vm::OP_PUSH,
            0x0A,
            0x00,
            vm::OP_COND_STATE_ARRAY,
            0x03,
            vm::OP_POP,
            vm::OP_COND_STATE_ARRAY,
            0x01,
            0xFF,
            0xFF,
            0xFF,
        ];
        let decompiled = decompile(ImageKind::Cod, &cod, &HashMap::new()).unwrap();
        for statement in [
            "timer[3] = 10",
            "require timer[3] == 0",
            "timer[1] = disabled",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert!(!decompiled.source.contains("state_array_"));
        assert_eq!(compile(&decompiled.source).unwrap(), cod);

        let edited = decompiled
            .source
            .replace("timer[3] = 10", "timer[3] = 25")
            .replace("timer[1] = disabled", "timer[1] = 0");
        let expected = vec![
            vm::OP_COND_STATE_ARRAY,
            0x03,
            0x19,
            0x00,
            vm::OP_PUSH,
            0x0A,
            0x00,
            vm::OP_COND_STATE_ARRAY,
            0x03,
            vm::OP_POP,
            vm::OP_COND_STATE_ARRAY,
            0x01,
            0x00,
            0x00,
            0xFF,
        ];
        assert_eq!(compile(&edited).unwrap(), expected);

        let bas = [0xA7, 0x34, 0x12, 0xFF];
        let dictionary = HashMap::from([
            (0x1234, "gladis".to_string()),
            (0x5678, "revelation".to_string()),
        ]);
        let decompiled = decompile(ImageKind::Bas, &bas, &dictionary).unwrap();
        assert!(decompiled.source.contains("offer topic \"gladis\""));
        assert!(!decompiled.source.contains("presentation_register"));
        assert_eq!(
            compile_with_dictionary(&decompiled.source, &dictionary).unwrap(),
            bas
        );
        let edited = decompiled
            .source
            .replace("offer topic \"gladis\"", "offer topic \"revelation\"");
        assert_eq!(
            compile_with_dictionary(&edited, &dictionary).unwrap(),
            [0xA7, 0x78, 0x56, 0xFF]
        );

        for invalid in [
            "timer[30] = 10",
            "timer[1] = 32768",
            "require timer[1] != 0",
            "offer idea \"gladis\"",
        ] {
            let source = format!("// format: {SOURCE_FORMAT}\n{invalid}\nhalt\n");
            assert!(
                compile_with_dictionary(&source, &dictionary).is_err(),
                "accepted {invalid}"
            );
        }
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
        for statement in ["when {", "} then {"] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert!(decompiled.source.contains("when {\n"));
        assert!(decompiled.source.contains("    require choice == 0x1234"));
        assert!(!decompiled.source.contains("// GUARD_POP"));
        assert_eq!(compile(&decompiled.source).unwrap(), image);

        let malformed = decompiled.source.replace("when {", "when wrong extra {");
        assert!(compile(&malformed).is_err());
    }

    #[test]
    fn alternate_exit_guards_compile_as_if_else_blocks() {
        let image = vec![
            0xA0, 0x0A, 0x00, // failed condition enters else body
            0xA3, 0x34, 0x12, // condition
            0xA1, // then
            0xA4, 0x0E, 0x00, // successful body skips else body
            0xA5, 0x02, 0x78, 0x56, // else body
            0xFF,
        ];
        let decompiled =
            decompile_structured_with_symbols(ImageKind::Cod, &image, &HashMap::new(), &[])
                .unwrap();
        assert_eq!(decompiled.structured_guards, 1);
        assert_eq!(decompiled.unstructured_guards, 0);
        assert!(decompiled.source.contains("} else {"));
        assert!(!decompiled.source.contains("guard_push"));
        assert!(!decompiled.source.contains("jump script_jump_target_1"));
        assert_eq!(compile(&decompiled.source).unwrap(), image);
    }

    #[test]
    fn structured_guard_preserves_a_nonlocal_entry_label() {
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
        assert_eq!(decompiled.structured_guards, 1);
        assert_eq!(decompiled.unstructured_guards, 0);
        assert!(decompiled.source.contains("when {"));
        assert!(decompiled.source.contains("script_jump_target_1:"));
        assert!(!decompiled.source.contains("guard_push"));
        assert_eq!(compile(&decompiled.source).unwrap(), image);
    }

    #[test]
    fn deb_object_aliases_compile_to_exact_var_offsets() {
        let image = vec![
            0xA6, 0x34, 0x12, 0xFF, 0x00, 0x80, 0x00, 0x00, // text line object
            0xC9, 0x34, 0x12, // clear the same object record
            0xFF,
        ];
        let symbols = vec![DebSymbol {
            name: "Tina_Burner".to_string(),
            offset: 0x1234,
            kind: 1,
        }];
        let decompiled =
            decompile_structured_with_symbols(ImageKind::Cod, &image, &HashMap::new(), &symbols)
                .unwrap();
        assert_eq!(decompiled.object_aliases, 1);
        assert_eq!(decompiled.object_alias_uses, 2);
        for statement in [
            "object Tina_Burner = 0x1234",
            "say Tina_Burner presentation=8",
            "record_clear Tina_Burner",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert_eq!(compile(&decompiled.source).unwrap(), image);

        let duplicate = concat!(
            "; format: bloodscript-ir-v1\n",
            "00000000: OBJECT object_same 1234\n",
            "00000000: OBJECT object_same 5678\n",
            "00000000: END\n",
        );
        assert!(compile(duplicate).is_err());

        let alias_as_immediate = concat!(
            "; format: bloodscript-ir-v1\n",
            "00000000: OBJECT object_value 1234\n",
            "00000000: SHARED_STATE BF object_value F5 C1 object_value\n",
            "00000007: END\n",
        );
        assert!(compile(alias_as_immediate).is_err());
    }

    #[test]
    fn field_aliases_require_exact_object_kind_matrix_evidence() {
        let image = vec![0xC0, 0x18, 0x01, 0xF5, 0xC1, 0x01, 0x00, 0xFF];
        let mut var = vec![0; 0x200];
        var[0x100..0x102].copy_from_slice(&2u16.to_le_bytes());
        let symbols = vec![DebSymbol {
            name: "actor".to_string(),
            offset: 0x0100,
            kind: 1,
        }];
        let decompiled =
            decompile_structured_cod_with_symbols(&image, &var, &HashMap::new(), &symbols).unwrap();
        assert_eq!(decompiled.object_aliases, 1);
        assert_eq!(decompiled.object_alias_uses, 0);
        assert_eq!(decompiled.field_aliases, 1);
        assert_eq!(decompiled.field_alias_uses, 1);
        for statement in [
            "object actor = 0x0100",
            "field actor.current_location = actor + 0x0018",
            "state[actor.current_location] = 1",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert_eq!(compile(&decompiled.source).unwrap(), image);

        let alias_as_immediate = concat!(
            "; format: bloodscript-ir-v1\n",
            "00000000: OBJECT object_actor 0100\n",
            "00000000: FIELD field_actor object_actor 0018\n",
            "00000000: SHARED_STATE BF field_actor F5 C1 field_actor\n",
            "00000007: END\n",
        );
        assert!(compile(alias_as_immediate).is_err());

        let mut ambiguous_var = vec![0; 0x200];
        ambiguous_var[0x00DE..0x00E0].copy_from_slice(&2u16.to_le_bytes());
        ambiguous_var[0x0100..0x0102].copy_from_slice(&2u16.to_le_bytes());
        let ambiguous_symbols = vec![
            DebSymbol {
                name: "first".to_string(),
                offset: 0x00DE,
                kind: 1,
            },
            DebSymbol {
                name: "second".to_string(),
                offset: 0x0100,
                kind: 1,
            },
        ];
        let ambiguous = decompile_structured_cod_with_symbols(
            &image,
            &ambiguous_var,
            &HashMap::new(),
            &ambiguous_symbols,
        )
        .unwrap();
        assert_eq!(ambiguous.field_aliases, 0);
        assert!(ambiguous.source.contains("state[0x0118] = 1"));

        let base_image = vec![0xC0, 0x02, 0x01, 0xF5, 0xC1, 0x01, 0x00, 0xFF];
        let mut base_var = vec![0; 0x200];
        base_var[0x0100..0x0102].copy_from_slice(&2u16.to_le_bytes());
        base_var[0x0102..0x0104].copy_from_slice(&4u16.to_le_bytes());
        let base_symbols = vec![
            DebSymbol {
                name: "owner".to_string(),
                offset: 0x0100,
                kind: 1,
            },
            DebSymbol {
                name: "exact".to_string(),
                offset: 0x0102,
                kind: 1,
            },
        ];
        let exact_base = decompile_structured_cod_with_symbols(
            &base_image,
            &base_var,
            &HashMap::new(),
            &base_symbols,
        )
        .unwrap();
        assert_eq!(exact_base.field_aliases, 0);
        assert_eq!(exact_base.object_aliases, 1);
        assert!(exact_base.source.contains("state[exact] = 1"));
    }

    #[test]
    fn inline_dictionary_words_are_interned_without_losing_offsets() {
        let image = vec![
            0xA3, 0x34, 0x12, // concept guard
            0xA6, 0x00, 0x20, 0xFF, 0x00, 0x80, 0x34, 0x12, 0x00, 0x00, // text
            0xFF,
        ];
        let dictionary = HashMap::from([(0x1234, "TALK".to_string())]);
        let decompiled =
            decompile_structured_with_symbols(ImageKind::Cod, &image, &dictionary, &[]).unwrap();
        assert_eq!(decompiled.dictionary_offsets, 1);
        assert_eq!(decompiled.dictionary_uses, 2);
        for statement in [
            "require choice == \"TALK\"",
            "say 0x2000 presentation=8 : \"TALK\"",
        ] {
            assert!(
                decompiled.source.contains(statement),
                "missing {statement} in:\n{}",
                decompiled.source
            );
        }
        assert!(!decompiled.source.contains("dic_word"));
        assert_eq!(
            compile_with_dictionary(&decompiled.source, &dictionary).unwrap(),
            image
        );

        let duplicate = concat!(
            "; format: bloodscript-ir-v1\n",
            "00000000: DIC_WORD dic_same 1234\n",
            "00000000: DIC_WORD dic_same 5678\n",
            "00000000: END\n",
        );
        assert!(compile(duplicate).is_err());

        let alias_as_immediate = concat!(
            "; format: bloodscript-ir-v1\n",
            "00000000: DIC_WORD dic_value 1234\n",
            "00000000: SHARED_STATE BF 2000 F5 C1 dic_value\n",
            "00000007: END\n",
        );
        assert!(compile(alias_as_immediate).is_err());

        let ambiguous_image = vec![
            0xA3, 0x34, 0x12, // first concept guard
            0xA3, 0x78, 0x56, // same text at a different DIC offset
            0xA3, 0x34, 0x12, // first offset again
            0xFF,
        ];
        let ambiguous_dictionary =
            HashMap::from([(0x1234, "SAME".to_string()), (0x5678, "SAME".to_string())]);
        let ambiguous = decompile_structured_with_symbols(
            ImageKind::Cod,
            &ambiguous_image,
            &ambiguous_dictionary,
            &[],
        )
        .unwrap();
        assert_eq!(
            ambiguous
                .source
                .lines()
                .filter(|line| line.trim() == "require choice == \"SAME\"")
                .count(),
            2
        );
        assert_eq!(ambiguous.source.matches("\"SAME\"@0x5678").count(), 1);
        assert_eq!(
            compile_with_dictionary(&ambiguous.source, &ambiguous_dictionary).unwrap(),
            ambiguous_image
        );

        let ambiguous_bare = concat!(
            "; format: bloodscript-v2\n",
            "CONCEPT_GUARD \"SAME\"@1234 0\n",
            "CONCEPT_GUARD \"SAME\"@5678 0\n",
            "CONCEPT_GUARD \"SAME\" 0\n",
            "END\n",
        );
        assert!(
            compile(ambiguous_bare)
                .unwrap_err()
                .to_string()
                .contains("multiple offsets")
        );
        assert_eq!(
            compile_with_dictionary(ambiguous_bare, &ambiguous_dictionary).unwrap(),
            ambiguous_image
        );
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
        assert!(decompiled.source.contains("proc entry enabled {"));
        assert!(!decompiled.source.contains(" until "));
        assert!(!decompiled.source.contains("entry_branch_"));
        assert!(decompiled.source.trim_end().ends_with('}'));
        assert!(!decompiled.source.lines().any(|line| {
            let line = line.trim_start();
            line.get(..9).is_some_and(|prefix| {
                prefix.ends_with(':') && prefix[..8].bytes().all(|byte| byte.is_ascii_hexdigit())
            })
        }));
        assert_eq!(compile(&decompiled.source).unwrap(), image);
    }

    #[test]
    fn nonstructural_procedure_target_keeps_the_explicit_fallback() {
        let image = vec![
            vm::OP_COND_JUMP,
            0x01,
            0x05,
            0x00,
            vm::OP_POP,
            vm::OP_COND_STATE_ARRAY,
            0x02,
            0x34,
            0x12,
            0xFF,
        ];
        let symbols = vec![DebSymbol {
            name: "entry".to_string(),
            offset: 1,
            kind: 2,
        }];
        let decompiled =
            decompile_with_symbols(ImageKind::Cod, &image, &HashMap::new(), &symbols).unwrap();
        assert!(
            decompiled
                .source
                .contains("proc entry enabled until entry_branch_1 {")
        );
        assert_eq!(compile(&decompiled.source).unwrap(), image);
    }

    #[test]
    fn procedure_activation_syntax_compiles_exact_bytes() {
        let image = vec![
            vm::OP_COND_JUMP,
            0x01,
            0x0D,
            0x00,
            vm::OP_POKE_BYTE,
            0x00,
            0x01,
            0x00,
            vm::OP_POKE_BYTE,
            0x01,
            0x34,
            0x12,
            vm::OP_POP,
            0xFF,
        ];
        let symbols = vec![DebSymbol {
            name: "entry".to_string(),
            offset: 1,
            kind: 2,
        }];
        let decompiled =
            decompile_with_symbols(ImageKind::Cod, &image, &HashMap::new(), &symbols).unwrap();
        assert!(decompiled.source.contains("proc entry enabled {"));
        assert!(!decompiled.source.contains(" until "));
        assert!(decompiled.source.contains("} then {"));
        assert!(decompiled.source.contains("entry.enabled = false"));
        assert!(decompiled.source.contains("poke_byte 0x1234 0x01"));
        assert_eq!(compile(&decompiled.source).unwrap(), image);

        let nonprocedure = decompiled.source.replace(
            "entry.enabled = false",
            "undeclared_procedure.enabled = false",
        );
        assert!(
            compile(&nonprocedure)
                .unwrap_err()
                .to_string()
                .contains("is not declared with proc")
        );
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
            "require choice == 0x0D26",
            "require choice != 0x0EE8",
            "request sequence \"fin.hnm\"",
            "poke_byte 0x1234 0x56",
            "sequence_slots[2] = \"scrut\"",
            "choice = none",
            "during contact",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert_eq!(compile(&decompiled.source).unwrap(), expected);

        let invalid_slot = vec![0xCC, 0x07, b'x', 0, 0, 0xFF];
        let fallback = decompile(ImageKind::Cod, &invalid_slot, &HashMap::new()).unwrap();
        assert!(fallback.source.contains("character_slot 0x07 \"x\""));
        assert_eq!(compile(&fallback.source).unwrap(), invalid_slot);
        assert!(compile("sequence_slots[7] = \"scrut\"\nhalt\n").is_err());
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
            0xAA, 0xAC, 0x34, 0x12, 0x0C, 0x00, 0xA3, 0x34, 0x12, 0x00, 0x00, 0xAC, 0x34, 0x12,
            0x00, 0x00, 0xA7, 0xBC, 0x9A, 0xFF,
        ];
        assert_eq!(compile(source).unwrap(), expected);

        let dictionary = HashMap::from([(0x1234, "topic".to_string())]);
        let decompiled = decompile(ImageKind::Bas, &expected, &dictionary).unwrap();
        assert_eq!(decompiled.raw_bytes, 0);
        for statement in [
            "yield",
            "yield_b",
            "selector_node \"topic\" selector_000C",
            "selector_000C:",
            "menu \"topic\"",
            "presentation_register 0x9ABC",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert_eq!(
            compile_with_dictionary(&decompiled.source, &dictionary).unwrap(),
            expected
        );
    }

    #[test]
    fn structured_selector_lists_compile_to_the_exact_low_level_tokens() {
        let image = vec![
            0xAA, 0xAC, 0x34, 0x12, 0x0C, 0x00, 0xA3, 0x00, 0x20, 0x00, 0x00, 0xAC, 0x00, 0x20,
            0x00, 0x00, 0xA3, 0x34, 0x12, 0x00, 0x00, 0xFF,
        ];
        let mut var = vec![0; 0x1C];
        var[0..2].copy_from_slice(&2u16.to_le_bytes());
        var[0x1A..0x1C].copy_from_slice(&1u16.to_le_bytes());
        let dictionary =
            HashMap::from([(0x1234, "talk".to_string()), (0x2000, "leave".to_string())]);
        let symbols = vec![DebSymbol {
            name: "actor".to_string(),
            offset: 0,
            kind: 1,
        }];

        let decompiled =
            decompile_structured_bas_with_symbols(&image, &var, &dictionary, &symbols).unwrap();
        assert_eq!(decompiled.structured_selector_lists, 1);
        assert_eq!(decompiled.structured_cases, 2);
        assert_eq!(decompiled.dictionary_offsets, 2);
        assert_eq!(decompiled.dictionary_uses, 4);
        for statement in [
            "selector actor_choices {",
            "case \"talk\" continues {",
            "case \"leave\" {",
            "menu \"leave\"",
            "menu \"talk\"",
        ] {
            assert!(decompiled.source.contains(statement), "missing {statement}");
        }
        assert!(!decompiled.source.contains("dic_word"));
        assert_eq!(
            compile_with_dictionary(&decompiled.source, &dictionary).unwrap(),
            image
        );

        let malformed = decompiled
            .source
            .replace("case \"talk\" continues {", "case \"talk\" {");
        assert!(compile_with_dictionary(&malformed, &dictionary).is_err());
    }

    #[test]
    fn every_shipped_bas_structures_into_exact_selector_lists() {
        let Some(root) = game_dir() else { return };
        let expected = [
            (1, 1, 1),
            (2, 10, 122),
            (3, 12, 98),
            (4, 10, 43),
            (5, 4, 57),
        ];
        for (script, list_count, case_count) in expected {
            let read = |extension: &str| {
                std::fs::read(root.join(format!("SCRIPT{script}.{extension}"))).unwrap()
            };
            let image = read("BAS");
            let var = read("VAR");
            let dictionary = crate::script::parse_dictionary(&read("DIC"));
            let symbols = crate::script::parse_deb(&read("DEB"));
            let source =
                decompile_structured_bas_with_symbols(&image, &var, &dictionary, &symbols).unwrap();
            assert_eq!(source.structured_selector_lists, list_count);
            assert_eq!(source.structured_cases, case_count);
            assert_eq!(
                compile_with_dictionary(&source.source, &dictionary).unwrap(),
                image
            );
        }
    }

    #[test]
    fn every_shipped_structured_image_uses_only_unambiguous_fields() {
        let Some(root) = game_dir() else { return };
        let expected = [
            (1, 8, 29, 0, 0, 4),
            (2, 78, 496, 5, 85, 259),
            (3, 105, 558, 5, 84, 219),
            (4, 81, 333, 4, 17, 87),
            (5, 80, 276, 1, 2, 113),
        ];
        let mut procedure_headers = 0;
        let mut else_blocks = 0;
        for (script, cod_fields, cod_uses, bas_fields, bas_uses, guards) in expected {
            let read = |extension: &str| {
                std::fs::read(root.join(format!("SCRIPT{script}.{extension}"))).unwrap()
            };
            let cod = read("COD");
            let bas = read("BAS");
            let var = read("VAR");
            let dictionary = crate::script::parse_dictionary(&read("DIC"));
            let symbols = crate::script::parse_deb(&read("DEB"));
            let cod_source =
                decompile_structured_cod_with_symbols(&cod, &var, &dictionary, &symbols).unwrap();
            let bas_source =
                decompile_structured_bas_with_symbols(&bas, &var, &dictionary, &symbols).unwrap();
            assert_eq!(
                (cod_source.field_aliases, cod_source.field_alias_uses),
                (cod_fields, cod_uses)
            );
            assert_eq!(
                (bas_source.field_aliases, bas_source.field_alias_uses),
                (bas_fields, bas_uses)
            );
            assert_eq!(cod_source.structured_guards, guards);
            assert_eq!(cod_source.unstructured_guards, 0);
            assert!(!cod_source.source.contains("guard_push"));
            assert!(!cod_source.source.contains("guard_pop"));
            assert!(!cod_source.source.contains("activation "));
            assert!(!cod_source.source.contains(" until "));
            assert!(!cod_source.source.lines().any(|line| line.trim() == "then"));
            procedure_headers += cod_source
                .source
                .lines()
                .filter(|line| {
                    line.starts_with("proc ")
                        && (line.ends_with(" enabled {") || line.ends_with(" disabled {"))
                })
                .count();
            else_blocks += cod_source.source.matches("} else {").count();
            assert_eq!(
                compile_with_dictionary(&cod_source.source, &dictionary).unwrap(),
                cod
            );
            assert_eq!(
                compile_with_dictionary(&bas_source.source, &dictionary).unwrap(),
                bas
            );
        }
        assert_eq!(procedure_headers, 480);
        assert_eq!(else_blocks, 44);
    }

    #[test]
    fn every_shipped_procedure_skip_target_is_structural() {
        let Some(root) = game_dir() else { return };
        let mut enabled = 0;
        let mut disabled = 0;

        for script in 1..=5 {
            let read = |extension: &str| {
                std::fs::read(root.join(format!("SCRIPT{script}.{extension}"))).unwrap()
            };
            let cod = read("COD");
            let symbols = crate::script::parse_deb(&read("DEB"));
            let functions = crate::script::functions_from_symbols(
                &format!("SCRIPT{script}"),
                &symbols,
                cod.len(),
            );
            let tokens = vm::walk(&cod, 0, cod.len());
            assert_eq!(cod.last(), Some(&0xFF));

            for (index, function) in functions.iter().enumerate() {
                let token = tokens
                    .iter()
                    .find(|token| token.offset() == function.offset)
                    .unwrap_or_else(|| panic!("{} has no entry token", function.name));
                let VmToken::ConditionalBlock { flags, target, .. } = token else {
                    panic!("{} does not begin with A9", function.name);
                };
                let expected_target = functions
                    .get(index + 1)
                    .map(|next| next.offset)
                    .unwrap_or(cod.len() - 1);
                assert_eq!(
                    usize::from(*target),
                    expected_target,
                    "SCRIPT{script} {} has a non-structural A9 target",
                    function.name
                );
                match flags & 1 {
                    0 => disabled += 1,
                    1 => enabled += 1,
                    _ => unreachable!(),
                }
            }
        }

        assert_eq!(enabled, 420);
        assert_eq!(disabled, 60);
    }

    #[test]
    fn every_shipped_countdown_and_offered_topic_has_domain_syntax() {
        let Some(root) = game_dir() else { return };
        let mut timers = 0;
        let mut offered_topics = 0;

        for script in 1..=5 {
            let read = |extension: &str| {
                std::fs::read(root.join(format!("SCRIPT{script}.{extension}"))).unwrap()
            };
            let cod = read("COD");
            let bas = read("BAS");
            let var = read("VAR");
            let dictionary = crate::script::parse_dictionary(&read("DIC"));
            let symbols = crate::script::parse_deb(&read("DEB"));
            let cod_source =
                decompile_structured_cod_with_symbols(&cod, &var, &dictionary, &symbols).unwrap();
            let bas_source =
                decompile_structured_bas_with_symbols(&bas, &var, &dictionary, &symbols).unwrap();

            timers += cod_source.source.matches("timer[").count();
            offered_topics += bas_source.source.matches("offer topic ").count();
            assert!(!cod_source.source.contains("state_array_"));
            assert!(!bas_source.source.contains("presentation_register"));
            assert_eq!(
                compile_with_dictionary(&cod_source.source, &dictionary).unwrap(),
                cod
            );
            assert_eq!(
                compile_with_dictionary(&bas_source.source, &dictionary).unwrap(),
                bas
            );
        }

        assert_eq!(timers, 75);
        assert_eq!(offered_topics, 19);
    }

    #[test]
    fn every_shipped_scene_gate_has_a_named_context() {
        let Some(root) = game_dir() else { return };
        let mut bridge = 0;
        let mut travel = 0;
        let mut contact = 0;

        for script in 1..=5 {
            let read = |extension: &str| {
                std::fs::read(root.join(format!("SCRIPT{script}.{extension}"))).unwrap()
            };
            let cod = read("COD");
            let var = read("VAR");
            let dictionary = crate::script::parse_dictionary(&read("DIC"));
            let symbols = crate::script::parse_deb(&read("DEB"));
            let source =
                decompile_structured_cod_with_symbols(&cod, &var, &dictionary, &symbols).unwrap();

            bridge += source.source.matches("during bridge").count();
            travel += source.source.matches("during travel").count();
            contact += source.source.matches("during contact").count();
            assert!(!source.source.contains("branch_presentation"));
            assert!(!source.source.contains("branch_gameflag"));
            assert!(!source.source.contains("branch_flag_274f"));
            assert_eq!(
                compile_with_dictionary(&source.source, &dictionary).unwrap(),
                cod
            );
        }

        assert_eq!((bridge, travel, contact), (113, 224, 65));
    }

    #[test]
    fn every_shipped_sequence_slot_names_a_descript_sequence() {
        let Some(root) = game_dir() else { return };
        let descript = crate::descript::DescriptDb::parse_file(root.join("DESCRIPT.DES")).unwrap();
        let sequence_names = descript
            .records
            .iter()
            .filter(|record| record.kind == crate::descript::RecordKind::Sequence)
            .map(|record| record.name.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let mut assignments = 0;

        for script in 1..=5 {
            let read = |extension: &str| {
                std::fs::read(root.join(format!("SCRIPT{script}.{extension}"))).unwrap()
            };
            let cod = read("COD");
            let var = read("VAR");
            let dictionary = crate::script::parse_dictionary(&read("DIC"));
            let symbols = crate::script::parse_deb(&read("DEB"));
            let source =
                decompile_structured_cod_with_symbols(&cod, &var, &dictionary, &symbols).unwrap();

            assignments += source.source.matches("sequence_slots[").count();
            assert!(!source.source.contains("character_slot"));
            for token in vm::walk(&cod, 0, cod.len()) {
                if let VmToken::CharacterSlot { slot, name, .. } = token {
                    assert!((1..=6).contains(&slot));
                    assert!(name.len() <= 15);
                    assert!(
                        sequence_names.contains(name.as_str()),
                        "SCRIPT{script} sequence slot {slot} names missing or non-sequence DESCRIPT record {name:?}"
                    );
                }
            }
            assert_eq!(
                compile_with_dictionary(&source.source, &dictionary).unwrap(),
                cod
            );
        }

        assert_eq!(assignments, 36);
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
                let rebuilt = compile_with_dictionary(&source.source, &dictionary).unwrap();
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
