//! Typed loading of resident and streamed SND banks.

use commander_blood_formats::snd::{SndBank, SndBankDecodeError};

/// Runtime role assigned to a decoded SND bank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundBankUsage {
    /// Short effects and voice reactions retained for direct playback.
    ResidentEffects,
    /// Dialogue or music clips selected through the streamed offset table.
    StreamedDialogue,
}

/// One decoded SND bank and its runtime role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedSoundBank {
    /// Runtime role selected by the caller.
    pub usage: SoundBankUsage,
    /// Validated owned clip table and payload.
    pub bank: SndBank,
}

/// Decode an SND bank when voice playback is enabled.
///
/// This translates `snd_bank_loader` at BLOODPRG routine offset `0x00C005`.
/// The original resident-versus-streamed intent remains explicit, while archive
/// handles, temporary files, EMS/XMS selection, page mapping, and transfer
/// chunks collapse into one validated owned payload supplied by the resource
/// layer.
pub fn load_sound_bank(
    playback_enabled: bool,
    usage: SoundBankUsage,
    encoded: &[u8],
) -> Result<Option<LoadedSoundBank>, SndBankDecodeError> {
    if !playback_enabled {
        return Ok(None);
    }
    Ok(Some(LoadedSoundBank {
        usage,
        bank: SndBank::decode(encoded)?,
    }))
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 12;
    const TEST_DELAY_BASE: u8 = 90;
    const TEST_DELAY_LIMIT: u8 = 165;

    #[derive(Deserialize)]
    struct BankLoaderOracle {
        name: String,
        sound_enabled: u8,
        mode: u16,
        source_kind: Option<String>,
        backend: Option<String>,
        clip_count: Option<u16>,
        payload_bytes: Option<usize>,
        payload_chunks: Vec<usize>,
    }

    #[test]
    fn loader_matches_every_original_storage_and_mode_vector() {
        let vectors: Vec<BankLoaderOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_c005_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let enabled = vector.sound_enabled & 1 != 0;
            let usage = if vector.mode == 0 {
                SoundBankUsage::ResidentEffects
            } else {
                SoundBankUsage::StreamedDialogue
            };
            let encoded = if enabled {
                encoded_bank(
                    vector.clip_count.unwrap(),
                    vector.payload_bytes.unwrap(),
                    case_index,
                )
            } else {
                Vec::new()
            };

            let loaded = load_sound_bank(enabled, usage, &encoded)
                .unwrap_or_else(|error| panic!("{}: {error}", vector.name));
            if !enabled {
                assert!(loaded.is_none(), "{}", vector.name);
                assert!(vector.source_kind.is_none(), "{}", vector.name);
                assert!(vector.backend.is_none(), "{}", vector.name);
                continue;
            }

            let loaded = loaded.unwrap();
            let clip_count = vector.clip_count.unwrap();
            let payload_bytes = vector.payload_bytes.unwrap();
            assert_eq!(loaded.usage, usage, "{}", vector.name);
            assert_eq!(
                loaded.bank.header().clip_count,
                clip_count,
                "{}",
                vector.name
            );
            assert_eq!(
                loaded.bank.header().dialogue_delay_base,
                TEST_DELAY_BASE,
                "{}",
                vector.name
            );
            assert_eq!(
                loaded.bank.header().dialogue_delay_limit,
                TEST_DELAY_LIMIT,
                "{}",
                vector.name
            );
            assert_eq!(
                loaded.bank.payload().len(),
                payload_bytes,
                "{}",
                vector.name
            );
            assert_eq!(
                loaded.bank.offsets(),
                expected_offsets(clip_count, payload_bytes).as_slice(),
                "{}",
                vector.name
            );
            assert!(
                matches!(
                    vector.source_kind.as_deref(),
                    Some("embedded" | "standalone")
                ),
                "{}",
                vector.name
            );
            assert!(
                matches!(
                    vector.backend.as_deref(),
                    Some("memory" | "ems" | "xms" | "file")
                ),
                "{}",
                vector.name
            );
            assert_eq!(
                vector.payload_chunks.iter().sum::<usize>(),
                payload_bytes,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn enabled_malformed_bank_is_not_partially_loaded() {
        assert_eq!(
            load_sound_bank(true, SoundBankUsage::ResidentEffects, &[1, 0, 4]),
            Err(SndBankDecodeError::HeaderTruncated { actual: 3 })
        );
    }

    fn encoded_bank(clip_count: u16, payload_bytes: usize, case_index: usize) -> Vec<u8> {
        let offsets = expected_offsets(clip_count, payload_bytes);
        let mut encoded = Vec::with_capacity(4 + offsets.len() * 4 + payload_bytes);
        encoded.extend_from_slice(&clip_count.to_le_bytes());
        encoded.push(TEST_DELAY_BASE);
        encoded.push(TEST_DELAY_LIMIT);
        for offset in offsets {
            encoded.extend_from_slice(&(offset as u32).to_le_bytes());
        }
        encoded.extend((0..payload_bytes).map(|index| (index * 31 + case_index * 43 + 7) as u8));
        encoded
    }

    fn expected_offsets(clip_count: u16, payload_bytes: usize) -> Vec<usize> {
        if clip_count == 0 {
            return vec![0];
        }
        (0..=usize::from(clip_count))
            .map(|index| payload_bytes * index / usize::from(clip_count))
            .collect()
    }
}
