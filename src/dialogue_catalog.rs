//! Static dialogue inventory joined to the existing COD/BAS control-flow parsers.

use std::collections::HashMap;

use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{bas_cfg, script, vm, vm_cfg, vm_profile, vm_source};

fn operands(words: &[u16], dictionary: &HashMap<u16, String>, sequel: bool) -> Result<Vec<Value>> {
    let mut result = Vec::new();
    let mut words = words.iter().copied();
    while let Some(word) = words.next() {
        result.push(if sequel && word == 1 {
            let offset = words
                .next()
                .ok_or_else(|| anyhow!("numeric text marker has no VAR operand"))?;
            json!({"kind": "state_number", "offset": offset})
        } else if sequel && word == 0xfffe {
            json!({"kind": "inventory_choices"})
        } else if word == 0xffff {
            json!({"kind": "separator"})
        } else {
            let text = dictionary
                .get(&word)
                .ok_or_else(|| anyhow!("missing dictionary word 0x{word:04X}"))?;
            json!({"kind": "dictionary", "offset": word, "text": text})
        });
    }
    Ok(result)
}

fn text_site(
    token: &vm::VmToken,
    dictionary: &HashMap<u16, String>,
    sequel: bool,
    symbols: &[script::DebSymbol],
) -> Result<Value> {
    let vm::VmToken::Text {
        offset,
        line_index,
        voice_selector,
        flags_b4,
        flags_b5,
        loop_target,
        control_word,
        word_offsets,
    } = token
    else {
        return Err(anyhow!("expected a text token"));
    };
    let decoded = operands(word_offsets, dictionary, sequel)?;
    let sections = decoded
        .split(|word| word["kind"] == "separator")
        .collect::<Vec<_>>();
    let separator = decoded.iter().position(|word| word["kind"] == "separator");
    let spoken = &decoded[..separator.unwrap_or(decoded.len())];
    let menu = separator
        .map(|index| &decoded[index + 1..])
        .unwrap_or_default();
    let pieces = spoken
        .iter()
        .map(|word| match word["kind"].as_str() {
            Some("dictionary") => word["text"].as_str().unwrap().to_owned(),
            Some("state_number") => format!("{{VAR:0x{:04X}}}", word["offset"].as_u64().unwrap()),
            Some("inventory_choices") => "{inventory_choices}".to_owned(),
            _ => unreachable!("spoken operands exclude separators"),
        })
        .collect::<Vec<_>>();
    let text = script::assemble_words(&pieces.iter().map(String::as_str).collect::<Vec<_>>());
    Ok(json!({
        "offset": offset, "record_offset": line_index,
        "record_name": symbols.iter().find(|symbol| symbol.kind == 1 && symbol.offset == *line_index).map(|symbol| &symbol.name),
        "text": text, "spoken_operands": spoken, "choice_operands": menu, "sections": sections,
        "dynamic": decoded.iter().any(|word| matches!(word["kind"].as_str(), Some("state_number" | "inventory_choices"))),
        "presentation_selector": voice_selector, "active_line": vm::text_selector_active_line_id(*voice_selector),
        "chatter": flags_b4 & 0x20 != 0, "flags_b4": flags_b4, "flags_b5": flags_b5,
        "skip_next_if_not_shown": vm::text_conditional_skip_count(*flags_b4, *flags_b5),
        "resume_offset": loop_target, "control_word": control_word,
        "recent_choice_count": if flags_b4 & 0x40 != 0 { flags_b5 & 7 } else { 0 },
    }))
}

/// Compile editable source with the established compiler, then scan every typed
/// instruction. No VM, game loop, display, save state, or input driver is run.
pub fn analyze(source: &str) -> Result<Value> {
    let images = vm_profile::compile(source)?;
    analyze_images(&images)
}

/// Analyze an already-decoded profile bundle using the same static pipeline.
pub fn analyze_images(images: &vm_profile::ProfileImages) -> Result<Value> {
    let sequel = images.dialect == vm_profile::ProfileDialect::BigBugBang;
    let dictionary = script::parse_dictionary(&images.dic);
    let symbols = script::parse_deb(&images.deb);
    let graph = if sequel {
        vm_cfg::analyze_big_bug_bang_cod(&images.name, &images.cod, &symbols)?
    } else {
        vm_cfg::analyze_cod(&images.name, &images.cod, &symbols)?
    };
    let tokens = if sequel {
        vm::walk_big_bug_bang(&images.cod, 0, images.cod.len())
    } else {
        vm::walk(&images.cod, 0, images.cod.len())
    };
    let mut instructions = Vec::new();
    let mut text_sites = Vec::new();
    let mut profile_requests = Vec::new();
    for token in &tokens {
        let block = graph
            .blocks
            .iter()
            .find(|block| (block.start..block.end_exclusive).contains(&token.offset()))
            .ok_or_else(|| anyhow!("instruction has no CFG block at 0x{:04X}", token.offset()))?;
        instructions.push(json!({"offset": token.offset(), "block": block.start,
            "procedure": block.procedure, "instruction": token,
            "description": vm_source::token_comment(token, &dictionary)}));
        if let vm::VmToken::Text { .. } = token {
            let mut site = text_site(token, &dictionary, sequel, &symbols)?;
            site["block"] = json!(block.start);
            site["procedure"] = json!(block.procedure);
            text_sites.push(site);
        }
        if let vm::VmToken::ScriptProfileRequest {
            offset,
            operand,
            profile_index,
            ..
        } = token
        {
            profile_requests.push(json!({"offset": offset, "procedure": block.procedure,
                "operand": operand, "profile_index": profile_index,
                "target": format!("SCRIPT{}", u32::from(*profile_index) + 1),
                "condition": "request only; deferred until presentation permits profile change"}));
        }
    }
    let bas = images.bas.as_ref().map(|bas| -> Result<Value> {
        let graph = bas_cfg::analyze_bas(&images.name, bas, &images.var, &dictionary, &symbols)?;
        let mut sites = Vec::new();
        let mut choices = Vec::new();
        for node in &graph.nodes {
            let list = &graph.lists[node.list_index];
            for event in &node.dialogue_events {
                let (_, vm_source::BasToken::Text(token)) = vm_source::bas_token_at(bas, event.offset, &dictionary)
                    .ok_or_else(|| anyhow!("missing BAS text at 0x{:04X}", event.offset))? else {
                        return Err(anyhow!("BAS dialogue event is not a text instruction"));
                    };
                let mut site = text_site(&token, &dictionary, false, &symbols)?;
                site["selector_node"] = json!(node.offset);
                site["object"] = json!(list.entrypoint.object_name);
                sites.push(site);
            }
            for (row, choice) in node.menu_choices.iter().enumerate() {
                // The native selector scan uses the first matching node in this
                // object's linked list; duplicate selectors are not extra paths.
                let matches = list.node_offsets.iter().filter_map(|offset| graph.nodes.iter()
                    .find(|candidate| candidate.offset == *offset && candidate.selector == choice.offset))
                    .map(|candidate| candidate.offset).collect::<Vec<_>>();
                choices.push(json!({"from_node": node.offset, "object": list.entrypoint.object_name,
                    "row": row, "choice": choice, "to_node": matches.first(),
                    "shadowed_nodes": matches.iter().skip(1).copied().collect::<Vec<_>>(),
                    "resolution": if matches.is_empty() { "no_local_selector_match" } else { "first_local_selector_match" }}));
            }
        }
        Ok(json!({"control_flow": graph, "text_sites": sites, "choice_edges": choices}))
    }).transpose()?;
    Ok(
        json!({"schema": 1, "profile": images.name, "game": if sequel { "bbb" } else { "cb" },
            "analysis": "static authored graph; conditions are not solved and runtime reachability is not asserted",
            "resources": {"cod_sha256": format!("{:x}", Sha256::digest(&images.cod)),
                "dic_sha256": format!("{:x}", Sha256::digest(&images.dic))},
            "symbols": symbols,
            "cod": {"control_flow": graph, "instructions": instructions, "text_sites": text_sites,
                "profile_requests": profile_requests},
            "bas": bas,
        }),
    )
}
