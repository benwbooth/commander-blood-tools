//! Typed structures for the DESCRIPT scene and dialogue database.

const FIRST_BACKGROUND_SLOT: u8 = 1;
const BACKGROUND_SLOT_COUNT: u8 = 4;
const LAST_BACKGROUND_SLOT: u8 = FIRST_BACKGROUND_SLOT + BACKGROUND_SLOT_COUNT - 1;
const PRINTABLE_NAME_START: u8 = 32;
const PRINTABLE_NAME_END: u8 = 127;

/// Semantic kind byte stored immediately before each DESCRIPT record length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DescriptRecordKind {
    /// Planet or local-place presentation record.
    Location = 1,
    /// Character conversation record.
    Character = 2,
    /// Standalone video sequence record.
    Sequence = 4,
    /// Inventory or world-object presentation record.
    Object = 15,
}

impl DescriptRecordKind {
    /// Decode one shipped record-kind byte.
    pub const fn decode(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Location),
            2 => Some(Self::Character),
            4 => Some(Self::Sequence),
            15 => Some(Self::Object),
            _ => None,
        }
    }

    /// Return the exact serialized kind byte.
    pub const fn encode(self) -> u8 {
        self as u8
    }
}

/// One of the four background-image cache slots encoded as values one through four.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DescriptBackgroundSlot(u8);

impl DescriptBackgroundSlot {
    /// Number of background images carried by a DESCRIPT location record.
    pub const COUNT: usize = BACKGROUND_SLOT_COUNT as usize;

    /// Decode the one-based slot number used by the shipped database.
    pub const fn decode(encoded: u8) -> Option<Self> {
        if encoded >= FIRST_BACKGROUND_SLOT && encoded <= LAST_BACKGROUND_SLOT {
            Some(Self(encoded - FIRST_BACKGROUND_SLOT))
        } else {
            None
        }
    }

    /// Return this slot's zero-based index in owned Rust collections.
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// Return the one-based value serialized in DESCRIPT.DES.
    pub const fn encode(self) -> u8 {
        self.0 + FIRST_BACKGROUND_SLOT
    }
}

/// One decoded opcode-03 request to cache a background LBM image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescriptBackgroundCommand {
    slot: DescriptBackgroundSlot,
    source_name: Box<[u8]>,
}

impl DescriptBackgroundCommand {
    /// Build a command from a validated slot and an owned resource name.
    pub fn new(slot: DescriptBackgroundSlot, source_name: Box<[u8]>) -> Self {
        Self { slot, source_name }
    }

    /// Return the background cache slot selected by this command.
    pub const fn slot(&self) -> DescriptBackgroundSlot {
        self.slot
    }

    /// Return the case-preserving LBM resource name.
    pub fn source_name(&self) -> &[u8] {
        &self.source_name
    }
}

/// Failure while decoding the payload following a DESCRIPT opcode-03 byte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DescriptBackgroundError {
    /// The command ended before its one-based cache slot.
    MissingSlot,
    /// The slot is outside the four-entry domain used by every shipped record.
    InvalidSlot(u8),
    /// The printable resource name reaches the end of the record without a stop byte.
    MissingNameTerminator,
}

/// Decode an opcode-03 payload while leaving its stop byte for the command dispatcher.
pub fn decode_background_command(
    payload: &[u8],
) -> Result<(DescriptBackgroundCommand, &[u8]), DescriptBackgroundError> {
    let (&encoded_slot, name_and_tail) = payload
        .split_first()
        .ok_or(DescriptBackgroundError::MissingSlot)?;
    let slot = DescriptBackgroundSlot::decode(encoded_slot)
        .ok_or(DescriptBackgroundError::InvalidSlot(encoded_slot))?;
    let name_length = name_and_tail
        .iter()
        .position(|byte| !(*byte >= PRINTABLE_NAME_START && *byte <= PRINTABLE_NAME_END))
        .ok_or(DescriptBackgroundError::MissingNameTerminator)?;
    let source_name = Box::from(&name_and_tail[..name_length]);

    Ok((
        DescriptBackgroundCommand::new(slot, source_name),
        &name_and_tail[name_length..],
    ))
}

/// One decoded opcode-05 caption shown through the subtitle reveal system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescriptCaptionCommand {
    text: Box<[u8]>,
}

impl DescriptCaptionCommand {
    /// Build a caption from owned game-font bytes without a trailing zero.
    pub fn new(text: Box<[u8]>) -> Self {
        Self { text }
    }

    /// Return the caption bytes exactly as authored in DESCRIPT.DES.
    pub fn text(&self) -> &[u8] {
        &self.text
    }
}

/// Failure while decoding the payload following a DESCRIPT opcode-05 byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptCaptionError {
    /// The record ended before the caption's terminating zero byte.
    MissingTerminator,
}

/// Decode a zero-terminated caption and return the unconsumed following bytes.
pub fn decode_caption_command(
    payload: &[u8],
) -> Result<(DescriptCaptionCommand, &[u8]), DescriptCaptionError> {
    let text_length = payload
        .iter()
        .position(|byte| *byte == u8::MIN)
        .ok_or(DescriptCaptionError::MissingTerminator)?;
    Ok((
        DescriptCaptionCommand::new(Box::from(&payload[..text_length])),
        &payload[text_length + 1..],
    ))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    const DIRECTORY_COUNT_SIZE: usize = 2;
    const DIRECTORY_ENTRY_SIZE: usize = 18;
    const DIRECTORY_NAME_SIZE: usize = 16;
    const EXPECTED_RECORD_COUNTS: [(DescriptRecordKind, usize); 4] = [
        (DescriptRecordKind::Location, 64),
        (DescriptRecordKind::Character, 35),
        (DescriptRecordKind::Sequence, 11),
        (DescriptRecordKind::Object, 35),
    ];

    fn original_asset() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood/DESCRIPT.DES")
    }

    #[test]
    fn every_shipped_directory_offset_has_a_known_preceding_kind() {
        let data = std::fs::read(original_asset()).unwrap();
        let count = usize::from(u16::from_le_bytes(data[..2].try_into().unwrap()));
        let mut counts = EXPECTED_RECORD_COUNTS.map(|(kind, _count)| (kind, 0));

        for index in 0..count {
            let entry = DIRECTORY_COUNT_SIZE + index * DIRECTORY_ENTRY_SIZE;
            let offset = usize::from(u16::from_le_bytes(
                data[entry + DIRECTORY_NAME_SIZE..entry + DIRECTORY_ENTRY_SIZE]
                    .try_into()
                    .unwrap(),
            ));
            let kind = DescriptRecordKind::decode(data[offset - 1]).unwrap();
            counts
                .iter_mut()
                .find(|(candidate, _count)| *candidate == kind)
                .unwrap()
                .1 += 1;
        }

        assert_eq!(counts, EXPECTED_RECORD_COUNTS);
    }

    #[test]
    fn background_payload_keeps_the_following_opcode_unconsumed() {
        const SLOT: u8 = 3;
        const NEXT_OPCODE: u8 = 15;

        let mut payload = vec![SLOT];
        payload.extend_from_slice(b"pterra1g.lbm");
        payload.push(NEXT_OPCODE);
        let (command, tail) = decode_background_command(&payload).unwrap();

        assert_eq!(command.slot().encode(), SLOT);
        assert_eq!(command.source_name(), b"pterra1g.lbm");
        assert_eq!(tail, &[NEXT_OPCODE]);
    }

    #[test]
    fn background_payload_rejects_slots_outside_the_shipped_domain() {
        const INVALID_SLOT: u8 = 128;

        assert_eq!(
            decode_background_command(&[INVALID_SLOT, 0]),
            Err(DescriptBackgroundError::InvalidSlot(INVALID_SLOT))
        );
    }

    #[test]
    fn caption_payload_preserves_game_font_bytes_and_consumes_its_terminator() {
        const NEXT_OPCODE: u8 = 6;

        let payload = [128, 255, 1, 0, NEXT_OPCODE];
        let (command, tail) = decode_caption_command(&payload).unwrap();

        assert_eq!(command.text(), &[128, 255, 1]);
        assert_eq!(tail, &[NEXT_OPCODE]);
    }
}
