//! Unified, byte-exact source for one Commander Blood VM profile.
//!
//! A profile owns five shipped images: COD logic, BAS conversations, the DEB
//! directory, the DIC lexicon, and the initial VAR state. BloodScript v8 derives
//! DEB and DIC from ordered declarations instead of exposing binary-shaped
//! directory and dictionary sections, and models VAR records as typed objects
//! instead of positional words.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use anyhow::{Context, Result, anyhow, bail};

use crate::bloodscript;
use crate::script;

const DIRECTORY_RECORD_BYTES: usize = 20;
const DIRECTORY_NAME_BYTES: usize = 16;
const KIND_SENTINEL: u16 = 0;
const KIND_OBJECT: u16 = 1;
const KIND_PROCEDURE: u16 = 2;
const KIND_CODE_LABEL: u16 = 4;
const KIND_STATE_LABEL: u16 = 5;

const SECTION_NAMES: [&str; 3] = ["state", "logic", "conversations"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileImages {
    pub name: String,
    pub cod: Vec<u8>,
    pub bas: Vec<u8>,
    pub deb: Vec<u8>,
    pub dic: Vec<u8>,
    pub var: Vec<u8>,
}

impl ProfileImages {
    pub fn image(&self, extension: &str) -> Option<&[u8]> {
        match extension {
            "COD" => Some(&self.cod),
            "BAS" => Some(&self.bas),
            "DEB" => Some(&self.deb),
            "DIC" => Some(&self.dic),
            "VAR" => Some(&self.var),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DirectoryRecord {
    name: [u8; DIRECTORY_NAME_BYTES],
    value: u16,
    kind: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StateObject {
    name: Vec<u8>,
    offset: u16,
    kind: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StateCompilation {
    image: Vec<u8>,
    objects: Vec<StateObject>,
    labels: HashMap<String, u16>,
    globals_offset: Option<u16>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StateObjectSpec {
    name: Vec<u8>,
    kind: u16,
    line: usize,
    directives: HashSet<String>,
    properties: HashMap<String, String>,
}

pub fn compile(source: &str) -> Result<ProfileImages> {
    let profile_name = parse_profile_name(source)?;
    let sections = parse_sections(source)?;
    let logic_source = section(&sections, "logic")?;
    let conversations_source = section(&sections, "conversations")?;
    let dic = compile_derived_dictionary(source, logic_source, conversations_source)?;
    let dictionary = script::parse_dictionary(&dic);
    let dictionary_entries = dictionary_entries(&dic)?;
    let mut state = compile_state(
        section(&sections, "state")?,
        logic_source,
        &dictionary_entries,
    )?;
    let (logic_body, directory_body) = prepare_logic_for_compile(logic_source, &state)?;
    let conversations_body = prepare_conversations_for_compile(conversations_source, &state)?;
    let conversations = compile_program(&conversations_body, "BAS", &dictionary)?;
    patch_conversation_roots(&mut state, &conversations)?;
    let logic = compile_program(&logic_body, "COD", &dictionary)?;
    let deb = compile_directory(&directory_body, &state, &logic)?;

    Ok(ProfileImages {
        name: profile_name,
        cod: logic.image,
        bas: conversations.image,
        deb,
        dic,
        var: state.image,
    })
}

pub fn decompile(images: &ProfileImages) -> Result<String> {
    let records = parse_directory(&images.deb)?;
    let symbols = script::parse_deb(&images.deb);
    let dictionary = script::parse_dictionary(&images.dic);
    let logic = bloodscript::decompile_structured_cod_with_symbols(
        &images.cod,
        &images.var,
        &dictionary,
        &symbols,
    )?;
    let conversations = bloodscript::decompile_structured_bas_with_symbols(
        &images.bas,
        &images.var,
        &dictionary,
        &symbols,
    )?;

    let state_label_ids = state_label_identifiers(&records);
    let procedure_ids = procedure_identifiers(&records, &logic.source)?;
    let logic_layout = bloodscript::compile_with_layout(&logic.source, &dictionary)?;
    let mut output = String::new();
    let mut logic_body =
        bloodscript::make_phrase_boundaries_explicit(program_body(&logic.source)?, &dictionary)?;
    let mut conversations_body = bloodscript::make_phrase_boundaries_explicit(
        program_body(&conversations.source)?,
        &dictionary,
    )?;
    let globals_offset = state_globals_offset(&images.var, &records)?;
    let global_names = global_state_names(&records, &state_label_ids, globals_offset);
    logic_body = replace_global_addresses(&logic_body, globals_offset, &global_names)?;
    conversations_body =
        replace_global_addresses(&conversations_body, globals_offset, &global_names)?;
    logic_body = raise_named_presentations(&logic_body)?;
    conversations_body = raise_named_presentations(&conversations_body)?;
    let concepts = dictionary_seed_words(images, &dictionary)?;
    let breaks = dictionary_break_words(&images.dic)?;

    writeln!(output, "bloodscript 8")?;
    writeln!(output, "profile {}", images.name)?;
    write!(output, "concepts")?;
    for concept in &concepts {
        write!(output, " {}", format_quoted_bytes(concept))?;
    }
    writeln!(output)?;
    writeln!(output)?;

    writeln!(output, "state {{")?;
    let dictionary_entries = dictionary_entries(&images.dic)?;
    write_state(&mut output, &images.var, &records, &dictionary_entries)?;
    writeln!(output, "}}")?;
    writeln!(output)?;

    writeln!(output, "logic {{")?;
    let logic_body = write_v8_logic(
        &records,
        &state_label_ids,
        &procedure_ids,
        &logic_body,
        &logic_layout.label_offsets,
        &images.var,
        globals_offset,
    )?;
    write_indented(&mut output, &logic_body, 4)?;
    writeln!(output, "}}")?;
    writeln!(output)?;

    writeln!(output, "conversations {{")?;
    let conversations_body = strip_program_declarations(&conversations_body);
    write_indented(&mut output, &conversations_body, 4)?;
    writeln!(output, "}}")?;

    output = integrate_dictionary_gaps(&output, &concepts, &breaks)?;
    let rebuilt = compile(&output)?;
    require_same_profile(&rebuilt, images)?;
    Ok(output)
}

pub fn require_same_profile(actual: &ProfileImages, expected: &ProfileImages) -> Result<()> {
    for extension in ["COD", "BAS", "DEB", "DIC", "VAR"] {
        let actual_image = actual.image(extension).expect("known profile extension");
        let expected_image = expected.image(extension).expect("known profile extension");
        if actual_image == expected_image {
            continue;
        }
        let first_difference = actual_image
            .iter()
            .zip(expected_image)
            .position(|(left, right)| left != right)
            .unwrap_or(actual_image.len().min(expected_image.len()));
        let actual_byte = actual_image.get(first_difference).copied();
        let expected_byte = expected_image.get(first_difference).copied();
        bail!(
            "compiled {}.{extension} differs at 0x{first_difference:04X}: compiled {actual_byte:?}, expected {expected_byte:?} (compiled {} bytes, expected {} bytes)",
            expected.name,
            actual_image.len(),
            expected_image.len()
        );
    }
    Ok(())
}

fn parse_profile_name(source: &str) -> Result<String> {
    let mut declarations = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"));
    if declarations.next() != Some("bloodscript 8") {
        bail!("unified source must begin with 'bloodscript 8'");
    }
    let profile = declarations
        .next()
        .and_then(|line| line.strip_prefix("profile "))
        .ok_or_else(|| anyhow!("unified source is missing its profile declaration"))?;
    if profile.is_empty()
        || !profile
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        bail!("invalid profile name {profile:?}");
    }
    Ok(profile.to_string())
}

fn parse_sections(source: &str) -> Result<HashMap<&str, &str>> {
    let mut starts = HashMap::new();
    let mut offset = 0usize;
    for line in source.split_inclusive('\n') {
        let without_newline = line.trim_end_matches(['\r', '\n']);
        for name in SECTION_NAMES {
            if without_newline == format!("{name} {{") {
                let open = offset + without_newline.len() - 1;
                if starts.insert(name, open).is_some() {
                    bail!("duplicate {name} section");
                }
            }
        }
        offset += line.len();
    }

    let mut sections = HashMap::new();
    for name in SECTION_NAMES {
        let open = starts
            .get(name)
            .copied()
            .ok_or_else(|| anyhow!("missing {name} section"))?;
        let close = matching_brace(source, open)
            .with_context(|| format!("finding the end of the {name} section"))?;
        sections.insert(name, &source[open + 1..close]);
    }
    Ok(sections)
}

fn matching_brace(source: &str, open: usize) -> Result<usize> {
    if source.as_bytes().get(open) != Some(&b'{') {
        bail!("expected '{{'");
    }
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    let mut line_comment = false;
    let mut cursor = open;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if line_comment {
            if byte == b'\n' {
                line_comment = false;
            }
            cursor += 1;
            continue;
        }
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            cursor += 1;
            continue;
        }
        if byte == b'/' && bytes.get(cursor + 1) == Some(&b'/') {
            line_comment = true;
            cursor += 2;
            continue;
        }
        match byte {
            b'"' => quoted = true,
            b'{' => depth += 1,
            b'}' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| anyhow!("closing brace precedes opening brace"))?;
                if depth == 0 {
                    return Ok(cursor);
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    bail!("section has no closing brace")
}

fn section<'a>(sections: &'a HashMap<&str, &str>, name: &str) -> Result<&'a str> {
    sections
        .get(name)
        .copied()
        .ok_or_else(|| anyhow!("missing {name} section"))
}

fn compile_program(
    body: &str,
    image: &str,
    dictionary: &HashMap<u16, String>,
) -> Result<bloodscript::Compilation> {
    let source = format!(
        "// format: bloodscript-program-v1\n// image: {image}\n\n{}\n",
        body.trim()
    );
    bloodscript::compile_with_layout(&source, dictionary)
        .with_context(|| format!("compiling unified {image} section"))
}

fn program_body(source: &str) -> Result<&str> {
    let mut parts = source.splitn(3, '\n');
    let format = parts
        .next()
        .ok_or_else(|| anyhow!("generated BloodScript has no format header"))?;
    let image = parts
        .next()
        .ok_or_else(|| anyhow!("generated BloodScript has no image header"))?;
    if format != "// format: bloodscript-program-v1" || !image.starts_with("// image: ") {
        bail!("unexpected generated BloodScript headers");
    }
    Ok(parts.next().unwrap_or_default().trim_matches('\n'))
}

fn compile_derived_dictionary(source: &str, logic: &str, conversations: &str) -> Result<Vec<u8>> {
    let concepts_line = source
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("concepts"))
        .ok_or_else(|| anyhow!("missing concepts declaration"))?;
    let concepts = parse_quoted_list(
        concepts_line
            .strip_prefix("concepts")
            .expect("matched concepts prefix"),
    )?;
    let breaks = source
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("dictionary blank after "))
        .map(|value| {
            let (word, rest) = parse_quoted_bytes(value)?;
            if !rest.trim().is_empty() {
                bail!("unexpected lexicon-break text {rest:?}");
            }
            Ok(word)
        })
        .collect::<Result<HashSet<_>>>()?;

    let mut image = vec![0];
    let mut seen = HashSet::new();
    let mut encountered_breaks = HashSet::new();
    let mut intern = |word: Vec<u8>| {
        if word.is_empty() || !seen.insert(word.clone()) {
            return;
        }
        image.extend_from_slice(&word);
        image.push(0);
        if breaks.contains(&word) {
            image.push(0);
            encountered_breaks.insert(word);
        }
    };
    for concept in concepts {
        intern(concept);
    }
    for word in dictionary_words_in_program(logic)? {
        intern(word);
    }
    for word in dictionary_words_in_program(conversations)? {
        intern(word);
    }
    drop(intern);
    if encountered_breaks != breaks {
        let missing = breaks
            .difference(&encountered_breaks)
            .next()
            .expect("unequal sets have a difference");
        bail!(
            "dictionary blank references a word that is never interned: {}",
            format_quoted_bytes(missing)
        );
    }
    image.extend_from_slice(&[0xff, 0]);
    Ok(image)
}

fn dictionary_words_in_program(body: &str) -> Result<Vec<Vec<u8>>> {
    let mut words = Vec::new();
    for (index, original) in body.lines().enumerate() {
        let line_number = index + 1;
        let line = code_before_comment(original).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("say ") {
            let (_, phrase_and_choices) = line
                .split_once(" : ")
                .ok_or_else(|| anyhow!("line {line_number}: say has no phrase separator"))?;
            let (phrase, rest) = parse_quoted_bytes(phrase_and_choices)
                .with_context(|| format!("line {line_number}: dialogue phrase"))?;
            words.extend(
                phrase
                    .split(|byte| matches!(*byte, b' ' | b'|'))
                    .filter(|word| !word.is_empty())
                    .map(<[u8]>::to_vec),
            );
            if let Some(choices) = rest.trim().strip_prefix("choices") {
                words.extend(parse_quoted_list(choices)?);
            } else if !rest.trim().is_empty() {
                bail!("line {line_number}: unexpected dialogue text {rest:?}");
            }
            continue;
        }
        let dictionary_operands = line.starts_with("require choice ")
            || line.contains(".topic = ")
            || line.starts_with("case ")
            || line.starts_with("menu ")
            || line.starts_with("offer topic ");
        if dictionary_operands {
            words.extend(parse_quoted_list(line)?);
        }
    }
    Ok(words)
}

fn parse_quoted_list(mut value: &str) -> Result<Vec<Vec<u8>>> {
    let mut values = Vec::new();
    loop {
        value = value
            .trim_start_matches(|character: char| character.is_whitespace() || character == ',');
        let Some(start) = value.find('"') else {
            break;
        };
        value = &value[start..];
        let (bytes, rest) = parse_quoted_bytes(value)?;
        values.push(bytes);
        value = rest;
    }
    Ok(values)
}

fn prepare_logic_for_compile(body: &str, state: &StateCompilation) -> Result<(String, String)> {
    let mut program = String::new();
    let mut directory = String::new();
    if state.labels.contains_key("tblood") {
        writeln!(directory, "state_label \"tblood\" = tblood")?;
    }
    for (index, original) in body.lines().enumerate() {
        let line_number = index + 1;
        let line = original.trim();
        if line.starts_with("dictionary blank after ") {
            continue;
        }
        if let Some(variable) = parse_variable_declaration(line, line_number)? {
            writeln!(
                directory,
                "state_label {} = {}",
                format_quoted_bytes(&variable.1),
                variable.0
            )?;
            continue;
        }
        if let Some(rest) = line.strip_prefix("export label ") {
            let (name, trailing) = parse_quoted_bytes(rest)
                .with_context(|| format!("logic line {line_number}: exported label name"))?;
            if trailing.trim().is_empty() {
                writeln!(
                    directory,
                    "code_label {} = {}",
                    format_quoted_bytes(&name),
                    identifier_from_bytes(&name)
                )?;
            } else {
                writeln!(directory, "code_label {rest}")?;
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("proc ") {
            let (identifier, rest) = rest
                .split_once(char::is_whitespace)
                .ok_or_else(|| anyhow!("logic line {line_number}: malformed procedure"))?;
            validate_identifier(identifier, line_number, "procedure")?;
            let rest = rest.trim_start();
            let (name, rewritten_rest) = if let Some(alias) = rest.strip_prefix("as ") {
                let (name, rest) = parse_quoted_bytes(alias)?;
                (name, rest.trim_start())
            } else {
                (identifier.as_bytes().to_vec(), rest)
            };
            writeln!(
                directory,
                "procedure {} = {identifier}",
                format_quoted_bytes(&name)
            )?;
            writeln!(program, "proc {identifier} {rewritten_rest}")?;
            continue;
        }
        writeln!(program, "{original}")?;
    }
    writeln!(directory, "sentinel")?;
    let program = lower_named_presentations(program.trim_matches('\n'))?;
    Ok((prepend_state_declarations(&program, state)?, directory))
}

fn prepare_conversations_for_compile(body: &str, state: &StateCompilation) -> Result<String> {
    let body = body
        .lines()
        .filter(|line| !line.trim().starts_with("dictionary blank after "))
        .collect::<Vec<_>>()
        .join("\n");
    let body = lower_named_presentations(body.trim_matches('\n'))?;
    prepend_state_declarations(&body, state)
}

fn lower_named_presentations(body: &str) -> Result<String> {
    rewrite_presentations(body, |actor, value, line| {
        if value.parse::<i16>().is_ok() {
            bail!(
                "line {line}: numeric presentation IDs are not valid in unified BloodScript; use the HNM or semantic presentation name"
            );
        }
        crate::presentation_catalog::active_line_id(actor, value)
            .ok_or_else(|| anyhow!("line {line}: unknown presentation {value:?} for {actor}"))
    })
}

fn raise_named_presentations(body: &str) -> Result<String> {
    rewrite_presentations(body, |actor, value, line| {
        let active_line_id = value.parse::<i16>().map_err(|_| {
            anyhow!("line {line}: generated presentation ID {value:?} is not decimal")
        })?;
        crate::presentation_catalog::symbolic_name(actor, active_line_id)
            .map(str::to_string)
            .ok_or_else(|| {
                anyhow!(
                    "line {line}: presentation ID {active_line_id} for {actor} has no recovered symbolic name"
                )
            })
    })
}

fn rewrite_presentations<T, F>(body: &str, mut rewrite: F) -> Result<String>
where
    T: ToString,
    F: FnMut(&str, &str, usize) -> Result<T>,
{
    let mut output = String::with_capacity(body.len());
    for (index, original) in body.lines().enumerate() {
        let line_number = index + 1;
        let code = code_before_comment(original).trim();
        let fields = code.split_whitespace().collect::<Vec<_>>();
        let rewritten = if fields.first() == Some(&"say") {
            let actor = fields
                .get(1)
                .copied()
                .ok_or_else(|| anyhow!("line {line_number}: say statement has no actor"))?;
            let presentation = fields
                .iter()
                .find_map(|field| field.strip_prefix("presentation="))
                .ok_or_else(|| anyhow!("line {line_number}: say statement has no presentation"))?;
            let replacement = format!(
                "presentation={}",
                rewrite(actor, presentation, line_number)?.to_string()
            );
            original.replacen(&format!("presentation={presentation}"), &replacement, 1)
        } else {
            original.to_string()
        };
        writeln!(output, "{rewritten}")?;
    }
    Ok(output.trim_end_matches('\n').to_string())
}

fn prepend_state_declarations(body: &str, state: &StateCompilation) -> Result<String> {
    let mut output = String::new();
    for object in &state.objects {
        let identifier = state_object_identifier(&object.name);
        writeln!(output, "object {identifier} = 0x{:04X}", object.offset)?;
    }
    if let Some(offset) = state.globals_offset {
        writeln!(output, "object globals = 0x{offset:04X}")?;
        for (name, address) in &state.labels {
            if *address >= offset {
                writeln!(
                    output,
                    "field globals.{name} = globals + 0x{:04X}",
                    address - offset
                )?;
            }
        }
    }
    for object in &state.objects {
        let identifier = state_object_identifier(&object.name);
        for (field, selector) in semantic_fields(object.kind) {
            let Some(offset) = crate::vm::field_offset(object.kind, selector) else {
                continue;
            };
            writeln!(
                output,
                "field {identifier}.{field} = {identifier} + 0x{offset:04X}"
            )?;
        }
    }
    writeln!(output)?;
    writeln!(output, "{body}")?;
    Ok(output)
}

fn semantic_fields(kind: u16) -> Vec<(&'static str, u8)> {
    let mut fields = vec![("flags", 0x00), ("action", 0x13)];
    if kind == 0x0002 {
        fields.extend([
            ("population", 0x01),
            ("aggressiveness", 0x03),
            ("energy", 0x04),
            ("encounter_count", 0x08),
            ("known_objects", 0x05),
            ("evolution", 0x07),
            ("race", 0x10),
            ("current_location", 0x11),
            ("universe", 0x0e),
            ("topic", 0x0f),
        ]);
    }
    if matches!(kind, 0x0008 | 0x0010) {
        fields.push(("position", 0x0b));
    }
    if kind == 0x0010 {
        fields.push(("current_location", 0x11));
    }
    if kind == 0x0200 {
        fields.extend([("position", 0x0b), ("current_location", 0x11)]);
    }
    if kind == 0x0400 {
        fields.push(("holder", 0x11));
    }
    fields
}

fn state_object_identifier(name: &[u8]) -> String {
    let decoded = crate::font::cp437_string(name);
    identifier_from_bytes(decoded.as_bytes())
}

fn strip_program_declarations(body: &str) -> String {
    body.lines()
        .filter(|line| {
            let line = line.trim();
            !line.starts_with("object ") && !line.starts_with("field ")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_matches('\n')
        .to_string()
}

fn write_v8_logic(
    records: &[DirectoryRecord],
    state_label_ids: &HashMap<usize, String>,
    procedure_ids: &HashMap<usize, String>,
    logic_body: &str,
    logic_label_offsets: &HashMap<String, u16>,
    var: &[u8],
    globals_offset: Option<u16>,
) -> Result<String> {
    let tblood_records = records
        .iter()
        .filter(|record| record.kind == KIND_STATE_LABEL && trimmed_name(&record.name) == b"tblood")
        .count();
    if tblood_records != 1 {
        bail!("DEB must contain exactly one compiler-injected tblood state symbol");
    }
    let clean = strip_program_declarations(logic_body);
    let (preamble, mut procedures) = split_procedure_chunks(&clean)?;
    let mut code_labels: HashMap<u16, Vec<&str>> = HashMap::new();
    for (identifier, &offset) in logic_label_offsets {
        code_labels.entry(offset).or_default().push(identifier);
    }
    for identifiers in code_labels.values_mut() {
        identifiers.sort_unstable();
    }
    let procedure_by_offset: HashMap<u16, &str> = records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.kind == KIND_PROCEDURE && record.value != 0)
        .filter_map(|(index, record)| {
            procedure_ids
                .get(&index)
                .map(|identifier| (record.value - 1, identifier.as_str()))
        })
        .collect();
    let mut output = String::new();
    if !preamble.trim().is_empty() {
        writeln!(output, "{}", preamble.trim_matches('\n'))?;
    }
    for (index, record) in records.iter().enumerate() {
        let name = format_quoted_bytes(trimmed_name(&record.name));
        match record.kind {
            KIND_OBJECT => {}
            KIND_SENTINEL
                if record.name.iter().all(|byte| *byte == 0) && record.value == u16::MAX => {}
            KIND_PROCEDURE => {
                let identifier = procedure_ids
                    .get(&index)
                    .ok_or_else(|| anyhow!("no generated procedure matches DEB record {index}"))?;
                let chunk = procedures
                    .remove(identifier)
                    .ok_or_else(|| anyhow!("missing source for procedure {identifier:?}"))?;
                let chunk =
                    add_procedure_export_name(&chunk, identifier, trimmed_name(&record.name))?;
                writeln!(output, "{}", chunk.trim_matches('\n'))?;
            }
            KIND_CODE_LABEL => {
                let preferred = identifier_from_bytes(trimmed_name(&record.name));
                let identifier = code_labels
                    .get(&record.value)
                    .and_then(|identifiers| {
                        identifiers
                            .iter()
                            .copied()
                            .find(|identifier| *identifier == preferred)
                            .or_else(|| {
                                let prefix = format!("{preferred}_");
                                identifiers
                                    .iter()
                                    .copied()
                                    .find(|identifier| identifier.starts_with(&prefix))
                            })
                            .or_else(|| identifiers.first().copied())
                    })
                    .or_else(|| procedure_by_offset.get(&record.value).copied())
                    .ok_or_else(|| {
                        anyhow!(
                            "DEB code label {name} at 0x{:04X} has no BloodScript label",
                            record.value
                        )
                    })?;
                if identifier == preferred {
                    writeln!(output, "export label {name}")?;
                } else {
                    writeln!(output, "export label {name} = {identifier}")?;
                }
            }
            KIND_STATE_LABEL => {
                let identifier = state_label_ids.get(&index).ok_or_else(|| {
                    anyhow!("no state marker was generated for DEB record {index}")
                })?;
                if trimmed_name(&record.name) == b"tblood" {
                    let globals_offset = globals_offset.ok_or_else(|| {
                        anyhow!("tblood exists but VAR has no compiler-state tail")
                    })?;
                    if record.value != globals_offset {
                        bail!(
                            "tblood is at VAR byte {}, expected compiler-state boundary {}",
                            record.value,
                            globals_offset
                        );
                    }
                    if read_word(var, usize::from(record.value))? != 0 {
                        bail!("compiler-injected tblood state must start at zero");
                    }
                    continue;
                }
                let globals_offset = globals_offset.ok_or_else(|| {
                    anyhow!("state variable {name} exists but VAR has no variable tail")
                })?;
                if record.value < globals_offset {
                    bail!("state symbol {name} is inside an object and has no typed source form");
                }
                let value = read_word(var, usize::from(record.value))?;
                if trimmed_name(&record.name) == identifier.as_bytes() {
                    writeln!(output, "global {identifier} = {value}")?;
                } else {
                    writeln!(output, "global {identifier} as {name} = {value}")?;
                }
            }
            _ => bail!("unsupported DEB kind {} in BloodScript v8", record.kind),
        }
    }
    if let Some(identifier) = procedures.keys().next() {
        bail!("procedure {identifier:?} has no DEB entry");
    }
    Ok(output.trim_matches('\n').to_string())
}

fn split_procedure_chunks(body: &str) -> Result<(String, HashMap<String, String>)> {
    let mut preamble = String::new();
    let mut procedures = HashMap::new();
    let mut current_name: Option<String> = None;
    let mut current = String::new();
    for line in body.lines() {
        if let Some(identifier) = line
            .trim()
            .strip_prefix("proc ")
            .and_then(|rest| rest.split_whitespace().next())
        {
            if let Some(name) = current_name.take() {
                procedures.insert(name, current.trim_matches('\n').to_string());
                current.clear();
            }
            current_name = Some(identifier.to_string());
        }
        if current_name.is_some() {
            writeln!(current, "{line}")?;
        } else {
            writeln!(preamble, "{line}")?;
        }
    }
    if let Some(name) = current_name {
        procedures.insert(name, current.trim_matches('\n').to_string());
    }
    Ok((preamble, procedures))
}

fn add_procedure_export_name(chunk: &str, identifier: &str, name: &[u8]) -> Result<String> {
    if name == identifier.as_bytes() {
        return Ok(chunk.to_string());
    }
    let mut lines = chunk.lines();
    let first = lines
        .next()
        .ok_or_else(|| anyhow!("procedure {identifier:?} has no source"))?;
    let prefix = format!("proc {identifier} ");
    let rest = first
        .trim()
        .strip_prefix(&prefix)
        .ok_or_else(|| anyhow!("procedure {identifier:?} has a malformed declaration"))?;
    let indent = &first[..first.len() - first.trim_start().len()];
    let mut output = format!(
        "{indent}proc {identifier} as {} {rest}",
        format_quoted_bytes(name)
    );
    for line in lines {
        writeln!(output)?;
        output.push_str(line);
    }
    Ok(output)
}

fn dictionary_seed_words(
    images: &ProfileImages,
    dictionary: &HashMap<u16, String>,
) -> Result<Vec<Vec<u8>>> {
    let entries = dictionary_entries(&images.dic)?;
    let physical = entries
        .iter()
        .filter(|(_, word)| !word.is_empty() && word.as_slice() != [0xff])
        .map(|(_, word)| word.clone())
        .collect::<Vec<_>>();
    let entry_by_offset = entries.iter().cloned().collect::<HashMap<_, _>>();
    let mut seen_offsets = HashSet::new();
    let first_use = bloodscript::dictionary_operand_order(&images.cod, &images.bas, dictionary)
        .into_iter()
        .filter(|offset| seen_offsets.insert(*offset))
        .filter_map(|offset| entry_by_offset.get(&offset).cloned())
        .filter(|word| !word.is_empty() && word.as_slice() != [0xff])
        .collect::<Vec<_>>();
    for seed_len in 0..=physical.len() {
        let seeds = physical[..seed_len].to_vec();
        let mut derived = seeds.clone();
        for word in &first_use {
            if !derived.contains(word) {
                derived.push(word.clone());
            }
        }
        if derived == physical {
            return Ok(seeds);
        }
    }
    bail!("physical dictionary order is not seed plus first use")
}

fn integrate_dictionary_gaps(
    source: &str,
    concepts: &[Vec<u8>],
    breaks: &[Vec<u8>],
) -> Result<String> {
    let mut seen = concepts.iter().cloned().collect::<HashSet<_>>();
    let mut remaining = breaks.iter().cloned().collect::<HashSet<_>>();
    let mut output = String::new();
    for original in source.lines() {
        writeln!(output, "{original}")?;
        let indentation = &original[..original.len() - original.trim_start().len()];
        for word in dictionary_words_in_program(original)? {
            if seen.insert(word.clone()) && remaining.remove(&word) {
                writeln!(
                    output,
                    "{indentation}dictionary blank after {}",
                    format_quoted_bytes(&word)
                )?;
            }
        }
    }
    if let Some(word) = remaining.into_iter().next() {
        bail!(
            "dictionary blank cannot be attached to first use of {}",
            format_quoted_bytes(&word)
        );
    }
    Ok(output)
}

fn dictionary_break_words(image: &[u8]) -> Result<Vec<Vec<u8>>> {
    let entries = dictionary_entries(image)?;
    let mut breaks = Vec::new();
    let mut previous = None;
    for (_, word) in entries {
        if word.is_empty() {
            if let Some(word) = previous.take() {
                breaks.push(word);
            }
        } else {
            previous = Some(word);
        }
    }
    Ok(breaks)
}

fn dictionary_entries(image: &[u8]) -> Result<Vec<(u16, Vec<u8>)>> {
    let mut entries = Vec::new();
    let mut cursor = 0usize;
    while cursor < image.len() {
        let offset = u16::try_from(cursor).map_err(|_| anyhow!("DIC exceeds 64 KiB"))?;
        let length = image[cursor..]
            .iter()
            .position(|byte| *byte == 0)
            .ok_or_else(|| anyhow!("unterminated DIC entry at {cursor}"))?;
        entries.push((offset, image[cursor..cursor + length].to_vec()));
        cursor += length + 1;
    }
    Ok(entries)
}

fn compile_state(
    body: &str,
    logic: &str,
    dictionary: &[(u16, Vec<u8>)],
) -> Result<StateCompilation> {
    let specs = parse_state_objects(body)?;
    if specs.is_empty() {
        bail!("state section contains no objects");
    }

    let mut objects = Vec::with_capacity(specs.len());
    let mut next_offset = 0usize;
    for spec in &specs {
        fixed_name(&spec.name, spec.line)?;
        let offset = u16::try_from(next_offset)
            .map_err(|_| anyhow!("state line {}: VAR image exceeds 64 KiB", spec.line))?;
        objects.push(StateObject {
            name: spec.name.clone(),
            offset,
            kind: spec.kind,
        });
        next_offset += object_record_size(spec.kind)
            .ok_or_else(|| anyhow!("state line {}: unsupported object kind", spec.line))?;
    }

    let by_name = objects
        .iter()
        .map(|object| (object.name.clone(), object.offset))
        .collect::<HashMap<_, _>>();
    let mut image = vec![0; next_offset];
    for (spec, object) in specs.iter().zip(&objects) {
        compile_state_object(&mut image, spec, object, &by_name, dictionary)?;
    }

    let orxx = objects
        .iter()
        .find(|object| object.kind == 0x0200 && object.name.eq_ignore_ascii_case(b"orxx"))
        .ok_or_else(|| anyhow!("state section is missing its navigation_controller \"orxx\""))?;
    let tblood_offset =
        u16::try_from(image.len()).map_err(|_| anyhow!("compiler state exceeds 64 KiB"))?;
    if tblood_offset != orxx.offset + 36 {
        bail!("compiler-injected tblood word does not immediately follow orxx");
    }
    let mut labels = HashMap::from([("tblood".to_string(), tblood_offset)]);
    image.extend_from_slice(&0u16.to_le_bytes());
    let globals_offset = Some(tblood_offset);
    for (index, original) in logic.lines().enumerate() {
        let line_number = index + 1;
        let line = code_before_comment(original).trim();
        let Some((identifier, _, value)) = parse_variable_declaration(line, line_number)? else {
            continue;
        };
        let address = u16::try_from(image.len())
            .map_err(|_| anyhow!("logic line {line_number}: VAR image exceeds 64 KiB"))?;
        if labels.insert(identifier.clone(), address).is_some() {
            bail!("logic line {line_number}: duplicate variable {identifier:?}");
        }
        image.extend_from_slice(&value.to_le_bytes());
    }

    Ok(StateCompilation {
        image,
        objects,
        labels,
        globals_offset,
    })
}

fn parse_state_objects(body: &str) -> Result<Vec<StateObjectSpec>> {
    let mut specs = Vec::new();
    let mut open: Option<StateObjectSpec> = None;
    for (index, original) in body.lines().enumerate() {
        let line_number = index + 1;
        let line = code_before_comment(original).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(spec) = open.as_mut() {
            if line == "}" {
                specs.push(open.take().expect("open object exists"));
                continue;
            }
            if let Some((name, value)) = line.split_once('=') {
                let name = name.trim();
                validate_identifier(name, line_number, "state property")?;
                if spec
                    .properties
                    .insert(name.to_string(), value.trim().to_string())
                    .is_some()
                {
                    bail!("state line {line_number}: duplicate property {name:?}");
                }
            } else {
                validate_identifier(line, line_number, "state directive")?;
                if !spec.directives.insert(line.to_string()) {
                    bail!("state line {line_number}: duplicate directive {line:?}");
                }
            }
            continue;
        }

        let (kind_name, rest) = line
            .split_once(char::is_whitespace)
            .ok_or_else(|| anyhow!("state line {line_number}: expected TYPE \"NAME\" {{"))?;
        let (name, rest) = parse_quoted_bytes(rest.trim_start())
            .with_context(|| format!("state line {line_number}: object name"))?;
        if rest.trim() != "{" {
            bail!("state line {line_number}: expected '{{' after object name");
        }
        let kind = parse_state_kind(kind_name, line_number)?;
        open = Some(StateObjectSpec {
            name,
            kind,
            line: line_number,
            directives: HashSet::new(),
            properties: HashMap::new(),
        });
    }
    if let Some(spec) = open {
        bail!(
            "state line {}: object {} has no closing brace",
            spec.line,
            format_quoted_bytes(&spec.name)
        );
    }
    Ok(specs)
}

fn compile_state_object(
    image: &mut [u8],
    spec: &StateObjectSpec,
    object: &StateObject,
    by_name: &HashMap<Vec<u8>, u16>,
    dictionary: &[(u16, Vec<u8>)],
) -> Result<()> {
    let start = usize::from(object.offset);
    write_word(image, start, spec.kind)?;
    write_word(image, start + 2, compile_status(spec)?)?;
    if kind_has_inline_name(spec.kind) {
        let name = fixed_name(&spec.name, spec.line)?;
        image[start + 4..start + 20].copy_from_slice(&name);
    }

    let baby1 = by_name
        .get(b"baby1".as_slice())
        .copied()
        .ok_or_else(|| anyhow!("state section is missing universe \"baby1\""))?;
    let property = |name: &str| spec.properties.get(name).map(String::as_str);
    let number = |name: &str, default: u16| -> Result<u16> {
        property(name).map_or(Ok(default), |value| {
            parse_u16(value, spec.line, &format!("{name} value"))
        })
    };
    let reference = |name: &str, default: u16| -> Result<u16> {
        property(name).map_or(Ok(default), |value| {
            parse_object_reference(value, by_name, spec.line, name)
        })
    };

    match spec.kind {
        0x0001 => {
            write_word(image, start + 4, number("population", 0)?)?;
            write_word(image, start + 6, reference("location", baby1)?)?;
            write_word(image, start + 32, reference("universe", baby1)?)?;
            require_properties(spec, &["population", "location", "universe"])?;
        }
        0x0002 => {
            write_word(image, start + 20, parse_race(property("race"), spec.line)?)?;
            write_word(image, start + 22, number("population", 50)?)?;
            write_word(image, start + 24, reference("location", u16::MAX)?)?;
            write_word(image, start + 50, number("aggressiveness", 0)?)?;
            write_word(image, start + 52, number("energy", 0)?)?;
            write_word(image, start + 54, number("encounters", 0)?)?;
            write_word(image, start + 56, number("evolution", 0)?)?;
            write_word(image, start + 68, reference("universe", baby1)?)?;
            write_word(
                image,
                start + 70,
                parse_topic(property("topic"), dictionary, spec.line)?,
            )?;
            require_properties(
                spec,
                &[
                    "race",
                    "population",
                    "location",
                    "aggressiveness",
                    "energy",
                    "encounters",
                    "evolution",
                    "universe",
                    "topic",
                ],
            )?;
        }
        0x0008 => {
            write_word(image, start + 20, number("visits", 0)?)?;
            write_word(image, start + 22, reference("location", baby1)?)?;
            write_pair(
                image,
                start + 24,
                parse_pair(property("position"), spec.line, "position")?,
            )?;
            write_word(image, start + 28, reference("universe", baby1)?)?;
            require_properties(spec, &["visits", "location", "position", "universe"])?;
        }
        0x0010 => {
            write_word(image, start + 20, number("visits", 0)?)?;
            write_word(image, start + 22, reference("location", baby1)?)?;
            write_pair(
                image,
                start + 24,
                parse_pair(property("position"), spec.line, "position")?,
            )?;
            write_word(image, start + 34, reference("universe", baby1)?)?;
            require_properties(spec, &["visits", "location", "position", "universe"])?;
        }
        0x0040 => require_properties(spec, &[])?,
        0x0080 => {
            write_word(image, start + 20, reference("parent", baby1)?)?;
            write_word(image, start + 22, reference("universe", baby1)?)?;
            require_properties(spec, &["parent", "universe"])?;
        }
        0x0100 => {
            write_word(image, start + 20, reference("universe1", baby1)?)?;
            write_word(image, start + 22, reference("universe2", baby1)?)?;
            write_pair(
                image,
                start + 24,
                parse_pair(property("position1"), spec.line, "position1")?,
            )?;
            write_pair(
                image,
                start + 28,
                parse_pair(property("position2"), spec.line, "position2")?,
            )?;
            require_properties(spec, &["universe1", "universe2", "position1", "position2"])?;
        }
        0x0200 => {
            write_word(image, start + 4, reference("location", baby1)?)?;
            write_pair(
                image,
                start + 6,
                parse_pair(property("position"), spec.line, "position")?,
            )?;
            write_word(image, start + 16, reference("universe", baby1)?)?;
            require_properties(spec, &["location", "position", "universe"])?;
        }
        0x0400 => {
            write_word(image, start + 20, reference("holder", baby1)?)?;
            write_word(image, start + 22, reference("universe", baby1)?)?;
            require_properties(spec, &["holder", "universe"])?;
        }
        _ => unreachable!("all source kinds are validated"),
    }
    Ok(())
}

fn patch_conversation_roots(
    state: &mut StateCompilation,
    conversations: &bloodscript::Compilation,
) -> Result<()> {
    for object in &state.objects {
        if object.kind != 0x0002 {
            continue;
        }
        let label = format!("{}_choices", state_object_identifier(&object.name));
        let root = conversations
            .selector_offsets
            .get(&label)
            .copied()
            .unwrap_or(0);
        write_word(&mut state.image, usize::from(object.offset) + 26, root)?;
    }
    Ok(())
}

fn parse_variable_declaration(
    line: &str,
    line_number: usize,
) -> Result<Option<(String, Vec<u8>, u16)>> {
    let Some(rest) = line.strip_prefix("global ") else {
        return Ok(None);
    };
    let (left, value) = rest
        .split_once('=')
        .ok_or_else(|| anyhow!("logic line {line_number}: expected global NAME = VALUE"))?;
    let left = left.trim();
    let (identifier, export_name) = if let Some((identifier, alias)) = left.split_once(" as ") {
        let identifier = identifier.trim();
        validate_identifier(identifier, line_number, "variable name")?;
        let (name, trailing) = parse_quoted_bytes(alias.trim())?;
        if !trailing.trim().is_empty() {
            bail!("logic line {line_number}: unexpected variable alias text");
        }
        (identifier.to_string(), name)
    } else {
        validate_identifier(left, line_number, "variable name")?;
        (left.to_string(), left.as_bytes().to_vec())
    };
    Ok(Some((
        identifier,
        export_name,
        parse_u16(value.trim(), line_number, "variable value")?,
    )))
}

fn state_globals_offset(var: &[u8], records: &[DirectoryRecord]) -> Result<Option<u16>> {
    let Some(last) = records
        .iter()
        .rev()
        .find(|record| record.kind == KIND_OBJECT)
    else {
        bail!("DEB directory has no object records");
    };
    let start = usize::from(last.value);
    let kind = read_word(var, start)?;
    let end = start
        .checked_add(
            object_record_size(kind)
                .ok_or_else(|| anyhow!("unknown final object kind {kind} at 0x{start:04X}"))?,
        )
        .ok_or_else(|| anyhow!("final object extent overflows"))?;
    if end > var.len() {
        bail!("final object extends beyond VAR image");
    }
    Ok((end < var.len()).then(|| u16::try_from(end).expect("VAR is limited to 64 KiB")))
}

fn global_state_names(
    records: &[DirectoryRecord],
    identifiers: &HashMap<usize, String>,
    globals_offset: Option<u16>,
) -> HashMap<u16, String> {
    let Some(globals_offset) = globals_offset else {
        return HashMap::new();
    };
    records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.kind == KIND_STATE_LABEL && record.value >= globals_offset)
        .filter_map(|(index, record)| {
            identifiers
                .get(&index)
                .cloned()
                .map(|identifier| (record.value, identifier))
        })
        .collect()
}

fn replace_global_addresses(
    body: &str,
    globals_offset: Option<u16>,
    names: &HashMap<u16, String>,
) -> Result<String> {
    let Some(globals_offset) = globals_offset else {
        return Ok(body.to_string());
    };
    let mut output = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(start) = rest.find("state[0x") {
        output.push_str(&rest[..start]);
        let candidate = &rest[start + "state[0x".len()..];
        let Some(end) = candidate.find(']') else {
            bail!("unterminated state address in generated program");
        };
        let digits = &candidate[..end];
        let address = u16::from_str_radix(digits, 16)
            .map_err(|_| anyhow!("invalid generated state address 0x{digits}"))?;
        if address < globals_offset || (address - globals_offset) % 2 != 0 {
            bail!("generated state address 0x{address:04X} is not a word in the global VAR tail");
        }
        if let Some(name) = names.get(&address) {
            write!(output, "globals.{name}")?;
        } else {
            write!(output, "globals[{}]", (address - globals_offset) / 2)?;
        }
        rest = &candidate[end + 1..];
    }
    output.push_str(rest);
    Ok(output)
}

fn write_state(
    output: &mut String,
    var: &[u8],
    records: &[DirectoryRecord],
    dictionary: &[(u16, Vec<u8>)],
) -> Result<()> {
    let objects: Vec<_> = records
        .iter()
        .filter(|record| record.kind == KIND_OBJECT)
        .collect();
    if objects.is_empty() {
        bail!("DEB directory has no object records");
    }
    let names = objects
        .iter()
        .map(|record| (record.value, trimmed_name(&record.name).to_vec()))
        .collect::<HashMap<_, _>>();

    let mut state_end = 0usize;
    for (position, record) in objects.iter().enumerate() {
        let start = usize::from(record.value);
        if start != state_end {
            bail!(
                "object {} starts at 0x{start:04X}, expected 0x{state_end:04X}",
                format_quoted_bytes(trimmed_name(&record.name))
            );
        }
        let kind = read_word(var, start).with_context(|| {
            format!(
                "reading kind of object {}",
                format_quoted_bytes(trimmed_name(&record.name))
            )
        })?;
        let size = object_record_size(kind).ok_or_else(|| {
            anyhow!(
                "object {} has unsupported kind {kind}",
                format_quoted_bytes(trimmed_name(&record.name))
            )
        })?;
        let end = start + size;
        if let Some(next) = objects.get(position + 1)
            && usize::from(next.value) != end
        {
            bail!(
                "object {} has {} bytes, expected {size}",
                format_quoted_bytes(trimmed_name(&record.name)),
                usize::from(next.value).saturating_sub(start)
            );
        }
        if end > var.len() {
            bail!("object at 0x{start:04X} extends beyond VAR");
        }
        writeln!(
            output,
            "    {} {} {{",
            format_state_kind(kind),
            format_quoted_bytes(trimmed_name(&record.name))
        )?;
        let flags = read_word(var, start + 2)?;
        write_status(output, kind, flags)?;
        verify_inline_name(var, start, kind, trimmed_name(&record.name))?;
        write_state_object_fields(output, var, start, kind, &names, dictionary)?;
        writeln!(output, "    }}")?;
        state_end = end;
    }
    Ok(())
}

fn write_state_object_fields(
    output: &mut String,
    var: &[u8],
    start: usize,
    kind: u16,
    names: &HashMap<u16, Vec<u8>>,
    dictionary: &[(u16, Vec<u8>)],
) -> Result<()> {
    match kind {
        0x0001 => {
            writeln!(
                output,
                "        population = {}",
                read_word(var, start + 4)?
            )?;
            write_reference(output, "location", read_word(var, start + 6)?, names)?;
            require_zero_words(var, start + 8, 3, "initial player action")?;
            require_zero_word(var, start + 14, "reserved player padding")?;
            require_zero_words(var, start + 16, 8, "initial player icon state")?;
            write_nondefault_reference(
                output,
                "universe",
                read_word(var, start + 32)?,
                names,
                b"baby1",
            )?;
        }
        0x0002 => {
            writeln!(
                output,
                "        race = {}",
                format_race(read_word(var, start + 20)?)?
            )?;
            writeln!(
                output,
                "        population = {}",
                read_word(var, start + 22)?
            )?;
            let location = read_word(var, start + 24)?;
            if location != u16::MAX {
                write_reference(output, "location", location, names)?;
            }
            require_zero_word(var, start + 28, "reserved character padding")?;
            require_zero_words(var, start + 30, 10, "initial known-object set")?;
            write_nonzero_number(output, "aggressiveness", read_word(var, start + 50)?)?;
            write_nonzero_number(output, "energy", read_word(var, start + 52)?)?;
            write_nonzero_number(output, "encounters", read_word(var, start + 54)?)?;
            write_nonzero_number(output, "evolution", read_word(var, start + 56)?)?;
            require_zero_words(var, start + 58, 3, "initial character action")?;
            require_zero_words(var, start + 64, 2, "reserved character padding")?;
            write_nondefault_reference(
                output,
                "universe",
                read_word(var, start + 68)?,
                names,
                b"baby1",
            )?;
            let topic = read_word(var, start + 70)?;
            if topic != 0 {
                let value = dictionary
                    .iter()
                    .find_map(|(offset, value)| (*offset == topic).then_some(value))
                    .ok_or_else(|| anyhow!("character topic offset {topic} is absent from DIC"))?;
                writeln!(output, "        topic = {}", format_quoted_bytes(value))?;
            }
        }
        0x0008 => {
            write_nonzero_number(output, "visits", read_word(var, start + 20)?)?;
            write_nondefault_reference(
                output,
                "location",
                read_word(var, start + 22)?,
                names,
                b"baby1",
            )?;
            write_pair_property(
                output,
                "position",
                read_word(var, start + 24)?,
                read_word(var, start + 26)?,
            )?;
            write_nondefault_reference(
                output,
                "universe",
                read_word(var, start + 28)?,
                names,
                b"baby1",
            )?;
        }
        0x0010 => {
            write_nonzero_number(output, "visits", read_word(var, start + 20)?)?;
            write_nondefault_reference(
                output,
                "location",
                read_word(var, start + 22)?,
                names,
                b"baby1",
            )?;
            write_pair_property(
                output,
                "position",
                read_word(var, start + 24)?,
                read_word(var, start + 26)?,
            )?;
            require_zero_words(var, start + 28, 3, "initial ship action")?;
            write_nondefault_reference(
                output,
                "universe",
                read_word(var, start + 34)?,
                names,
                b"baby1",
            )?;
        }
        0x0040 => {}
        0x0080 => {
            write_reference(output, "parent", read_word(var, start + 20)?, names)?;
            write_nondefault_reference(
                output,
                "universe",
                read_word(var, start + 22)?,
                names,
                b"baby1",
            )?;
        }
        0x0100 => {
            write_reference(output, "universe1", read_word(var, start + 20)?, names)?;
            write_reference(output, "universe2", read_word(var, start + 22)?, names)?;
            write_pair_property(
                output,
                "position1",
                read_word(var, start + 24)?,
                read_word(var, start + 26)?,
            )?;
            write_pair_property(
                output,
                "position2",
                read_word(var, start + 28)?,
                read_word(var, start + 30)?,
            )?;
        }
        0x0200 => {
            write_nondefault_reference(
                output,
                "location",
                read_word(var, start + 4)?,
                names,
                b"baby1",
            )?;
            let position = (read_word(var, start + 6)?, read_word(var, start + 8)?);
            if position != (0, 0) {
                write_pair_property(output, "position", position.0, position.1)?;
            }
            require_zero_words(var, start + 10, 3, "initial navigation action")?;
            write_nondefault_reference(
                output,
                "universe",
                read_word(var, start + 16)?,
                names,
                b"baby1",
            )?;
            require_zero_words(var, start + 18, 9, "reserved navigation-controller padding")?;
        }
        0x0400 => {
            write_reference(output, "holder", read_word(var, start + 20)?, names)?;
            write_nondefault_reference(
                output,
                "universe",
                read_word(var, start + 22)?,
                names,
                b"baby1",
            )?;
        }
        _ => bail!("unsupported state kind {kind}"),
    }
    Ok(())
}

fn kind_has_inline_name(kind: u16) -> bool {
    !matches!(kind, 0x0001 | 0x0200)
}

fn compile_status(spec: &StateObjectSpec) -> Result<u16> {
    let mut status = 0u16;
    for directive in &spec.directives {
        let bit = match (spec.kind, directive.as_str()) {
            (_, "active") => 0,
            (0x0002 | 0x0008 | 0x0010 | 0x0040 | 0x0080 | 0x0100 | 0x0400, "known") => 1,
            (0x0002, "leader") => 2,
            (0x0002, "at_war") => 3,
            (0x0002, "present") => 4,
            (0x0002, "portable") => 5,
            (0x0001 | 0x0002, "acting") => 15,
            (0x0080, "full") => 2,
            (0x0400, "enabled") => 2,
            _ => bail!(
                "state line {}: directive {directive:?} is not valid for {}",
                spec.line,
                format_state_kind(spec.kind)
            ),
        };
        status |= 1 << bit;
    }
    Ok(status)
}

fn write_status(output: &mut String, kind: u16, flags: u16) -> Result<()> {
    let mut remaining = flags;
    let mut emit = |mask: u16, name: &str| -> Result<()> {
        if remaining & mask != 0 {
            writeln!(output, "        {name}")?;
            remaining &= !mask;
        }
        Ok(())
    };
    emit(1, "active")?;
    if matches!(
        kind,
        0x0002 | 0x0008 | 0x0010 | 0x0040 | 0x0080 | 0x0100 | 0x0400
    ) {
        emit(2, "known")?;
    }
    match kind {
        0x0001 => emit(0x8000, "acting")?,
        0x0002 => {
            emit(4, "leader")?;
            emit(8, "at_war")?;
            emit(16, "present")?;
            emit(32, "portable")?;
            emit(0x8000, "acting")?;
        }
        0x0080 => emit(4, "full")?,
        0x0400 => emit(4, "enabled")?,
        _ => {}
    }
    if remaining != 0 {
        bail!(
            "{} record has unsupported status bits {remaining}",
            format_state_kind(kind)
        );
    }
    Ok(())
}

fn verify_inline_name(var: &[u8], start: usize, kind: u16, name: &[u8]) -> Result<()> {
    if !kind_has_inline_name(kind) {
        return Ok(());
    }
    let expected = fixed_name(name, 0)?;
    if var.get(start + 4..start + 20) != Some(expected.as_slice()) {
        bail!(
            "{} {} does not contain its directory name in the proven inline-name field",
            format_state_kind(kind),
            format_quoted_bytes(name)
        );
    }
    Ok(())
}

fn require_properties(spec: &StateObjectSpec, allowed: &[&str]) -> Result<()> {
    if let Some(name) = spec
        .properties
        .keys()
        .find(|name| !allowed.contains(&name.as_str()))
    {
        bail!(
            "state line {}: property {name:?} is not valid for {}",
            spec.line,
            format_state_kind(spec.kind)
        );
    }
    Ok(())
}

fn parse_object_reference(
    value: &str,
    by_name: &HashMap<Vec<u8>, u16>,
    line: usize,
    field: &str,
) -> Result<u16> {
    match value {
        "nowhere" => return Ok(u16::MAX),
        "aboard" => {
            return by_name
                .get(b"blood".as_slice())
                .copied()
                .ok_or_else(|| anyhow!("state line {line}: aboard requires player \"blood\""));
        }
        _ => {}
    }
    let (name, rest) = parse_quoted_bytes(value)
        .with_context(|| format!("state line {line}: {field} object reference"))?;
    if !rest.trim().is_empty() {
        bail!("state line {line}: unexpected text after {field} object reference");
    }
    by_name.get(&name).copied().ok_or_else(|| {
        anyhow!(
            "state line {line}: {field} references unknown object {}",
            format_quoted_bytes(&name)
        )
    })
}

fn parse_pair(value: Option<&str>, line: usize, field: &str) -> Result<(u16, u16)> {
    let Some(value) = value else {
        return Ok((0, 0));
    };
    let inner = value
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .ok_or_else(|| anyhow!("state line {line}: {field} must be (X, Y)"))?;
    let (x, y) = inner
        .split_once(',')
        .ok_or_else(|| anyhow!("state line {line}: {field} must contain two values"))?;
    Ok((
        parse_u16(x.trim(), line, &format!("{field} x"))?,
        parse_u16(y.trim(), line, &format!("{field} y"))?,
    ))
}

fn parse_topic(value: Option<&str>, dictionary: &[(u16, Vec<u8>)], line: usize) -> Result<u16> {
    let Some(value) = value else {
        return Ok(0);
    };
    if value == "none" {
        return Ok(0);
    }
    let (word, rest) = parse_quoted_bytes(value)?;
    if !rest.trim().is_empty() {
        bail!("state line {line}: unexpected text after topic");
    }
    dictionary
        .iter()
        .find_map(|(offset, candidate)| (candidate == &word).then_some(*offset))
        .ok_or_else(|| {
            anyhow!(
                "state line {line}: topic {} is absent from the profile dictionary",
                format_quoted_bytes(&word)
            )
        })
}

const RACES: [&str; 16] = [
    "croolis_red",
    "croolis_green",
    "migrax",
    "slimers",
    "izwals",
    "sinox",
    "waves",
    "tromps",
    "kam",
    "tubular_brain",
    "quizzers",
    "zen",
    "scruters",
    "robots",
    "bob",
    "gluxx",
];

fn parse_race(value: Option<&str>, line: usize) -> Result<u16> {
    let value = value.unwrap_or("sinox");
    RACES
        .iter()
        .position(|race| *race == value)
        .map(|bit| 1u16 << bit)
        .ok_or_else(|| anyhow!("state line {line}: unknown race {value:?}"))
}

fn format_race(value: u16) -> Result<&'static str> {
    if value.count_ones() != 1 {
        bail!("race value {value} is not a single recovered race bit");
    }
    RACES
        .get(value.trailing_zeros() as usize)
        .copied()
        .ok_or_else(|| anyhow!("race value {value} is outside the recovered race table"))
}

fn write_reference(
    output: &mut String,
    field: &str,
    value: u16,
    names: &HashMap<u16, Vec<u8>>,
) -> Result<()> {
    if value == u16::MAX {
        writeln!(output, "        {field} = nowhere")?;
        return Ok(());
    }
    let name = names
        .get(&value)
        .ok_or_else(|| anyhow!("{field} references no object at VAR offset {value}"))?;
    writeln!(output, "        {field} = {}", format_quoted_bytes(name))?;
    Ok(())
}

fn write_nondefault_reference(
    output: &mut String,
    field: &str,
    value: u16,
    names: &HashMap<u16, Vec<u8>>,
    default_name: &[u8],
) -> Result<()> {
    if names.get(&value).is_some_and(|name| name == default_name) {
        return Ok(());
    }
    write_reference(output, field, value, names)
}

fn write_pair_property(output: &mut String, field: &str, x: u16, y: u16) -> Result<()> {
    writeln!(output, "        {field} = ({x}, {y})")?;
    Ok(())
}

fn write_nonzero_number(output: &mut String, field: &str, value: u16) -> Result<()> {
    if value != 0 {
        writeln!(output, "        {field} = {value}")?;
    }
    Ok(())
}

fn require_zero_word(var: &[u8], offset: usize, role: &str) -> Result<()> {
    let value = read_word(var, offset)?;
    if value != 0 {
        bail!("{role} at VAR byte {offset} is {value}, expected its initial zero state");
    }
    Ok(())
}

fn require_zero_words(var: &[u8], offset: usize, count: usize, role: &str) -> Result<()> {
    for index in 0..count {
        require_zero_word(var, offset + index * 2, role)?;
    }
    Ok(())
}

fn write_word(bytes: &mut [u8], offset: usize, value: u16) -> Result<()> {
    let destination = bytes
        .get_mut(offset..offset + 2)
        .ok_or_else(|| anyhow!("word at byte {offset} is outside VAR"))?;
    destination.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_pair(bytes: &mut [u8], offset: usize, value: (u16, u16)) -> Result<()> {
    write_word(bytes, offset, value.0)?;
    write_word(bytes, offset + 2, value.1)
}

fn parse_state_kind(value: &str, line: usize) -> Result<u16> {
    match value {
        "player" => Ok(1),
        "character" => Ok(2),
        "planet" => Ok(8),
        "ship" => Ok(16),
        "universe" => Ok(64),
        "location" => Ok(128),
        "black_hole" => Ok(256),
        "navigation_controller" => Ok(512),
        "item" => Ok(1024),
        _ => bail!("state line {line}: unknown object type {value:?}"),
    }
}

fn format_state_kind(kind: u16) -> String {
    match kind {
        1 => "player".to_string(),
        2 => "character".to_string(),
        8 => "planet".to_string(),
        16 => "ship".to_string(),
        64 => "universe".to_string(),
        128 => "location".to_string(),
        256 => "black_hole".to_string(),
        512 => "navigation_controller".to_string(),
        1024 => "item".to_string(),
        _ => unreachable!("unsupported kinds are rejected before rendering"),
    }
}

fn object_record_size(kind: u16) -> Option<usize> {
    // Adjacent DEB object offsets establish these sizes. The final 0x0200 orxx
    // record is 36 bytes: every shipped profile and the sequel place the
    // compiler-injected kind-5 `tblood` word exactly at orxx + 36.
    match kind {
        0x0001 => Some(34),
        0x0002 => Some(72),
        0x0008 => Some(30),
        0x0010 => Some(36),
        0x0040 => Some(20),
        0x0080 => Some(24),
        0x0100 => Some(32),
        0x0200 => Some(36),
        0x0400 => Some(24),
        _ => None,
    }
}

fn compile_directory(
    body: &str,
    state: &StateCompilation,
    logic: &bloodscript::Compilation,
) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    for object in &state.objects {
        write_directory_record(&mut output, &object.name, object.offset, KIND_OBJECT, 0)?;
    }

    for (index, original) in body.lines().enumerate() {
        let line_number = index + 1;
        let line = code_before_comment(original).trim();
        if line.is_empty() {
            continue;
        }
        if line == "sentinel" {
            write_directory_record(&mut output, &[], u16::MAX, KIND_SENTINEL, line_number)?;
            continue;
        }
        if let Some(rest) = line.strip_prefix("raw ") {
            let (kind, rest) = rest
                .split_once(' ')
                .ok_or_else(|| anyhow!("directory line {line_number}: malformed raw symbol"))?;
            let kind = parse_u16(kind, line_number, "raw symbol kind")?;
            let (name, rest) = parse_quoted_bytes(rest)
                .with_context(|| format!("directory line {line_number}: symbol name"))?;
            let value = rest
                .trim()
                .strip_prefix('=')
                .ok_or_else(|| anyhow!("directory line {line_number}: expected '='"))?;
            let value = parse_u16(value.trim(), line_number, "raw symbol value")?;
            write_directory_record(&mut output, &name, value, kind, line_number)?;
            continue;
        }
        let (kind_name, rest) = line
            .split_once(' ')
            .ok_or_else(|| anyhow!("directory line {line_number}: malformed symbol"))?;
        let (name, rest) = parse_quoted_bytes(rest)
            .with_context(|| format!("directory line {line_number}: symbol name"))?;
        let target = rest
            .trim()
            .strip_prefix('=')
            .ok_or_else(|| anyhow!("directory line {line_number}: expected '='"))?
            .trim();
        validate_identifier(target, line_number, "symbol target")?;
        let (value, kind) = match kind_name {
            "procedure" => {
                let offset = logic
                    .procedure_offsets
                    .get(target)
                    .copied()
                    .ok_or_else(|| {
                        anyhow!("directory line {line_number}: unknown procedure target {target:?}")
                    })?;
                (
                    offset.checked_add(1).ok_or_else(|| {
                        anyhow!("directory line {line_number}: procedure address overflows")
                    })?,
                    KIND_PROCEDURE,
                )
            }
            "code_label" => (
                logic.label_offsets.get(target).copied().ok_or_else(|| {
                    anyhow!("directory line {line_number}: unknown code label target {target:?}")
                })?,
                KIND_CODE_LABEL,
            ),
            "state_label" => (
                state.labels.get(target).copied().ok_or_else(|| {
                    anyhow!("directory line {line_number}: unknown state marker target {target:?}")
                })?,
                KIND_STATE_LABEL,
            ),
            _ => bail!(
                "directory line {line_number}: expected procedure, code_label, state_label, raw, or sentinel"
            ),
        };
        write_directory_record(&mut output, &name, value, kind, line_number)?;
    }
    Ok(output)
}

fn parse_directory(bytes: &[u8]) -> Result<Vec<DirectoryRecord>> {
    if bytes.len() % DIRECTORY_RECORD_BYTES != 0 {
        bail!(
            "DEB image is {} bytes, not a multiple of {DIRECTORY_RECORD_BYTES}",
            bytes.len()
        );
    }
    Ok(bytes
        .chunks_exact(DIRECTORY_RECORD_BYTES)
        .map(|record| DirectoryRecord {
            name: record[..DIRECTORY_NAME_BYTES]
                .try_into()
                .expect("fixed directory name slice"),
            value: u16::from_le_bytes([record[16], record[17]]),
            kind: u16::from_le_bytes([record[18], record[19]]),
        })
        .collect())
}

fn write_directory_record(
    output: &mut Vec<u8>,
    name: &[u8],
    value: u16,
    kind: u16,
    line: usize,
) -> Result<()> {
    let name = fixed_name(name, line)?;
    output.extend_from_slice(&name);
    output.extend_from_slice(&value.to_le_bytes());
    output.extend_from_slice(&kind.to_le_bytes());
    Ok(())
}

fn fixed_name(name: &[u8], line: usize) -> Result<[u8; DIRECTORY_NAME_BYTES]> {
    if name.len() > DIRECTORY_NAME_BYTES {
        bail!(
            "line {line}: directory name is {} bytes, maximum is {DIRECTORY_NAME_BYTES}",
            name.len()
        );
    }
    let mut field = [0u8; DIRECTORY_NAME_BYTES];
    field[..name.len()].copy_from_slice(name);
    Ok(field)
}

fn state_label_identifiers(records: &[DirectoryRecord]) -> HashMap<usize, String> {
    let mut used = HashSet::new();
    let mut identifiers = HashMap::new();
    for (index, record) in records.iter().enumerate() {
        if record.kind != KIND_STATE_LABEL {
            continue;
        }
        let base = identifier_from_bytes(trimmed_name(&record.name));
        let identifier = unique_identifier(base, record.value, &mut used);
        identifiers.insert(index, identifier);
    }
    identifiers
}

fn procedure_identifiers(
    records: &[DirectoryRecord],
    logic_source: &str,
) -> Result<HashMap<usize, String>> {
    let procedures: Vec<_> = logic_source
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("proc ")
                .and_then(|rest| rest.split_whitespace().next())
                .map(str::to_string)
        })
        .collect();
    let mut directory_procedures: Vec<_> = records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.kind == KIND_PROCEDURE)
        .collect();
    directory_procedures.sort_by_key(|(_, record)| record.value);
    if directory_procedures.len() != procedures.len() {
        bail!(
            "DEB has {} procedures but structured COD has {}",
            directory_procedures.len(),
            procedures.len()
        );
    }
    Ok(directory_procedures
        .into_iter()
        .zip(procedures)
        .map(|((index, _), identifier)| (index, identifier))
        .collect())
}

fn identifier_from_bytes(name: &[u8]) -> String {
    let mut output = String::new();
    for &byte in name {
        if byte.is_ascii_alphanumeric() || byte == b'_' {
            output.push(char::from(byte));
        } else {
            write!(output, "_{byte:02X}").expect("writing to String cannot fail");
        }
    }
    if output.is_empty() {
        output.push_str("unnamed");
    }
    if output.as_bytes()[0].is_ascii_digit() {
        output.insert(0, '_');
    }
    output
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

fn read_word(bytes: &[u8], offset: usize) -> Result<u16> {
    let word = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| anyhow!("word at 0x{offset:04X} is outside image"))?;
    Ok(u16::from_le_bytes([word[0], word[1]]))
}

fn trimmed_name(name: &[u8; DIRECTORY_NAME_BYTES]) -> &[u8] {
    let end = name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(name.len());
    if name[end..].iter().all(|byte| *byte == 0) {
        &name[..end]
    } else {
        name
    }
}

fn format_quoted_bytes(bytes: &[u8]) -> String {
    let mut output = String::from("\"");
    for &byte in bytes {
        match byte {
            b'"' => output.push_str("\\\""),
            b'\\' => output.push_str("\\\\"),
            b'\n' => output.push_str("\\n"),
            b'\r' => output.push_str("\\r"),
            b'\t' => output.push_str("\\t"),
            0 => output.push_str("\\0"),
            0x20..=0x7e => output.push(char::from(byte)),
            0x80..=0xaf => output.push_str(&crate::font::cp437_string(&[byte])),
            _ => write!(output, "\\x{byte:02X}").expect("writing to String cannot fail"),
        }
    }
    output.push('"');
    output
}

fn parse_quoted_bytes(value: &str) -> Result<(Vec<u8>, &str)> {
    let Some(mut rest) = value.strip_prefix('"') else {
        bail!("expected quoted byte string");
    };
    let mut output = Vec::new();
    loop {
        let Some(ch) = rest.chars().next() else {
            bail!("unterminated quoted byte string");
        };
        rest = &rest[ch.len_utf8()..];
        match ch {
            '"' => return Ok((output, rest)),
            '\\' => {
                let escape = rest
                    .chars()
                    .next()
                    .ok_or_else(|| anyhow!("unterminated escape"))?;
                rest = &rest[escape.len_utf8()..];
                match escape {
                    '"' => output.push(b'"'),
                    '\\' => output.push(b'\\'),
                    'n' => output.push(b'\n'),
                    'r' => output.push(b'\r'),
                    't' => output.push(b'\t'),
                    '0' => output.push(0),
                    'x' => {
                        let digits = rest
                            .get(..2)
                            .ok_or_else(|| anyhow!("short hexadecimal byte escape"))?;
                        output.push(
                            u8::from_str_radix(digits, 16).map_err(|_| {
                                anyhow!("invalid hexadecimal byte escape \\x{digits}")
                            })?,
                        );
                        rest = &rest[2..];
                    }
                    _ => bail!("unsupported escape \\{escape}"),
                }
            }
            _ => output.push(crate::font::cp437_byte(ch).ok_or_else(|| {
                anyhow!("character {ch:?} is not representable in the game encoding")
            })?),
        }
    }
}

fn code_before_comment(line: &str) -> &str {
    let mut quoted = false;
    let mut escaped = false;
    let bytes = line.as_bytes();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
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
        } else if byte == b'/' && bytes.get(cursor + 1) == Some(&b'/') {
            return &line[..cursor];
        }
        cursor += 1;
    }
    line
}

fn parse_u16(value: &str, line: usize, field: &str) -> Result<u16> {
    let value = value.trim_end_matches(',');
    value
        .strip_prefix("0x")
        .and_then(|digits| u16::from_str_radix(digits, 16).ok())
        .or_else(|| value.parse().ok())
        .ok_or_else(|| anyhow!("line {line}: invalid {field} {value:?}"))
}

fn validate_identifier(value: &str, line: usize, field: &str) -> Result<()> {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        bail!("line {line}: {field} cannot be empty");
    };
    if !(first.is_ascii_alphabetic() || first == b'_')
        || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        bail!("line {line}: invalid {field} {value:?}");
    }
    Ok(())
}

fn write_indented(output: &mut String, body: &str, spaces: usize) -> Result<()> {
    let indent = " ".repeat(spaces);
    for line in body.lines() {
        if line.is_empty() {
            output.push('\n');
        } else {
            writeln!(output, "{indent}{line}")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn unified_profile_compiles_all_five_images() {
        let source = r#"bloodscript 8
profile SCRIPT1
concepts "hello"

state {
    universe "baby1" {
        known
    }
    universe "root" {
    }
    navigation_controller "orxx" {
        active
    }
}

logic {
    halt
}

conversations {
    halt
}
"#;
        let profile = compile(source).unwrap();
        assert_eq!(profile.cod, [0xff]);
        assert_eq!(profile.bas, [0xff]);
        assert_eq!(profile.dic, [0, b'h', b'e', b'l', b'l', b'o', 0, 0xff, 0]);
        assert_eq!(profile.var.len(), 78);
        assert_eq!(profile.deb.len(), 100);
        assert_eq!(&profile.deb[..5], b"baby1");
        assert_eq!(profile.deb[18], 1);
    }

    #[test]
    fn quoted_bytes_preserve_cp437_and_raw_values() {
        let bytes = [b'p', b'o', b'r', b't', b'e', b'_', 0x82, 0xfe];
        let quoted = format_quoted_bytes(&bytes);
        let (rebuilt, rest) = parse_quoted_bytes(&quoted).unwrap();
        assert_eq!(rebuilt, bytes);
        assert!(rest.is_empty());
        assert!(quoted.contains('é'));
        assert!(quoted.contains("\\xFE"));
    }

    #[test]
    fn unified_dialogue_uses_recovered_presentation_names() {
        let source = concat!(
            "say Bob_Morlock presentation=bobb.hnm : \"Hello\"\n",
            "say Honk presentation=text_only chatter : \"Report\"\n",
            "say Ulikan presentation=early_morning_signoff : \"See ya\"",
        );
        let lowered = lower_named_presentations(source).unwrap();
        assert!(lowered.contains("Bob_Morlock presentation=10"));
        assert!(lowered.contains("Honk presentation=8 chatter"));
        assert!(lowered.contains("Ulikan presentation=26"));
        assert_eq!(raise_named_presentations(&lowered).unwrap(), source);

        let error =
            lower_named_presentations("say Bob_Morlock presentation=10 : \"opaque selector\"")
                .unwrap_err()
                .to_string();
        assert!(error.contains("numeric presentation IDs are not valid"));
    }

    #[test]
    fn canonical_profiles_match_all_shipped_vm_bytes() {
        let game_dir = Path::new("accuracy/cblood_install/cblood");
        let source_dir = Path::new("re/vm/profiles");
        if !game_dir.join("SCRIPT1.COD").is_file() || !source_dir.is_dir() {
            return;
        }
        let mut compared_bytes = 0usize;
        for script in 1..=5 {
            let name = format!("SCRIPT{script}");
            let source =
                std::fs::read_to_string(source_dir.join(format!("script{script}.blood"))).unwrap();
            let compiled = compile(&source).unwrap();
            let shipped = ProfileImages {
                name,
                cod: std::fs::read(game_dir.join(format!("SCRIPT{script}.COD"))).unwrap(),
                bas: std::fs::read(game_dir.join(format!("SCRIPT{script}.BAS"))).unwrap(),
                deb: std::fs::read(game_dir.join(format!("SCRIPT{script}.DEB"))).unwrap(),
                dic: std::fs::read(game_dir.join(format!("SCRIPT{script}.DIC"))).unwrap(),
                var: std::fs::read(game_dir.join(format!("SCRIPT{script}.VAR"))).unwrap(),
            };
            require_same_profile(&compiled, &shipped).unwrap();
            compared_bytes += PROFILE_EXTENSIONS_FOR_TEST
                .iter()
                .map(|extension| shipped.image(extension).unwrap().len())
                .sum::<usize>();
        }
        assert_eq!(compared_bytes, 317_835);
    }

    #[test]
    fn shipped_dictionary_order_is_seed_plus_program_first_use() {
        let game_dir = Path::new("accuracy/cblood_install/cblood");
        if !game_dir.join("SCRIPT1.COD").is_file() {
            return;
        }
        for script in 1..=5 {
            let images = ProfileImages {
                name: format!("SCRIPT{script}"),
                cod: std::fs::read(game_dir.join(format!("SCRIPT{script}.COD"))).unwrap(),
                bas: std::fs::read(game_dir.join(format!("SCRIPT{script}.BAS"))).unwrap(),
                deb: std::fs::read(game_dir.join(format!("SCRIPT{script}.DEB"))).unwrap(),
                dic: std::fs::read(game_dir.join(format!("SCRIPT{script}.DIC"))).unwrap(),
                var: std::fs::read(game_dir.join(format!("SCRIPT{script}.VAR"))).unwrap(),
            };
            let dictionary = script::parse_dictionary(&images.dic);
            let seeds = dictionary_seed_words(&images, &dictionary).unwrap();
            let expected = if script == 3 {
                vec![b"talk".to_vec(), b"hello".to_vec(), b"rien".to_vec()]
            } else {
                vec![b"talk".to_vec(), b"hello".to_vec()]
            };
            assert_eq!(seeds, expected, "SCRIPT{script}");
        }
    }

    const PROFILE_EXTENSIONS_FOR_TEST: [&str; 5] = ["COD", "BAS", "DEB", "DIC", "VAR"];
}
