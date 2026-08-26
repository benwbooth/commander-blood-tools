//! Typed traversal of BloodScript BAS selector-case lists.

use std::collections::BTreeSet;
use std::fmt;

use commander_blood_formats::bas::{ScriptBas, ScriptBasInstruction};
use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::script::ScriptWordId;

/// Malformed selector linkage rejected before executing a response body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptSelectorError {
    /// A selector root or next link does not identify a typed selector node.
    MissingNode {
        /// Rejected BAS source position.
        source_offset: ScriptCodeOffset,
    },
    /// Following next links revisits a node instead of reaching a terminator.
    Cycle {
        /// First revisited BAS source position.
        source_offset: ScriptCodeOffset,
    },
}

impl fmt::Display for ScriptSelectorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ScriptSelectorError {}

/// Find the response body selected from one linked BAS case list.
///
/// This translates `value_scan_match` at BLOODPRG file offset `0x00577A`.
/// A match returns the first typed token after the four-byte selector node;
/// exhausting the linked list returns `None`.
pub fn find_selector_body(
    dialogue: &ScriptBas,
    root: ScriptCodeOffset,
    selected: ScriptWordId,
) -> Result<Option<ScriptCodeOffset>, ScriptSelectorError> {
    let mut current = root;
    let mut visited = BTreeSet::new();

    loop {
        if !visited.insert(current) {
            return Err(ScriptSelectorError::Cycle {
                source_offset: current,
            });
        }
        let token = dialogue
            .tokens()
            .iter()
            .find(|token| token.source_offset() == current)
            .ok_or(ScriptSelectorError::MissingNode {
                source_offset: current,
            })?;
        let ScriptBasInstruction::SelectorNode { selector, next } = token.instruction() else {
            return Err(ScriptSelectorError::MissingNode {
                source_offset: current,
            });
        };
        if *selector == selected {
            return Ok(Some(token.end_offset()));
        }
        let Some(next) = *next else {
            return Ok(None);
        };
        current = next;
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::bas::{ScriptBasInstruction, decode_script_bas};
    use commander_blood_formats::script::{ScriptDictionary, decode_script_dictionary};
    use serde::Deserialize;

    use super::*;

    const PROFILE_COUNT: usize = 5;
    const EXPECTED_SELECTOR_NODE_COUNTS: [usize; PROFILE_COUNT] = [1, 122, 98, 43, 57];
    const EXPECTED_TOTAL_SELECTOR_NODE_COUNT: usize = 321;
    const SELECTOR_YIELD_OPCODE: u8 = 0xAC;
    const BAS_END_MARKER: u8 = 0xFF;

    #[derive(Deserialize)]
    struct SelectorGraph {
        lists: Vec<SelectorList>,
        nodes: Vec<SelectorNode>,
    }

    #[derive(Deserialize)]
    struct SelectorList {
        node_offsets: Vec<usize>,
    }

    #[derive(Deserialize)]
    struct SelectorNode {
        offset: usize,
        body_start: usize,
    }

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn selector_at(dialogue: &ScriptBas, source_offset: usize) -> ScriptWordId {
        let token = dialogue
            .tokens()
            .iter()
            .find(|token| token.source_offset().index() == source_offset)
            .unwrap();
        let ScriptBasInstruction::SelectorNode { selector, .. } = token.instruction() else {
            panic!("selector graph position is not a typed node");
        };
        *selector
    }

    #[test]
    fn every_recovered_selector_node_resolves_to_its_authored_body() {
        let root = workspace_root();
        let mut total_nodes = usize::MIN;

        for profile in 1..=PROFILE_COUNT {
            let assets = root.join("accuracy/cblood_install/cblood");
            let dictionary = decode_script_dictionary(
                &std::fs::read(assets.join(format!("SCRIPT{profile}.DIC"))).unwrap(),
            )
            .unwrap();
            let dialogue = decode_script_bas(
                &std::fs::read(assets.join(format!("SCRIPT{profile}.BAS"))).unwrap(),
                &dictionary,
            )
            .unwrap();
            let graph: SelectorGraph = serde_json::from_slice(
                &std::fs::read(root.join(format!(
                    "re/vm/bas-control-flow/script{profile}.bas.cfg.json"
                )))
                .unwrap(),
            )
            .unwrap();
            assert_eq!(
                graph.nodes.len(),
                EXPECTED_SELECTOR_NODE_COUNTS[profile - 1]
            );

            for list in &graph.lists {
                let root_offset = ScriptCodeOffset::new(list.node_offsets[0]);
                let list_selectors = list
                    .node_offsets
                    .iter()
                    .map(|offset| selector_at(&dialogue, *offset))
                    .collect::<BTreeSet<_>>();
                for node_offset in &list.node_offsets {
                    let selected = selector_at(&dialogue, *node_offset);
                    let first_matching_node = list
                        .node_offsets
                        .iter()
                        .find(|candidate| selector_at(&dialogue, **candidate) == selected)
                        .unwrap();
                    let expected_body = graph
                        .nodes
                        .iter()
                        .find(|node| node.offset == *first_matching_node)
                        .unwrap()
                        .body_start;
                    assert_eq!(
                        find_selector_body(&dialogue, root_offset, selected).unwrap(),
                        Some(ScriptCodeOffset::new(expected_body))
                    );
                    total_nodes += 1;
                }
                let absent = dictionary
                    .words()
                    .map(|(word, _bytes)| word)
                    .find(|word| !list_selectors.contains(word))
                    .unwrap();
                assert_eq!(
                    find_selector_body(&dialogue, root_offset, absent).unwrap(),
                    None
                );
            }
        }

        assert_eq!(total_nodes, EXPECTED_TOTAL_SELECTOR_NODE_COUNT);
    }

    fn malformed_dialogue(next: u16) -> (ScriptBas, ScriptDictionary, ScriptCodeOffset) {
        let dictionary = decode_script_dictionary(b"alpha\0beta\0").unwrap();
        let mut bytes = vec![SELECTOR_YIELD_OPCODE];
        bytes.extend_from_slice(&u16::MIN.to_le_bytes());
        bytes.extend_from_slice(&next.to_le_bytes());
        bytes.push(BAS_END_MARKER);
        let dialogue = decode_script_bas(&bytes, &dictionary).unwrap();
        (dialogue, dictionary, ScriptCodeOffset::new(1))
    }

    #[test]
    fn malformed_selector_links_are_rejected_without_untyped_byte_access() {
        let (cyclic, dictionary, root) = malformed_dialogue(1);
        let beta = dictionary.resolve_source_offset(6).unwrap();
        assert_eq!(
            find_selector_body(&cyclic, root, beta).unwrap_err(),
            ScriptSelectorError::Cycle {
                source_offset: root,
            }
        );

        let (misaligned, dictionary, root) = malformed_dialogue(2);
        let beta = dictionary.resolve_source_offset(6).unwrap();
        assert_eq!(
            find_selector_body(&misaligned, root, beta).unwrap_err(),
            ScriptSelectorError::MissingNode {
                source_offset: ScriptCodeOffset::new(2),
            }
        );
    }
}
