//! Owned dialogue resources, decoded only when a dialogue path needs them.

use std::fmt;
use std::sync::OnceLock;

use commander_blood_formats::bas::{ScriptBas, ScriptBasError, decode_script_bas};
use commander_blood_formats::script::ScriptDictionary;

use super::ResourceId;

/// A dialogue consumer must validate its source before traversing BAS structures.
/// Ordinary Commander fixtures can supply an already decoded program directly.
pub trait ScriptDialogueSource: fmt::Debug {
    /// Decode the complete resource or retain its precise format error.
    fn decoded(&self) -> Result<&ScriptBas, ScriptBasError>;
}

impl ScriptDialogueSource for ScriptBas {
    fn decoded(&self) -> Result<&ScriptBas, ScriptBasError> {
        Ok(self)
    }
}

/// The actual resource bound to the native dialogue slot, not necessarily BAS.
///
/// BLOOD2PG's missing-resource resolver retains the preceding COD binding.
/// Keeping those bytes and their identity avoids inventing an empty BAS image.
#[derive(Clone, Debug)]
pub struct ScriptProfileDialogue {
    resource: ResourceId,
    bytes: Box<[u8]>,
    dictionary: ScriptDictionary,
    decoded: OnceLock<Result<ScriptBas, ScriptBasError>>,
}

impl PartialEq for ScriptProfileDialogue {
    fn eq(&self, other: &Self) -> bool {
        self.resource == other.resource
            && self.bytes == other.bytes
            && self.dictionary == other.dictionary
    }
}

impl Eq for ScriptProfileDialogue {}

impl ScriptProfileDialogue {
    pub(super) fn new(resource: ResourceId, bytes: &[u8], dictionary: &ScriptDictionary) -> Self {
        Self {
            resource,
            bytes: bytes.into(),
            dictionary: dictionary.clone(),
            decoded: OnceLock::new(),
        }
    }

    /// Identity of the resident resource actually supplying these bytes.
    pub const fn resource(&self) -> ResourceId {
        self.resource
    }

    /// Exact source bytes, including undecoded or currently inactive programs.
    pub fn encoded_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Decode on first use, caching either the complete program or its error.
    pub fn decoded(&self) -> Result<&ScriptBas, ScriptBasError> {
        self.decoded
            .get_or_init(|| decode_script_bas(&self.bytes, &self.dictionary))
            .as_ref()
            .map_err(Clone::clone)
    }
}

impl ScriptDialogueSource for ScriptProfileDialogue {
    fn decoded(&self) -> Result<&ScriptBas, ScriptBasError> {
        Self::decoded(self)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        ScriptRuntime, ScriptSelectionOutcome, ScriptSelectorError, ScriptSelectorState,
        collect_selector_menu, commit_selected_concept, find_selector_body,
    };
    use super::*;
    use commander_blood_formats::code::ScriptCodeOffset;
    use commander_blood_formats::script::decode_script_dictionary;

    #[test]
    fn sequel_unused_dialogue_is_retained_but_entered_invalid_dialogue_errors() {
        let dictionary = decode_script_dictionary(b"choice\0").unwrap();
        let resource = ResourceId::new(5);
        let dialogue = ScriptProfileDialogue::new(resource, &[159], &dictionary);
        let before = dialogue.clone();
        let mut runtime = ScriptRuntime::new();
        let mut selector = ScriptSelectorState::default();
        assert_eq!(
            commit_selected_concept(&mut runtime, &dictionary, &dialogue, None, &mut selector)
                .unwrap(),
            ScriptSelectionOutcome::NoSelection
        );
        assert!(!collect_selector_menu(&dialogue, &mut selector, &mut None).unwrap());
        assert!(dialogue.decoded.get().is_none());
        let word = dictionary.resolve_source_offset(0).unwrap();
        assert!(matches!(
            find_selector_body(&dialogue, ScriptCodeOffset::new(0), word),
            Err(ScriptSelectorError::Dialogue(
                ScriptBasError::UnsupportedByte { byte: 159, .. }
            ))
        ));
        assert!(dialogue.decoded.get().is_some());
        assert_eq!(
            dialogue, before,
            "decoding must not change resource identity or bytes"
        );
        assert_eq!(dialogue.resource(), resource);
        assert_eq!(dialogue.encoded_bytes(), &[159]);
        assert_eq!(dialogue.decoded(), dialogue.decoded());
    }
}
