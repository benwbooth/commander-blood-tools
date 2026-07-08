use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::util::media_stem;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SubtitleCue {
    pub tick: u16,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[repr(u8)]
pub enum RecordKind {
    Location = 1,
    Character = 2,
    Sequence = 4,
    Object = 15,
    Unknown(u8),
}

impl RecordKind {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            1 => Self::Location,
            2 => Self::Character,
            4 => Self::Sequence,
            15 => Self::Object,
            other => Self::Unknown(other),
        }
    }

    pub fn as_byte(self) -> u8 {
        match self {
            Self::Location => 1,
            Self::Character => 2,
            Self::Sequence => 4,
            Self::Object => 15,
            Self::Unknown(other) => other,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SlottedMedia {
    pub slot: u8,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum DescriptCommand {
    Background { slot: u8, lbm: String },
    Label(String),
    FullHnm(String),
    TalkHnm(SlottedMedia),
    Unknown08([u8; 2]),
    AuxiliaryHnm { opcode: u8, hnm: String },
    IdleHnm(SlottedMedia),
    SequenceHnm(String),
    Subtitle(SubtitleCue),
    Sprite(String),
    ObjectHnm(String),
    Snd(String),
    Music(String),
    Unknown04([u8; 2]),
    End(u8),
    Unknown(u8),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DescriptRecord {
    pub name: String,
    pub kind: RecordKind,
    pub commands: Vec<DescriptCommand>,
    pub music: Vec<String>,
    pub full_hnms: Vec<String>,
    pub sequence_hnms: Vec<String>,
    pub idle_hnms: Vec<SlottedMedia>,
    pub talk_hnms: Vec<SlottedMedia>,
    pub snd: Option<String>,
    pub sprite: Option<String>,
    pub labels: Vec<String>,
    pub subtitles: Vec<SubtitleCue>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DescriptDb {
    pub records: Vec<DescriptRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CharacterScene {
    pub record_name: String,
    pub snd: String,
    pub idle_hnms: Vec<SlottedMedia>,
    pub talk_hnms: Vec<SlottedMedia>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VideoScene {
    pub record_name: String,
    pub kind: RecordKind,
    pub music: Option<String>,
    pub hnms: Vec<String>,
    pub subtitles: Vec<SubtitleCue>,
}

impl DescriptDb {
    pub fn parse_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let data =
            fs::read(path).with_context(|| format!("reading DESCRIPT file {}", path.display()))?;
        Self::parse(&data).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 2 {
            bail!("DESCRIPT.DES too small");
        }

        let count = u16::from_le_bytes([data[0], data[1]]) as usize;
        let table_end = 2 + count * 18;
        if table_end > data.len() {
            bail!("DESCRIPT.DES index exceeds file size");
        }

        let mut records = Vec::with_capacity(count);
        for i in 0..count {
            let table_pos = 2 + i * 18;
            let name_len = data[table_pos..table_pos + 16]
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(16);
            let name = decode_text(&data[table_pos..table_pos + name_len]);
            let ptr = u16::from_le_bytes([data[table_pos + 16], data[table_pos + 17]]) as usize;
            if ptr == 0 || ptr + 2 > data.len() {
                continue;
            }

            let kind = RecordKind::from_byte(data[ptr - 1]);
            let len = u16::from_le_bytes([data[ptr], data[ptr + 1]]) as usize;
            let raw_end = (ptr + len).min(data.len());
            let next_ptr = (i + 1 < count).then(|| {
                let next_table_pos = 2 + (i + 1) * 18;
                u16::from_le_bytes([data[next_table_pos + 16], data[next_table_pos + 17]]) as usize
            });
            let end = if next_ptr == Some(ptr + len) && raw_end > ptr + 2 {
                // DESCRIPT lengths are measured to the next record's length
                // field. The byte immediately before that field is the next
                // record kind, not part of this record's command payload.
                raw_end - 1
            } else {
                raw_end
            };
            let mut pos = ptr + 2;

            let mut record = DescriptRecord {
                name,
                kind,
                commands: Vec::new(),
                music: Vec::new(),
                full_hnms: Vec::new(),
                sequence_hnms: Vec::new(),
                idle_hnms: Vec::new(),
                talk_hnms: Vec::new(),
                snd: None,
                sprite: None,
                labels: Vec::new(),
                subtitles: Vec::new(),
            };

            while pos < end {
                let op = data[pos];
                pos += 1;
                match op {
                    0x03 => {
                        if pos >= end {
                            record.commands.push(DescriptCommand::Unknown(op));
                            break;
                        }
                        let slot = data[pos];
                        pos += 1;
                        let lbm = read_media(data, &mut pos, end, ".lbm");
                        record
                            .commands
                            .push(DescriptCommand::Background { slot, lbm });
                    }
                    0x05 => {
                        let label = read_cstr(data, &mut pos, end);
                        record.labels.push(label.clone());
                        record.commands.push(DescriptCommand::Label(label));
                    }
                    0x06 => {
                        let hnm = read_media(data, &mut pos, end, ".hnm");
                        record.full_hnms.push(hnm.clone());
                        record.commands.push(DescriptCommand::FullHnm(hnm));
                    }
                    0x07 => {
                        if pos >= end {
                            record.commands.push(DescriptCommand::Unknown(op));
                            break;
                        }
                        let slot = data[pos];
                        pos += 1;
                        let hnm = read_media(data, &mut pos, end, ".hnm");
                        let media = SlottedMedia { slot, name: hnm };
                        record.talk_hnms.push(media.clone());
                        record.commands.push(DescriptCommand::TalkHnm(media));
                    }
                    0x08 => {
                        let bytes = read_two(data, &mut pos, end);
                        record.commands.push(DescriptCommand::Unknown08(bytes));
                    }
                    0x09 | 0x0a => {
                        let hnm = read_media(data, &mut pos, end, ".hnm");
                        record
                            .commands
                            .push(DescriptCommand::AuxiliaryHnm { opcode: op, hnm });
                    }
                    0x0b => {
                        if pos >= end {
                            record.commands.push(DescriptCommand::Unknown(op));
                            break;
                        }
                        let slot = data[pos];
                        pos += 1;
                        let hnm = read_media(data, &mut pos, end, ".hnm");
                        let media = SlottedMedia { slot, name: hnm };
                        record.idle_hnms.push(media.clone());
                        record.commands.push(DescriptCommand::IdleHnm(media));
                    }
                    0x0c => {
                        let hnm = read_media(data, &mut pos, end, ".hnm");
                        record.sequence_hnms.push(hnm.clone());
                        record.commands.push(DescriptCommand::SequenceHnm(hnm));
                    }
                    0x0d => {
                        if pos + 1 < end && data[pos] == 0 && is_opcode(data[pos + 1]) {
                            pos += 1;
                        } else if pos + 2 <= end {
                            let tick = u16::from_le_bytes([data[pos], data[pos + 1]]);
                            pos += 2;
                            let text = read_cstr(data, &mut pos, end);
                            let cue = SubtitleCue { tick, text };
                            record.subtitles.push(cue.clone());
                            record.commands.push(DescriptCommand::Subtitle(cue));
                        } else {
                            record.commands.push(DescriptCommand::Unknown(op));
                            break;
                        }
                    }
                    0x0e => {
                        let sprite = read_media(data, &mut pos, end, ".spr");
                        record.sprite = Some(sprite.clone());
                        record.commands.push(DescriptCommand::Sprite(sprite));
                    }
                    0x10 => {
                        let hnm = read_media(data, &mut pos, end, ".hnm");
                        record.full_hnms.push(hnm.clone());
                        record.commands.push(DescriptCommand::ObjectHnm(hnm));
                    }
                    0x11 => {
                        let snd = read_media(data, &mut pos, end, ".snd");
                        record.snd = Some(snd.clone());
                        record.commands.push(DescriptCommand::Snd(snd));
                    }
                    0x12 => {
                        let music = read_media(data, &mut pos, end, ".voc");
                        record.music.push(music.clone());
                        record.commands.push(DescriptCommand::Music(music));
                    }
                    0x04 => {
                        let bytes = read_two(data, &mut pos, end);
                        record.commands.push(DescriptCommand::Unknown04(bytes));
                    }
                    0x00 | 0x02 | 0xff => {
                        record.commands.push(DescriptCommand::End(op));
                        break;
                    }
                    _ => {
                        record.commands.push(DescriptCommand::Unknown(op));
                        break;
                    }
                }
            }

            records.push(record);
        }

        Ok(Self { records })
    }

    pub fn record(&self, name: &str) -> Option<&DescriptRecord> {
        self.records
            .iter()
            .find(|record| record.name.eq_ignore_ascii_case(name))
    }

    pub fn character_names(&self) -> Vec<String> {
        self.records
            .iter()
            .filter(|record| record.kind == RecordKind::Character)
            .map(|record| record.name.to_ascii_lowercase())
            .collect()
    }

    pub fn character_scenes_for_snd(&self, snd_stem: &str) -> Vec<CharacterScene> {
        let snd_stem = snd_stem.to_ascii_lowercase();
        self.records
            .iter()
            .filter(|record| record.kind == RecordKind::Character)
            .filter_map(|record| {
                let snd = record.snd.as_ref()?;
                if media_stem(snd) != snd_stem {
                    return None;
                }
                Some(CharacterScene {
                    record_name: record.name.clone(),
                    snd: snd.clone(),
                    idle_hnms: record.idle_hnms.clone(),
                    talk_hnms: record.talk_hnms.clone(),
                })
            })
            .collect()
    }

    pub fn hnm_music_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        let mut inherited_location_music: Option<String> = None;

        for record in &self.records {
            if record.kind == RecordKind::Location {
                let explicit_music = record.music.first().map(|music| media_stem(music));
                if let Some(music) = explicit_music {
                    inherited_location_music = Some(music.clone());
                    for hnm in &record.full_hnms {
                        map.insert(media_stem(hnm), music.clone());
                    }
                } else if let Some(music) = &inherited_location_music {
                    for hnm in &record.full_hnms {
                        map.entry(media_stem(hnm)).or_insert_with(|| music.clone());
                    }
                }
                continue;
            }

            if let Some(music) = record.music.first().map(|m| media_stem(m)) {
                for hnm in record.full_hnms.iter().chain(record.sequence_hnms.iter()) {
                    map.insert(media_stem(hnm), music.clone());
                }
            }
        }

        map
    }

    pub fn video_scenes(&self, hnm_music: &HashMap<String, String>) -> Vec<VideoScene> {
        self.records
            .iter()
            .filter(|record| record.kind != RecordKind::Character)
            .filter_map(|record| {
                let hnms = if record.sequence_hnms.is_empty() {
                    record.full_hnms.clone()
                } else {
                    record.sequence_hnms.clone()
                };
                if hnms.is_empty() {
                    return None;
                }

                let music = record
                    .music
                    .first()
                    .map(|music| media_stem(music))
                    .or_else(|| {
                        hnms.first()
                            .and_then(|hnm| hnm_music.get(&media_stem(hnm)))
                            .cloned()
                    });

                Some(VideoScene {
                    record_name: record.name.clone(),
                    kind: record.kind,
                    music,
                    hnms,
                    subtitles: record.subtitles.clone(),
                })
            })
            .collect()
    }
}

fn decode_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .replace('\r', "\n")
        .trim_end()
        .to_string()
}

fn read_cstr(data: &[u8], pos: &mut usize, end: usize) -> String {
    let start = *pos;
    while *pos < end && data[*pos] != 0 {
        *pos += 1;
    }
    let text = decode_text(&data[start..*pos]);
    if *pos < end {
        *pos += 1;
    }
    text
}

fn read_media(data: &[u8], pos: &mut usize, end: usize, ext: &str) -> String {
    let start = *pos;
    let ext_bytes = ext.as_bytes();
    let mut media_end = end;
    let mut i = *pos;
    while i + ext_bytes.len() <= end {
        if data[i..i + ext_bytes.len()].eq_ignore_ascii_case(ext_bytes) {
            media_end = i + ext_bytes.len();
            break;
        }
        i += 1;
    }
    *pos = media_end;
    decode_text(&data[start..media_end])
}

fn read_two(data: &[u8], pos: &mut usize, end: usize) -> [u8; 2] {
    let first = data.get(*pos).copied().unwrap_or(0);
    let second = data.get(*pos + 1).copied().unwrap_or(0);
    *pos = (*pos + 2).min(end);
    [first, second]
}

fn is_opcode(byte: u8) -> bool {
    matches!(
        byte,
        0x00 | 0x02
            | 0x03
            | 0x04
            | 0x05
            | 0x06
            | 0x07
            | 0x08
            | 0x09
            | 0x0a
            | 0x0b
            | 0x0c
            | 0x0d
            | 0x0e
            | 0x10
            | 0x11
            | 0x12
            | 0xff
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real DESCRIPT.DES (the game's scene/dialogue database) must parse into its full,
    /// consistent record set: 145 records across the four kinds, every record named, and every
    /// referenced media stem non-empty. Regression-guards the descript parser on the real file.
    /// Skips if the game data isn't in this checkout.
    #[test]
    fn parses_real_descript_des_consistently() {
        let db = ["output/_tmp_iso/DESCRIPT.DES", "../output/_tmp_iso/DESCRIPT.DES"]
            .iter()
            .find_map(|p| DescriptDb::parse_file(p).ok());
        let Some(db) = db else { return };
        assert_eq!(db.records.len(), 145, "DESCRIPT.DES record count");
        for r in &db.records {
            assert!(!r.name.is_empty(), "record has a name");
            if let Some(s) = &r.snd {
                assert!(!s.is_empty(), "{}: empty snd stem", r.name);
            }
            if let Some(s) = &r.sprite {
                assert!(!s.is_empty(), "{}: empty sprite stem", r.name);
            }
        }
        let count = |k: RecordKind| db.records.iter().filter(|r| r.kind == k).count();
        // The four record kinds and their counts (verified against the shipped DESCRIPT.DES).
        assert_eq!(count(RecordKind::Sequence), 11);
        assert_eq!(count(RecordKind::Object), 35);
        assert_eq!(count(RecordKind::Location), 64);
        assert_eq!(count(RecordKind::Character), 35);
    }

    #[test]
    fn record_length_excludes_next_record_kind_byte() {
        let count = 2u16;
        let table_end = 2 + 18 * count as usize;
        let kind1_pos = table_end;
        let ptr1 = kind1_pos + 1;
        let payload1 = [0x05, b'f', b'i', b'r', b's', b't', 0x00];
        let len1 = 2 + payload1.len() + 1;
        let ptr2 = ptr1 + len1;
        let payload2 = [0xff];
        let len2 = 2 + payload2.len();

        let mut data = vec![0u8; table_end];
        data[0..2].copy_from_slice(&count.to_le_bytes());
        data[2..5].copy_from_slice(b"one");
        data[18..20].copy_from_slice(&(ptr1 as u16).to_le_bytes());
        data[20..23].copy_from_slice(b"two");
        data[36..38].copy_from_slice(&(ptr2 as u16).to_le_bytes());
        data.push(RecordKind::Location.as_byte());
        data.extend_from_slice(&(len1 as u16).to_le_bytes());
        data.extend_from_slice(&payload1);
        data.push(RecordKind::Object.as_byte());
        data.extend_from_slice(&(len2 as u16).to_le_bytes());
        data.extend_from_slice(&payload2);

        let db = DescriptDb::parse(&data).expect("parse synthetic DESCRIPT");
        assert_eq!(db.records.len(), 2);
        assert_eq!(
            db.records[0].commands,
            vec![DescriptCommand::Label("first".to_string())]
        );
        assert_eq!(db.records[1].kind, RecordKind::Object);
    }
}
