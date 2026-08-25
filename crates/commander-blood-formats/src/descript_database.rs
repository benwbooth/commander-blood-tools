//! Lossless parser for the DESCRIPT scene and dialogue database.

use crate::descript::{
    DescriptBackgroundCommand, DescriptCaptionCommand, DescriptIdleClip, DescriptLocationLayout,
    DescriptMusicName, DescriptRecordKind, DescriptSequenceSubtitle, DescriptSoundBankName,
    DescriptSpriteName, DescriptTalkClip, DescriptVideoName, decode_background_command,
    decode_caption_command, decode_idle_clip, decode_location_layout, decode_music_name,
    decode_sequence_subtitle, decode_sound_bank_name, decode_sprite_name, decode_talk_clip,
    decode_video_name,
};

const DIRECTORY_COUNT_BYTES: usize = size_of::<u16>();
const DIRECTORY_ENTRY_BYTES: usize = 18;
const DIRECTORY_NAME_BYTES: usize = 16;
const RECORD_LENGTH_BYTES: usize = size_of::<u16>();
const END_OF_DATABASE_OPCODE: u8 = u8::MAX;

/// One typed command from a DESCRIPT record's authored byte stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DescriptCommand {
    /// Cache one location background image.
    Background(DescriptBackgroundCommand),
    /// Stage a location or ship caption.
    Caption(DescriptCaptionCommand),
    /// Select a location's primary HNM.
    LocationVideo(DescriptVideoName),
    /// Add one character talk animation.
    TalkClip(DescriptTalkClip),
    /// Select a location video's first display row.
    LocationLayout(DescriptLocationLayout),
    /// Select a character HNM for the right-side view.
    CharacterRightVideo(DescriptVideoName),
    /// Select a character HNM for the left-side view.
    CharacterLeftVideo(DescriptVideoName),
    /// Select a character idle animation.
    IdleClip(DescriptIdleClip),
    /// Add one standalone sequence HNM.
    SequenceVideo(DescriptVideoName),
    /// Add one frame-thresholded centered sequence subtitle.
    SequenceSubtitle(DescriptSequenceSubtitle),
    /// Select a character portrait SPR.
    CharacterSprite(DescriptSpriteName),
    /// Select an inventory or world-object HNM.
    ObjectVideo(DescriptVideoName),
    /// Select a character chatter and reaction SND bank.
    SoundBank(DescriptSoundBankName),
    /// Select normalized location or sequence background music.
    Music(DescriptMusicName),
}

/// Serialized marker that terminates one DESCRIPT command stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptRecordEnd {
    /// The marker is the semantic kind of the next serialized record.
    NextRecord(DescriptRecordKind),
    /// The marker explicitly ends the database.
    EndOfDatabase,
}

/// One named, typed DESCRIPT record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescriptRecord {
    name: Box<[u8]>,
    kind: DescriptRecordKind,
    commands: Box<[DescriptCommand]>,
    end: DescriptRecordEnd,
}

impl DescriptRecord {
    /// Return the case-sensitive directory name exactly as stored.
    pub fn name(&self) -> &[u8] {
        &self.name
    }

    /// Return this record's semantic kind.
    pub const fn kind(&self) -> DescriptRecordKind {
        self.kind
    }

    /// Return commands in authored application order.
    pub fn commands(&self) -> &[DescriptCommand] {
        &self.commands
    }

    /// Return the marker that terminated this record.
    pub const fn end(&self) -> DescriptRecordEnd {
        self.end
    }
}

/// Fully decoded DESCRIPT database in serialized directory order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescriptDatabase {
    records: Box<[DescriptRecord]>,
}

impl DescriptDatabase {
    /// Decode a complete DESCRIPT.DES byte stream into owned typed records.
    pub fn parse(data: &[u8]) -> Result<Self, DescriptDatabaseError> {
        let encoded_count = data
            .get(..DIRECTORY_COUNT_BYTES)
            .ok_or(DescriptDatabaseError::MissingDirectoryCount)?;
        let record_count = usize::from(u16::from_le_bytes(encoded_count.try_into().unwrap()));
        let directory_payload_bytes = record_count
            .checked_mul(DIRECTORY_ENTRY_BYTES)
            .ok_or(DescriptDatabaseError::DirectorySizeOverflow)?;
        let directory_end = DIRECTORY_COUNT_BYTES
            .checked_add(directory_payload_bytes)
            .ok_or(DescriptDatabaseError::DirectorySizeOverflow)?;
        if data.len() < directory_end {
            return Err(DescriptDatabaseError::TruncatedDirectory);
        }

        let mut records = Vec::with_capacity(record_count);
        for record_index in 0..record_count {
            let entry_start = DIRECTORY_COUNT_BYTES + record_index * DIRECTORY_ENTRY_BYTES;
            let name_field = &data[entry_start..entry_start + DIRECTORY_NAME_BYTES];
            let name_length = name_field.iter().position(|byte| *byte == u8::MIN).ok_or(
                DescriptDatabaseError::UnterminatedDirectoryName(record_index),
            )?;
            let encoded_offset =
                &data[entry_start + DIRECTORY_NAME_BYTES..entry_start + DIRECTORY_ENTRY_BYTES];
            let record_offset = usize::from(u16::from_le_bytes(encoded_offset.try_into().unwrap()));
            let record = decode_record(
                data,
                record_index,
                Box::from(&name_field[..name_length]),
                record_offset,
            )?;
            records.push(record);
        }

        Ok(Self {
            records: records.into_boxed_slice(),
        })
    }

    /// Return records in serialized directory order.
    pub fn records(&self) -> &[DescriptRecord] {
        &self.records
    }

    /// Find the first exact case-sensitive directory-name match.
    pub fn lookup(&self, name: &[u8]) -> Option<&DescriptRecord> {
        self.records.iter().find(|record| record.name() == name)
    }
}

/// Failure while decoding DESCRIPT database framing or a typed command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptDatabaseError {
    /// The input ended before its little-endian directory count.
    MissingDirectoryCount,
    /// The directory byte count overflowed the host collection size.
    DirectorySizeOverflow,
    /// The input ended inside the fixed-size directory table.
    TruncatedDirectory,
    /// The indexed directory name has no zero within its 16-byte field.
    UnterminatedDirectoryName(usize),
    /// The indexed directory entry does not point to a complete record kind.
    MissingRecordKind(usize),
    /// The indexed record has a kind byte outside the shipped domain.
    UnknownRecordKind(usize, u8),
    /// The indexed record does not contain its complete length word.
    MissingRecordLength(usize),
    /// The indexed record length is too short to contain a command and terminator.
    InvalidRecordLength(usize),
    /// The indexed record extends past the available database bytes.
    TruncatedRecord(usize),
    /// The indexed record ended without a serialized stop marker.
    MissingRecordEnd(usize),
    /// The indexed record contains an opcode outside the recovered dispatch table.
    UnknownOpcode(usize, u8),
    /// The indexed record contains a malformed payload for the given opcode.
    MalformedCommand(usize, u8),
}

fn decode_record(
    data: &[u8],
    record_index: usize,
    name: Box<[u8]>,
    record_offset: usize,
) -> Result<DescriptRecord, DescriptDatabaseError> {
    let &encoded_kind = record_offset
        .checked_sub(1)
        .and_then(|kind_offset| data.get(kind_offset))
        .ok_or(DescriptDatabaseError::MissingRecordKind(record_index))?;
    let kind = DescriptRecordKind::decode(encoded_kind).ok_or(
        DescriptDatabaseError::UnknownRecordKind(record_index, encoded_kind),
    )?;
    let encoded_length = data
        .get(record_offset..record_offset + RECORD_LENGTH_BYTES)
        .ok_or(DescriptDatabaseError::MissingRecordLength(record_index))?;
    let record_length = usize::from(u16::from_le_bytes(encoded_length.try_into().unwrap()));
    if record_length <= RECORD_LENGTH_BYTES {
        return Err(DescriptDatabaseError::InvalidRecordLength(record_index));
    }
    let record_end = record_offset
        .checked_add(record_length)
        .ok_or(DescriptDatabaseError::TruncatedRecord(record_index))?;
    let mut payload = data
        .get(record_offset + RECORD_LENGTH_BYTES..record_end)
        .ok_or(DescriptDatabaseError::TruncatedRecord(record_index))?;
    let mut commands = Vec::new();

    let end = loop {
        let (&opcode, tail) = payload
            .split_first()
            .ok_or(DescriptDatabaseError::MissingRecordEnd(record_index))?;
        payload = tail;
        match opcode {
            0 | END_OF_DATABASE_OPCODE => break DescriptRecordEnd::EndOfDatabase,
            1 | 2 | 4 | 15 => {
                let next_kind = DescriptRecordKind::decode(opcode).unwrap();
                break DescriptRecordEnd::NextRecord(next_kind);
            }
            3 => {
                let (command, tail) = decode_background_command(payload)
                    .map_err(|_| DescriptDatabaseError::MalformedCommand(record_index, opcode))?;
                commands.push(DescriptCommand::Background(command));
                payload = tail;
            }
            5 => {
                let (command, tail) = decode_caption_command(payload)
                    .map_err(|_| DescriptDatabaseError::MalformedCommand(record_index, opcode))?;
                commands.push(DescriptCommand::Caption(command));
                payload = tail;
            }
            6 => {
                let (video, tail) = decode_video_name(payload)
                    .map_err(|_| DescriptDatabaseError::MalformedCommand(record_index, opcode))?;
                commands.push(DescriptCommand::LocationVideo(video));
                payload = tail;
            }
            7 => {
                let (clip, tail) = decode_talk_clip(payload)
                    .map_err(|_| DescriptDatabaseError::MalformedCommand(record_index, opcode))?;
                commands.push(DescriptCommand::TalkClip(clip));
                payload = tail;
            }
            8 => {
                let (layout, tail) = decode_location_layout(payload)
                    .map_err(|_| DescriptDatabaseError::MalformedCommand(record_index, opcode))?;
                commands.push(DescriptCommand::LocationLayout(layout));
                payload = tail;
            }
            9 => {
                let (video, tail) = decode_video_name(payload)
                    .map_err(|_| DescriptDatabaseError::MalformedCommand(record_index, opcode))?;
                commands.push(DescriptCommand::CharacterRightVideo(video));
                payload = tail;
            }
            10 => {
                let (video, tail) = decode_video_name(payload)
                    .map_err(|_| DescriptDatabaseError::MalformedCommand(record_index, opcode))?;
                commands.push(DescriptCommand::CharacterLeftVideo(video));
                payload = tail;
            }
            11 => {
                let (clip, tail) = decode_idle_clip(payload)
                    .map_err(|_| DescriptDatabaseError::MalformedCommand(record_index, opcode))?;
                commands.push(DescriptCommand::IdleClip(clip));
                payload = tail;
            }
            12 => {
                let (video, tail) = decode_video_name(payload)
                    .map_err(|_| DescriptDatabaseError::MalformedCommand(record_index, opcode))?;
                commands.push(DescriptCommand::SequenceVideo(video));
                payload = tail;
            }
            13 => {
                let (subtitle, tail) = decode_sequence_subtitle(payload)
                    .map_err(|_| DescriptDatabaseError::MalformedCommand(record_index, opcode))?;
                commands.push(DescriptCommand::SequenceSubtitle(subtitle));
                payload = tail;
            }
            14 => {
                let (sprite, tail) = decode_sprite_name(payload)
                    .map_err(|_| DescriptDatabaseError::MalformedCommand(record_index, opcode))?;
                commands.push(DescriptCommand::CharacterSprite(sprite));
                payload = tail;
            }
            16 => {
                let (video, tail) = decode_video_name(payload)
                    .map_err(|_| DescriptDatabaseError::MalformedCommand(record_index, opcode))?;
                commands.push(DescriptCommand::ObjectVideo(video));
                payload = tail;
            }
            17 => {
                let (sound_bank, tail) = decode_sound_bank_name(payload)
                    .map_err(|_| DescriptDatabaseError::MalformedCommand(record_index, opcode))?;
                commands.push(DescriptCommand::SoundBank(sound_bank));
                payload = tail;
            }
            18 => {
                let (music, tail) = decode_music_name(payload)
                    .map_err(|_| DescriptDatabaseError::MalformedCommand(record_index, opcode))?;
                commands.push(DescriptCommand::Music(music));
                payload = tail;
            }
            _ => return Err(DescriptDatabaseError::UnknownOpcode(record_index, opcode)),
        }
    };

    Ok(DescriptRecord {
        name,
        kind,
        commands: commands.into_boxed_slice(),
        end,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    use super::*;

    const EXPECTED_RECORD_COUNT: usize = 145;
    const EXPECTED_SEQUENCE_SUBTITLE_COUNT: usize = 48;
    const EXPECTED_SEQUENCE_VIDEO_COUNT: usize = 59;
    const EXPECTED_MUSIC_COUNT: usize = 55;
    const EXPECTED_CHARACTER_SPRITE_COUNT: usize = 25;
    const EXPECTED_COMMAND_VARIANTS: [&str; 14] = [
        "background",
        "caption",
        "character_left_video",
        "character_right_video",
        "character_sprite",
        "idle_clip",
        "location_layout",
        "location_video",
        "music",
        "object_video",
        "sequence_subtitle",
        "sequence_video",
        "sound_bank",
        "talk_clip",
    ];

    fn original_asset() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood/DESCRIPT.DES")
    }

    #[test]
    fn shipped_database_decodes_every_record_and_known_command() {
        let data = std::fs::read(original_asset()).unwrap();
        let database = DescriptDatabase::parse(&data).unwrap();

        assert_eq!(database.records().len(), EXPECTED_RECORD_COUNT);
        assert_eq!(
            database.lookup(b"present").unwrap().kind(),
            DescriptRecordKind::Sequence
        );
        assert!(database.lookup(b"PRESENT").is_none());

        let commands = database.records().iter().flat_map(DescriptRecord::commands);
        let mut sequence_subtitles = 0;
        let mut sequence_videos = 0;
        let mut music = 0;
        let mut character_sprites = 0;
        let mut command_variants = BTreeSet::new();
        for command in commands {
            let variant = match command {
                DescriptCommand::Background(_) => "background",
                DescriptCommand::Caption(_) => "caption",
                DescriptCommand::LocationVideo(_) => "location_video",
                DescriptCommand::TalkClip(_) => "talk_clip",
                DescriptCommand::LocationLayout(_) => "location_layout",
                DescriptCommand::CharacterRightVideo(_) => "character_right_video",
                DescriptCommand::CharacterLeftVideo(_) => "character_left_video",
                DescriptCommand::IdleClip(_) => "idle_clip",
                DescriptCommand::SequenceVideo(_) => {
                    sequence_videos += 1;
                    "sequence_video"
                }
                DescriptCommand::SequenceSubtitle(_) => {
                    sequence_subtitles += 1;
                    "sequence_subtitle"
                }
                DescriptCommand::CharacterSprite(_) => {
                    character_sprites += 1;
                    "character_sprite"
                }
                DescriptCommand::ObjectVideo(_) => "object_video",
                DescriptCommand::SoundBank(_) => "sound_bank",
                DescriptCommand::Music(_) => {
                    music += 1;
                    "music"
                }
            };
            command_variants.insert(variant);
        }

        assert_eq!(sequence_subtitles, EXPECTED_SEQUENCE_SUBTITLE_COUNT);
        assert_eq!(sequence_videos, EXPECTED_SEQUENCE_VIDEO_COUNT);
        assert_eq!(music, EXPECTED_MUSIC_COUNT);
        assert_eq!(character_sprites, EXPECTED_CHARACTER_SPRITE_COUNT);
        assert_eq!(
            command_variants,
            EXPECTED_COMMAND_VARIANTS.into_iter().collect()
        );
        for expected_end in [
            DescriptRecordEnd::NextRecord(DescriptRecordKind::Location),
            DescriptRecordEnd::NextRecord(DescriptRecordKind::Character),
            DescriptRecordEnd::NextRecord(DescriptRecordKind::Sequence),
            DescriptRecordEnd::NextRecord(DescriptRecordKind::Object),
            DescriptRecordEnd::EndOfDatabase,
        ] {
            assert!(
                database
                    .records()
                    .iter()
                    .any(|record| record.end() == expected_end)
            );
        }
    }

    #[test]
    fn shipped_location_caption_preserves_its_carriage_return() {
        let data = std::fs::read(original_asset()).unwrap();
        let database = DescriptDatabase::parse(&data).unwrap();
        let ondoya = database.lookup(b"Ondoya").unwrap();
        let caption = ondoya
            .commands()
            .iter()
            .find_map(|command| match command {
                DescriptCommand::Caption(caption) => Some(caption),
                _ => None,
            })
            .unwrap();

        assert_eq!(caption.text(), b"planet Ondoya\r");
        assert!(matches!(
            ondoya.commands().last(),
            Some(DescriptCommand::LocationVideo(video)) if video.as_bytes() == b"ondoya.hnm"
        ));
    }

    #[test]
    fn malformed_database_framing_is_rejected() {
        assert_eq!(
            DescriptDatabase::parse(&[]),
            Err(DescriptDatabaseError::MissingDirectoryCount)
        );
        assert_eq!(
            DescriptDatabase::parse(&[1, 0]),
            Err(DescriptDatabaseError::TruncatedDirectory)
        );
    }
}
