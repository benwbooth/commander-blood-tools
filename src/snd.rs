use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

pub const SND_CLIP_HEADER_LEN: usize = 6;
pub const SND_PCM_FORMAT_TAG: u8 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SndClip {
    pub original_index: usize,
    pub file_offset: usize,
    pub pcm_file_offset: usize,
    pub sample_rate_code: u8,
    pub sample_rate: u32,
    pub pcm: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SndBank {
    header_end: usize,
    clips: Vec<Option<SndClip>>,
}

impl SndBank {
    pub fn read(path: &Path) -> Result<Self> {
        let data = fs::read(path).with_context(|| format!("read SND bank {}", path.display()))?;
        Self::parse(&data)
    }

    /// Parse the SND bank layout consumed by BLOODPRG.EXE's `snd_clip_player`.
    ///
    /// The recovered player enters with AX as the original clip index, resolves
    /// that index through the bank offset table, skips the 6-byte per-clip
    /// header, then streams unsigned 8-bit PCM bytes to the SND driver.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 6 {
            bail!("SND file is too small for a header");
        }

        let clip_count = u16::from_le_bytes([data[0], data[1]]) as usize;
        let header_end = 4usize
            .checked_add(
                (clip_count + 1)
                    .checked_mul(4)
                    .context("SND clip table size overflow")?,
            )
            .context("SND header size overflow")?;
        if header_end > data.len() {
            bail!("SND clip table extends past end of file");
        }

        let mut offsets = Vec::with_capacity(clip_count + 1);
        for idx in 0..=clip_count {
            let pos = 4 + idx * 4;
            offsets.push(u32::from_le_bytes([
                data[pos],
                data[pos + 1],
                data[pos + 2],
                data[pos + 3],
            ]) as usize);
        }

        let mut clips = vec![None; clip_count];
        for clip_index in 0..clip_count {
            let clip_start = match header_end.checked_add(offsets[clip_index]) {
                Some(offset) => offset,
                None => continue,
            };
            let clip_end = match header_end.checked_add(offsets[clip_index + 1]) {
                Some(offset) => offset,
                None => continue,
            };
            let pcm_start = match clip_start.checked_add(SND_CLIP_HEADER_LEN) {
                Some(offset) => offset,
                None => continue,
            };
            if pcm_start > data.len() || clip_end > data.len() || clip_end < pcm_start {
                continue;
            }
            if data[clip_start] != SND_PCM_FORMAT_TAG {
                continue;
            }

            let sample_rate_code = data[clip_start + 4];
            clips[clip_index] = Some(SndClip {
                original_index: clip_index,
                file_offset: clip_start,
                pcm_file_offset: pcm_start,
                sample_rate_code,
                sample_rate: snd_sample_rate(sample_rate_code),
                pcm: data[pcm_start..clip_end].to_vec(),
            });
        }

        Ok(Self { header_end, clips })
    }

    pub fn header_end(&self) -> usize {
        self.header_end
    }

    pub fn clip_count(&self) -> usize {
        self.clips.len()
    }

    pub fn clip(&self, original_index: usize) -> Option<&SndClip> {
        self.clips.get(original_index)?.as_ref()
    }

    pub fn clips(&self) -> impl Iterator<Item = &SndClip> {
        self.clips.iter().filter_map(Option::as_ref)
    }
}

pub fn snd_sample_rate(sample_rate_code: u8) -> u32 {
    if sample_rate_code < 255 {
        1_000_000 / (256 - sample_rate_code as u32)
    } else {
        11111
    }
}

/// Parse a Creative VOC file (the game's `mu/*.voc` music) into its unsigned 8-bit
/// mono PCM samples + sample rate. Handles block type 1 (sound data: u24 length,
/// time-constant byte, codec byte, samples; codec 0 = raw u8 PCM) and type 2
/// (continuation), skipping other block types; stops at the type-0 terminator.
/// Returns `None` if the header magic is missing or no PCM block is found.
pub fn parse_voc_pcm(data: &[u8]) -> Option<(Vec<u8>, u32)> {
    const MAGIC: &[u8] = b"Creative Voice File\x1a";
    if !data.starts_with(MAGIC) || data.len() < 26 {
        return None;
    }
    let header_size = u16::from_le_bytes([data[20], data[21]]) as usize;
    let mut pos = header_size;
    let mut pcm = Vec::new();
    let mut rate: Option<u32> = None;
    while pos < data.len() {
        let block_type = data[pos];
        if block_type == 0 {
            break; // terminator
        }
        if pos + 4 > data.len() {
            break;
        }
        let len = u32::from_le_bytes([data[pos + 1], data[pos + 2], data[pos + 3], 0]) as usize;
        let body = pos + 4;
        let end = (body + len).min(data.len());
        match block_type {
            1 if len >= 2 => {
                let tc = data[body];
                let codec = data[body + 1];
                if codec == 0 {
                    rate.get_or_insert_with(|| snd_sample_rate(tc));
                    pcm.extend_from_slice(&data[body + 2..end]);
                }
            }
            2 => pcm.extend_from_slice(&data[body..end]),
            _ => {} // markers, silence, repeat blocks: skip
        }
        pos = body + len;
    }
    rate.filter(|_| !pcm.is_empty()).map(|r| (pcm, r))
}

/// Mix one unsigned 8-bit SND sample into another.
///
/// This ports BLOODPRG.EXE `0xBB6D..0xBB74`: `lodsb; add al,es:[di];
/// rcr al,1; stosb`. The add carry becomes bit 7 during the rotate, which is
/// exactly `floor((source + destination) / 2)` for two u8 samples.
pub fn snd_mix_average(source: u8, destination: u8) -> u8 {
    ((source as u16 + destination as u16) / 2) as u8
}

/// The `rep`-style loop around [`snd_mix_average`] — `0xBB6D..0xBB74` runs
/// `lodsb / add al,es:[di] / rcr al,1 / stosb` per sample, so mixing a buffer is
/// that pair applied element-wise over the shorter of the two.
pub fn mix_unsigned_pcm_average(destination: &mut [u8], source: &[u8]) -> usize {
    let len = destination.len().min(source.len());
    for idx in 0..len {
        destination[idx] = snd_mix_average(source[idx], destination[idx]);
    }
    len
}

/// The two voice buffers the streamer alternates between, `0xBBE4..0xBC2F`.
///
/// ```text
///   0xBBE7  les di,[0xbb7]              the loaded sound data
///   0xBBEB  mov [si],di / [si+2],es     voice A points AT IT
///   0xBBF0  mov word [si+4],0x4000      length 16384
///   0xBC1E  add di,0x4008               voice B starts one buffer + 8 later
///   0xBC2F  mov byte [si+6],0           and starts NOT in state 3
/// ```
///
/// This answers what a voice buffer holds before the stream mixes into it, which
/// `docs/port-validation.md` had open: the LOADED SOUND DATA. Nothing fills it
/// with silence — `les di,[0xbb7]` is the file, and the two voices are its two
/// halves. So a lone sound is not halved toward `0x80`; the mix at `0xBB6D`
/// averages the incoming chunk with sound already there.
pub const SND_VOICE_BUFFER_LEN: u16 = 0x4000;
/// Voice B begins `0x4008` past voice A (`add di,0x4008` @`0xBC1E`) — the buffer
/// length plus the 8 bytes of gap the header occupies.
pub const SND_VOICE_BUFFER_STRIDE: u16 = 0x4008;
/// `cmp byte es:[di+4],0xd3` @`0xBBFE`: the sound file's header byte at `+4`, and
/// the ONLY thing that sets the half-rate flag `gs:[0xBA2]` (`0xBC05`).
///
/// As a Sound Blaster time constant, `0xD3` is `1000000/(256-211)` = 22222 Hz —
/// the rate that needs decimating to reach the device. The port must read this
/// from the data, never assume it: it is exactly the kind of content-bearing
/// value CLAUDE.md forbids hardcoding a decision about.
pub const SND_HEADER_HALF_RATE_TIME_CONSTANT: u8 = 0xD3;

/// Does this sound's header select the half-rate mix loop? `0xBBFE`/`0xBC05`.
///
/// `header` is the file's leading bytes; the byte at `+4` decides. A header too
/// short to contain it selects full rate, since the compare cannot match.
pub fn snd_header_is_half_rate(header: &[u8]) -> bool {
    header.get(4).copied() == Some(SND_HEADER_HALF_RATE_TIME_CONSTANT)
}

/// The two voices' `(offset, length)` within the loaded sound data, `0xBBE4`
/// and `0xBC1E`..`0xBC2A`.
pub fn snd_voice_buffer_spans() -> [(u16, u16); 2] {
    [
        (0, SND_VOICE_BUFFER_LEN),
        (SND_VOICE_BUFFER_STRIDE, SND_VOICE_BUFFER_LEN),
    ]
}

/// Where a streamed chunk lands in a voice's ring buffer, `0xBB2E..0xBB4E`.
///
/// The mixer is a STREAMER, which the prologue makes plain: `0xBAF7 mov ah,0x3F /
/// int 0x21` reads the next chunk from a file, `0xBAFD sub cx,6` drops a 6-byte
/// header, and `0xBB0B add cx,cx` DOUBLES the output count when the half-rate flag
/// `gs:[0xBA2]` is set — the same flag that picks the `0xBB5B` loop, so the two
/// halves of that decode confirm each other from unrelated instructions.
///
/// It writes into whichever of exactly TWO voice structs is in state 3
/// (`0xBB0D mov bp,0xb89`, `0xBB10 mov dx,0xb91`, `cmp byte [bp+6],3`), at an
/// offset derived from the device's play position:
///
/// ```text
///   0xBB28  lcall gs:[0xcf3]         AX = the play position
///   0xBB2E  cmp ax,-1 / je           no position -> nothing to do
///   0xBB33  sub ax,[bp+4] / jns / neg    offset = |position - length|
///   0xBB3A  mov bx,cx                    BX carries the sample count
///   0xBB3C  cmp ax,[bp+4] / jae          past the end -> it is ALL remainder
///   0xBB41  add di,ax                    otherwise write at that offset
///   0xBB43  sub ax,[bp+4] / neg ax       space = length - offset
///   0xBB48  sub bx,ax / js               all of it fits...
///   0xBB4C  mov cx,ax                    ... or clamp to the space and carry BX
/// ```
///
/// `0xBB76 or bx,bx` then runs the leftover, so this is a ring buffer written in
/// up to two spans. Returns the first span; `remainder` is what wraps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamMixSpan {
    /// Where in the voice buffer this span starts (`add di,ax` @`0xBB41`).
    pub offset: u16,
    /// How many samples this span mixes (`cx` after the clamp @`0xBB4C`).
    pub count: u16,
    /// What is left for the wrap pass (`bx` at `0xBB76`).
    pub remainder: u16,
}

/// `0xBB2E..0xBB4E`. `position` is the play cursor `gs:[0xcf3]` returns; `0xFFFF`
/// (its `-1`) means the device gave none and nothing is mixed.
pub fn stream_mix_span(position: u16, buffer_len: u16, samples: u16) -> Option<StreamMixSpan> {
    if position == 0xFFFF {
        return None; // 0xBB2E
    }
    if buffer_len == 0 {
        return None; // no buffer to land in; the game always has one
    }
    // 0xBB33..0xBB38: the absolute difference, computed with a sign test and neg.
    let offset = if position >= buffer_len {
        position - buffer_len
    } else {
        buffer_len - position
    };
    if offset >= buffer_len {
        // 0xBB3C `jae 0xBB76`: this pass writes nothing; it is all remainder.
        return Some(StreamMixSpan { offset, count: 0, remainder: samples });
    }
    let space = buffer_len - offset; // 0xBB43..0xBB46
    let count = samples.min(space); // 0xBB48/0xBB4C
    Some(StreamMixSpan {
        offset,
        count,
        remainder: samples.saturating_sub(space),
    })
}

/// The HALF-RATE mix loop, `0xBB5B..0xBB69` — the variant the normal loop at
/// `0xBB6D` is chosen against by `test byte gs:[0xba2],1` @`0xBB53`.
///
/// ```text
///   0xBB5B  mov al,[si]        read WITHOUT advancing
///   0xBB5D  test cl,1
///   0xBB60  jne 0xBB63         counter ODD  -> do not advance
///   0xBB62  inc si             counter EVEN -> advance
///   0xBB63  add al,es:[di] / rcr al,1 / stosb    the same average as 0xBB6D
///   0xBB69  loop 0xBB5B
/// ```
///
/// So the source is consumed once per TWO output samples: the voice plays an
/// octave down, at half its sample rate, mixed by the identical average. The
/// distinction from `0xBB6D` is only the `si` advance — the mixing arithmetic is
/// shared, which is why [`snd_mix_average`] serves both.
///
/// `count` is the loop counter CX, and its parity sets the phase: an even count
/// advances on the first sample (`A,B,B,C,C…`), an odd count holds the first
/// sample instead (`A,A,B,B…`). Both are half rate; the game's phase depends on
/// where the buffer boundary falls, so the port reproduces the counter rather
/// than picking one.
///
/// Returns the number of destination samples written.
pub fn mix_unsigned_pcm_half_rate(destination: &mut [u8], source: &[u8], count: u16) -> usize {
    let mut src = 0usize;
    let mut counter = count;
    let mut written = 0usize;
    for slot in destination.iter_mut() {
        if counter == 0 || src >= source.len() {
            break; // `loop` exhausted, or the source ran out
        }
        let sample = source[src]; // 0xBB5B: read first
        if counter & 1 == 0 {
            src += 1; // 0xBB62: advance only on an EVEN counter
        }
        *slot = snd_mix_average(sample, *slot); // 0xBB63..0xBB68
        written += 1;
        counter -= 1; // the `loop` instruction's implicit dec
    }
    written
}

/// Mix several u8 PCM sources into one buffer with the game's rule, in order.
///
/// `0xBB6D`'s `lodsb / add al,es:[di] / rcr al,1 / stosb` averages ONE source into
/// the destination, so mixing N sources is that applied N times — and the result
/// is order-dependent by construction: an earlier source is halved again by every
/// later mix, so the LAST source dominates. That is not a rounding artefact to be
/// corrected into an equal-weight average; it is what the routine does.
///
/// The destination starts at `SILENCE` (0x80, the unsigned-PCM zero level) rather
/// than 0, because 0 is full-negative and would drag every mix toward it.
///
/// Returns the number of samples written, which is the length of the LONGEST
/// source — shorter sources stop contributing once exhausted, leaving the running
/// mix to continue rather than truncating it.
pub fn mix_unsigned_pcm_sources(sources: &[&[u8]], out: &mut [u8]) -> usize {
    out.fill(SILENCE);
    let mut written = 0usize;
    for source in sources {
        let len = source.len().min(out.len());
        for idx in 0..len {
            out[idx] = snd_mix_average(source[idx], out[idx]);
        }
        written = written.max(len);
    }
    written
}

/// The unsigned-PCM zero level: `0x80`, not `0`.
pub const SILENCE: u8 = 0x80;

#[cfg(test)]
mod tests {

    /// `0xBBFE`/`0xBC05`: the half-rate flag is DATA -- a header byte, not a
    /// setting the port may choose.
    #[test]
    fn half_rate_comes_from_the_sound_headers_time_constant() {
        let mut header = [0u8; 6];
        assert!(!snd_header_is_half_rate(&header), "0 is not 0xD3");
        header[4] = SND_HEADER_HALF_RATE_TIME_CONSTANT;
        assert!(snd_header_is_half_rate(&header));
        // A neighbouring byte must not trigger it -- the compare is at +4 only.
        let mut other = [0u8; 6];
        other[3] = SND_HEADER_HALF_RATE_TIME_CONSTANT;
        other[5] = SND_HEADER_HALF_RATE_TIME_CONSTANT;
        assert!(!snd_header_is_half_rate(&other));
        // Too short to hold the byte -> full rate, not a panic.
        assert!(!snd_header_is_half_rate(&[0xD3, 0xD3]));

        // The two voices are the loaded data's two halves (0xBBF0/0xBC1E).
        let spans = snd_voice_buffer_spans();
        assert_eq!(spans[0], (0, 0x4000));
        assert_eq!(spans[1], (0x4008, 0x4000));
        assert_eq!(
            spans[1].0 - spans[0].1,
            8,
            "the 8-byte gap between them is the header"
        );
    }

    /// `0xBB2E..0xBB4E`: the ring-buffer span, including the wrap remainder.
    #[test]
    fn stream_mix_span_clamps_to_the_buffer_and_carries_the_wrap() {
        // -1 from gs:[0xcf3] means no play position (0xBB2E).
        assert_eq!(stream_mix_span(0xFFFF, 0x400, 0x100), None);

        // Everything fits: BX goes negative at 0xBB48, so nothing wraps.
        let span = stream_mix_span(0x300, 0x400, 0x40).unwrap();
        assert_eq!(span.offset, 0x100, "|position - length|");
        assert_eq!(span.count, 0x40);
        assert_eq!(span.remainder, 0, "js 0xBB4E -- all of it fits");

        // More samples than space: cx is clamped and the rest wraps. Note the
        // offset is |position - length|, so a SMALL position gives a LARGE offset
        // and therefore little space -- the direction that overflows.
        let span = stream_mix_span(0x40, 0x400, 0x100).unwrap();
        assert_eq!(span.offset, 0x3C0, "|0x40 - 0x400|");
        assert_eq!(span.count, 0x40, "clamped to length - offset (0xBB4C)");
        assert_eq!(span.remainder, 0xC0, "bx = samples - space (0xBB48)");

        // A position at or past the length gives offset 0 -- the full buffer.
        let span = stream_mix_span(0x400, 0x400, 0x10).unwrap();
        assert_eq!((span.offset, span.count, span.remainder), (0, 0x10, 0));
    }

    /// `0xBB5B`: the source is consumed once per two output samples, and the
    /// counter's PARITY sets which sample is doubled first.
    #[test]
    fn half_rate_mix_consumes_the_source_every_other_sample() {
        // Silence destination, so each output is (source + 0x80)/2 and the
        // SEQUENCE of source samples is what the assertion is about.
        let expect = |src: &[u8], count: u16, n: usize| -> Vec<u8> {
            let mut dst = vec![SILENCE; n];
            mix_unsigned_pcm_half_rate(&mut dst, src, count);
            dst
        };
        let src = [0x00u8, 0x40, 0xC0];
        let mix = |s: u8| snd_mix_average(s, SILENCE);

        // EVEN count: advance on the first sample -> A,B,B,C,C
        assert_eq!(
            expect(&src, 6, 5),
            vec![mix(0x00), mix(0x40), mix(0x40), mix(0xC0), mix(0xC0)]
        );
        // ODD count: hold the first sample -> A,A,B,B,C
        assert_eq!(
            expect(&src, 5, 5),
            vec![mix(0x00), mix(0x00), mix(0x40), mix(0x40), mix(0xC0)]
        );

        // It stops when the SOURCE runs out, not when the buffer does.
        let mut dst = vec![SILENCE; 32];
        let written = mix_unsigned_pcm_half_rate(&mut dst, &src, 32);
        // 5, not 6: an EVEN counter advances on the very first sample, so the
        // first source sample plays ONCE and only the rest are doubled
        // (A,B,B,C,C). The odd-counter phase above is the one that doubles A.
        assert_eq!(written, 5, "A,B,B,C,C -- the head is not doubled");
        assert_eq!(dst[5..], vec![SILENCE; 27][..], "the rest is untouched");

        // A zero counter writes nothing (`dec cx / je` before the body).
        let mut dst2 = vec![SILENCE; 4];
        assert_eq!(mix_unsigned_pcm_half_rate(&mut dst2, &src, 0), 0);
    }

    /// Mixing N sources is the one-source average applied N times, so the result
    /// is ORDER-DEPENDENT: each earlier source is halved again by every later mix.
    /// A port that "fixed" this into an equal-weight average would be quieter on
    /// the first source and louder on the last than the game.
    #[test]
    fn mixing_several_sources_is_order_dependent_by_construction() {
        use super::{SILENCE, mix_unsigned_pcm_sources, snd_mix_average};

        let a = [0xFFu8; 4];
        let b = [0x00u8; 4];
        let mut ab = [0u8; 4];
        let mut ba = [0u8; 4];
        mix_unsigned_pcm_sources(&[&a, &b], &mut ab);
        mix_unsigned_pcm_sources(&[&b, &a], &mut ba);
        assert_ne!(ab, ba, "swapping the sources must change the result");

        // And each step is exactly the decoded single-source average.
        let step1 = snd_mix_average(a[0], SILENCE);
        assert_eq!(ab[0], snd_mix_average(b[0], step1));

        // A SHORTER source stops contributing without truncating the mix.
        let short = [0xFFu8; 2];
        let long = [0x40u8; 4];
        let mut out = [0u8; 4];
        let written = mix_unsigned_pcm_sources(&[&short, &long], &mut out);
        assert_eq!(written, 4, "the longest source sets the length");
        assert_eq!(out[3], snd_mix_average(long[3], SILENCE), "past the short source");

        // Silence is 0x80: mixing nothing leaves the buffer at the zero level.
        let mut empty = [0u8; 4];
        assert_eq!(mix_unsigned_pcm_sources(&[], &mut empty), 0);
        assert_eq!(empty, [SILENCE; 4]);
    }

    /// `add al,X / rcr al,1` versus the port's 16-bit average, over ALL 65536
    /// input pairs.
    ///
    /// The doc argues they are equal because the add's CARRY becomes bit 7 during
    /// the rotate. That reasoning is right, but it is reasoning — and the entity
    /// draw scale (audit-fixes #93) was a case where 8-bit and 16-bit arithmetic
    /// looked interchangeable and were not. Here the whole domain fits in a sweep,
    /// so the claim can simply be checked.
    #[test]
    fn snd_mix_average_is_the_add_rcr_pair_over_every_input() {
        for source in 0u16..=0xFF {
            for destination in 0u16..=0xFF {
                // add al,X : an 8-bit add whose carry-out is bit 8 of the sum.
                let sum = source + destination;
                let al = (sum & 0xFF) as u8;
                let carry = (sum >> 8) & 1;
                // rcr al,1 : carry rotates INTO bit 7, bit 0 rotates out.
                let rotated = ((carry as u8) << 7) | (al >> 1);
                assert_eq!(
                    super::snd_mix_average(source as u8, destination as u8),
                    rotated,
                    "source {source} destination {destination}"
                );
            }
        }
    }
    use super::*;

    fn collect(root: &str, ext: &str) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        let mut stack = vec![std::path::PathBuf::from(root)];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.filter_map(|e| e.ok()) {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|s| s.to_str()) == Some(ext) {
                    out.push(p);
                }
            }
        }
        out
    }

    /// Every real SND voice/sfx bank in the game data must parse into a bank with clips.
    /// Broad robustness check (the sprite equivalent found a real decoder bug). Skips if absent.
    #[test]
    fn parses_every_real_snd_bank() {
        let files = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .find(|r| std::path::Path::new(r).exists())
            .map(|r| collect(r, "snd"))
            .unwrap_or_default();
        if files.is_empty() {
            return;
        }
        let mut checked = 0;
        for p in &files {
            let data = std::fs::read(p).unwrap();
            let bank = SndBank::parse(&data)
                .unwrap_or_else(|e| panic!("{}: SND parse failed: {e}", p.display()));
            assert!(bank.clips().count() > 0, "{}: no clips", p.display());
            checked += 1;
        }
        assert!(checked >= 20, "parsed the SND set ({checked})");
    }

    /// Every real .voc music/voice file must be a valid Creative VOC (either yields PCM, or is a
    /// recognised non-PCM/silence VOC). Verifies the VOC parser handles the whole set.
    #[test]
    fn parses_every_real_voc() {
        let files = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .find(|r| std::path::Path::new(r).exists())
            .map(|r| collect(r, "voc"))
            .unwrap_or_default();
        if files.is_empty() {
            return;
        }
        let mut checked = 0;
        let mut with_pcm = 0;
        for p in &files {
            let data = std::fs::read(p).unwrap();
            // Must at least have the "Creative Voice File" signature.
            assert!(data.starts_with(b"Creative Voice File"), "{}: not a VOC", p.display());
            if let Some((pcm, rate)) = parse_voc_pcm(&data) {
                assert!(!pcm.is_empty() && rate > 0, "{}: empty PCM", p.display());
                with_pcm += 1;
            }
            checked += 1;
        }
        assert!(checked >= 40, "checked the VOC set ({checked})");
        assert!(with_pcm > 0, "at least some VOCs yield PCM");
    }

    #[test]
    fn parses_real_voc_music() {
        // The game's intro/scene music: header magic, type-1 block (tc 0xA6 ->
        // 11111 Hz), u8 PCM. Skips when assets aren't present in this checkout.
        for p in [
            "output/_tmp_dat/mu/blintr.voc",
            "../output/_tmp_dat/mu/blintr.voc",
        ] {
            if let Ok(data) = std::fs::read(p) {
                let (pcm, rate) = parse_voc_pcm(&data).expect("valid voc");
                assert_eq!(rate, 11111);
                assert!(pcm.len() > 100_000, "substantial music data: {}", pcm.len());
                return;
            }
        }
    }

    fn test_snd(clips: &[&[u8]]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&(clips.len() as u16).to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        let mut offset = 0u32;
        for clip in clips {
            data.extend_from_slice(&offset.to_le_bytes());
            offset += clip.len() as u32;
        }
        data.extend_from_slice(&offset.to_le_bytes());
        for clip in clips {
            data.extend_from_slice(clip);
        }
        data
    }

    #[test]
    fn resolves_clips_by_original_ax_index() {
        let clip0 = [1, 0, 0, 0, 156, 0, 10, 11];
        let clip1 = [1, 0, 0, 0, 255, 0, 20, 21, 22];
        let bank = SndBank::parse(&test_snd(&[&clip0, &clip1])).expect("parse SND");

        assert_eq!(bank.clip_count(), 2);
        assert_eq!(bank.header_end(), 16);
        let first = bank.clip(0).expect("clip 0");
        let second = bank.clip(1).expect("clip 1");
        assert_eq!(first.original_index, 0);
        assert_eq!(first.sample_rate, 10_000);
        assert_eq!(first.pcm, vec![10, 11]);
        assert_eq!(second.original_index, 1);
        assert_eq!(second.sample_rate, 11_111);
        assert_eq!(second.pcm, vec![20, 21, 22]);
        assert!(bank.clip(2).is_none());
    }

    #[test]
    fn preserves_indices_when_a_slot_is_not_pcm() {
        let not_pcm = [2, 0, 0, 0, 156, 0, 10, 11];
        let clip1 = [1, 0, 0, 0, 156, 0, 20, 21];
        let bank = SndBank::parse(&test_snd(&[&not_pcm, &clip1])).expect("parse SND");

        assert!(bank.clip(0).is_none());
        assert_eq!(bank.clip(1).expect("clip 1").pcm, vec![20, 21]);
        assert_eq!(
            bank.clips()
                .map(|clip| clip.original_index)
                .collect::<Vec<_>>(),
            vec![1]
        );
    }

    #[test]
    fn mix_average_matches_add_then_rcr_for_every_u8_pair() {
        for source in 0..=u8::MAX {
            for destination in 0..=u8::MAX {
                let sum = source as u16 + destination as u16;
                let al_after_add = sum as u8;
                let carry = sum > u8::MAX as u16;
                let add_rcr = (al_after_add >> 1) | if carry { 0x80 } else { 0 };
                assert_eq!(snd_mix_average(source, destination), add_rcr);
            }
        }
    }

    #[test]
    fn mixes_pcm_prefix_and_reports_sample_count() {
        let mut destination = [10, 200, 128, 99];
        let mixed = mix_unsigned_pcm_average(&mut destination, &[30, 100, 255]);

        assert_eq!(mixed, 3);
        assert_eq!(destination, [20, 150, 191, 99]);
    }
}
