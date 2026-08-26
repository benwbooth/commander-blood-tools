//! Typed traversal of BloodScript BAS selector-case lists.

use std::collections::BTreeSet;
use std::fmt;

use commander_blood_formats::bas::{ScriptBas, ScriptBasInstruction};
use commander_blood_formats::code::ScriptCodeOffset;
use commander_blood_formats::script::{ScriptDictionary, ScriptWordId};

use super::{ScriptRuntime, ScriptWordHistory};

/// Number of recent concepts retained by the original dialogue runtime.
pub const SCRIPT_CONCEPT_HISTORY_LENGTH: usize = 8;
const HISTORY_INSERTION_STEP: usize = 1;

/// Mutable eight-concept history owned by one loaded script profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptConceptHistory {
    entries: [Option<ScriptWordId>; SCRIPT_CONCEPT_HISTORY_LENGTH],
    next_index: usize,
}

impl Default for ScriptConceptHistory {
    fn default() -> Self {
        Self {
            entries: [None; SCRIPT_CONCEPT_HISTORY_LENGTH],
            next_index: usize::MIN,
        }
    }
}

impl ScriptConceptHistory {
    /// Construct a history ring with an explicit next insertion position.
    pub const fn new(
        entries: [Option<ScriptWordId>; SCRIPT_CONCEPT_HISTORY_LENGTH],
        next_index: usize,
    ) -> Option<Self> {
        if next_index < SCRIPT_CONCEPT_HISTORY_LENGTH {
            Some(Self {
                entries,
                next_index,
            })
        } else {
            None
        }
    }

    /// Return the concepts in physical ring order.
    pub const fn entries(&self) -> &[Option<ScriptWordId>; SCRIPT_CONCEPT_HISTORY_LENGTH] {
        &self.entries
    }

    /// Return the position that receives the next concept.
    pub const fn next_index(&self) -> usize {
        self.next_index
    }

    /// Append one selected concept and advance around the fixed ring.
    pub fn push(&mut self, concept: ScriptWordId) {
        self.entries[self.next_index] = Some(concept);
        self.next_index =
            (self.next_index + HISTORY_INSERTION_STEP) % SCRIPT_CONCEPT_HISTORY_LENGTH;
    }

    /// Produce the immutable history view consumed by text conditions.
    pub fn snapshot(&self) -> ScriptWordHistory {
        ScriptWordHistory::new(self.entries, self.next_index)
            .expect("owned concept history always retains a valid insertion position")
    }
}

/// Active selector concept and the BAS body that implements its response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptSelectorBranch {
    /// Concept that selected this response.
    pub concept: ScriptWordId,
    /// First decoded instruction in the response body.
    pub body: ScriptCodeOffset,
}

/// Owned dialogue-selector state associated with one loaded script profile.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptSelectorState {
    history: ScriptConceptHistory,
    current_control: Option<ScriptWordId>,
    parent_control: Option<ScriptWordId>,
    current_branch: Option<ScriptSelectorBranch>,
    parent_branch: Option<ScriptSelectorBranch>,
    pending_presentation_words: Vec<ScriptWordId>,
}

impl ScriptSelectorState {
    /// Borrow the recent-concept history.
    pub const fn history(&self) -> &ScriptConceptHistory {
        &self.history
    }

    /// Mutably borrow the recent-concept history.
    pub fn history_mut(&mut self) -> &mut ScriptConceptHistory {
        &mut self.history
    }

    /// Return the currently executing selector response.
    pub const fn current_branch(&self) -> Option<ScriptSelectorBranch> {
        self.current_branch
    }

    /// Return the concept currently controlling actor dialogue dispatch.
    pub const fn current_control(&self) -> Option<ScriptWordId> {
        self.current_control
    }

    /// Return the selector response suspended beneath the current branch.
    pub const fn parent_branch(&self) -> Option<ScriptSelectorBranch> {
        self.parent_branch
    }

    /// Return the suspended parent concept, when nested dialogue is active.
    pub const fn parent_control(&self) -> Option<ScriptWordId> {
        self.parent_control
    }

    /// Replace current and parent dialogue controls with interned concepts.
    pub fn set_control_selections(
        &mut self,
        current: Option<ScriptWordId>,
        parent: Option<ScriptWordId>,
    ) {
        self.current_control = current;
        self.parent_control = parent;
    }

    pub(super) fn select_control_branch(
        &mut self,
        concept: ScriptWordId,
        body: Option<ScriptCodeOffset>,
    ) {
        self.current_control = Some(concept);
        if let Some(body) = body {
            self.current_branch = Some(ScriptSelectorBranch { concept, body });
        }
    }

    /// Return words prepared by the current text-presentation pass.
    pub fn pending_presentation_words(&self) -> &[ScriptWordId] {
        &self.pending_presentation_words
    }

    /// Replace words prepared by the current text-presentation pass.
    pub fn replace_presentation_words(&mut self, words: impl IntoIterator<Item = ScriptWordId>) {
        self.pending_presentation_words.clear();
        self.pending_presentation_words.extend(words);
    }
}

/// Observable result of consuming one selected dialogue concept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptSelectionOutcome {
    /// No concept was awaiting selector dispatch.
    NoSelection,
    /// The concept entered history and selector traversal completed.
    Committed {
        /// Consumed concept identity.
        concept: ScriptWordId,
        /// Matched response body, including bodies that are not menus.
        matched_body: Option<ScriptCodeOffset>,
        /// Whether the matched body became the active menu branch.
        menu_activated: bool,
    },
}

/// Invalid typed data encountered while committing a selected concept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptSelectionError {
    /// A resumed concept has no encoding in the active profile dictionary.
    MissingDictionaryEncoding {
        /// Concept that belongs to a different or malformed dictionary.
        concept: ScriptWordId,
    },
    /// A matched selector node has no decoded response instruction.
    MissingBody {
        /// Expected response-body position.
        source_offset: ScriptCodeOffset,
    },
    /// Selector linkage failed typed validation.
    Selector(ScriptSelectorError),
}

impl fmt::Display for ScriptSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ScriptSelectionError {}

impl From<ScriptSelectorError> for ScriptSelectionError {
    fn from(source: ScriptSelectorError) -> Self {
        Self::Selector(source)
    }
}

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

/// Commit a selected concept to history and enter its authored menu response.
///
/// This translates `vm_flag_test_67b1` at BLOODPRG file offset `0x005791`.
/// The original's mutable pointers and packed branch globals become owned
/// concept history and typed BAS positions in a flat runtime model.
pub fn commit_selected_concept(
    runtime: &mut ScriptRuntime,
    dictionary: &ScriptDictionary,
    dialogue: &ScriptBas,
    selector_root: Option<ScriptCodeOffset>,
    state: &mut ScriptSelectorState,
) -> Result<ScriptSelectionOutcome, ScriptSelectionError> {
    let Some(concept) = runtime.selected_concept() else {
        return Ok(ScriptSelectionOutcome::NoSelection);
    };

    let encoded_resume_value = runtime
        .selector_resume_active()
        .then(|| {
            dictionary
                .source_offset(concept)
                .ok_or(ScriptSelectionError::MissingDictionaryEncoding { concept })
        })
        .transpose()?;
    let matched_body = selector_root
        .map(|root| find_selector_body(dialogue, root, concept))
        .transpose()?
        .flatten();
    let menu_activated = if let Some(body) = matched_body {
        let instruction = dialogue
            .tokens()
            .iter()
            .find(|token| token.source_offset() == body)
            .ok_or(ScriptSelectionError::MissingBody {
                source_offset: body,
            })?
            .instruction();
        matches!(instruction, ScriptBasInstruction::Menu(_))
    } else {
        false
    };

    runtime.take_selected_concept();
    if let Some(encoded_resume_value) = encoded_resume_value {
        let saved = runtime.save_resume_concept(concept, encoded_resume_value);
        debug_assert!(saved, "selector-active resume remains armed during commit");
    }
    state.pending_presentation_words.clear();
    state.history.push(concept);
    if menu_activated {
        state.parent_control = state
            .current_control
            .or(state.current_branch.map(|branch| branch.concept));
        state.current_control = Some(concept);
        state.parent_branch = state.current_branch;
        state.current_branch = matched_body.map(|body| ScriptSelectorBranch { concept, body });
    }

    Ok(ScriptSelectionOutcome::Committed {
        concept,
        matched_body,
        menu_activated,
    })
}

/// Publish the active BAS menu and append one pending offered topic.
///
/// This translates `vm_op_a3_collect` at BLOODPRG file offset `0x005AFD`.
/// A non-menu branch leaves both the existing choice list and offered topic
/// unchanged. Typed vectors replace the native terminated output buffer.
pub fn collect_selector_menu(
    dialogue: &ScriptBas,
    state: &mut ScriptSelectorState,
    offered_topic: &mut Option<ScriptWordId>,
) -> Result<bool, ScriptSelectionError> {
    let Some(branch) = state.current_branch else {
        return Ok(false);
    };
    let instruction = dialogue
        .tokens()
        .iter()
        .find(|token| token.source_offset() == branch.body)
        .ok_or(ScriptSelectionError::MissingBody {
            source_offset: branch.body,
        })?
        .instruction();
    let ScriptBasInstruction::Menu(words) = instruction else {
        return Ok(false);
    };

    state.pending_presentation_words.clear();
    state
        .pending_presentation_words
        .extend(words.iter().copied());
    if let Some(topic) = offered_topic.take() {
        state.pending_presentation_words.push(topic);
    }
    Ok(true)
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
    const SELECTION_ORACLE_VECTOR_COUNT: usize = 14;
    const MENU_COLLECTION_ORACLE_VECTOR_COUNT: usize = 10;
    const EXPECTED_UNALIGNED_NATIVE_RING_VECTOR_COUNT: usize = 8;
    const EXPECTED_IGNORED_MENU_COLLECTION_VECTORS: usize = 3;
    const EXPECTED_EMPTY_MENU_COLLECTION_VECTORS: usize = 2;
    const EXPECTED_MALFORMED_MENU_COLLECTION_VECTORS: usize = 3;
    const EXPECTED_VALID_MENU_COLLECTION_VECTORS: usize = 2;
    const MENU_COLLECTION_VECTOR_KIND_COUNT: usize = 4;
    const SERIALIZED_WORD_SIZE: usize = 2;
    const NATIVE_HISTORY_WORD_SIZE: u8 = 2;
    const NATIVE_SELECTOR_RESUME_BIT: u8 = 2;
    const SELECTOR_YIELD_OPCODE: u8 = 0xAC;
    const MENU_OPCODE: u8 = 0xA3;
    const BAS_END_MARKER: u8 = 0xFF;
    const MENU_WORD_SOURCE_OFFSET: u16 = 2;
    const FIRST_SELECTOR_SOURCE_OFFSET: u16 = 0x1111;
    const SECOND_SELECTOR_SOURCE_OFFSET: u16 = 0x2222;
    const HIGH_SELECTOR_SOURCE_OFFSET: u16 = 0xF123;
    const RESUME_TARGET: usize = 99;
    const INITIAL_BRANCH_BODY: usize = 7;

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

    #[derive(Deserialize)]
    struct SelectionOracle {
        name: String,
        pending_value: u16,
        resume_state: u8,
        resume_after: u16,
        ring_before: u8,
        ring_after: u8,
        pc_saved: u16,
        matched_opcode: u8,
        branch_taken: bool,
    }

    #[derive(Deserialize)]
    struct MenuCollectionOracle {
        name: String,
        opcode: u8,
        source_word_offsets: Vec<u16>,
        source_words: Vec<u16>,
        deferred_before: u16,
        deferred_after: u16,
        written_words: Vec<(u16, u16)>,
        direction: String,
    }

    #[repr(usize)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum MenuCollectionVectorKind {
        IgnoredOpcode,
        EmptyMenu,
        MalformedTraversal,
        ValidMenu,
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

    fn selection_dictionary() -> ScriptDictionary {
        const PREFIXED_MENU_WORDS: &[u8] = b"x\0ok\0";

        let mut bytes =
            vec![u8::MIN; usize::from(HIGH_SELECTOR_SOURCE_OFFSET) + HISTORY_INSERTION_STEP];
        bytes[..PREFIXED_MENU_WORDS.len()].copy_from_slice(PREFIXED_MENU_WORDS);
        decode_script_dictionary(&bytes).unwrap()
    }

    fn append_body(bytes: &mut Vec<u8>, menu: bool) -> ScriptCodeOffset {
        let body = ScriptCodeOffset::new(bytes.len());
        if menu {
            bytes.push(MENU_OPCODE);
            bytes.extend_from_slice(&MENU_WORD_SOURCE_OFFSET.to_le_bytes());
            bytes.extend_from_slice(&u16::MIN.to_le_bytes());
        } else {
            bytes.push(BAS_END_MARKER);
        }
        body
    }

    fn menu_collection_vector_kind(vector: &MenuCollectionOracle) -> MenuCollectionVectorKind {
        if vector.opcode != MENU_OPCODE {
            return MenuCollectionVectorKind::IgnoredOpcode;
        }
        if vector.source_words.is_empty() {
            return MenuCollectionVectorKind::EmptyMenu;
        }
        let source_wraps = vector
            .source_word_offsets
            .windows(2)
            .any(|positions| positions[1] < positions[0]);
        let impossible_dictionary_word = vector.source_words.contains(&u16::MAX);
        if vector.direction != "forward" || source_wraps || impossible_dictionary_word {
            MenuCollectionVectorKind::MalformedTraversal
        } else {
            MenuCollectionVectorKind::ValidMenu
        }
    }

    fn dictionary_with_words_at(offsets: &[u16]) -> ScriptDictionary {
        const TEST_WORD_LENGTH: usize = 2;
        const TERMINATOR_SIZE: usize = 1;

        let last_offset = offsets.iter().copied().max().unwrap();
        let mut bytes =
            vec![u8::MIN; usize::from(last_offset) + TEST_WORD_LENGTH + TERMINATOR_SIZE];
        for (index, offset) in offsets.iter().copied().enumerate() {
            let start = usize::from(offset);
            let label_byte = b'a' + u8::try_from(index).unwrap();
            bytes[start..start + TEST_WORD_LENGTH].fill(label_byte);
        }
        decode_script_dictionary(&bytes).unwrap()
    }

    fn valid_menu_from_oracle(
        vector: &MenuCollectionOracle,
    ) -> (ScriptDictionary, ScriptBas, Vec<ScriptWordId>) {
        let mut dictionary_offsets = vector.source_words.clone();
        if vector.deferred_before != u16::MIN {
            dictionary_offsets.push(vector.deferred_before);
        }
        let dictionary = dictionary_with_words_at(&dictionary_offsets);
        let mut bytes = vec![MENU_OPCODE];
        for source_word in &vector.source_words {
            bytes.extend_from_slice(&source_word.to_le_bytes());
        }
        bytes.extend_from_slice(&u16::MIN.to_le_bytes());
        let dialogue = decode_script_bas(&bytes, &dictionary).unwrap();
        let words = vector
            .source_words
            .iter()
            .map(|source_word| dictionary.resolve_source_offset(*source_word).unwrap())
            .collect();
        (dictionary, dialogue, words)
    }

    fn selection_dialogue(
        dictionary: &ScriptDictionary,
        first_selector: u16,
        first_menu: bool,
        second_menu: bool,
    ) -> (
        ScriptBas,
        ScriptCodeOffset,
        ScriptCodeOffset,
        ScriptCodeOffset,
    ) {
        let mut bytes = vec![SELECTOR_YIELD_OPCODE];
        let root = ScriptCodeOffset::new(bytes.len());
        bytes.extend_from_slice(&first_selector.to_le_bytes());
        let first_next_position = bytes.len();
        bytes.extend_from_slice(&u16::MIN.to_le_bytes());
        let first_body = append_body(&mut bytes, first_menu);

        bytes.push(SELECTOR_YIELD_OPCODE);
        let second_node = u16::try_from(bytes.len()).unwrap();
        bytes[first_next_position..first_next_position + SERIALIZED_WORD_SIZE]
            .copy_from_slice(&second_node.to_le_bytes());
        bytes.extend_from_slice(&SECOND_SELECTOR_SOURCE_OFFSET.to_le_bytes());
        bytes.extend_from_slice(&u16::MIN.to_le_bytes());
        let second_body = append_body(&mut bytes, second_menu);

        (
            decode_script_bas(&bytes, dictionary).unwrap(),
            root,
            first_body,
            second_body,
        )
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

    #[test]
    fn selected_concept_commit_accounts_for_every_original_natural_vector() {
        let vectors: Vec<SelectionOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_5791_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), SELECTION_ORACLE_VECTOR_COUNT);
        let dictionary = selection_dictionary();
        let menu_word = dictionary
            .resolve_source_offset(MENU_WORD_SOURCE_OFFSET)
            .unwrap();
        let mut unaligned_native_ring_vectors = usize::MIN;

        for vector in vectors {
            let selected = (vector.pending_value != u16::MIN).then(|| {
                dictionary
                    .resolve_source_offset(vector.pending_value)
                    .unwrap()
            });
            let first_selector = if vector.pending_value == HIGH_SELECTOR_SOURCE_OFFSET {
                HIGH_SELECTOR_SOURCE_OFFSET
            } else {
                FIRST_SELECTOR_SOURCE_OFFSET
            };
            let matches_first = matches!(
                vector.pending_value,
                FIRST_SELECTOR_SOURCE_OFFSET | HIGH_SELECTOR_SOURCE_OFFSET
            );
            let matches_second = vector.pending_value == SECOND_SELECTOR_SOURCE_OFFSET;
            assert_eq!(
                vector.branch_taken,
                vector.matched_opcode == MENU_OPCODE,
                "{}",
                vector.name
            );
            let first_menu = matches_first && vector.branch_taken;
            let second_menu = matches_second && vector.branch_taken;
            let (dialogue, root, first_body, second_body) =
                selection_dialogue(&dictionary, first_selector, first_menu, second_menu);
            let selector_root = (vector.pc_saved != u16::MIN).then_some(root);

            let mut runtime = ScriptRuntime::new();
            runtime.set_selected_concept(selected);
            if vector.resume_state != u8::MIN {
                runtime.arm_resume(ScriptCodeOffset::new(RESUME_TARGET), vector.resume_after);
            }
            if vector.resume_state & NATIVE_SELECTOR_RESUME_BIT != u8::MIN {
                assert!(runtime.activate_selector_resume());
            }

            // Odd native indices intentionally exercise an unaligned word write;
            // the flat model retains the corresponding valid concept slot.
            let insertion_index = usize::from(vector.ring_before / NATIVE_HISTORY_WORD_SIZE);
            let initial_branch = ScriptSelectorBranch {
                concept: menu_word,
                body: ScriptCodeOffset::new(INITIAL_BRANCH_BODY),
            };
            let mut state = ScriptSelectorState {
                history: ScriptConceptHistory::new(
                    [None; SCRIPT_CONCEPT_HISTORY_LENGTH],
                    insertion_index,
                )
                .unwrap(),
                current_control: Some(menu_word),
                parent_control: None,
                current_branch: Some(initial_branch),
                parent_branch: None,
                pending_presentation_words: vec![menu_word],
            };
            let outcome = commit_selected_concept(
                &mut runtime,
                &dictionary,
                &dialogue,
                selector_root,
                &mut state,
            )
            .unwrap();

            let Some(selected) = selected else {
                assert_eq!(outcome, ScriptSelectionOutcome::NoSelection);
                assert_eq!(state.history.next_index(), insertion_index);
                assert_eq!(state.pending_presentation_words(), &[menu_word]);
                assert_eq!(state.current_branch(), Some(initial_branch));
                continue;
            };

            let expected_body = if vector.matched_opcode == u8::MIN {
                None
            } else if matches_first {
                Some(first_body)
            } else if matches_second {
                Some(second_body)
            } else {
                panic!("{} reports an unmatched payload", vector.name);
            };
            assert_eq!(
                outcome,
                ScriptSelectionOutcome::Committed {
                    concept: selected,
                    matched_body: expected_body,
                    menu_activated: vector.branch_taken,
                },
                "{}",
                vector.name
            );
            assert_eq!(state.history.entries()[insertion_index], Some(selected));
            assert_eq!(
                state.history.next_index(),
                (insertion_index + HISTORY_INSERTION_STEP) % SCRIPT_CONCEPT_HISTORY_LENGTH
            );
            assert_eq!(
                state.history.next_index(),
                usize::from(vector.ring_after / NATIVE_HISTORY_WORD_SIZE),
                "{}",
                vector.name
            );
            if vector.ring_before % NATIVE_HISTORY_WORD_SIZE != u8::MIN {
                unaligned_native_ring_vectors += 1;
                assert_ne!(vector.ring_after % NATIVE_HISTORY_WORD_SIZE, u8::MIN);
            }
            assert!(state.pending_presentation_words().is_empty());
            assert_eq!(runtime.selected_concept(), None);

            if vector.branch_taken {
                assert_eq!(state.parent_branch(), Some(initial_branch));
                assert_eq!(
                    state.current_branch(),
                    Some(ScriptSelectorBranch {
                        concept: selected,
                        body: expected_body.unwrap(),
                    })
                );
            } else {
                assert_eq!(state.parent_branch(), None);
                assert_eq!(state.current_branch(), Some(initial_branch));
            }

            if vector.resume_state & NATIVE_SELECTOR_RESUME_BIT != u8::MIN {
                assert_eq!(runtime.alternate_concept(), Some(selected));
                assert_eq!(runtime.resume_state().unwrap().value, vector.pending_value);
                assert!(runtime.selector_resume_active());
            } else {
                assert_eq!(runtime.alternate_concept(), None);
            }
        }

        assert_eq!(
            unaligned_native_ring_vectors,
            EXPECTED_UNALIGNED_NATIVE_RING_VECTOR_COUNT
        );
    }

    #[test]
    fn menu_collection_accounts_for_every_original_natural_vector() {
        let vectors: Vec<MenuCollectionOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_5afd_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), MENU_COLLECTION_ORACLE_VECTOR_COUNT);
        let mut kind_counts = [usize::MIN; MENU_COLLECTION_VECTOR_KIND_COUNT];

        for vector in vectors {
            let kind = menu_collection_vector_kind(&vector);
            kind_counts[kind as usize] += HISTORY_INSERTION_STEP;
            if kind != MenuCollectionVectorKind::ValidMenu {
                continue;
            }

            let (dictionary, dialogue, menu_words) = valid_menu_from_oracle(&vector);
            let concept = menu_words[usize::MIN];
            let mut state = ScriptSelectorState {
                current_branch: Some(ScriptSelectorBranch {
                    concept,
                    body: ScriptCodeOffset::new(usize::MIN),
                }),
                pending_presentation_words: vec![concept],
                ..ScriptSelectorState::default()
            };
            let mut offered_topic = (vector.deferred_before != u16::MIN).then(|| {
                dictionary
                    .resolve_source_offset(vector.deferred_before)
                    .unwrap()
            });

            assert!(collect_selector_menu(&dialogue, &mut state, &mut offered_topic).unwrap());
            assert_eq!(
                offered_topic,
                (vector.deferred_after != u16::MIN).then(|| {
                    dictionary
                        .resolve_source_offset(vector.deferred_after)
                        .unwrap()
                }),
                "{}",
                vector.name
            );
            let encoded_words = state
                .pending_presentation_words()
                .iter()
                .map(|word| dictionary.source_offset(*word).unwrap())
                .collect::<Vec<_>>();
            let mut expected_words = vector
                .written_words
                .iter()
                .map(|(_destination, value)| *value)
                .collect::<Vec<_>>();
            assert_eq!(expected_words.pop(), Some(u16::MIN));
            assert_eq!(encoded_words, expected_words, "{}", vector.name);
        }

        assert_eq!(
            kind_counts[MenuCollectionVectorKind::IgnoredOpcode as usize],
            EXPECTED_IGNORED_MENU_COLLECTION_VECTORS
        );
        assert_eq!(
            kind_counts[MenuCollectionVectorKind::EmptyMenu as usize],
            EXPECTED_EMPTY_MENU_COLLECTION_VECTORS
        );
        assert_eq!(
            kind_counts[MenuCollectionVectorKind::MalformedTraversal as usize],
            EXPECTED_MALFORMED_MENU_COLLECTION_VECTORS
        );
        assert_eq!(
            kind_counts[MenuCollectionVectorKind::ValidMenu as usize],
            EXPECTED_VALID_MENU_COLLECTION_VECTORS
        );
    }

    #[test]
    fn every_shipped_selector_menu_collects_interned_words_and_an_offered_topic() {
        let root = workspace_root();
        let mut total_menus = usize::MIN;

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

            for node in &graph.nodes {
                let body = ScriptCodeOffset::new(node.body_start);
                let token = dialogue
                    .tokens()
                    .iter()
                    .find(|token| token.source_offset() == body)
                    .unwrap();
                let ScriptBasInstruction::Menu(expected_words) = token.instruction() else {
                    panic!("shipped selector body is not a menu");
                };
                let offered = expected_words[usize::MIN];
                let mut offered_topic = Some(offered);
                let mut state = ScriptSelectorState {
                    current_branch: Some(ScriptSelectorBranch {
                        concept: offered,
                        body,
                    }),
                    ..ScriptSelectorState::default()
                };

                assert!(collect_selector_menu(&dialogue, &mut state, &mut offered_topic).unwrap());
                let mut expected = expected_words.to_vec();
                expected.push(offered);
                assert_eq!(state.pending_presentation_words(), expected);
                assert_eq!(offered_topic, None);
                total_menus += HISTORY_INSERTION_STEP;
            }
        }

        assert_eq!(total_menus, EXPECTED_TOTAL_SELECTOR_NODE_COUNT);
    }

    #[test]
    fn non_menu_branches_preserve_existing_choices_and_offered_topic() {
        let dictionary = decode_script_dictionary(b"alpha\0").unwrap();
        let concept = dictionary.resolve_source_offset(u16::MIN).unwrap();
        let dialogue = decode_script_bas(&[BAS_END_MARKER], &dictionary).unwrap();
        let mut state = ScriptSelectorState {
            current_branch: Some(ScriptSelectorBranch {
                concept,
                body: ScriptCodeOffset::new(usize::MIN),
            }),
            pending_presentation_words: vec![concept],
            ..ScriptSelectorState::default()
        };
        let mut offered_topic = Some(concept);

        assert!(!collect_selector_menu(&dialogue, &mut state, &mut offered_topic).unwrap());
        assert_eq!(state.pending_presentation_words(), &[concept]);
        assert_eq!(offered_topic, Some(concept));
    }

    #[test]
    fn selection_commit_rejects_cross_dictionary_resume_without_partial_updates() {
        let dictionary = decode_script_dictionary(b"ok\0").unwrap();
        let foreign = decode_script_dictionary(b"foreign\0word\0").unwrap();
        let foreign_word_offset = u16::try_from(b"foreign\0".len()).unwrap();
        let concept = foreign.resolve_source_offset(foreign_word_offset).unwrap();
        let dialogue = decode_script_bas(&[BAS_END_MARKER], &dictionary).unwrap();
        let mut runtime = ScriptRuntime::new();
        runtime.set_selected_concept(Some(concept));
        runtime.arm_resume(ScriptCodeOffset::new(RESUME_TARGET), u16::MIN);
        runtime.activate_selector_resume();
        let mut state = ScriptSelectorState::default();

        assert_eq!(
            commit_selected_concept(&mut runtime, &dictionary, &dialogue, None, &mut state)
                .unwrap_err(),
            ScriptSelectionError::MissingDictionaryEncoding { concept }
        );
        assert_eq!(runtime.selected_concept(), Some(concept));
        assert_eq!(state, ScriptSelectorState::default());
    }

    #[test]
    fn selection_commit_rejects_a_selector_without_a_response_body() {
        let dictionary = decode_script_dictionary(b"alpha\0").unwrap();
        let concept = dictionary.resolve_source_offset(u16::MIN).unwrap();
        let mut bytes = vec![SELECTOR_YIELD_OPCODE];
        let root = ScriptCodeOffset::new(bytes.len());
        bytes.extend_from_slice(&u16::MIN.to_le_bytes());
        bytes.extend_from_slice(&u16::MIN.to_le_bytes());
        let dialogue = decode_script_bas(&bytes, &dictionary).unwrap();
        let missing_body = ScriptCodeOffset::new(bytes.len());
        let mut runtime = ScriptRuntime::new();
        runtime.set_selected_concept(Some(concept));
        let mut state = ScriptSelectorState::default();

        assert_eq!(
            commit_selected_concept(&mut runtime, &dictionary, &dialogue, Some(root), &mut state,)
                .unwrap_err(),
            ScriptSelectionError::MissingBody {
                source_offset: missing_body,
            }
        );
        assert_eq!(runtime.selected_concept(), Some(concept));
        assert_eq!(state, ScriptSelectorState::default());
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
