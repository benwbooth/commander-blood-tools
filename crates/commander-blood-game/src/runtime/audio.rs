//! SDL3 playback for validated original unsigned 8-bit PCM resources.

use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Result, anyhow, bail};
use commander_blood_formats::snd::{SndClip, VocPcm};
use sdl3::AudioSubsystem;
use sdl3::audio::{
    AudioCallback, AudioFormat, AudioFormatNum, AudioSpec, AudioStream, AudioStreamWithCallback,
};

const RUNTIME_AUDIO_OUTPUT_RATE_HZ: u32 = 48_000;
const RUNTIME_AUDIO_CHANNEL_COUNT: i32 = 1;
const PCM_FRACTIONAL_BITS: u32 = 32;
const UNSIGNED_PCM_SILENCE: u8 = 128;
const UNSIGNED_PCM_SCALE: f32 = 128.0;

/// Validated owned unsigned 8-bit mono PCM ready for runtime playback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePcmClip {
    sample_rate_hz: u32,
    samples: Arc<[u8]>,
}

impl RuntimePcmClip {
    /// Own one nonempty unsigned 8-bit mono PCM stream and its source rate.
    pub fn new(sample_rate_hz: u32, samples: impl Into<Arc<[u8]>>) -> Result<Self> {
        let samples = samples.into();
        if sample_rate_hz == u32::MIN {
            bail!("PCM sample rate must be nonzero");
        }
        if samples.is_empty() {
            bail!("PCM clip must contain at least one sample");
        }
        Ok(Self {
            sample_rate_hz,
            samples,
        })
    }

    /// Own the decoded PCM and authored rate from a Creative Voice resource.
    pub fn from_voc(voc: &VocPcm) -> Self {
        Self {
            sample_rate_hz: voc.sample_rate_hz(),
            samples: Arc::from(voc.samples()),
        }
    }

    /// Own one decoded clip from a validated SND bank.
    pub fn from_snd_clip(clip: SndClip<'_>) -> Result<Self> {
        let sample_rate_hz = clip
            .sample_rate_hz()
            .ok_or_else(|| anyhow!("SND clip {} has no rate code", clip.index()))?;
        let samples = clip
            .pcm()
            .ok_or_else(|| anyhow!("SND clip {} has no PCM payload", clip.index()))?;
        Self::new(sample_rate_hz, Arc::<[u8]>::from(samples))
    }

    /// Return the source rate before SDL output conversion.
    pub const fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    /// Return the complete unsigned 8-bit mono PCM payload.
    pub fn samples(&self) -> &[u8] {
        &self.samples
    }
}

#[derive(Clone, Debug)]
struct PcmCursor {
    clip: RuntimePcmClip,
    fractional_position: u64,
    fractional_step: u64,
    looped: bool,
}

impl PcmCursor {
    fn new(clip: RuntimePcmClip, output_rate_hz: u32, looped: bool) -> Self {
        Self {
            fractional_step: (u64::from(clip.sample_rate_hz) << PCM_FRACTIONAL_BITS)
                / u64::from(output_rate_hz),
            clip,
            fractional_position: u64::MIN,
            looped,
        }
    }

    fn source_position(&self) -> u64 {
        self.fractional_position >> PCM_FRACTIONAL_BITS
    }

    fn next_sample(&mut self) -> Option<u8> {
        let sample_count = self.clip.samples.len() as u64;
        let mut position = self.source_position();
        if position >= sample_count {
            if !self.looped {
                return None;
            }
            self.fractional_position %= sample_count << PCM_FRACTIONAL_BITS;
            position = self.source_position();
        }
        let sample = self.clip.samples[position as usize];
        self.fractional_position = self.fractional_position.wrapping_add(self.fractional_step);
        Some(sample)
    }
}

/// Device-independent mixer implementing the game's unsigned-PCM layering rule.
#[derive(Clone, Debug)]
pub struct RuntimePcmMixer {
    output_rate_hz: u32,
    background: Option<PcmCursor>,
    foreground: Option<PcmCursor>,
}

impl RuntimePcmMixer {
    /// Create an empty mono mixer at one concrete host output rate.
    pub fn new(output_rate_hz: u32) -> Result<Self> {
        if output_rate_hz == u32::MIN {
            bail!("audio output rate must be nonzero");
        }
        Ok(Self {
            output_rate_hz,
            background: None,
            foreground: None,
        })
    }

    /// Start or replace looping background music without disturbing a voice clip.
    pub fn play_background(&mut self, clip: RuntimePcmClip) {
        self.background = Some(PcmCursor::new(clip, self.output_rate_hz, true));
    }

    /// Start a one-shot foreground clip mixed over any active background.
    pub fn play_foreground(&mut self, clip: RuntimePcmClip) {
        self.foreground = Some(PcmCursor::new(clip, self.output_rate_hz, false));
    }

    /// Stop all prior sound and start one exclusive one-shot clip.
    pub fn play_exclusive(&mut self, clip: RuntimePcmClip) {
        self.background = None;
        self.play_foreground(clip);
    }

    /// Stop both music and foreground playback.
    pub fn stop_all(&mut self) {
        self.background = None;
        self.foreground = None;
    }

    /// Stop looping background music without interrupting foreground speech.
    pub fn stop_background(&mut self) {
        self.background = None;
    }

    /// Return the current source-sample position of looping music.
    pub fn background_position(&self) -> Option<u64> {
        self.background.as_ref().map(PcmCursor::source_position)
    }

    /// Return the current source-sample position of the foreground clip.
    pub fn foreground_position(&self) -> Option<u64> {
        self.foreground.as_ref().map(PcmCursor::source_position)
    }

    /// Render host-rate unsigned samples after applying original PCM averaging.
    pub fn render_unsigned(&mut self, output: &mut [u8]) {
        for destination in output {
            let background = self.background.as_mut().and_then(PcmCursor::next_sample);
            let foreground = self.foreground.as_mut().and_then(PcmCursor::next_sample);
            *destination = match (background, foreground) {
                (Some(background), Some(foreground)) => {
                    average_unsigned_pcm(foreground, background)
                }
                (Some(sample), None) | (None, Some(sample)) => sample,
                (None, None) => UNSIGNED_PCM_SILENCE,
            };
        }
        if self
            .foreground
            .as_ref()
            .is_some_and(|cursor| cursor.source_position() >= cursor.clip.samples.len() as u64)
        {
            self.foreground = None;
        }
    }
}

impl Default for RuntimePcmMixer {
    fn default() -> Self {
        Self::new(RUNTIME_AUDIO_OUTPUT_RATE_HZ).expect("the fixed output rate is nonzero")
    }
}

#[derive(Debug, Default)]
struct SharedAudioState {
    mixer: RuntimePcmMixer,
    callback_error: Option<String>,
}

struct RuntimeAudioCallback {
    shared: Arc<Mutex<SharedAudioState>>,
    unsigned_samples: Vec<u8>,
    output_samples: Vec<f32>,
}

impl AudioCallback<f32> for RuntimeAudioCallback {
    fn callback(&mut self, stream: &mut AudioStream, requested: i32) {
        let requested = usize::try_from(requested).unwrap_or(usize::MIN);
        self.unsigned_samples
            .resize(requested, UNSIGNED_PCM_SILENCE);
        self.output_samples
            .resize(requested, <f32 as AudioFormatNum>::SILENCE);
        {
            let mut shared = lock_shared(&self.shared);
            shared.mixer.render_unsigned(&mut self.unsigned_samples);
        }
        for (destination, sample) in self
            .output_samples
            .iter_mut()
            .zip(self.unsigned_samples.iter().copied())
        {
            *destination = (f32::from(sample) - UNSIGNED_PCM_SCALE) / UNSIGNED_PCM_SCALE;
        }
        if let Err(error) = stream.put_data_f32(&self.output_samples) {
            lock_shared(&self.shared).callback_error = Some(error.to_string());
        }
    }
}

/// Live SDL3 audio stream backed by the deterministic PCM mixer.
pub struct RuntimeAudioHost {
    shared: Arc<Mutex<SharedAudioState>>,
    stream: AudioStreamWithCallback<RuntimeAudioCallback>,
}

impl RuntimeAudioHost {
    /// Open and resume the default SDL3 playback stream.
    pub fn open(audio: &AudioSubsystem) -> Result<Self> {
        let shared = Arc::new(Mutex::new(SharedAudioState::default()));
        let callback = RuntimeAudioCallback {
            shared: Arc::clone(&shared),
            unsigned_samples: Vec::new(),
            output_samples: Vec::new(),
        };
        let spec = AudioSpec {
            freq: Some(RUNTIME_AUDIO_OUTPUT_RATE_HZ as i32),
            channels: Some(RUNTIME_AUDIO_CHANNEL_COUNT),
            format: Some(AudioFormat::f32_sys()),
        };
        let stream = audio
            .open_playback_stream(&spec, callback)
            .map_err(|error| anyhow!("opening SDL3 playback stream: {error}"))?;
        stream
            .resume()
            .map_err(|error| anyhow!("resuming SDL3 playback stream: {error}"))?;
        Ok(Self { shared, stream })
    }

    /// Start or replace looping background music.
    pub fn play_background(&mut self, clip: RuntimePcmClip) -> Result<()> {
        self.check_callback()?;
        lock_shared(&self.shared).mixer.play_background(clip);
        Ok(())
    }

    /// Start one foreground clip over active music.
    pub fn play_foreground(&mut self, clip: RuntimePcmClip) -> Result<()> {
        self.check_callback()?;
        lock_shared(&self.shared).mixer.play_foreground(clip);
        Ok(())
    }

    /// Stop prior sound and start one exclusive clip.
    pub fn play_exclusive(&mut self, clip: RuntimePcmClip) -> Result<()> {
        self.check_callback()?;
        lock_shared(&self.shared).mixer.play_exclusive(clip);
        self.stream
            .clear()
            .map_err(|error| anyhow!("clearing SDL3 audio stream: {error}"))?;
        Ok(())
    }

    /// Stop every source and clear samples already queued in SDL.
    pub fn stop_all(&mut self) -> Result<()> {
        lock_shared(&self.shared).mixer.stop_all();
        self.stream
            .clear()
            .map_err(|error| anyhow!("clearing SDL3 audio stream: {error}"))?;
        self.check_callback()
    }

    /// Stop background music while allowing a foreground clip to finish.
    pub fn stop_background(&mut self) -> Result<()> {
        lock_shared(&self.shared).mixer.stop_background();
        self.stream
            .clear()
            .map_err(|error| anyhow!("clearing SDL3 audio stream: {error}"))?;
        self.check_callback()
    }

    /// Return the current source-sample position of looping music.
    pub fn background_position(&self) -> Option<u64> {
        lock_shared(&self.shared).mixer.background_position()
    }

    /// Return the current source-sample position of the foreground clip.
    pub fn foreground_position(&self) -> Option<u64> {
        lock_shared(&self.shared).mixer.foreground_position()
    }

    /// Surface any asynchronous SDL callback failure on the game thread.
    pub fn check_callback(&self) -> Result<()> {
        let error = lock_shared(&self.shared).callback_error.take();
        match error {
            Some(error) => Err(anyhow!("SDL3 audio callback failed: {error}")),
            None => Ok(()),
        }
    }
}

fn lock_shared(shared: &Arc<Mutex<SharedAudioState>>) -> MutexGuard<'_, SharedAudioState> {
    shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn average_unsigned_pcm(source: u8, destination: u8) -> u8 {
    ((u16::from(source) + u16::from(destination)) / 2) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_OUTPUT_RATE_HZ: u32 = 10;
    const HALF_OUTPUT_RATE_HZ: u32 = TEST_OUTPUT_RATE_HZ / 2;

    #[test]
    fn foreground_is_averaged_over_music_in_the_unsigned_domain() {
        let mut mixer = RuntimePcmMixer::new(TEST_OUTPUT_RATE_HZ).unwrap();
        mixer.play_background(RuntimePcmClip::new(TEST_OUTPUT_RATE_HZ, [0, 64, 255]).unwrap());
        mixer.play_foreground(RuntimePcmClip::new(TEST_OUTPUT_RATE_HZ, [255, 192]).unwrap());
        let mut output = [0; 4];
        mixer.render_unsigned(&mut output);

        assert_eq!(output, [127, 128, 255, 0]);
        assert_eq!(mixer.foreground_position(), None);
        assert_eq!(mixer.background_position(), Some(1));
    }

    #[test]
    fn source_rate_conversion_repeats_samples_at_half_output_rate() {
        let mut mixer = RuntimePcmMixer::new(TEST_OUTPUT_RATE_HZ).unwrap();
        mixer.play_foreground(RuntimePcmClip::new(HALF_OUTPUT_RATE_HZ, [1, 2, 3]).unwrap());
        let mut output = [0; 6];
        mixer.render_unsigned(&mut output);

        assert_eq!(output, [1, 1, 2, 2, 3, 3]);
        assert_eq!(mixer.foreground_position(), None);
    }

    #[test]
    fn exclusive_playback_replaces_music_and_prior_foreground() {
        let mut mixer = RuntimePcmMixer::new(TEST_OUTPUT_RATE_HZ).unwrap();
        mixer.play_background(RuntimePcmClip::new(TEST_OUTPUT_RATE_HZ, [1]).unwrap());
        mixer.play_foreground(RuntimePcmClip::new(TEST_OUTPUT_RATE_HZ, [2]).unwrap());
        mixer.play_exclusive(RuntimePcmClip::new(TEST_OUTPUT_RATE_HZ, [3]).unwrap());
        let mut output = [0; 2];
        mixer.render_unsigned(&mut output);

        assert_eq!(output, [3, UNSIGNED_PCM_SILENCE]);
        assert_eq!(mixer.background_position(), None);
        assert_eq!(mixer.foreground_position(), None);
    }

    #[test]
    fn stopping_background_preserves_foreground_playback() {
        let mut mixer = RuntimePcmMixer::new(TEST_OUTPUT_RATE_HZ).unwrap();
        mixer.play_background(RuntimePcmClip::new(TEST_OUTPUT_RATE_HZ, [1, 2]).unwrap());
        mixer.play_foreground(RuntimePcmClip::new(TEST_OUTPUT_RATE_HZ, [9, 8]).unwrap());

        mixer.stop_background();
        let mut output = [0; 2];
        mixer.render_unsigned(&mut output);

        assert_eq!(output, [9, 8]);
        assert_eq!(mixer.background_position(), None);
    }
}
