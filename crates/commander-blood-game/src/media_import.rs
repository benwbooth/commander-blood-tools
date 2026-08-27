//! Lossless standard-media derivatives generated from the imported loose store.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use commander_blood_formats::archive::BloodResourceName;
use commander_blood_formats::snd::{SndBank, VocPcm, snd_sample_rate_hz};
use commander_blood_formats::wav::{decode_unsigned_pcm_wave, encode_unsigned_pcm_wave};
use serde::{Deserialize, Serialize};

use crate::asset_import::{
    ASSET_MANIFEST_FILENAME, ImportedAssetManifest, ImportedMediaKind, checked_relative_path,
    replace_directory, sha256_hex, temporary_sibling,
};

const MEDIA_DIRECTORY_NAME: &str = "media-v1";
const MEDIA_MANIFEST_FILENAME: &str = "manifest.json";
const MEDIA_SCHEMA_VERSION: u32 = 1;
const AUDIO_DIRECTORY_NAME: &str = "audio";
const TEMPORARY_MEDIA_INFIX: &str = "generate";
const WAVE_EXTENSION: &str = "wav";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum NormalizedAudioKind {
    Voc,
    SndClip,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct NormalizedAudioEntry {
    source_resource_name: String,
    clip_index: Option<u16>,
    path: String,
    sample_rate_hz: u32,
    sample_rate_code: u8,
    sample_count: usize,
    sha256: String,
    kind: NormalizedAudioKind,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct NormalizedMediaManifest {
    schema_version: u32,
    source_asset_manifest_sha256: String,
    audio: Vec<NormalizedAudioEntry>,
}

impl NormalizedMediaManifest {
    fn load(asset_root: &Path, source_asset_manifest_sha256: &str) -> Result<Self> {
        let media_root = asset_root.join(MEDIA_DIRECTORY_NAME);
        let manifest_path = media_root.join(MEDIA_MANIFEST_FILENAME);
        let encoded = std::fs::read(&manifest_path).with_context(|| {
            format!(
                "reading normalized media manifest {}",
                manifest_path.display()
            )
        })?;
        let manifest: Self = serde_json::from_slice(&encoded).with_context(|| {
            format!(
                "decoding normalized media manifest {}",
                manifest_path.display()
            )
        })?;
        manifest.validate(&media_root, source_asset_manifest_sha256, false)?;
        Ok(manifest)
    }

    fn validate(
        &self,
        media_root: &Path,
        source_asset_manifest_sha256: &str,
        verify_hashes: bool,
    ) -> Result<()> {
        if self.schema_version != MEDIA_SCHEMA_VERSION {
            bail!(
                "unsupported normalized media schema {}; expected {}",
                self.schema_version,
                MEDIA_SCHEMA_VERSION
            );
        }
        if self.source_asset_manifest_sha256 != source_asset_manifest_sha256 {
            bail!("normalized media was generated from a different asset manifest");
        }
        let mut identities = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for entry in &self.audio {
            BloodResourceName::new(entry.source_resource_name.as_bytes()).with_context(|| {
                format!(
                    "invalid normalized audio source {}",
                    entry.source_resource_name
                )
            })?;
            if !identities.insert((entry.source_resource_name.as_str(), entry.clip_index)) {
                bail!(
                    "duplicate normalized audio identity {} {:?}",
                    entry.source_resource_name,
                    entry.clip_index
                );
            }
            if !paths.insert(entry.path.as_str()) {
                bail!("duplicate normalized audio path {}", entry.path);
            }
            let path = checked_relative_path(media_root, &entry.path)?;
            let encoded = std::fs::read(&path)
                .with_context(|| format!("reading normalized audio {}", path.display()))?;
            if verify_hashes && sha256_hex(&encoded) != entry.sha256 {
                bail!("normalized audio hash mismatch: {}", path.display());
            }
            let wave = decode_unsigned_pcm_wave(&encoded)
                .with_context(|| format!("decoding normalized audio {}", path.display()))?;
            if wave.sample_rate_hz() != entry.sample_rate_hz
                || wave.samples().len() != entry.sample_count
                || snd_sample_rate_hz(entry.sample_rate_code) != entry.sample_rate_hz
            {
                bail!("normalized audio metadata mismatch: {}", path.display());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct NormalizedPcmClip {
    pub(crate) sample_rate_hz: u32,
    pub(crate) sample_rate_code: u8,
    pub(crate) samples: Arc<[u8]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NormalizedMediaStore {
    media_root: PathBuf,
    manifest: NormalizedMediaManifest,
}

impl NormalizedMediaStore {
    pub(crate) fn prepare(
        asset_root: &Path,
        asset_manifest: &ImportedAssetManifest,
    ) -> Result<Self> {
        let source_asset_manifest_sha256 = asset_manifest_sha256(asset_root)?;
        if let Ok(manifest) =
            NormalizedMediaManifest::load(asset_root, &source_asset_manifest_sha256)
        {
            return Ok(Self {
                media_root: asset_root.join(MEDIA_DIRECTORY_NAME),
                manifest,
            });
        }

        eprintln!(
            "Generating lossless WAVE audio derivatives in {}. This one-time operation may take several minutes.",
            asset_root.join(MEDIA_DIRECTORY_NAME).display()
        );
        let media_root = asset_root.join(MEDIA_DIRECTORY_NAME);
        let temporary_root = temporary_sibling(&media_root, TEMPORARY_MEDIA_INFIX)?;
        if temporary_root.exists() {
            std::fs::remove_dir_all(&temporary_root).with_context(|| {
                format!(
                    "removing interrupted media generation {}",
                    temporary_root.display()
                )
            })?;
        }
        std::fs::create_dir_all(temporary_root.join(AUDIO_DIRECTORY_NAME)).with_context(|| {
            format!(
                "creating normalized audio directory {}",
                temporary_root.display()
            )
        })?;

        let generated = generate_audio(
            asset_root,
            &temporary_root,
            asset_manifest,
            source_asset_manifest_sha256.clone(),
        );
        let manifest = match generated {
            Ok(manifest) => manifest,
            Err(error) => {
                let _ = std::fs::remove_dir_all(&temporary_root);
                return Err(error);
            }
        };
        replace_directory(&temporary_root, &media_root)?;
        Ok(Self {
            media_root,
            manifest,
        })
    }

    pub(crate) const fn audio_entry_count(&self) -> usize {
        self.manifest.audio.len()
    }

    pub(crate) fn root(&self) -> &Path {
        &self.media_root
    }

    pub(crate) fn source_asset_manifest_sha256(&self) -> &str {
        &self.manifest.source_asset_manifest_sha256
    }

    pub(crate) fn load_voc(&self, name: &BloodResourceName) -> Result<NormalizedPcmClip> {
        self.load(name, None, NormalizedAudioKind::Voc)
    }

    fn load(
        &self,
        name: &BloodResourceName,
        clip_index: Option<u16>,
        kind: NormalizedAudioKind,
    ) -> Result<NormalizedPcmClip> {
        let key = String::from_utf8(name.archive_lookup_key().into_vec())
            .expect("validated ASCII resource name");
        let entry = self
            .manifest
            .audio
            .iter()
            .find(|entry| {
                entry.source_resource_name == key
                    && entry.clip_index == clip_index
                    && entry.kind == kind
            })
            .with_context(|| format!("no normalized audio derivative for {key}"))?;
        let path = checked_relative_path(&self.media_root, &entry.path)?;
        let encoded = std::fs::read(&path)
            .with_context(|| format!("reading normalized audio {}", path.display()))?;
        let wave = decode_unsigned_pcm_wave(&encoded)
            .with_context(|| format!("decoding normalized audio {}", path.display()))?;
        if wave.sample_rate_hz() != entry.sample_rate_hz
            || wave.samples().len() != entry.sample_count
        {
            bail!(
                "normalized audio changed after manifest validation: {}",
                path.display()
            );
        }
        Ok(NormalizedPcmClip {
            sample_rate_hz: wave.sample_rate_hz(),
            sample_rate_code: entry.sample_rate_code,
            samples: Arc::from(wave.samples()),
        })
    }
}

fn generate_audio(
    asset_root: &Path,
    temporary_root: &Path,
    asset_manifest: &ImportedAssetManifest,
    source_asset_manifest_sha256: String,
) -> Result<NormalizedMediaManifest> {
    let mut audio = Vec::new();
    for resource in &asset_manifest.resources {
        if !matches!(
            resource.media_kind,
            ImportedMediaKind::VocAudio | ImportedMediaKind::SndBank
        ) {
            continue;
        }
        let source_path = checked_relative_path(asset_root, &resource.path)?;
        let encoded = std::fs::read(&source_path)
            .with_context(|| format!("reading imported audio {}", source_path.display()))?;
        match resource.media_kind {
            ImportedMediaKind::VocAudio => {
                let voc = VocPcm::decode(&encoded)
                    .with_context(|| format!("decoding imported VOC {}", source_path.display()))?;
                audio.push(write_wave(
                    temporary_root,
                    &resource.resource_name,
                    None,
                    voc.sample_rate_hz(),
                    voc.sample_rate_code(),
                    voc.samples(),
                    NormalizedAudioKind::Voc,
                )?);
            }
            ImportedMediaKind::SndBank => {
                let bank = SndBank::decode(&encoded)
                    .with_context(|| format!("decoding imported SND {}", source_path.display()))?;
                for clip_index in 0..bank.header().clip_count {
                    let clip = bank
                        .clip(usize::from(clip_index))
                        .context("validated SND bank omitted an authored clip")?;
                    let samples = clip.pcm().context("SND clip has no PCM payload")?;
                    let sample_rate_hz = clip
                        .sample_rate_hz()
                        .context("SND clip has no sample-rate code")?;
                    let sample_rate_code = clip
                        .sample_rate_code()
                        .context("SND clip has no sample-rate code")?;
                    audio.push(write_wave(
                        temporary_root,
                        &resource.resource_name,
                        Some(clip_index),
                        sample_rate_hz,
                        sample_rate_code,
                        samples,
                        NormalizedAudioKind::SndClip,
                    )?);
                }
            }
            ImportedMediaKind::HnmVideo | ImportedMediaKind::NativeData => unreachable!(),
        }
    }
    audio.sort_by(|left, right| {
        (&left.source_resource_name, left.clip_index)
            .cmp(&(&right.source_resource_name, right.clip_index))
    });

    let manifest = NormalizedMediaManifest {
        schema_version: MEDIA_SCHEMA_VERSION,
        source_asset_manifest_sha256,
        audio,
    };
    manifest.validate(temporary_root, &manifest.source_asset_manifest_sha256, true)?;
    let manifest_path = temporary_root.join(MEDIA_MANIFEST_FILENAME);
    let mut encoded =
        serde_json::to_vec_pretty(&manifest).context("encoding normalized media manifest")?;
    encoded.push(b'\n');
    std::fs::write(&manifest_path, encoded).with_context(|| {
        format!(
            "writing normalized media manifest {}",
            manifest_path.display()
        )
    })?;
    Ok(manifest)
}

fn write_wave(
    root: &Path,
    resource_name: &str,
    clip_index: Option<u16>,
    sample_rate_hz: u32,
    sample_rate_code: u8,
    samples: &[u8],
    kind: NormalizedAudioKind,
) -> Result<NormalizedAudioEntry> {
    let mut source_path = PathBuf::from(resource_name.replace('\\', "/"));
    source_path.set_extension("");
    let relative = match clip_index {
        Some(index) => PathBuf::from(AUDIO_DIRECTORY_NAME)
            .join(source_path)
            .join(format!("{index:03}.{WAVE_EXTENSION}")),
        None => PathBuf::from(AUDIO_DIRECTORY_NAME)
            .join(source_path)
            .with_extension(WAVE_EXTENSION),
    };
    let relative = relative
        .to_str()
        .context("normalized audio path is not UTF-8")?
        .to_owned();
    let destination = checked_relative_path(root, &relative)?;
    let parent = destination
        .parent()
        .context("normalized audio path has no parent")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("creating normalized audio directory {}", parent.display()))?;
    let wave = encode_unsigned_pcm_wave(sample_rate_hz, samples)
        .context("encoding normalized PCM WAVE")?;
    std::fs::write(&destination, &wave)
        .with_context(|| format!("writing normalized audio {}", destination.display()))?;
    Ok(NormalizedAudioEntry {
        source_resource_name: resource_name.to_owned(),
        clip_index,
        path: relative,
        sample_rate_hz,
        sample_rate_code,
        sample_count: samples.len(),
        sha256: sha256_hex(&wave),
        kind,
    })
}

fn asset_manifest_sha256(asset_root: &Path) -> Result<String> {
    let path = asset_root.join(ASSET_MANIFEST_FILENAME);
    let encoded = std::fs::read(&path)
        .with_context(|| format!("reading imported asset manifest {}", path.display()))?;
    Ok(sha256_hex(&encoded))
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::wav::decode_unsigned_pcm_wave;

    use super::*;

    #[test]
    fn wave_paths_keep_resource_identity_and_clip_index() {
        let root = std::env::temp_dir().join(format!(
            "commander-blood-wave-path-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let entry = write_wave(
            &root,
            r"SN\TB.SND",
            Some(7),
            11_111,
            u8::MAX,
            &[0, 128, 255],
            NormalizedAudioKind::SndClip,
        )
        .unwrap();
        assert_eq!(entry.path, "audio/SN/TB/007.wav");
        let wave =
            decode_unsigned_pcm_wave(&std::fs::read(root.join(&entry.path)).unwrap()).unwrap();
        assert_eq!(wave.samples(), [0, 128, 255]);
        std::fs::remove_dir_all(root).unwrap();
    }
}
