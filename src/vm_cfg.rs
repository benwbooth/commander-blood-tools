//! Static control-flow recovery for the typed Commander Blood COD stream.
//!
//! This analysis follows the native VM's branch-stack rules. It deliberately
//! keeps frame-resume edges separate from immediate program-counter transfers.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{Result, anyhow, bail};
use serde::Serialize;

use crate::script::{DebSymbol, functions_from_symbols};
use crate::vm::{self, VmToken};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Fallthrough,
    BlockEnter,
    BlockSkip,
    GuardEnter,
    GuardExit,
    GuardPass,
    GuardFailure,
    Jump,
    TextContinue,
    TextSkip,
    FrameResume,
}

impl EdgeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fallthrough => "fallthrough",
            Self::BlockEnter => "block_enter",
            Self::BlockSkip => "block_skip",
            Self::GuardEnter => "guard_enter",
            Self::GuardExit => "guard_exit",
            Self::GuardPass => "guard_pass",
            Self::GuardFailure => "guard_failure",
            Self::Jump => "jump",
            Self::TextContinue => "text_continue",
            Self::TextSkip => "text_skip",
            Self::FrameResume => "frame_resume",
        }
    }

    fn is_immediate(self) -> bool {
        self != Self::FrameResume
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct InstructionEdge {
    pub from: usize,
    pub to: usize,
    pub kind: EdgeKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BasicBlock {
    pub start: usize,
    pub end_exclusive: usize,
    pub instruction_count: usize,
    pub procedure: String,
    pub reachable: bool,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct BlockEdge {
    pub from_block: usize,
    pub from_instruction: usize,
    pub to_block: usize,
    pub kind: EdgeKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CodControlFlow {
    pub script: String,
    pub image_bytes: usize,
    pub instruction_count: usize,
    pub procedure_count: usize,
    pub block_count: usize,
    pub reachable_block_count: usize,
    pub edge_count: usize,
    pub poke_instruction_count: usize,
    pub patched_block_flag_count: usize,
    pub mutable_block_flag_count: usize,
    pub unresolved_guard_branches: Vec<usize>,
    pub blocks: Vec<BasicBlock>,
    pub edges: Vec<BlockEdge>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct FlowState {
    query: bool,
    guard_stack: Vec<u16>,
}

impl FlowState {
    fn entry() -> Self {
        Self {
            query: false,
            guard_stack: Vec::new(),
        }
    }
}

const MAX_FLOW_STATES: usize = 250_000;
const MAX_GUARD_DEPTH: usize = 64;

pub fn analyze_cod(script: &str, image: &[u8], symbols: &[DebSymbol]) -> Result<CodControlFlow> {
    let tokens = vm::walk(image, 0, image.len());
    let end_marker = validate_typed_stream(image, &tokens)?;
    let functions = functions_from_symbols(script, symbols, image.len());
    let offsets: BTreeSet<usize> = tokens
        .iter()
        .map(VmToken::offset)
        .chain(std::iter::once(end_marker))
        .collect();
    let token_indices: BTreeMap<usize, usize> = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| (token.offset(), index))
        .collect();

    let (block_flags, poke_instruction_count, patched_block_flag_count) =
        collect_block_flag_values(&tokens)?;
    let mutable_block_flag_count = block_flags
        .values()
        .filter(|values| values.len() > 1)
        .count();

    let mut states: BTreeMap<usize, BTreeSet<FlowState>> = BTreeMap::new();
    let mut queue = VecDeque::new();
    let mut entry_offsets: BTreeSet<usize> =
        functions.iter().map(|function| function.offset).collect();
    entry_offsets.insert(0);
    for offset in entry_offsets {
        enqueue_state(
            &mut states,
            &mut queue,
            offset,
            FlowState::entry(),
            &offsets,
        )?;
    }
    let mut edges = BTreeSet::new();
    let mut unresolved = BTreeSet::new();
    let mut state_count: usize = states.values().map(BTreeSet::len).sum();

    while let Some((offset, state)) = queue.pop_front() {
        if offset == end_marker {
            continue;
        }
        let index = *token_indices
            .get(&offset)
            .ok_or_else(|| anyhow!("flow reached non-instruction offset 0x{offset:04X}"))?;
        let token = &tokens[index];
        let next = tokens
            .get(index + 1)
            .map(VmToken::offset)
            .unwrap_or(end_marker);

        let mut successors = Vec::new();
        match token {
            VmToken::ConditionalBlock { flags, target, .. } => {
                let values = block_flags.get(&offset).cloned().unwrap_or_else(|| {
                    let mut values = BTreeSet::new();
                    values.insert(*flags);
                    values
                });
                if values.iter().any(|value| value & 1 != 0) {
                    successors.push((
                        next,
                        FlowState {
                            query: true,
                            guard_stack: vec![*target],
                        },
                        EdgeKind::BlockEnter,
                    ));
                }
                if values.iter().any(|value| value & 1 == 0) {
                    successors.push((usize::from(*target), state.clone(), EdgeKind::BlockSkip));
                }
            }
            VmToken::GuardPush { target, .. } => {
                let mut entered = state.clone();
                entered.query = true;
                entered.guard_stack.push(*target);
                if entered.guard_stack.len() > MAX_GUARD_DEPTH {
                    bail!("guard stack exceeds {MAX_GUARD_DEPTH} entries at 0x{offset:04X}");
                }
                successors.push((next, entered, EdgeKind::GuardEnter));
            }
            VmToken::GuardPop { .. } => {
                let mut exited = state.clone();
                exited.query = false;
                if exited.guard_stack.len() > 1 {
                    exited.guard_stack.pop();
                }
                successors.push((next, exited, EdgeKind::GuardExit));
            }
            VmToken::Jump { target, .. } => {
                successors.push((usize::from(*target), state.clone(), EdgeKind::Jump));
            }
            VmToken::Text {
                flags_b4,
                flags_b5,
                loop_target,
                ..
            } => {
                successors.push((next, state.clone(), EdgeKind::TextContinue));
                if let Some(skip) = vm::text_conditional_skip_count(*flags_b4, *flags_b5) {
                    let skip_target = tokens
                        .get(index + 1 + usize::from(skip))
                        .map(VmToken::offset)
                        .unwrap_or(end_marker);
                    if skip_target != next {
                        successors.push((skip_target, state.clone(), EdgeKind::TextSkip));
                    }
                }
                let resume = loop_target.map(usize::from).unwrap_or(next);
                successors.push((resume, state.clone(), EdgeKind::FrameResume));
            }
            _ if token_may_branch(token, state.query) => {
                if let Some(target) = state.guard_stack.last().copied() {
                    successors.push((next, state.clone(), EdgeKind::GuardPass));
                    let mut failed = state.clone();
                    failed.query = false;
                    failed.guard_stack.pop();
                    successors.push((usize::from(target), failed, EdgeKind::GuardFailure));
                } else {
                    unresolved.insert(offset);
                    successors.push((next, state.clone(), EdgeKind::Fallthrough));
                }
            }
            _ => successors.push((next, state.clone(), EdgeKind::Fallthrough)),
        }

        for (to, next_state, kind) in successors {
            if !offsets.contains(&to) {
                bail!(
                    "{} edge from 0x{offset:04X} reaches non-instruction offset 0x{to:04X}",
                    kind.as_str()
                );
            }
            edges.insert(InstructionEdge {
                from: offset,
                to,
                kind,
            });
            if enqueue_state(&mut states, &mut queue, to, next_state, &offsets)? {
                state_count += 1;
                if state_count > MAX_FLOW_STATES {
                    bail!("control-flow state set exceeds {MAX_FLOW_STATES} entries");
                }
            }
        }
    }

    let (blocks, block_edges) =
        build_blocks(image, &tokens, end_marker, &functions, &states, &edges)?;
    let reachable_block_count = blocks.iter().filter(|block| block.reachable).count();

    Ok(CodControlFlow {
        script: script.to_string(),
        image_bytes: image.len(),
        instruction_count: tokens.len() + 1,
        procedure_count: functions.len(),
        block_count: blocks.len(),
        reachable_block_count,
        edge_count: block_edges.len(),
        poke_instruction_count,
        patched_block_flag_count,
        mutable_block_flag_count,
        unresolved_guard_branches: unresolved.into_iter().collect(),
        blocks,
        edges: block_edges,
    })
}

fn validate_typed_stream(image: &[u8], tokens: &[VmToken]) -> Result<usize> {
    let mut cursor = 0usize;
    for token in tokens {
        if token.offset() != cursor {
            bail!("undecoded COD gap 0x{cursor:04X}..0x{:04X}", token.offset());
        }
        let encoded = vm::encode_token(token)
            .ok_or_else(|| anyhow!("untyped token at COD offset 0x{cursor:04X}"))?;
        let end = cursor + encoded.len();
        if image.get(cursor..end) != Some(encoded.as_slice()) {
            bail!("token at COD offset 0x{cursor:04X} does not re-encode");
        }
        cursor = end;
    }
    if image.get(cursor) != Some(&0xFF) || cursor + 1 != image.len() {
        bail!("COD stream does not end in one decoded 0xFF marker");
    }
    Ok(cursor)
}

fn collect_block_flag_values(
    tokens: &[VmToken],
) -> Result<(BTreeMap<usize, BTreeSet<u8>>, usize, usize)> {
    let mut values = BTreeMap::new();
    let mut flag_addresses = BTreeMap::new();
    for token in tokens {
        if let VmToken::ConditionalBlock { offset, flags, .. } = token {
            values
                .entry(*offset)
                .or_insert_with(BTreeSet::new)
                .insert(*flags);
            flag_addresses.insert(offset + 1, *offset);
        }
    }

    let mut poke_instruction_count = 0usize;
    let mut patched = BTreeSet::new();
    for token in tokens {
        let VmToken::PokeByte { address, value, .. } = token else {
            continue;
        };
        poke_instruction_count += 1;
        let address = usize::from(*address);
        let Some(block) = flag_addresses.get(&address).copied() else {
            bail!("POKE_BYTE target 0x{address:04X} is not an A9 block flag");
        };
        values.entry(block).or_default().insert(*value);
        patched.insert(block);
    }
    Ok((values, poke_instruction_count, patched.len()))
}

fn token_may_branch(token: &VmToken, query: bool) -> bool {
    match token {
        VmToken::ConceptGuard { .. }
        | VmToken::GlobalWordCompare { .. }
        | VmToken::GlobalPairCompare { .. }
        | VmToken::FlagBranch { .. } => true,
        VmToken::StateArray { value, .. } => value.is_none(),
        VmToken::Actor { .. } => true,
        VmToken::RecordEntry { entry_opcode, .. } => query || *entry_opcode != 0xC6,
        VmToken::RecordState { opcode, .. } => query || *opcode == 0xC1,
        VmToken::RecordLink { .. }
        | VmToken::BitFlag { .. }
        | VmToken::SharedState { .. }
        | VmToken::SharedBitState { .. }
        | VmToken::RecordWildcard { .. }
        | VmToken::PairRecord { .. }
        | VmToken::RecordTriple { .. } => query,
        _ => false,
    }
}

fn enqueue_state(
    states: &mut BTreeMap<usize, BTreeSet<FlowState>>,
    queue: &mut VecDeque<(usize, FlowState)>,
    offset: usize,
    state: FlowState,
    boundaries: &BTreeSet<usize>,
) -> Result<bool> {
    if !boundaries.contains(&offset) {
        bail!("attempted to enqueue non-instruction offset 0x{offset:04X}");
    }
    if states.entry(offset).or_default().insert(state.clone()) {
        queue.push_back((offset, state));
        Ok(true)
    } else {
        Ok(false)
    }
}

fn build_blocks(
    image: &[u8],
    tokens: &[VmToken],
    end_marker: usize,
    functions: &[crate::script::ScriptFunction],
    states: &BTreeMap<usize, BTreeSet<FlowState>>,
    instruction_edges: &BTreeSet<InstructionEdge>,
) -> Result<(Vec<BasicBlock>, Vec<BlockEdge>)> {
    let mut instruction_offsets: Vec<usize> = tokens.iter().map(VmToken::offset).collect();
    instruction_offsets.push(end_marker);
    let offset_set: BTreeSet<usize> = instruction_offsets.iter().copied().collect();
    let next_offsets: BTreeMap<usize, usize> = instruction_offsets
        .windows(2)
        .map(|pair| (pair[0], pair[1]))
        .collect();

    let mut leaders = BTreeSet::from([0usize, end_marker]);
    for function in functions {
        if offset_set.contains(&function.offset) {
            leaders.insert(function.offset);
        }
    }

    let mut immediate_outgoing: BTreeMap<usize, Vec<&InstructionEdge>> = BTreeMap::new();
    for edge in instruction_edges
        .iter()
        .filter(|edge| edge.kind.is_immediate())
    {
        immediate_outgoing.entry(edge.from).or_default().push(edge);
        if edge.to != *next_offsets.get(&edge.from).unwrap_or(&end_marker) {
            leaders.insert(edge.to);
        }
    }
    for (&source, outgoing) in &immediate_outgoing {
        let next = *next_offsets.get(&source).unwrap_or(&end_marker);
        let transfers = outgoing.len() > 1
            || outgoing
                .iter()
                .any(|edge| edge.to != next || edge.kind == EdgeKind::Jump);
        if transfers {
            leaders.insert(next);
        }
    }
    for edge in instruction_edges
        .iter()
        .filter(|edge| edge.kind == EdgeKind::FrameResume)
    {
        leaders.insert(edge.to);
    }

    let leader_list: Vec<usize> = leaders.into_iter().collect();
    let mut blocks = Vec::new();
    for (index, &start) in leader_list.iter().enumerate() {
        let end_exclusive = leader_list.get(index + 1).copied().unwrap_or(image.len());
        let instruction_count = instruction_offsets
            .iter()
            .filter(|&&offset| offset >= start && offset < end_exclusive)
            .count();
        if instruction_count == 0 {
            continue;
        }
        let procedure = functions
            .iter()
            .rev()
            .find(|function| function.offset <= start)
            .map(|function| format!("{}_{:04X}", function.name, function.offset))
            .unwrap_or_else(|| {
                format!(
                    "{script}_0000",
                    script = functions.first().map_or("script", |f| f.script.as_str())
                )
            });
        let reachable = instruction_offsets
            .iter()
            .filter(|&&offset| offset >= start && offset < end_exclusive)
            .any(|offset| states.contains_key(offset));
        blocks.push(BasicBlock {
            start,
            end_exclusive,
            instruction_count,
            procedure,
            reachable,
        });
    }

    let block_for = |offset: usize| {
        blocks
            .iter()
            .rev()
            .find(|block| block.start <= offset && offset < block.end_exclusive)
            .map(|block| block.start)
    };
    let mut edges = BTreeSet::new();
    for edge in instruction_edges {
        let from_block = block_for(edge.from)
            .ok_or_else(|| anyhow!("no block owns source 0x{:04X}", edge.from))?;
        let to_block =
            block_for(edge.to).ok_or_else(|| anyhow!("no block owns target 0x{:04X}", edge.to))?;
        edges.insert(BlockEdge {
            from_block,
            from_instruction: edge.from,
            to_block,
            kind: edge.kind,
        });
    }
    Ok((blocks, edges.into_iter().collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_guard_pass_failure_and_direct_jump_edges() {
        let image = vec![
            0xA9, 0x01, 0x0B, 0x00, // block, failure -> END
            0xA3, 0x34, 0x12, // concept guard
            0xA1, // leave query mode
            0xA4, 0x0B, 0x00, // jump -> END
            0xFF,
        ];
        let symbols = vec![DebSymbol {
            name: "entry".to_string(),
            offset: 1,
            kind: 2,
        }];
        let graph = analyze_cod("SCRIPTX", &image, &symbols).unwrap();
        assert!(graph.unresolved_guard_branches.is_empty());
        for (from, to, kind) in [
            (0x0004, 0x0007, EdgeKind::GuardPass),
            (0x0004, 0x000B, EdgeKind::GuardFailure),
            (0x0008, 0x000B, EdgeKind::Jump),
        ] {
            assert!(
                graph.edges.iter().any(|edge| {
                    edge.from_instruction == from && edge.to_block == to && edge.kind == kind
                }),
                "missing {kind:?} edge {from:04X}->{to:04X}"
            );
        }
    }

    #[test]
    fn self_modifying_a9_flag_has_enter_and_skip_edges() {
        let image = vec![
            0xA9, 0x01, 0x08, 0x00, // initially enters
            0xAB, 0x00, 0x01, 0x00, // later changes its flag to skip
            0xFF,
        ];
        let graph = analyze_cod("SCRIPTX", &image, &[]).unwrap();
        assert_eq!(graph.poke_instruction_count, 1);
        assert_eq!(graph.mutable_block_flag_count, 1);
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.kind == EdgeKind::BlockEnter)
        );
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.kind == EdgeKind::BlockSkip)
        );
    }

    #[test]
    fn frame_resume_preserves_the_active_guard() {
        let image = vec![
            0xA9, 0x01, 0x11, 0x00, // block, failure -> END
            0xA6, 0x00, 0x00, 0xFF, 0x00, 0x80, 0x00, 0x00, // text/yield
            0xC1, 0x40, 0x00, 0x02, 0x00, // guarded record-state query
            0xFF,
        ];
        let graph = analyze_cod("SCRIPTX", &image, &[]).unwrap();
        assert!(graph.unresolved_guard_branches.is_empty());
        assert!(graph.edges.iter().any(|edge| {
            edge.from_instruction == 0x0004
                && edge.to_block == 0x000C
                && edge.kind == EdgeKind::FrameResume
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from_instruction == 0x000C
                && edge.to_block == 0x0011
                && edge.kind == EdgeKind::GuardFailure
        }));
    }
}
