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

pub fn mix_unsigned_pcm_average(destination: &mut [u8], source: &[u8]) -> usize {
    let len = destination.len().min(source.len());
    for idx in 0..len {
        destination[idx] = snd_mix_average(source[idx], destination[idx]);
    }
    len
}

#[cfg(test)]
mod tests {
    use super::*;

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
