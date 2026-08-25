//! Typed structures for the DESCRIPT scene and dialogue database.

const FIRST_BACKGROUND_SLOT: u8 = 1;
const BACKGROUND_SLOT_COUNT: u8 = 4;
const LAST_BACKGROUND_SLOT: u8 = FIRST_BACKGROUND_SLOT + BACKGROUND_SLOT_COUNT - 1;
const PRINTABLE_NAME_START: u8 = 32;
const PRINTABLE_NAME_END: u8 = 127;
const NO_TALK_BACKGROUND_ID: u8 = u8::MAX;

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

/// Case-preserving HNM resource name decoded from a DESCRIPT video command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescriptVideoName(Box<[u8]>);

impl DescriptVideoName {
    /// Build a video name from owned bytes without a trailing zero.
    pub fn new(source_name: Box<[u8]>) -> Self {
        Self(source_name)
    }

    /// Return the HNM resource name exactly as authored.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Failure while decoding a printable DESCRIPT resource name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptResourceNameError {
    /// The printable name reaches the end of the record without a stop byte.
    MissingStopByte,
}

fn decode_printable_resource_name(
    payload: &[u8],
) -> Result<(Box<[u8]>, &[u8]), DescriptResourceNameError> {
    let name_length = payload
        .iter()
        .position(|byte| !(*byte >= PRINTABLE_NAME_START && *byte <= PRINTABLE_NAME_END))
        .ok_or(DescriptResourceNameError::MissingStopByte)?;
    Ok((Box::from(&payload[..name_length]), &payload[name_length..]))
}

/// Decode a printable HNM name while leaving the following opcode unconsumed.
pub fn decode_video_name(
    payload: &[u8],
) -> Result<(DescriptVideoName, &[u8]), DescriptResourceNameError> {
    let (source_name, tail) = decode_printable_resource_name(payload)?;
    Ok((DescriptVideoName::new(source_name), tail))
}

/// Case-preserving SND bank name decoded from a DESCRIPT audio command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescriptSoundBankName(Box<[u8]>);

impl DescriptSoundBankName {
    /// Build a sound-bank name from owned bytes without a trailing zero.
    pub fn new(source_name: Box<[u8]>) -> Self {
        Self(source_name)
    }

    /// Return the SND bank name exactly as authored.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Decode a printable SND bank name while leaving the following opcode unconsumed.
pub fn decode_sound_bank_name(
    payload: &[u8],
) -> Result<(DescriptSoundBankName, &[u8]), DescriptResourceNameError> {
    let (source_name, tail) = decode_printable_resource_name(payload)?;
    Ok((DescriptSoundBankName::new(source_name), tail))
}

/// Background selection paired with one character talk animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptTalkBackground {
    /// Draw the talk animation without one of the four cached LBM backgrounds.
    None,
    /// Draw over one explicitly selected cached background.
    Cached(DescriptBackgroundSlot),
}

/// One opcode-07 character talk animation and its background selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescriptTalkClip {
    background: DescriptTalkBackground,
    video: DescriptVideoName,
}

impl DescriptTalkClip {
    /// Build one decoded talk clip.
    pub fn new(background: DescriptTalkBackground, video: DescriptVideoName) -> Self {
        Self { background, video }
    }

    /// Return the background selected for this animation.
    pub const fn background(&self) -> DescriptTalkBackground {
        self.background
    }

    /// Return the HNM resource used for this animation.
    pub fn video(&self) -> &DescriptVideoName {
        &self.video
    }
}

/// Failure while decoding an opcode-07 character talk animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptTalkClipError {
    /// The command ended before its background ID.
    MissingBackground,
    /// The background ID is neither one through four nor the shipped no-background sentinel.
    InvalidBackground(u8),
    /// The HNM name reaches the end of the record without a stop byte.
    MissingStopByte,
}

/// Decode a talk animation while leaving the following opcode unconsumed.
pub fn decode_talk_clip(
    payload: &[u8],
) -> Result<(DescriptTalkClip, &[u8]), DescriptTalkClipError> {
    let (&encoded_background, video_and_tail) = payload
        .split_first()
        .ok_or(DescriptTalkClipError::MissingBackground)?;
    let background = if encoded_background == NO_TALK_BACKGROUND_ID {
        DescriptTalkBackground::None
    } else {
        DescriptBackgroundSlot::decode(encoded_background)
            .map(DescriptTalkBackground::Cached)
            .ok_or(DescriptTalkClipError::InvalidBackground(encoded_background))?
    };
    let (video_name, tail) = decode_printable_resource_name(video_and_tail)
        .map_err(|_| DescriptTalkClipError::MissingStopByte)?;
    Ok((
        DescriptTalkClip::new(background, DescriptVideoName::new(video_name)),
        tail,
    ))
}

/// Vertical placement of a location scene's primary video.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DescriptLocationLayout {
    top_row: u16,
}

impl DescriptLocationLayout {
    /// Build a layout from its authored top row.
    pub const fn new(top_row: u16) -> Self {
        Self { top_row }
    }

    /// Return the first display row occupied by the scene video.
    pub const fn top_row(self) -> u16 {
        self.top_row
    }
}

/// Failure while decoding an opcode-08 location layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptLocationLayoutError {
    /// The command does not contain the complete little-endian row word.
    MissingTopRow,
}

/// Decode a location scene's top row and return the following command bytes.
pub fn decode_location_layout(
    payload: &[u8],
) -> Result<(DescriptLocationLayout, &[u8]), DescriptLocationLayoutError> {
    let (encoded_row, tail) = payload
        .split_at_checked(size_of::<u16>())
        .ok_or(DescriptLocationLayoutError::MissingTopRow)?;
    let top_row = u16::from_le_bytes(encoded_row.try_into().unwrap());
    Ok((DescriptLocationLayout::new(top_row), tail))
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

    #[test]
    fn video_name_payload_keeps_the_following_opcode_unconsumed() {
        const NEXT_OPCODE: u8 = 9;

        let mut payload = b"pterra10.hnm".to_vec();
        payload.push(NEXT_OPCODE);
        let (video, tail) = decode_video_name(&payload).unwrap();

        assert_eq!(video.as_bytes(), b"pterra10.hnm");
        assert_eq!(tail, &[NEXT_OPCODE]);
    }

    #[test]
    fn sound_bank_payload_uses_the_shared_printable_name_framing() {
        const NEXT_OPCODE: u8 = 18;

        let mut payload = b"scrut.snd".to_vec();
        payload.push(NEXT_OPCODE);
        let (bank, tail) = decode_sound_bank_name(&payload).unwrap();

        assert_eq!(bank.as_bytes(), b"scrut.snd");
        assert_eq!(tail, &[NEXT_OPCODE]);
    }

    #[test]
    fn talk_clip_decodes_cached_and_no_background_variants() {
        const CACHED_BACKGROUND: u8 = 4;
        const NEXT_OPCODE: u8 = 17;

        let mut cached_payload = vec![CACHED_BACKGROUND];
        cached_payload.extend_from_slice(b"scr01.hnm");
        cached_payload.push(NEXT_OPCODE);
        let (cached, tail) = decode_talk_clip(&cached_payload).unwrap();
        assert_eq!(
            cached.background(),
            DescriptTalkBackground::Cached(
                DescriptBackgroundSlot::decode(CACHED_BACKGROUND).unwrap()
            )
        );
        assert_eq!(cached.video().as_bytes(), b"scr01.hnm");
        assert_eq!(tail, &[NEXT_OPCODE]);

        let no_background_payload = [NO_TALK_BACKGROUND_ID, NEXT_OPCODE];
        let (no_background, tail) = decode_talk_clip(&no_background_payload).unwrap();
        assert_eq!(no_background.background(), DescriptTalkBackground::None);
        assert!(no_background.video().as_bytes().is_empty());
        assert_eq!(tail, &[NEXT_OPCODE]);
    }

    #[test]
    fn location_layout_decodes_the_shipped_top_row() {
        const NEXT_OPCODE: u8 = 5;
        const SHIPPED_LOCATION_TOP_ROW: u16 = 35;

        let mut payload = SHIPPED_LOCATION_TOP_ROW.to_le_bytes().to_vec();
        payload.push(NEXT_OPCODE);
        let (layout, tail) = decode_location_layout(&payload).unwrap();

        assert_eq!(layout.top_row(), SHIPPED_LOCATION_TOP_ROW);
        assert_eq!(tail, &[NEXT_OPCODE]);
    }
}
