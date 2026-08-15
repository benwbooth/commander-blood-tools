//! Static control-flow recovery for Commander Blood's BAS selector lists.
//!
//! A BAS entry stored in an object's selector-2 `.VAR` field points at the
//! one-byte `AC` prefix before a linked list of `{selector, next, body}` nodes.
//! The native dispatcher compares selectors and follows `next` on a mismatch;
//! a match executes the node body until its `AC`, `AA`, or `FF` terminator.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::{anyhow, bail, Result};
use serde::Serialize;

use crate::script::DebSymbol;
use crate::vm;
use crate::vm_source::{self, BasToken};

const BAS_CODE_FIELD_SELECTOR: u8 = 2;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BasTerminatorKind {
    YieldA,
    YieldB,
    End,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BasEdgeKind {
    SelectorMatch,
    SelectorMismatch,
    SelectorMissExit,
    BodyYield,
    BodyEnd,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct BasEdge {
    pub from: usize,
    pub to: Option<usize>,
    pub kind: BasEdgeKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BasEntrypoint {
    pub object_name: String,
    pub object_offset: u16,
    pub object_kind: u16,
    pub prefix_yield_b: usize,
    pub root_node: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BasSelectorNode {
    pub offset: usize,
    pub selector: u16,
    pub selector_name: String,
    pub next: Option<usize>,
    pub body_start: usize,
    pub menu_offset: usize,
    pub body_end: usize,
    pub terminator: BasTerminatorKind,
    pub body_token_count: usize,
    pub list_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BasSelectorList {
    pub index: usize,
    pub entrypoint: BasEntrypoint,
    pub terminal_node: usize,
    pub end_exclusive: usize,
    pub node_offsets: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BasControlFlow {
    pub script: String,
    pub image_bytes: usize,
    pub token_count: usize,
    pub selector_node_count: usize,
    pub list_count: usize,
    pub entrypoint_count: usize,
    pub direct_next_count: usize,
    pub edge_count: usize,
    pub entrypoints: Vec<BasEntrypoint>,
    pub lists: Vec<BasSelectorList>,
    pub nodes: Vec<BasSelectorNode>,
    pub edges: Vec<BasEdge>,
}

#[derive(Clone, Debug)]
struct DecodedToken {
    end: usize,
    token: BasToken,
}

/// Recover and validate every selector list in one shipped BAS image.
///
/// This joins three independently recovered structures: the sequential BAS
/// grammar, the native selector-2 object field lookup, and the linked selector
/// scan. Any undecoded byte, dangling link, unowned node, or disagreement
/// between physical roots and object-derived entrypoints is an error.
pub fn analyze_bas(
    script: &str,
    image: &[u8],
    var: &[u8],
    dictionary: &HashMap<u16, String>,
    symbols: &[DebSymbol],
) -> Result<BasControlFlow> {
    let tokens = decode_typed_stream(image, dictionary)?;
    let token_indices: BTreeMap<usize, usize> = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.token.offset(), index))
        .collect();
    let selector_nodes: BTreeSet<usize> = tokens
        .iter()
        .filter_map(|decoded| match decoded.token {
            BasToken::SelectorNode { offset, .. } => Some(offset),
            _ => None,
        })
        .collect();
    let physical_roots = physical_roots(&tokens)?;
    let entrypoints = object_entrypoints(var, symbols, &tokens, &token_indices)?;
    let entry_roots: BTreeSet<usize> = entrypoints.iter().map(|entry| entry.root_node).collect();
    if entry_roots != physical_roots {
        let missing: Vec<_> = physical_roots.difference(&entry_roots).copied().collect();
        let extra: Vec<_> = entry_roots.difference(&physical_roots).copied().collect();
        bail!(
            "BAS object entrypoints disagree with physical list roots; missing={missing:04X?} extra={extra:04X?}"
        );
    }

    let mut lists = Vec::new();
    let mut nodes = Vec::new();
    let mut edges = BTreeSet::new();
    let mut owned_nodes = BTreeSet::new();

    for (list_index, entrypoint) in entrypoints.iter().enumerate() {
        let mut node_offset = entrypoint.root_node;
        let mut node_offsets = Vec::new();
        let terminal_node;
        let end_exclusive;
        let mut chain_seen = BTreeSet::new();

        loop {
            if !chain_seen.insert(node_offset) {
                bail!("BAS selector cycle reaches 0x{node_offset:04X}");
            }
            if !owned_nodes.insert(node_offset) {
                bail!("BAS selector node 0x{node_offset:04X} belongs to multiple lists");
            }
            node_offsets.push(node_offset);

            let node_index = *token_indices
                .get(&node_offset)
                .ok_or_else(|| anyhow!("missing BAS selector token at 0x{node_offset:04X}"))?;
            let (selector, next) = match tokens[node_index].token {
                BasToken::SelectorNode { selector, next, .. } => (selector, next),
                _ => bail!("BAS list reaches non-selector token at 0x{node_offset:04X}"),
            };
            let body_index = node_index + 1;
            let Some(body_head) = tokens.get(body_index) else {
                bail!("BAS selector node 0x{node_offset:04X} has no body");
            };
            let menu_offset = match body_head.token {
                BasToken::Menu { offset, .. } if offset == node_offset + 4 => offset,
                _ => bail!("BAS selector body at 0x{node_offset:04X} does not begin with MENU"),
            };

            let (terminator_index, terminator) = if next != 0 {
                let target = usize::from(next);
                let target_index = *token_indices.get(&target).ok_or_else(|| {
                    anyhow!("BAS selector next offset 0x{next:04X} is not a token boundary")
                })?;
                if target_index <= body_index {
                    bail!(
                        "BAS selector next offset 0x{next:04X} does not follow node 0x{node_offset:04X}"
                    );
                }
                if tokens[body_index..target_index - 1]
                    .iter()
                    .any(|token| matches!(token.token, BasToken::SelectorNode { .. }))
                {
                    bail!("BAS selector link from 0x{node_offset:04X} skips a physical node");
                }
                let terminator_index = target_index - 1;
                match tokens[terminator_index].token {
                    BasToken::YieldB { .. } => (terminator_index, BasTerminatorKind::YieldB),
                    _ => bail!("BAS selector link to 0x{target:04X} is not preceded by YIELD_B"),
                }
            } else {
                let terminator_index = (body_index..tokens.len())
                    .find(|&index| {
                        matches!(
                            tokens[index].token,
                            BasToken::Yield { .. } | BasToken::End { .. }
                        )
                    })
                    .ok_or_else(|| {
                        anyhow!("terminal BAS selector 0x{node_offset:04X} has no terminator")
                    })?;
                if tokens[body_index..terminator_index].iter().any(|token| {
                    matches!(
                        token.token,
                        BasToken::YieldB { .. } | BasToken::SelectorNode { .. }
                    )
                }) {
                    bail!("terminal BAS selector 0x{node_offset:04X} crosses another node");
                }
                let kind = match tokens[terminator_index].token {
                    BasToken::Yield { .. } => BasTerminatorKind::YieldA,
                    BasToken::End { .. } => BasTerminatorKind::End,
                    _ => unreachable!(),
                };
                (terminator_index, kind)
            };

            let body_start = body_head.token.offset();
            let body_end = tokens[terminator_index].token.offset();
            let body_token_count = terminator_index - body_index;
            let next_offset = (next != 0).then_some(usize::from(next));
            nodes.push(BasSelectorNode {
                offset: node_offset,
                selector,
                selector_name: dictionary
                    .get(&selector)
                    .cloned()
                    .ok_or_else(|| anyhow!("unknown BAS selector 0x{selector:04X}"))?,
                next: next_offset,
                body_start,
                menu_offset,
                body_end,
                terminator,
                body_token_count,
                list_index,
            });
            edges.insert(BasEdge {
                from: node_offset,
                to: Some(body_start),
                kind: BasEdgeKind::SelectorMatch,
            });
            edges.insert(BasEdge {
                from: node_offset,
                to: next_offset,
                kind: if next_offset.is_some() {
                    BasEdgeKind::SelectorMismatch
                } else {
                    BasEdgeKind::SelectorMissExit
                },
            });
            edges.insert(BasEdge {
                from: body_end,
                to: None,
                kind: if terminator == BasTerminatorKind::End {
                    BasEdgeKind::BodyEnd
                } else {
                    BasEdgeKind::BodyYield
                },
            });

            if let Some(next_offset) = next_offset {
                node_offset = next_offset;
            } else {
                terminal_node = node_offset;
                end_exclusive = tokens[terminator_index].end;
                break;
            }
        }

        lists.push(BasSelectorList {
            index: list_index,
            entrypoint: entrypoint.clone(),
            terminal_node,
            end_exclusive,
            node_offsets,
        });
    }

    if owned_nodes != selector_nodes {
        let unowned: Vec<_> = selector_nodes.difference(&owned_nodes).copied().collect();
        bail!("BAS selector nodes are not owned by an object list: {unowned:04X?}");
    }

    nodes.sort_by_key(|node| node.offset);
    let direct_next_count = nodes.iter().filter(|node| node.next.is_some()).count();
    let edges: Vec<_> = edges.into_iter().collect();
    Ok(BasControlFlow {
        script: script.to_string(),
        image_bytes: image.len(),
        token_count: tokens.len(),
        selector_node_count: nodes.len(),
        list_count: lists.len(),
        entrypoint_count: entrypoints.len(),
        direct_next_count,
        edge_count: edges.len(),
        entrypoints,
        lists,
        nodes,
        edges,
    })
}

fn decode_typed_stream(
    image: &[u8],
    dictionary: &HashMap<u16, String>,
) -> Result<Vec<DecodedToken>> {
    let mut tokens = Vec::new();
    let mut cursor = 0usize;
    while cursor < image.len() {
        let (end, token) = vm_source::bas_token_at(image, cursor, dictionary)
            .ok_or_else(|| anyhow!("undecoded BAS byte at 0x{cursor:04X}"))?;
        let encoded = token
            .encode()
            .ok_or_else(|| anyhow!("unencodable BAS token at 0x{cursor:04X}"))?;
        if image.get(cursor..end) != Some(encoded.as_slice()) {
            bail!("BAS token at 0x{cursor:04X} does not re-encode exactly");
        }
        tokens.push(DecodedToken { end, token });
        cursor = end;
    }
    if !matches!(
        tokens.last().map(|token| &token.token),
        Some(BasToken::End { .. })
    ) {
        bail!("BAS stream does not end in a decoded 0xFF marker");
    }
    Ok(tokens)
}

fn physical_roots(tokens: &[DecodedToken]) -> Result<BTreeSet<usize>> {
    let mut roots = BTreeSet::new();
    for (index, decoded) in tokens.iter().enumerate() {
        let BasToken::SelectorNode { offset, .. } = decoded.token else {
            continue;
        };
        if index == 0 || !matches!(tokens[index - 1].token, BasToken::YieldB { .. }) {
            bail!("BAS selector node 0x{offset:04X} is not preceded by YIELD_B");
        }
        if index >= 2 && matches!(tokens[index - 2].token, BasToken::Yield { .. }) {
            roots.insert(offset);
        }
    }
    Ok(roots)
}

fn object_entrypoints(
    var: &[u8],
    symbols: &[DebSymbol],
    tokens: &[DecodedToken],
    token_indices: &BTreeMap<usize, usize>,
) -> Result<Vec<BasEntrypoint>> {
    let mut entrypoints = Vec::new();
    for symbol in symbols.iter().filter(|symbol| symbol.kind == 1) {
        let object_offset = usize::from(symbol.offset);
        let object_kind = read_word(var, object_offset).ok_or_else(|| {
            anyhow!(
                "DEB object {:?} offset 0x{:04X} is outside VAR",
                symbol.name,
                symbol.offset
            )
        })?;
        let Some(field_offset) =
            vm::vm_field_offset(BAS_CODE_FIELD_SELECTOR, object_kind).filter(|offset| *offset != 0)
        else {
            continue;
        };
        let field = object_offset + usize::from(field_offset);
        let code_offset = read_word(var, field).ok_or_else(|| {
            anyhow!(
                "BAS field for DEB object {:?} at VAR 0x{field:04X} is truncated",
                symbol.name
            )
        })?;
        if code_offset == 0 {
            continue;
        }
        let prefix_yield_b = usize::from(code_offset);
        let prefix_index = *token_indices.get(&prefix_yield_b).ok_or_else(|| {
            anyhow!(
                "BAS entry for object {:?} points to non-token offset 0x{code_offset:04X}",
                symbol.name
            )
        })?;
        if !matches!(tokens[prefix_index].token, BasToken::YieldB { .. }) {
            bail!(
                "BAS entry for object {:?} at 0x{code_offset:04X} is not YIELD_B",
                symbol.name
            );
        }
        let root_node = tokens
            .get(prefix_index + 1)
            .map(|token| token.token.offset())
            .ok_or_else(|| anyhow!("BAS entry for object {:?} has no selector", symbol.name))?;
        if root_node != prefix_yield_b + 1
            || !matches!(
                tokens[prefix_index + 1].token,
                BasToken::SelectorNode { .. }
            )
        {
            bail!(
                "BAS entry for object {:?} at 0x{code_offset:04X} is not followed by a selector node",
                symbol.name
            );
        }
        entrypoints.push(BasEntrypoint {
            object_name: symbol.name.clone(),
            object_offset: symbol.offset,
            object_kind,
            prefix_yield_b,
            root_node,
        });
    }
    entrypoints.sort_by_key(|entry| entry.prefix_yield_b);
    for pair in entrypoints.windows(2) {
        if pair[0].root_node == pair[1].root_node {
            bail!(
                "multiple DEB objects own BAS selector root 0x{:04X}",
                pair[0].root_node
            );
        }
    }
    Ok(entrypoints)
}

fn read_word(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes([
        *bytes.get(offset)?,
        *bytes.get(offset + 1)?,
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_inputs() -> (Vec<u8>, Vec<u8>, HashMap<u16, String>, Vec<DebSymbol>) {
        let image = vec![
            0xAA, // list delimiter
            0xAC, // object entry
            0x34, 0x12, 0x0C, 0x00, // selector 0x1234, next node 0x000C
            0xA3, 0x00, 0x20, 0x00, 0x00, // MENU 0x2000
            0xAC, // matched body yields; prefix for the next node
            0x00, 0x20, 0x00, 0x00, // selector 0x2000, terminal
            0xA3, 0x34, 0x12, 0x00, 0x00, // MENU 0x1234
            0xFF,
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
        (image, var, dictionary, symbols)
    }

    #[test]
    fn recovers_object_entry_and_selector_edges() {
        let (image, var, dictionary, symbols) = synthetic_inputs();
        let graph = analyze_bas("SCRIPTX", &image, &var, &dictionary, &symbols).unwrap();
        assert_eq!(graph.token_count, 8);
        assert_eq!(graph.selector_node_count, 2);
        assert_eq!(graph.list_count, 1);
        assert_eq!(graph.direct_next_count, 1);
        assert_eq!(graph.edge_count, 6);
        assert_eq!(graph.lists[0].node_offsets, vec![2, 12]);
        assert_eq!(graph.nodes[0].body_end, 11);
        assert_eq!(graph.nodes[0].terminator, BasTerminatorKind::YieldB);
        assert_eq!(graph.nodes[1].terminator, BasTerminatorKind::End);
        assert!(graph.edges.contains(&BasEdge {
            from: 2,
            to: Some(12),
            kind: BasEdgeKind::SelectorMismatch,
        }));
    }

    #[test]
    fn rejects_selector_links_that_are_not_token_boundaries() {
        let (mut image, var, dictionary, symbols) = synthetic_inputs();
        image[4..6].copy_from_slice(&13u16.to_le_bytes());
        let error = analyze_bas("SCRIPTX", &image, &var, &dictionary, &symbols)
            .unwrap_err()
            .to_string();
        assert!(error.contains("not a token boundary"), "{error}");
    }

    #[test]
    fn shipped_bas_lists_match_object_fields() {
        let Some(root) = std::env::var_os("CBLOOD_GAME_DIR").map(std::path::PathBuf::from) else {
            eprintln!("skipping: CBLOOD_GAME_DIR is not set");
            return;
        };
        let expected = [
            (1, 1, 1),
            (2, 10, 122),
            (3, 12, 98),
            (4, 10, 43),
            (5, 4, 57),
        ];
        for (script, list_count, node_count) in expected {
            let read = |extension: &str| {
                std::fs::read(root.join(format!("SCRIPT{script}.{extension}"))).unwrap()
            };
            let image = read("BAS");
            let var = read("VAR");
            let dictionary = crate::script::parse_dictionary(&read("DIC"));
            let symbols = crate::script::parse_deb(&read("DEB"));
            let graph = analyze_bas(
                &format!("SCRIPT{script}"),
                &image,
                &var,
                &dictionary,
                &symbols,
            )
            .unwrap();
            assert_eq!(graph.list_count, list_count);
            assert_eq!(graph.entrypoint_count, list_count);
            assert_eq!(graph.selector_node_count, node_count);
        }
    }
}
