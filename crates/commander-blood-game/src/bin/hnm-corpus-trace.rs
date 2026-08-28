//! Emit deterministic Rust-decoder results for every compressed HNM frame.

use std::ffi::OsStr;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use commander_blood_game::native::bloodprg::{
    PresentationPayload, PresentationPayloadKind, decode_presentation_payload,
    decode_presentation_rect, presentation_payload_kind,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const WORD_BYTE_COUNT: usize = size_of::<u16>();
const DOUBLE_WORD_BYTE_COUNT: usize = size_of::<u32>();
const HNM_HEADER_EXTENT_BYTE_COUNT: usize = WORD_BYTE_COUNT;
const ENTRY_HEADER_BYTE_COUNT: usize = WORD_BYTE_COUNT;
const SIDE_RECORD_HEADER_BYTE_COUNT: usize = WORD_BYTE_COUNT * 2;
const PALETTE_BLOCK_HEADER_BYTE_COUNT: usize = 2;
const SOUND_RECORD_MARKER: u16 = u16::from_le_bytes([b's', b'd']);
const PALETTE_RECORD_MARKER: u16 = u16::from_le_bytes([b'p', b'l']);
const LINK_RECORD_MARKER: u16 = u16::from_le_bytes([b'm', b'm']);
const COMPRESSED_LAYOUT_FLAG: u16 = 0x0200;
const PALETTE_TERMINATOR: [u8; 2] = [u8::MAX, u8::MAX];
const METADATA_PADDING_BYTE: u8 = u8::MAX;
const COMPLETE_PALETTE_COLOR_COUNT: usize = 256;
const RGB_COMPONENT_COUNT: usize = 3;
const TRANSPARENT_ROW_MODE: u8 = u8::MAX;
const SEGMENT_BYTE_COUNT: usize = 65_536;
const STAGING_PATTERN_STEP: usize = 23;
const STAGING_PATTERN_PAGE_STEP: usize = 11;
const STAGING_PATTERN_SEED: usize = 31;
const FRAMEBUFFER_PATTERN_STEP: usize = 37;
const FRAMEBUFFER_PATTERN_PAGE_STEP: usize = 13;
const FRAMEBUFFER_PATTERN_SEED: usize = 19;

#[derive(Serialize)]
struct DecodeTrace {
    resource: String,
    frame_index: usize,
    entry_offset: usize,
    payload_offset: usize,
    payload_length: usize,
    layout: u16,
    row_mode: u16,
    codec: &'static str,
    decoded_length: usize,
    consumed_bytes: usize,
    decoded_sha256: String,
    staging_sha256: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CodecFilter {
    Any,
    Ab,
    Ad,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TraceMode {
    Payload,
    TransparentRectangle,
}

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let input = arguments
        .next()
        .map(PathBuf::from)
        .context("usage: hnm-corpus-trace <loose-resource-root> [--codec ab|ad]")?;
    let mut codec_filter = CodecFilter::Any;
    let mut trace_mode = TraceMode::Payload;
    while let Some(argument) = arguments.next() {
        if argument == "--rect" {
            trace_mode = TraceMode::TransparentRectangle;
            continue;
        }
        if argument != "--codec" {
            bail!("unknown argument {}", argument.to_string_lossy());
        }
        codec_filter = match arguments
            .next()
            .context("--codec requires ab or ad")?
            .to_string_lossy()
            .as_ref()
        {
            "ab" => CodecFilter::Ab,
            "ad" => CodecFilter::Ad,
            codec => bail!("unsupported codec filter {codec}; expected ab or ad"),
        };
    }
    let (root, mut resources) = if input.is_file() {
        let root = input
            .parent()
            .context("HNM input file has no parent directory")?
            .to_owned();
        (root, vec![input])
    } else {
        let mut resources = Vec::new();
        collect_hnm_resources(&input, &mut resources)?;
        (input, resources)
    };
    resources.sort();
    if resources.is_empty() {
        bail!("no HNM resources found beneath {}", root.display());
    }

    let stdout = std::io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    for path in resources {
        trace_resource(&root, &path, codec_filter, trace_mode, &mut output)?;
    }
    output.flush().context("flushing HNM corpus trace")?;
    Ok(())
}

fn collect_hnm_resources(directory: &Path, resources: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(directory)
        .with_context(|| format!("reading resource directory {}", directory.display()))?
    {
        let entry = entry.with_context(|| format!("reading entry in {}", directory.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_hnm_resources(&path, resources)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(OsStr::new("hnm")))
        {
            resources.push(path);
        }
    }
    Ok(())
}

fn trace_resource(
    root: &Path,
    path: &Path,
    codec_filter: CodecFilter,
    trace_mode: TraceMode,
    output: &mut impl Write,
) -> Result<()> {
    let source =
        std::fs::read(path).with_context(|| format!("reading HNM resource {}", path.display()))?;
    let header_extent = usize::from(read_word(&source, 0, path)?);
    if header_extent > source.len() {
        bail!(
            "{}: header extent {header_extent} exceeds {} bytes",
            path.display(),
            source.len()
        );
    }
    let metadata_position = palette_metadata_position(&source, header_extent, path)?;
    let table = source
        .get(metadata_position..header_extent)
        .context("validated HNM offset table disappeared")?;
    if table.len() % DOUBLE_WORD_BYTE_COUNT != 0 {
        bail!(
            "{}: frame-offset table is not dword aligned",
            path.display()
        );
    }
    let offsets: Vec<_> = table
        .chunks_exact(DOUBLE_WORD_BYTE_COUNT)
        .map(|bytes| u32::from_le_bytes(bytes.try_into().expect("four-byte HNM offset")) as usize)
        .collect();
    if offsets.len() < 2 {
        bail!(
            "{}: frame-offset table has no terminal offset",
            path.display()
        );
    }

    let relative = path
        .strip_prefix(root)
        .with_context(|| format!("{} is not beneath {}", path.display(), root.display()))?;
    let resource = relative
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    for (frame_index, frame_offsets) in offsets.windows(2).enumerate() {
        trace_frame(
            &source,
            path,
            &resource,
            header_extent,
            frame_index,
            frame_offsets[0],
            frame_offsets[1],
            codec_filter,
            trace_mode,
            output,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn trace_frame(
    source: &[u8],
    path: &Path,
    resource: &str,
    header_extent: usize,
    frame_index: usize,
    relative_start: usize,
    relative_end: usize,
    codec_filter: CodecFilter,
    trace_mode: TraceMode,
    output: &mut impl Write,
) -> Result<()> {
    let entry_start = header_extent
        .checked_add(relative_start)
        .context("HNM entry start overflow")?;
    let next_entry_start = header_extent
        .checked_add(relative_end)
        .context("HNM entry end overflow")?;
    if entry_start > next_entry_start || next_entry_start > source.len() {
        bail!(
            "{} frame {frame_index}: invalid offset range {entry_start}..{next_entry_start}",
            path.display()
        );
    }
    let entry_extent = usize::from(read_word(source, entry_start, path)?);
    let entry_end = entry_start
        .checked_add(entry_extent)
        .filter(|end| *end <= next_entry_start && *end <= source.len())
        .with_context(|| {
            format!(
                "{} frame {frame_index}: entry extent {entry_extent} exceeds its offset range",
                path.display()
            )
        })?;
    let mut cursor = entry_start + ENTRY_HEADER_BYTE_COUNT;
    let mut marker_start = cursor;
    let mut layout = read_word(source, cursor, path)?;

    if layout == SOUND_RECORD_MARKER {
        cursor = side_record_end(source, marker_start, entry_end, path, frame_index)?;
        marker_start = cursor;
        layout = read_word(source, cursor, path)?;
    }
    while layout == PALETTE_RECORD_MARKER {
        cursor = side_record_end(source, marker_start, entry_end, path, frame_index)?;
        marker_start = cursor;
        layout = read_word(source, cursor, path)?;
    }
    if layout == LINK_RECORD_MARKER {
        return Ok(());
    }
    let row_mode_position = cursor + WORD_BYTE_COUNT;
    let row_mode = read_word(source, row_mode_position, path)?;
    let rows = row_mode.to_le_bytes()[0];
    if rows == u8::MIN || layout & COMPRESSED_LAYOUT_FLAG == 0 {
        return Ok(());
    }
    let payload_offset = row_mode_position + WORD_BYTE_COUNT;
    let payload = source.get(payload_offset..entry_end).with_context(|| {
        format!(
            "{} frame {frame_index}: payload is truncated",
            path.display()
        )
    })?;
    let kind = presentation_payload_kind(payload).with_context(|| {
        format!(
            "{} frame {frame_index}: Rust presentation decoder rejected the signature",
            path.display()
        )
    })?;
    if trace_mode == TraceMode::TransparentRectangle {
        if kind != PresentationPayloadKind::Ad || row_mode.to_le_bytes()[1] != TRANSPARENT_ROW_MODE
        {
            return Ok(());
        }
        let mut staging = patterned_segment(
            STAGING_PATTERN_STEP,
            STAGING_PATTERN_PAGE_STEP,
            STAGING_PATTERN_SEED,
        );
        let mut framebuffer = patterned_segment(
            FRAMEBUFFER_PATTERN_STEP,
            FRAMEBUFFER_PATTERN_PAGE_STEP,
            FRAMEBUFFER_PATTERN_SEED,
        );
        let outcome = decode_presentation_rect(
            payload,
            &mut staging,
            &mut framebuffer,
            usize::MIN,
            layout,
            row_mode,
        )
        .with_context(|| {
            format!(
                "{} frame {frame_index}: Rust rectangular decoder rejected the payload",
                path.display()
            )
        })?;
        let record = DecodeTrace {
            resource: resource.to_owned(),
            frame_index,
            entry_offset: entry_start,
            payload_offset,
            payload_length: payload.len(),
            layout,
            row_mode,
            codec: "rect_ad",
            decoded_length: framebuffer.len(),
            consumed_bytes: outcome.consumed_bytes,
            decoded_sha256: format!("{:x}", Sha256::digest(&framebuffer)),
            staging_sha256: Some(format!("{:x}", Sha256::digest(&staging))),
        };
        write_trace(output, path, frame_index, &record)?;
        return Ok(());
    }
    if matches!(
        (codec_filter, kind),
        (CodecFilter::Ab, PresentationPayloadKind::Ad)
            | (CodecFilter::Ad, PresentationPayloadKind::Ab)
    ) {
        return Ok(());
    }
    let decoded = decode_presentation_payload(payload).with_context(|| {
        format!(
            "{} frame {frame_index}: Rust presentation decoder rejected the payload",
            path.display()
        )
    })?;
    let (codec, bytes, consumed_bytes) = match decoded {
        PresentationPayload::Ab(outcome) => ("ab", outcome.bytes, outcome.consumed_bytes),
        PresentationPayload::Ad(outcome) => ("ad", outcome.bytes, outcome.consumed_bytes),
        PresentationPayload::Unrecognized { checksum } => {
            bail!(
                "{} frame {frame_index}: compressed payload has checksum {checksum:#04x}",
                path.display()
            )
        }
    };
    let record = DecodeTrace {
        resource: resource.to_owned(),
        frame_index,
        entry_offset: entry_start,
        payload_offset,
        payload_length: payload.len(),
        layout,
        row_mode,
        codec,
        decoded_length: bytes.len(),
        consumed_bytes,
        decoded_sha256: format!("{:x}", Sha256::digest(&bytes)),
        staging_sha256: None,
    };
    write_trace(output, path, frame_index, &record)?;
    Ok(())
}

fn write_trace(
    output: &mut impl Write,
    path: &Path,
    frame_index: usize,
    record: &DecodeTrace,
) -> Result<()> {
    serde_json::to_writer(&mut *output, record)
        .with_context(|| format!("encoding trace for {} frame {frame_index}", path.display()))?;
    output.write_all(b"\n").context("writing trace newline")?;
    Ok(())
}

fn patterned_segment(step: usize, page_step: usize, seed: usize) -> Vec<u8> {
    (usize::MIN..SEGMENT_BYTE_COUNT)
        .map(|offset| (offset * step + (offset >> u8::BITS) * page_step + seed) as u8)
        .collect()
}

fn palette_metadata_position(source: &[u8], header_extent: usize, path: &Path) -> Result<usize> {
    let mut cursor = HNM_HEADER_EXTENT_BYTE_COUNT;
    loop {
        let header = source
            .get(cursor..cursor + PALETTE_BLOCK_HEADER_BYTE_COUNT)
            .filter(|_| cursor + PALETTE_BLOCK_HEADER_BYTE_COUNT <= header_extent)
            .with_context(|| format!("{}: bootstrap palette is truncated", path.display()))?;
        cursor += header.len();
        if header == PALETTE_TERMINATOR {
            break;
        }
        let color_count = if header[1] == u8::MIN {
            COMPLETE_PALETTE_COLOR_COUNT
        } else {
            usize::from(header[1])
        };
        let component_count = color_count
            .checked_mul(RGB_COMPONENT_COUNT)
            .context("bootstrap palette extent overflow")?;
        cursor = cursor
            .checked_add(component_count)
            .filter(|position| *position <= header_extent)
            .with_context(|| format!("{}: bootstrap palette exceeds its header", path.display()))?;
    }
    while cursor < header_extent && source[cursor] == METADATA_PADDING_BYTE {
        cursor += 1;
    }
    Ok(cursor)
}

fn side_record_end(
    source: &[u8],
    start: usize,
    entry_end: usize,
    path: &Path,
    frame_index: usize,
) -> Result<usize> {
    let extent_position = start + WORD_BYTE_COUNT;
    let extent = usize::from(read_word(source, extent_position, path)?);
    start
        .checked_add(extent)
        .filter(|end| extent >= SIDE_RECORD_HEADER_BYTE_COUNT && *end <= entry_end)
        .with_context(|| {
            format!(
                "{} frame {frame_index}: side-record extent {extent} is invalid",
                path.display()
            )
        })
}

fn read_word(source: &[u8], position: usize, path: &Path) -> Result<u16> {
    let bytes = source
        .get(position..position + WORD_BYTE_COUNT)
        .with_context(|| format!("{}: word at {position} is truncated", path.display()))?;
    Ok(u16::from_le_bytes(
        bytes.try_into().expect("validated two-byte HNM word"),
    ))
}
