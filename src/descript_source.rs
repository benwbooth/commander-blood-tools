//! Readable, byte-exact source for the shared `DESCRIPT.DES` presentation database.

use std::collections::HashSet;
use std::fmt::Write;

use anyhow::{Context, Result, anyhow, bail};

const DIRECTORY_COUNT_BYTES: usize = 2;
const DIRECTORY_ENTRY_BYTES: usize = 18;
const DIRECTORY_NAME_BYTES: usize = 16;
const RECORD_LENGTH_BYTES: usize = 2;
const FINAL_RECORD_MARKER: u8 = u8::MAX;

const OP_BACKGROUND: u8 = 3;
const OP_CAPTION: u8 = 5;
const OP_LOCATION_VIDEO: u8 = 6;
const OP_TALK: u8 = 7;
const OP_TOP_ROW: u8 = 8;
const OP_RIGHT_VIDEO: u8 = 9;
const OP_LEFT_VIDEO: u8 = 10;
const OP_IDLE: u8 = 11;
const OP_SEQUENCE_VIDEO: u8 = 12;
const OP_SUBTITLE: u8 = 13;
const OP_PORTRAIT: u8 = 14;
const OP_OBJECT_VIDEO: u8 = 16;
const OP_SOUND_BANK: u8 = 17;
const OP_MUSIC: u8 = 18;

const FIRST_BACKGROUND_SLOT: u8 = 1;
const LAST_BACKGROUND_SLOT: u8 = 4;
const NO_BACKGROUND: u8 = u8::MAX;
const RESOURCE_BYTE_MIN: u8 = 32;
const MUSIC_BYTE_MIN: u8 = 33;
const RESOURCE_BYTE_MAX: u8 = 127;

/// Readable source plus basic coverage counts from one decompilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decompilation {
    /// Canonical `descript 1` source.
    pub source: String,
    /// Number of named records.
    pub record_count: usize,
    /// Number of ordered presentation commands.
    pub command_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RecordKind {
    Location,
    Character,
    Sequence,
    Object,
}

impl RecordKind {
    fn decode(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Location),
            2 => Some(Self::Character),
            4 => Some(Self::Sequence),
            15 => Some(Self::Object),
            _ => None,
        }
    }

    fn encode(self) -> u8 {
        match self {
            Self::Location => 1,
            Self::Character => 2,
            Self::Sequence => 4,
            Self::Object => 15,
        }
    }

    fn keyword(self) -> &'static str {
        match self {
            Self::Location => "location",
            Self::Character => "character",
            Self::Sequence => "sequence",
            Self::Object => "object",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "location" => Some(Self::Location),
            "character" => Some(Self::Character),
            "sequence" => Some(Self::Sequence),
            "object" => Some(Self::Object),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Background {
    None,
    Slot(u8),
}

impl Background {
    fn decode(value: u8) -> Option<Self> {
        if value == NO_BACKGROUND {
            Some(Self::None)
        } else if (FIRST_BACKGROUND_SLOT..=LAST_BACKGROUND_SLOT).contains(&value) {
            Some(Self::Slot(value))
        } else {
            None
        }
    }

    fn encode(self) -> u8 {
        match self {
            Self::None => NO_BACKGROUND,
            Self::Slot(slot) => slot,
        }
    }

    fn source(self) -> String {
        match self {
            Self::None => "none".to_string(),
            Self::Slot(slot) => background_view_name(slot).to_string(),
        }
    }
}

fn background_view_name(slot: u8) -> &'static str {
    match slot {
        1 => "front",
        2 => "right",
        3 => "left",
        4 => "back",
        _ => unreachable!("validated DESCRIPT background slot"),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Command {
    Background {
        slot: u8,
        image: Vec<u8>,
    },
    Caption(Vec<u8>),
    Video(Vec<u8>),
    TopRow(u16),
    Talk {
        video: Vec<u8>,
        background: Background,
    },
    RightVideo(Vec<u8>),
    LeftVideo(Vec<u8>),
    Idle {
        video: Vec<u8>,
        background: Background,
    },
    Portrait(Vec<u8>),
    SoundBank(Vec<u8>),
    Subtitle {
        first_frame: u16,
        text: Vec<u8>,
    },
    Music(Vec<u8>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Record {
    name: Vec<u8>,
    kind: RecordKind,
    commands: Vec<Command>,
}

/// Decompile a complete presentation database into canonical readable source.
/// The function recompiles its result and rejects any non-exact round trip.
pub fn decompile(image: &[u8]) -> Result<Decompilation> {
    let records = decode_image(image)?;
    let source = render_source(&records)?;
    let rebuilt = compile(&source)?;
    if rebuilt != image {
        bail!("internal DESCRIPT source round trip changed the binary image");
    }
    Ok(Decompilation {
        command_count: records.iter().map(|record| record.commands.len()).sum(),
        record_count: records.len(),
        source,
    })
}

/// Compile `descript 1` source into a complete `DESCRIPT.DES` image.
pub fn compile(source: &str) -> Result<Vec<u8>> {
    let records = parse_source(source)?;
    encode_image(&records)
}

fn decode_image(image: &[u8]) -> Result<Vec<Record>> {
    let count_bytes = image
        .get(..DIRECTORY_COUNT_BYTES)
        .ok_or_else(|| anyhow!("DESCRIPT image has no record count"))?;
    let record_count = usize::from(u16::from_le_bytes(count_bytes.try_into().unwrap()));
    if record_count == 0 {
        bail!("DESCRIPT image has no records");
    }
    let directory_end = DIRECTORY_COUNT_BYTES
        .checked_add(
            record_count
                .checked_mul(DIRECTORY_ENTRY_BYTES)
                .ok_or_else(|| anyhow!("DESCRIPT directory size overflow"))?,
        )
        .ok_or_else(|| anyhow!("DESCRIPT directory size overflow"))?;
    if image.len() <= directory_end {
        bail!("DESCRIPT directory or first record is truncated");
    }

    let mut directory = Vec::with_capacity(record_count);
    for index in 0..record_count {
        let start = DIRECTORY_COUNT_BYTES + index * DIRECTORY_ENTRY_BYTES;
        let name_field = &image[start..start + DIRECTORY_NAME_BYTES];
        let name_len = name_field
            .iter()
            .position(|byte| *byte == 0)
            .ok_or_else(|| anyhow!("record {index} has no directory-name terminator"))?;
        if name_field[name_len..].iter().any(|byte| *byte != 0) {
            bail!("record {index} has nonzero directory-name padding");
        }
        let offset = usize::from(u16::from_le_bytes(
            image[start + DIRECTORY_NAME_BYTES..start + DIRECTORY_ENTRY_BYTES]
                .try_into()
                .unwrap(),
        ));
        directory.push((name_field[..name_len].to_vec(), offset));
    }

    if directory[0].1 != directory_end + 1 {
        bail!("first DESCRIPT record is not packed immediately after the directory");
    }

    let mut records = Vec::with_capacity(record_count);
    for index in 0..record_count {
        let (name, offset) = &directory[index];
        if index > 0 && *offset <= directory[index - 1].1 {
            bail!("DESCRIPT directory offsets are not strictly increasing");
        }
        let kind_byte = *image
            .get(offset.saturating_sub(1))
            .ok_or_else(|| anyhow!("record {index} has no kind byte"))?;
        let kind = RecordKind::decode(kind_byte)
            .ok_or_else(|| anyhow!("record {index} has unknown kind {kind_byte}"))?;
        let length_bytes = image
            .get(*offset..offset + RECORD_LENGTH_BYTES)
            .ok_or_else(|| anyhow!("record {index} has no complete length"))?;
        let length = usize::from(u16::from_le_bytes(length_bytes.try_into().unwrap()));
        let end = offset
            .checked_add(length)
            .ok_or_else(|| anyhow!("record {index} length overflow"))?;
        let expected_end = directory
            .get(index + 1)
            .map_or(image.len(), |entry| entry.1);
        if end != expected_end {
            bail!(
                "record {index} length ends at {end}, but packed directory order requires {expected_end}"
            );
        }
        if length <= RECORD_LENGTH_BYTES {
            bail!("record {index} is too short");
        }
        let expected_marker = directory
            .get(index + 1)
            .map_or(FINAL_RECORD_MARKER, |(_, next)| image[next - 1]);
        if image[end - 1] != expected_marker {
            bail!("record {index} has an unexpected end marker");
        }
        let commands = decode_commands(kind, &image[offset + RECORD_LENGTH_BYTES..end - 1])
            .with_context(|| format!("decoding record {}", quote(name)))?;
        records.push(Record {
            name: name.clone(),
            kind,
            commands,
        });
    }
    Ok(records)
}

fn decode_commands(kind: RecordKind, payload: &[u8]) -> Result<Vec<Command>> {
    let mut cursor = 0usize;
    let mut commands = Vec::new();
    while cursor < payload.len() {
        let opcode = payload[cursor];
        cursor += 1;
        let command = match opcode {
            OP_BACKGROUND if kind == RecordKind::Location => {
                let slot = take_byte(payload, &mut cursor, "background slot")?;
                if !(FIRST_BACKGROUND_SLOT..=LAST_BACKGROUND_SLOT).contains(&slot) {
                    bail!("background slot {slot} is outside 1 through 4");
                }
                Command::Background {
                    slot,
                    image: take_resource(payload, &mut cursor, RESOURCE_BYTE_MIN, "background")?,
                }
            }
            OP_CAPTION if kind == RecordKind::Location => {
                Command::Caption(take_zero_terminated(payload, &mut cursor, "caption")?)
            }
            OP_LOCATION_VIDEO if kind == RecordKind::Location => Command::Video(take_resource(
                payload,
                &mut cursor,
                RESOURCE_BYTE_MIN,
                "location video",
            )?),
            OP_TOP_ROW if kind == RecordKind::Location => {
                Command::TopRow(take_word(payload, &mut cursor, "top row")?)
            }
            OP_TALK if kind == RecordKind::Character => {
                let encoded = take_byte(payload, &mut cursor, "talk background")?;
                let background = Background::decode(encoded)
                    .ok_or_else(|| anyhow!("invalid talk background {encoded}"))?;
                Command::Talk {
                    video: take_resource(payload, &mut cursor, RESOURCE_BYTE_MIN, "talk video")?,
                    background,
                }
            }
            OP_RIGHT_VIDEO if kind == RecordKind::Character => Command::RightVideo(take_resource(
                payload,
                &mut cursor,
                RESOURCE_BYTE_MIN,
                "right video",
            )?),
            OP_LEFT_VIDEO if kind == RecordKind::Character => Command::LeftVideo(take_resource(
                payload,
                &mut cursor,
                RESOURCE_BYTE_MIN,
                "left video",
            )?),
            OP_IDLE if kind == RecordKind::Character => {
                let encoded = take_byte(payload, &mut cursor, "idle background")?;
                let background = Background::decode(encoded)
                    .ok_or_else(|| anyhow!("invalid idle background {encoded}"))?;
                Command::Idle {
                    video: take_resource(payload, &mut cursor, RESOURCE_BYTE_MIN, "idle video")?,
                    background,
                }
            }
            OP_PORTRAIT if kind == RecordKind::Character => Command::Portrait(take_resource(
                payload,
                &mut cursor,
                RESOURCE_BYTE_MIN,
                "portrait",
            )?),
            OP_SOUND_BANK if kind == RecordKind::Character => Command::SoundBank(take_resource(
                payload,
                &mut cursor,
                RESOURCE_BYTE_MIN,
                "sound bank",
            )?),
            OP_SEQUENCE_VIDEO if kind == RecordKind::Sequence => Command::Video(take_resource(
                payload,
                &mut cursor,
                RESOURCE_BYTE_MIN,
                "sequence video",
            )?),
            OP_SUBTITLE if kind == RecordKind::Sequence => {
                let first_frame = take_word(payload, &mut cursor, "subtitle frame")?;
                let text = take_zero_terminated(payload, &mut cursor, "subtitle")?;
                Command::Subtitle { first_frame, text }
            }
            OP_OBJECT_VIDEO if kind == RecordKind::Object => Command::Video(take_resource(
                payload,
                &mut cursor,
                RESOURCE_BYTE_MIN,
                "object video",
            )?),
            OP_MUSIC if matches!(kind, RecordKind::Location | RecordKind::Sequence) => {
                Command::Music(take_resource(
                    payload,
                    &mut cursor,
                    MUSIC_BYTE_MIN,
                    "music",
                )?)
            }
            _ => bail!(
                "opcode {opcode} is not valid in a {} record",
                kind.keyword()
            ),
        };
        commands.push(command);
    }
    Ok(commands)
}

fn take_byte(payload: &[u8], cursor: &mut usize, field: &str) -> Result<u8> {
    let value = payload
        .get(*cursor)
        .copied()
        .ok_or_else(|| anyhow!("missing {field}"))?;
    *cursor += 1;
    Ok(value)
}

fn take_word(payload: &[u8], cursor: &mut usize, field: &str) -> Result<u16> {
    let bytes = payload
        .get(*cursor..*cursor + 2)
        .ok_or_else(|| anyhow!("missing {field}"))?;
    *cursor += 2;
    Ok(u16::from_le_bytes(bytes.try_into().unwrap()))
}

fn take_resource(payload: &[u8], cursor: &mut usize, minimum: u8, _field: &str) -> Result<Vec<u8>> {
    let start = *cursor;
    while payload
        .get(*cursor)
        .is_some_and(|byte| (minimum..=RESOURCE_BYTE_MAX).contains(byte))
    {
        *cursor += 1;
    }
    Ok(payload[start..*cursor].to_vec())
}

fn take_zero_terminated(payload: &[u8], cursor: &mut usize, field: &str) -> Result<Vec<u8>> {
    let start = *cursor;
    while payload.get(*cursor).is_some_and(|byte| *byte != 0) {
        *cursor += 1;
    }
    if *cursor == payload.len() {
        bail!("{field} has no terminating zero");
    }
    let value = payload[start..*cursor].to_vec();
    *cursor += 1;
    Ok(value)
}

fn render_source(records: &[Record]) -> Result<String> {
    let mut source = String::from("descript 1\n");
    for record in records {
        writeln!(
            source,
            "\n{} {} {{",
            record.kind.keyword(),
            quote(&record.name)
        )?;
        for command in &record.commands {
            source.push_str("    ");
            match command {
                Command::Background { slot, image } => {
                    writeln!(
                        source,
                        "background {} {}",
                        background_view_name(*slot),
                        quote(image)
                    )?;
                }
                Command::Caption(text) => writeln!(source, "caption {}", quote(text))?,
                Command::Video(video) => writeln!(source, "video {}", quote(video))?,
                Command::TopRow(row) => writeln!(source, "top_row {row}")?,
                Command::Talk { video, background } => {
                    writeln!(source, "talk {} over {}", quote(video), background.source())?
                }
                Command::RightVideo(video) => {
                    writeln!(source, "right_video {}", quote(video))?;
                }
                Command::LeftVideo(video) => {
                    writeln!(source, "left_video {}", quote(video))?;
                }
                Command::Idle { video, background } => {
                    writeln!(source, "idle {} over {}", quote(video), background.source())?
                }
                Command::Portrait(sprite) => {
                    writeln!(source, "portrait {}", quote(sprite))?;
                }
                Command::SoundBank(bank) => {
                    writeln!(source, "sound_bank {}", quote(bank))?;
                }
                Command::Subtitle { first_frame, text } => {
                    writeln!(source, "subtitle frame={first_frame} {}", quote(text))?
                }
                Command::Music(music) => writeln!(source, "music {}", quote(music))?,
            }
        }
        source.push_str("}\n");
    }
    Ok(source)
}

fn parse_source(source: &str) -> Result<Vec<Record>> {
    let lines = source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let text = line.trim();
            (!text.is_empty() && !text.starts_with('#')).then_some((index + 1, text))
        })
        .collect::<Vec<_>>();
    let Some((header_line, header)) = lines.first().copied() else {
        bail!("DESCRIPT source is empty");
    };
    if header != "descript 1" {
        bail!("line {header_line}: source must begin with 'descript 1'");
    }

    let mut records = Vec::new();
    let mut names = HashSet::new();
    let mut cursor = 1usize;
    while cursor < lines.len() {
        let (line_number, header) = lines[cursor];
        cursor += 1;
        let (kind_text, rest) = header
            .split_once(char::is_whitespace)
            .ok_or_else(|| anyhow!("line {line_number}: malformed record declaration"))?;
        let kind = RecordKind::parse(kind_text)
            .ok_or_else(|| anyhow!("line {line_number}: unknown record kind {kind_text:?}"))?;
        let (name, trailing) = parse_quoted(rest.trim(), line_number)?;
        if trailing.trim() != "{" {
            bail!("line {line_number}: expected '{{' after the record name");
        }
        if name.is_empty() {
            bail!("line {line_number}: record name cannot be empty");
        }
        if name.len() >= DIRECTORY_NAME_BYTES {
            bail!("line {line_number}: record name exceeds 15 bytes");
        }
        if !names.insert(name.clone()) {
            bail!("line {line_number}: duplicate record name {}", quote(&name));
        }

        let mut commands = Vec::new();
        let mut closed = false;
        while cursor < lines.len() {
            let (command_line, text) = lines[cursor];
            cursor += 1;
            if text == "}" {
                closed = true;
                break;
            }
            commands.push(parse_command(kind, text, command_line)?);
        }
        if !closed {
            bail!(
                "line {line_number}: record {} has no closing brace",
                quote(&name)
            );
        }
        records.push(Record {
            name,
            kind,
            commands,
        });
    }
    if records.is_empty() {
        bail!("DESCRIPT source has no records");
    }
    if records.len() > usize::from(u16::MAX) {
        bail!("DESCRIPT source has too many records");
    }
    Ok(records)
}

fn parse_command(kind: RecordKind, text: &str, line: usize) -> Result<Command> {
    let (keyword, rest) = text
        .split_once(char::is_whitespace)
        .map_or((text, ""), |(keyword, rest)| (keyword, rest.trim()));
    match (kind, keyword) {
        (RecordKind::Location, "background") => {
            let (view_text, image_text) = rest
                .split_once(char::is_whitespace)
                .ok_or_else(|| anyhow!("line {line}: background expects VIEW and IMAGE"))?;
            let slot = parse_background_view(view_text, line)?;
            Ok(Command::Background {
                slot,
                image: parse_only_quoted(image_text.trim(), line)?,
            })
        }
        (RecordKind::Location, "caption") => Ok(Command::Caption(parse_only_quoted(rest, line)?)),
        (RecordKind::Location | RecordKind::Sequence | RecordKind::Object, "video") => {
            Ok(Command::Video(parse_only_quoted(rest, line)?))
        }
        (RecordKind::Location, "top_row") => Ok(Command::TopRow(parse_u16(rest, line, "row")?)),
        (RecordKind::Location | RecordKind::Sequence, "music") => {
            Ok(Command::Music(parse_only_quoted(rest, line)?))
        }
        (RecordKind::Character, "talk") => parse_character_clip(rest, line, true),
        (RecordKind::Character, "right_video") => {
            Ok(Command::RightVideo(parse_only_quoted(rest, line)?))
        }
        (RecordKind::Character, "left_video") => {
            Ok(Command::LeftVideo(parse_only_quoted(rest, line)?))
        }
        (RecordKind::Character, "idle") => parse_character_clip(rest, line, false),
        (RecordKind::Character, "portrait") => {
            Ok(Command::Portrait(parse_only_quoted(rest, line)?))
        }
        (RecordKind::Character, "sound_bank") => {
            Ok(Command::SoundBank(parse_only_quoted(rest, line)?))
        }
        (RecordKind::Sequence, "subtitle") => {
            let (frame, text) = rest
                .split_once(char::is_whitespace)
                .ok_or_else(|| anyhow!("line {line}: subtitle expects frame=N and TEXT"))?;
            let first_frame = frame
                .strip_prefix("frame=")
                .ok_or_else(|| anyhow!("line {line}: subtitle expects frame=N"))?;
            Ok(Command::Subtitle {
                first_frame: parse_u16(first_frame, line, "subtitle frame")?,
                text: parse_only_quoted(text.trim(), line)?,
            })
        }
        _ => bail!(
            "line {line}: command {keyword:?} is not valid in a {} record",
            kind.keyword()
        ),
    }
}

fn parse_character_clip(rest: &str, line: usize, talk: bool) -> Result<Command> {
    let (video, trailing) = parse_quoted(rest, line)?;
    let background = trailing
        .trim()
        .strip_prefix("over ")
        .ok_or_else(|| anyhow!("line {line}: clip expects 'over VIEW' or 'over none'"))?;
    let background = if background == "none" {
        Background::None
    } else {
        Background::Slot(parse_background_view(background, line)?)
    };
    Ok(if talk {
        Command::Talk { video, background }
    } else {
        Command::Idle { video, background }
    })
}

fn parse_background_view(value: &str, line: usize) -> Result<u8> {
    match value {
        "front" => Ok(1),
        "right" => Ok(2),
        "left" => Ok(3),
        "back" => Ok(4),
        _ => bail!("line {line}: background view must be front, right, left, or back"),
    }
}

fn parse_u16(value: &str, line: usize, field: &str) -> Result<u16> {
    value
        .parse::<u16>()
        .map_err(|_| anyhow!("line {line}: {field} must be a decimal number"))
}

fn parse_only_quoted(value: &str, line: usize) -> Result<Vec<u8>> {
    let (bytes, trailing) = parse_quoted(value, line)?;
    if !trailing.trim().is_empty() {
        bail!("line {line}: unexpected text after quoted value");
    }
    Ok(bytes)
}

fn parse_quoted(value: &str, line: usize) -> Result<(Vec<u8>, &str)> {
    let bytes = value.as_bytes();
    if bytes.first() != Some(&b'"') {
        bail!("line {line}: expected a quoted value");
    }
    let mut output = Vec::new();
    let mut cursor = 1usize;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        cursor += 1;
        match byte {
            b'"' => return Ok((output, &value[cursor..])),
            b'\\' => {
                let escape = *bytes
                    .get(cursor)
                    .ok_or_else(|| anyhow!("line {line}: incomplete string escape"))?;
                cursor += 1;
                match escape {
                    b'"' => output.push(b'"'),
                    b'\\' => output.push(b'\\'),
                    b'n' => output.push(b'\n'),
                    b'r' => output.push(b'\r'),
                    b't' => output.push(b'\t'),
                    b'x' => {
                        let digits = bytes
                            .get(cursor..cursor + 2)
                            .ok_or_else(|| anyhow!("line {line}: incomplete \\xNN escape"))?;
                        let digits = std::str::from_utf8(digits).unwrap();
                        output.push(u8::from_str_radix(digits, 16).map_err(|_| {
                            anyhow!("line {line}: invalid byte escape \\x{digits}")
                        })?);
                        cursor += 2;
                    }
                    _ => bail!("line {line}: unsupported string escape"),
                }
            }
            0..=127 => output.push(byte),
            _ => bail!("line {line}: non-ASCII source bytes must use \\xNN"),
        }
    }
    bail!("line {line}: unterminated quoted value")
}

fn encode_image(records: &[Record]) -> Result<Vec<u8>> {
    let encoded_commands = records
        .iter()
        .map(encode_commands)
        .collect::<Result<Vec<_>>>()?;
    let directory_end = DIRECTORY_COUNT_BYTES + records.len() * DIRECTORY_ENTRY_BYTES;
    let mut next_offset = directory_end + 1;
    let mut offsets = Vec::with_capacity(records.len());
    let mut lengths = Vec::with_capacity(records.len());
    for (index, commands) in encoded_commands.iter().enumerate() {
        let length = RECORD_LENGTH_BYTES + commands.len() + 1;
        let encoded_length = u16::try_from(length)
            .map_err(|_| anyhow!("record {} exceeds the 16-bit length domain", index + 1))?;
        let encoded_offset = u16::try_from(next_offset)
            .map_err(|_| anyhow!("record {} exceeds the 16-bit offset domain", index + 1))?;
        offsets.push(encoded_offset);
        lengths.push(encoded_length);
        next_offset = next_offset
            .checked_add(length)
            .ok_or_else(|| anyhow!("DESCRIPT image size overflow"))?;
    }

    let mut image = Vec::with_capacity(next_offset);
    image.extend_from_slice(&(records.len() as u16).to_le_bytes());
    for (record, offset) in records.iter().zip(&offsets) {
        if record.name.is_empty() || record.name.len() >= DIRECTORY_NAME_BYTES {
            bail!(
                "record name {} does not fit the directory",
                quote(&record.name)
            );
        }
        if record.name.contains(&0) {
            bail!("record name {} contains a zero byte", quote(&record.name));
        }
        image.extend_from_slice(&record.name);
        image.resize(image.len() + DIRECTORY_NAME_BYTES - record.name.len(), 0);
        image.extend_from_slice(&offset.to_le_bytes());
    }

    image.push(records[0].kind.encode());
    for index in 0..records.len() {
        image.extend_from_slice(&lengths[index].to_le_bytes());
        image.extend_from_slice(&encoded_commands[index]);
        image.push(
            records
                .get(index + 1)
                .map_or(FINAL_RECORD_MARKER, |record| record.kind.encode()),
        );
    }
    debug_assert_eq!(image.len(), next_offset);
    Ok(image)
}

fn encode_commands(record: &Record) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    for command in &record.commands {
        match (record.kind, command) {
            (RecordKind::Location, Command::Background { slot, image }) => {
                validate_resource(image, RESOURCE_BYTE_MIN, "background image")?;
                encoded.extend([OP_BACKGROUND, *slot]);
                encoded.extend_from_slice(image);
            }
            (RecordKind::Location, Command::Caption(text)) => {
                validate_zero_terminated_value(text, "caption")?;
                encoded.push(OP_CAPTION);
                encoded.extend_from_slice(text);
                encoded.push(0);
            }
            (RecordKind::Location, Command::Video(video)) => {
                append_resource(&mut encoded, OP_LOCATION_VIDEO, video, "location video")?;
            }
            (RecordKind::Location, Command::TopRow(row)) => {
                encoded.push(OP_TOP_ROW);
                encoded.extend_from_slice(&row.to_le_bytes());
            }
            (RecordKind::Character, Command::Talk { video, background }) => {
                validate_resource(video, RESOURCE_BYTE_MIN, "talk video")?;
                encoded.extend([OP_TALK, background.encode()]);
                encoded.extend_from_slice(video);
            }
            (RecordKind::Character, Command::RightVideo(video)) => {
                append_resource(&mut encoded, OP_RIGHT_VIDEO, video, "right video")?;
            }
            (RecordKind::Character, Command::LeftVideo(video)) => {
                append_resource(&mut encoded, OP_LEFT_VIDEO, video, "left video")?;
            }
            (RecordKind::Character, Command::Idle { video, background }) => {
                validate_resource(video, RESOURCE_BYTE_MIN, "idle video")?;
                encoded.extend([OP_IDLE, background.encode()]);
                encoded.extend_from_slice(video);
            }
            (RecordKind::Character, Command::Portrait(sprite)) => {
                append_resource(&mut encoded, OP_PORTRAIT, sprite, "portrait")?;
            }
            (RecordKind::Character, Command::SoundBank(bank)) => {
                append_resource(&mut encoded, OP_SOUND_BANK, bank, "sound bank")?;
            }
            (RecordKind::Sequence, Command::Video(video)) => {
                append_resource(&mut encoded, OP_SEQUENCE_VIDEO, video, "sequence video")?;
            }
            (RecordKind::Sequence, Command::Subtitle { first_frame, text }) => {
                validate_zero_terminated_value(text, "subtitle")?;
                encoded.push(OP_SUBTITLE);
                encoded.extend_from_slice(&first_frame.to_le_bytes());
                encoded.extend_from_slice(text);
                encoded.push(0);
            }
            (RecordKind::Object, Command::Video(video)) => {
                append_resource(&mut encoded, OP_OBJECT_VIDEO, video, "object video")?;
            }
            (RecordKind::Location | RecordKind::Sequence, Command::Music(music)) => {
                append_resource_with_minimum(
                    &mut encoded,
                    OP_MUSIC,
                    music,
                    MUSIC_BYTE_MIN,
                    "music",
                )?;
            }
            _ => bail!(
                "command is not valid in record {} of kind {}",
                quote(&record.name),
                record.kind.keyword()
            ),
        }
    }
    Ok(encoded)
}

fn append_resource(encoded: &mut Vec<u8>, opcode: u8, value: &[u8], field: &str) -> Result<()> {
    append_resource_with_minimum(encoded, opcode, value, RESOURCE_BYTE_MIN, field)
}

fn append_resource_with_minimum(
    encoded: &mut Vec<u8>,
    opcode: u8,
    value: &[u8],
    minimum: u8,
    field: &str,
) -> Result<()> {
    validate_resource(value, minimum, field)?;
    encoded.push(opcode);
    encoded.extend_from_slice(value);
    Ok(())
}

fn validate_resource(value: &[u8], minimum: u8, field: &str) -> Result<()> {
    if value.is_empty() {
        bail!("{field} cannot be empty");
    }
    if value
        .iter()
        .any(|byte| !(minimum..=RESOURCE_BYTE_MAX).contains(byte))
    {
        bail!("{field} contains a byte outside the serialized printable range");
    }
    Ok(())
}

fn validate_zero_terminated_value(value: &[u8], field: &str) -> Result<()> {
    if value.contains(&0) {
        bail!("{field} contains an embedded zero byte");
    }
    Ok(())
}

fn quote(value: &[u8]) -> String {
    let mut output = String::from("\"");
    for byte in value {
        match *byte {
            b'"' => output.push_str("\\\""),
            b'\\' => output.push_str("\\\\"),
            b'\n' => output.push_str("\\n"),
            b'\r' => output.push_str("\\r"),
            b'\t' => output.push_str("\\t"),
            32..=126 => output.push(char::from(*byte)),
            other => write!(output, "\\x{other:02X}").expect("writing to String cannot fail"),
        }
    }
    output.push('"');
    output
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    fn original_asset() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("accuracy/cblood_install/cblood/DESCRIPT.DES")
    }

    #[test]
    fn shipped_descript_decompiles_and_recompiles_byte_exactly() {
        let original = std::fs::read(original_asset()).unwrap();
        let decompiled = decompile(&original).unwrap();
        assert_eq!(decompiled.record_count, 145);
        assert_eq!(decompiled.command_count, 1221);
        assert_eq!(compile(&decompiled.source).unwrap(), original);
        assert!(decompiled.source.contains("location \"Pterra\" {"));
        assert!(decompiled.source.contains("music \"ulysse.voc\""));
        assert!(decompiled.source.contains("character \"Scruter_Mac\" {"));
        assert!(decompiled.source.contains("talk \"scr01.hnm\" over back"));
    }

    #[test]
    fn canonical_source_matches_the_complete_shipped_database() {
        let original = std::fs::read(original_asset()).unwrap();
        let source_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("re/descript/DESCRIPT.descript");
        let source = std::fs::read_to_string(source_path).unwrap();
        assert_eq!(compile(&source).unwrap(), original);
        assert_eq!(decompile(&original).unwrap().source, source);
    }

    #[test]
    fn source_order_derives_directory_offsets_lengths_and_record_markers() {
        let source = r#"descript 1

location "world" {
    caption "planet world\r"
    background front "world.lbm"
    video "world.hnm"
    top_row 40
    music "world.voc"
}

character "speaker" {
    sound_bank "speaker.snd"
    talk "talk.hnm" over front
    idle "idle.hnm" over none
}

sequence "intro" {
    video "intro.hnm"
    subtitle frame=30 "Welcome"
    music "intro.voc"
}

object "key" {
    video "key.hnm"
}
"#;
        let image = compile(source).unwrap();
        let decompiled = decompile(&image).unwrap();
        assert_eq!(decompiled.source, source);
        assert_eq!(decompiled.record_count, 4);
        assert_eq!(decompiled.command_count, 12);
    }

    #[test]
    fn numeric_background_slots_are_not_accepted_as_source_syntax() {
        let source = r#"descript 1

location "world" {
    background 2 "world.lbm"
}
"#;
        let error = compile(source).unwrap_err().to_string();
        assert!(
            error.contains("background view must be front, right, left, or back"),
            "{error}"
        );
    }
}
