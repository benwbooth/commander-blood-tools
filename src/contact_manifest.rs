//! Binary-derived census of every contact dialogue entry in the shipped COD images.
//!
//! The manifest is deliberately built from decoded bytecode, DEB symbols, DIC words, and the
//! recovered CFG. It does not scrape the human-readable decompiler output.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;

use crate::script::{self, DebSymbol, ScriptFunction};
use crate::vm::{self, VmToken};
use crate::vm_cfg::{self, CodControlFlow};

const SCRIPT_COUNT: usize = 5;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ContactEntryToken {
    pub offset: usize,
    pub kind: String,
    pub token: VmToken,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ContactPresentation {
    pub predicate_offset: usize,
    pub object_offset: u16,
    pub object: String,
    pub action_record_offset: u16,
    pub related_record_offset: u16,
    pub inverted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ContactText {
    pub opcode_offset: usize,
    pub word_list_offset: usize,
    pub line_index: u16,
    pub actor_object_offset: u16,
    pub actor: String,
    pub voice_selector: u8,
    pub flags_b4: u8,
    pub flags_b5: u8,
    pub loop_target: Option<u16>,
    pub control_word: Option<u16>,
    pub word_offsets: Vec<u16>,
    pub subtitle: String,
    pub choices: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ContactProcedure {
    pub script: String,
    pub procedure: String,
    pub procedure_offset: usize,
    pub procedure_end: usize,
    pub activation_flags: u8,
    pub activation_target: u16,
    pub activation_enabled: bool,
    pub contact_offset: usize,
    pub cfg_procedure: String,
    pub entry_class: String,
    pub entry_tokens: Vec<ContactEntryToken>,
    pub presentations: Vec<ContactPresentation>,
    pub contact_object_offset: u16,
    pub contact_object: String,
    pub texts: Vec<ContactText>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScriptContactCount {
    pub script: String,
    pub procedures: usize,
    pub direct_entries: usize,
    pub conditioned_entries: usize,
    pub texts: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ContactManifest {
    pub format_version: u32,
    pub procedure_count: usize,
    pub direct_entry_count: usize,
    pub conditioned_entry_count: usize,
    pub text_count: usize,
    pub scripts: Vec<ScriptContactCount>,
    pub procedures: Vec<ContactProcedure>,
}

pub fn analyze_game_dir(game_dir: &Path) -> Result<ContactManifest> {
    let mut procedures = Vec::new();
    let mut scripts = Vec::new();

    for script_number in 1..=SCRIPT_COUNT {
        let script_name = format!("SCRIPT{script_number}");
        let cod_path = game_dir.join(format!("{script_name}.COD"));
        let deb_path = game_dir.join(format!("{script_name}.DEB"));
        let dic_path = game_dir.join(format!("{script_name}.DIC"));
        let cod =
            std::fs::read(&cod_path).with_context(|| format!("reading {}", cod_path.display()))?;
        let symbols = script::parse_deb(
            &std::fs::read(&deb_path).with_context(|| format!("reading {}", deb_path.display()))?,
        );
        let dictionary = script::parse_dictionary(
            &std::fs::read(&dic_path).with_context(|| format!("reading {}", dic_path.display()))?,
        );
        let graph = vm_cfg::analyze_cod(&script_name, &cod, &symbols)
            .with_context(|| format!("analyzing {script_name} control flow"))?;
        let functions = script::functions_from_symbols(&script_name, &symbols, cod.len());
        let tokens = vm::walk(&cod, 0, cod.len());
        if let Some(VmToken::Invalid { offset, byte }) = tokens.last() {
            bail!("{script_name} has invalid byte 0x{byte:02X} at 0x{offset:04X}");
        }

        let script_start = procedures.len();
        for (function_index, function) in functions.iter().enumerate() {
            let end = functions
                .get(function_index + 1)
                .map(|next| next.offset)
                .unwrap_or(cod.len());
            let procedure_tokens = tokens
                .iter()
                .filter(|token| token.offset() >= function.offset && token.offset() < end)
                .collect::<Vec<_>>();
            let contact_indices = procedure_tokens
                .iter()
                .enumerate()
                .filter_map(|(index, token)| match token {
                    VmToken::FlagBranch { opcode, .. }
                        if *opcode == vm::OP_COND_BRANCH_FLAG_274F =>
                    {
                        Some(index)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            if contact_indices.is_empty() {
                continue;
            }
            if contact_indices.len() != 1 {
                bail!(
                    "{script_name} procedure {} contains {} during-contact branches",
                    function.name,
                    contact_indices.len()
                );
            }
            procedures.push(analyze_procedure(
                &script_name,
                function,
                end,
                &procedure_tokens,
                contact_indices[0],
                &symbols,
                &dictionary,
                &graph,
            )?);
        }

        let script_procedures = &procedures[script_start..];
        scripts.push(ScriptContactCount {
            script: script_name,
            procedures: script_procedures.len(),
            direct_entries: script_procedures
                .iter()
                .filter(|procedure| procedure.entry_class == "direct")
                .count(),
            conditioned_entries: script_procedures
                .iter()
                .filter(|procedure| procedure.entry_class == "conditioned")
                .count(),
            texts: script_procedures
                .iter()
                .map(|procedure| procedure.texts.len())
                .sum(),
        });
    }

    Ok(ContactManifest {
        format_version: 1,
        procedure_count: procedures.len(),
        direct_entry_count: procedures
            .iter()
            .filter(|procedure| procedure.entry_class == "direct")
            .count(),
        conditioned_entry_count: procedures
            .iter()
            .filter(|procedure| procedure.entry_class == "conditioned")
            .count(),
        text_count: procedures
            .iter()
            .map(|procedure| procedure.texts.len())
            .sum(),
        scripts,
        procedures,
    })
}

fn analyze_procedure(
    script_name: &str,
    function: &ScriptFunction,
    procedure_end: usize,
    procedure_tokens: &[&VmToken],
    contact_index: usize,
    symbols: &[DebSymbol],
    dictionary: &HashMap<u16, String>,
    graph: &CodControlFlow,
) -> Result<ContactProcedure> {
    let contact_offset = procedure_tokens[contact_index].offset();
    let first_text_index = procedure_tokens
        .iter()
        .enumerate()
        .skip(contact_index + 1)
        .find_map(|(index, token)| matches!(token, VmToken::Text { .. }).then_some(index))
        .ok_or_else(|| {
            anyhow!(
                "{script_name} procedure {} has during-contact at 0x{contact_offset:04X} but no text",
                function.name
            )
        })?;

    let (activation_flags, activation_target) = match procedure_tokens.first() {
        Some(VmToken::ConditionalBlock { flags, target, .. }) => (*flags, *target),
        _ => bail!(
            "{script_name} procedure {} does not begin with a conditional activation block",
            function.name
        ),
    };
    let activation_target = usize::from(activation_target);
    let target_is_next_procedure = activation_target == procedure_end;
    let target_is_final_halt = procedure_end == graph.image_bytes
        && activation_target.checked_add(1) == Some(procedure_end);
    if activation_flags & !1 != 0 || !(target_is_next_procedure || target_is_final_halt) {
        bail!(
            "{script_name} procedure {} has unexpected activation flags 0x{activation_flags:02X} or target 0x{activation_target:04X}; expected target 0x{procedure_end:04X}",
            function.name
        );
    }

    // Conditions may precede or follow D1 in the encoded stream. Recover the entire
    // activation predicate region instead of treating the source ordering as semantic.
    let entry_tokens = procedure_tokens[..first_text_index]
        .iter()
        .filter(|token| {
            !matches!(
                token,
                VmToken::ConditionalBlock { .. }
                    | VmToken::GuardPop { .. }
                    | VmToken::FlagBranch {
                        opcode: vm::OP_COND_BRANCH_FLAG_274F,
                        ..
                    }
            )
        })
        .map(|token| ContactEntryToken {
            offset: token.offset(),
            kind: token_kind(token).to_string(),
            token: (*token).clone(),
        })
        .collect::<Vec<_>>();
    if let Some(token) = entry_tokens
        .iter()
        .find(|token| !is_contact_entry_token(&token.token))
    {
        bail!(
            "{script_name} procedure {} has unexpected {} token at 0x{:04X} in its contact entry guard",
            function.name,
            token.kind,
            token.offset
        );
    }

    let mut presentations = Vec::new();
    for token in &entry_tokens {
        let VmToken::Actor {
            record_offset,
            related_record_offset,
            inverted,
            ..
        } = &token.token
        else {
            continue;
        };
        let object_offset = record_offset.checked_sub(vm::TALK_FIELD).ok_or_else(|| {
            anyhow!(
                "{script_name} procedure {} C4 record 0x{record_offset:04X} at 0x{:04X} is below TALK field",
                function.name,
                token.offset
            )
        })?;
        let object = object_symbol(symbols, object_offset).ok_or_else(|| {
            anyhow!(
                "{script_name} procedure {} C4 at 0x{:04X} has unresolved object 0x{object_offset:04X}",
                function.name,
                token.offset
            )
        })?;
        presentations.push(ContactPresentation {
            predicate_offset: token.offset,
            object_offset,
            object: object.to_string(),
            action_record_offset: *record_offset,
            related_record_offset: *related_record_offset,
            inverted: *inverted,
        });
    }

    let texts = procedure_tokens[first_text_index..]
        .iter()
        .filter_map(|token| match token {
            VmToken::Text {
                offset,
                line_index,
                voice_selector,
                flags_b4,
                flags_b5,
                loop_target,
                control_word,
                word_offsets,
            } => Some(build_text(
                script_name,
                &function.name,
                *offset,
                *line_index,
                *voice_selector,
                *flags_b4,
                *flags_b5,
                *loop_target,
                *control_word,
                word_offsets,
                symbols,
                dictionary,
            )),
            _ => None,
        })
        .collect::<Result<Vec<_>>>()?;
    let first_text = texts.first().expect("first text was located above");
    let entry_class = if entry_tokens.len() == 1
        && presentations.len() == 1
        && presentations[0].object_offset == first_text.actor_object_offset
    {
        "direct"
    } else {
        "conditioned"
    };

    let cfg_procedure = cfg_procedure_at(graph, contact_offset).ok_or_else(|| {
        anyhow!(
            "{script_name} procedure {} contact offset 0x{contact_offset:04X} is absent from its CFG",
            function.name
        )
    })?;
    let expected_cfg_procedure = format!("{}_{:04X}", function.name, function.offset);
    if !cfg_procedure.eq_ignore_ascii_case(&expected_cfg_procedure) {
        bail!(
            "{script_name} contact at 0x{contact_offset:04X} belongs to DEB procedure {expected_cfg_procedure} but CFG procedure {cfg_procedure}"
        );
    }

    Ok(ContactProcedure {
        script: script_name.to_string(),
        procedure: function.name.clone(),
        procedure_offset: function.offset,
        procedure_end,
        activation_flags,
        activation_target: activation_target as u16,
        activation_enabled: activation_flags & 1 != 0,
        contact_offset,
        cfg_procedure: cfg_procedure.to_string(),
        entry_class: entry_class.to_string(),
        entry_tokens,
        presentations,
        contact_object_offset: first_text.actor_object_offset,
        contact_object: first_text.actor.clone(),
        texts,
    })
}

#[allow(clippy::too_many_arguments)]
fn build_text(
    script_name: &str,
    procedure_name: &str,
    opcode_offset: usize,
    line_index: u16,
    voice_selector: u8,
    flags_b4: u8,
    flags_b5: u8,
    loop_target: Option<u16>,
    control_word: Option<u16>,
    word_offsets: &[u16],
    symbols: &[DebSymbol],
    dictionary: &HashMap<u16, String>,
) -> Result<ContactText> {
    // TEXT stores the object record directly (Bob is 0x004A). The C4 presentation
    // predicate above addresses that object's TALK field instead (Bob + 0x3A = 0x0084).
    let actor_object_offset = line_index;
    let actor = object_symbol(symbols, actor_object_offset).ok_or_else(|| {
        anyhow!(
            "{script_name} procedure {procedure_name} text at 0x{opcode_offset:04X} has unresolved actor object 0x{actor_object_offset:04X}"
        )
    })?;
    let separator = word_offsets.iter().position(|offset| *offset == u16::MAX);
    let (display_offsets, choice_offsets) = match separator {
        Some(index) => (&word_offsets[..index], &word_offsets[index + 1..]),
        None => (word_offsets, &[][..]),
    };
    let display_words = resolve_words(
        script_name,
        procedure_name,
        opcode_offset,
        display_offsets,
        dictionary,
    )?;
    let choices = resolve_words(
        script_name,
        procedure_name,
        opcode_offset,
        choice_offsets,
        dictionary,
    )?
    .into_iter()
    .map(str::to_string)
    .collect();
    let subtitle = script::assemble_words(&display_words);
    let word_list_offset = opcode_offset
        + 6
        + usize::from(loop_target.is_some()) * 2
        + usize::from(control_word.is_some()) * 2;

    Ok(ContactText {
        opcode_offset,
        word_list_offset,
        line_index,
        actor_object_offset,
        actor: actor.to_string(),
        voice_selector,
        flags_b4,
        flags_b5,
        loop_target,
        control_word,
        word_offsets: word_offsets.to_vec(),
        subtitle,
        choices,
    })
}

fn resolve_words<'a>(
    script_name: &str,
    procedure_name: &str,
    opcode_offset: usize,
    offsets: &[u16],
    dictionary: &'a HashMap<u16, String>,
) -> Result<Vec<&'a str>> {
    offsets
        .iter()
        .map(|offset| {
            dictionary.get(offset).map(String::as_str).ok_or_else(|| {
                anyhow!(
                    "{script_name} procedure {procedure_name} text at 0x{opcode_offset:04X} references missing DIC word 0x{offset:04X}"
                )
            })
        })
        .collect()
}

fn object_symbol(symbols: &[DebSymbol], offset: u16) -> Option<&str> {
    symbols
        .iter()
        .find(|symbol| symbol.kind == 1 && symbol.offset == offset)
        .map(|symbol| symbol.name.as_str())
}

fn cfg_procedure_at(graph: &CodControlFlow, offset: usize) -> Option<&str> {
    graph
        .blocks
        .iter()
        .find(|block| block.start <= offset && offset < block.end_exclusive)
        .map(|block| block.procedure.as_str())
}

fn token_kind(token: &VmToken) -> &'static str {
    match token {
        VmToken::Text { .. } => "text",
        VmToken::GuardPush { .. } => "guard_push",
        VmToken::GuardPop { .. } => "guard_pop",
        VmToken::ConceptGuard { .. } => "concept_guard",
        VmToken::Jump { .. } => "jump",
        VmToken::StateArray { .. } => "state_array",
        VmToken::ConditionalBlock { .. } => "conditional_block",
        VmToken::LoadString { .. } => "load_string",
        VmToken::PokeByte { .. } => "poke_byte",
        VmToken::CharacterSlot { .. } => "character_slot",
        VmToken::ClearAlternateConcept { .. } => "clear_alternate_concept",
        VmToken::FlagBranch { .. } => "flag_branch",
        VmToken::Actor { .. } => "actor",
        VmToken::RecordLink { .. } => "record_link",
        VmToken::RecordEntry { .. } => "record_entry",
        VmToken::RecordClear { .. } => "record_clear",
        VmToken::BitFlag { .. } => "bit_flag",
        VmToken::SharedState { .. } => "shared_state",
        VmToken::SharedBitState { .. } => "shared_bit_state",
        VmToken::RecordWildcard { .. } => "record_wildcard",
        VmToken::RecordState { .. } => "record_state",
        VmToken::GlobalWordCompare { .. } => "global_word_compare",
        VmToken::GlobalPairCompare { .. } => "global_pair_compare",
        VmToken::PairRecord { .. } => "pair_record",
        VmToken::RecordTriple { .. } => "record_triple",
        VmToken::ScriptProfileRequest { .. } => "script_profile_request",
        VmToken::Op { .. } => "op",
        VmToken::Invalid { .. } => "invalid",
    }
}

fn is_contact_entry_token(token: &VmToken) -> bool {
    matches!(
        token,
        VmToken::GuardPush { .. }
            | VmToken::StateArray { .. }
            | VmToken::Actor { .. }
            | VmToken::SharedState { .. }
            | VmToken::SharedBitState { .. }
            | VmToken::RecordWildcard { .. }
    )
}

pub fn tsv(manifest: &ContactManifest) -> String {
    let mut output = String::from(
        "script\tprocedure\tprocedure_offset\tcontact_offset\tentry_class\tcontact_object\tobject_offset\tentry_tokens\tpresentations\ttexts\tfirst_text_offset\tfirst_subtitle\n",
    );
    for procedure in &manifest.procedures {
        let first = &procedure.texts[0];
        let subtitle = first.subtitle.replace(['\t', '\n', '\r'], " ");
        output.push_str(&format!(
            "{}\t{}\t0x{:04X}\t0x{:04X}\t{}\t{}\t0x{:04X}\t{}\t{}\t{}\t0x{:04X}\t{}\n",
            procedure.script,
            procedure.procedure,
            procedure.procedure_offset,
            procedure.contact_offset,
            procedure.entry_class,
            procedure.contact_object,
            procedure.contact_object_offset,
            procedure.entry_tokens.len(),
            procedure.presentations.len(),
            procedure.texts.len(),
            first.opcode_offset,
            subtitle,
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped_game_dir() -> &'static Path {
        Path::new("accuracy/cblood_install/cblood")
    }

    #[test]
    fn shipped_contact_census_is_complete() {
        if !shipped_game_dir().join("SCRIPT5.COD").exists() {
            return;
        }
        let manifest = analyze_game_dir(shipped_game_dir()).unwrap();
        assert_eq!(
            manifest
                .scripts
                .iter()
                .map(|script| script.procedures)
                .collect::<Vec<_>>(),
            [1, 15, 16, 19, 14]
        );
        assert_eq!(manifest.procedure_count, 65);
        assert_eq!(manifest.direct_entry_count, 29);
        assert_eq!(manifest.conditioned_entry_count, 36);
        assert_eq!(manifest.text_count, 661);
        assert_eq!(
            manifest
                .procedures
                .iter()
                .map(|procedure| procedure.presentations.len())
                .sum::<usize>(),
            64
        );
        assert_eq!(
            manifest
                .procedures
                .iter()
                .filter(|procedure| !procedure.activation_enabled)
                .count(),
            5
        );
        assert_eq!(
            manifest.direct_entry_count + manifest.conditioned_entry_count,
            manifest.procedure_count
        );
        assert!(manifest.text_count > manifest.procedure_count);
    }

    #[test]
    fn bob_manifest_fields_are_exact_binary_offsets() {
        if !shipped_game_dir().join("SCRIPT1.COD").exists() {
            return;
        }
        let manifest = analyze_game_dir(shipped_game_dir()).unwrap();
        let bob = manifest
            .procedures
            .iter()
            .find(|procedure| procedure.script == "SCRIPT1" && procedure.procedure == "BOB1")
            .unwrap();
        assert_eq!(bob.entry_class, "direct");
        assert_eq!(bob.contact_object, "Bob_Morlock");
        assert_eq!(bob.contact_object_offset, 0x004A);
        assert_eq!(bob.presentations.len(), 1);
        assert_eq!(bob.presentations[0].action_record_offset, 0x0084);
        assert_eq!(bob.presentations[0].related_record_offset, 0x0028);
        assert_eq!(bob.texts[0].opcode_offset, 0x0788);
        assert_eq!(bob.texts[0].word_list_offset, 0x078E);
        let mission = bob
            .texts
            .iter()
            .find(|text| text.opcode_offset == 0x07E2)
            .unwrap();
        assert_eq!(mission.word_list_offset, 0x07EA);
        assert_eq!(mission.choices, ["yes", "no"]);
    }
}
