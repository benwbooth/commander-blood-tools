//! SDL3 playback for validated original unsigned 8-bit PCM resources.

use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Context, Result, anyhow, bail};
use commander_blood_formats::snd::{
    SND_CLIP_HEADER_BYTE_COUNT, SndClip, VocPcm, snd_sample_rate_hz,
};
use sdl3::AudioSubsystem;
use sdl3::audio::{
    AudioCallback, AudioFormat, AudioFormatNum, AudioSpec, AudioStream, AudioStreamWithCallback,
};

use crate::native::bloodprg::{
    AudioDriverRequests, AudioPlaybackBanks, AudioPlaybackOutcome, AudioPlaybackState,
    AudioStreamBuffer, AudioStreamBufferStatus, AudioStreamLoadOutcome,
    AudioStreamPlaybackPosition, AudioStreamRefillOutcome, AudioStreamStartOutcome,
    AudioStreamState, AudioStreamSubmission, AudioStreamSubmissionKind, SpeakerGateAction,
    load_audio_pcm_stream_source, load_audio_stream_source, refill_audio_stream,
    start_audio_stream, update_audio_playback,
};

const RUNTIME_AUDIO_OUTPUT_RATE_HZ: u32 = 48_000;
const RUNTIME_AUDIO_CHANNEL_COUNT: i32 = 1;
const PCM_FRACTIONAL_BITS: u32 = 32;
const UNSIGNED_PCM_SILENCE: u8 = 128;
const UNSIGNED_PCM_SCALE: f32 = 128.0;
const SND_CLIP_RATE_CODE_INDEX: usize = 4;
const LAST_STREAM_BUFFER_INDEX: usize = 1;
const PC_SPEAKER_PIT_CLOCK_HZ: u32 = 1_193_182;
const PC_SPEAKER_PIT_DIVISOR: usize = 0x2E9C;
const PC_SPEAKER_LOW_SAMPLE: u8 = u8::MIN;
const PC_SPEAKER_HIGH_SAMPLE: u8 = u8::MAX;

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

    fn from_encoded_snd(encoded: &[u8]) -> Result<Self> {
        let rate_code = encoded
            .get(SND_CLIP_RATE_CODE_INDEX)
            .copied()
            .context("encoded SND clip has no sample-rate byte")?;
        let samples = encoded
            .get(SND_CLIP_HEADER_BYTE_COUNT..)
            .context("encoded SND clip has no PCM payload")?;
        Self::new(snd_sample_rate_hz(rate_code), Arc::<[u8]>::from(samples))
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

#[derive(Clone, Debug)]
struct StreamBufferCursor {
    buffer_index: usize,
    fractional_position: u64,
    fractional_step: u64,
    total_fractional_position: u64,
}

/// Flat host ownership for the recovered two-buffer music stream.
#[derive(Clone, Debug)]
struct RuntimeMusicStream {
    playback: AudioPlaybackState,
    stream: AudioStreamState,
    output_rate_hz: u32,
    source_rate_hz: Option<u32>,
    cursor: Option<StreamBufferCursor>,
}

impl RuntimeMusicStream {
    fn new(output_rate_hz: u32) -> Self {
        let empty_buffer = || AudioStreamBuffer {
            header: [u8::MIN; SND_CLIP_HEADER_BYTE_COUNT],
            samples: Box::new([]),
            status: AudioStreamBufferStatus::Free,
        };
        Self {
            playback: AudioPlaybackState {
                playback_enabled: true,
                driver_requests: AudioDriverRequests::default(),
                packed_stream_samples: false,
                stream_buffers: [empty_buffer(), empty_buffer()],
            },
            stream: AudioStreamState {
                channel_active: true,
                source: None,
                next_page_index: u16::MIN,
                block_header: [u8::MIN; SND_CLIP_HEADER_BYTE_COUNT],
                music_resource_changed: false,
            },
            output_rate_hz,
            source_rate_hz: None,
            cursor: None,
        }
    }

    fn load(&mut self, encoded_voc: &[u8], source_rate_hz: u32) -> Result<Option<&'static [u8]>> {
        let outcome = load_audio_stream_source(&mut self.playback, &mut self.stream, encoded_voc)
            .map_err(anyhow::Error::new)
            .context("loading recovered navigation-audio stream state")?;
        let wait_prompt = match outcome {
            AudioStreamLoadOutcome::Inactive => return Ok(None),
            AudioStreamLoadOutcome::Loaded { wait_prompt } => wait_prompt,
        };
        self.source_rate_hz = Some(source_rate_hz);
        self.cursor = None;
        for buffer in &mut self.playback.stream_buffers {
            buffer.status = AudioStreamBufferStatus::Free;
        }
        Ok(Some(wait_prompt))
    }

    fn load_pcm(
        &mut self,
        samples: &[u8],
        source_rate_hz: u32,
        sample_rate_code: u8,
    ) -> Result<Option<&'static [u8]>> {
        let outcome = load_audio_pcm_stream_source(
            &mut self.playback,
            &mut self.stream,
            samples,
            sample_rate_code,
        )
        .map_err(anyhow::Error::new)
        .context("loading normalized navigation-audio stream state")?;
        let wait_prompt = match outcome {
            AudioStreamLoadOutcome::Inactive => return Ok(None),
            AudioStreamLoadOutcome::Loaded { wait_prompt } => wait_prompt,
        };
        self.source_rate_hz = Some(source_rate_hz);
        self.cursor = None;
        for buffer in &mut self.playback.stream_buffers {
            buffer.status = AudioStreamBufferStatus::Free;
        }
        Ok(Some(wait_prompt))
    }

    fn start(&mut self) -> Result<()> {
        let outcome = start_audio_stream(&mut self.playback, &mut self.stream)
            .map_err(anyhow::Error::new)
            .context("starting recovered navigation-audio stream")?;
        match outcome {
            AudioStreamStartOutcome::Inactive => Ok(()),
            AudioStreamStartOutcome::Started(submission) => {
                self.submit(submission);
                Ok(())
            }
        }
    }

    fn refill(&mut self) -> Result<AudioStreamRefillOutcome> {
        let position = self.playback_position();
        let outcome = refill_audio_stream(&mut self.playback, &mut self.stream, || position)
            .map_err(anyhow::Error::new)
            .context("refilling recovered navigation-audio stream")?;
        if let AudioStreamRefillOutcome::Submitted(submission) = outcome {
            self.submit(submission);
        }
        Ok(outcome)
    }

    fn update_clip(
        &mut self,
        request: crate::native::bloodprg::AudioClipRequest,
        banks: AudioPlaybackBanks<'_>,
    ) -> Result<AudioPlaybackOutcome> {
        let position = match self.playback_position() {
            AudioStreamPlaybackPosition::Playing(remaining) => Some(remaining),
            AudioStreamPlaybackPosition::Stopped | AudioStreamPlaybackPosition::Unavailable => None,
        };
        update_audio_playback(&mut self.playback, request, banks, || position)
            .map_err(anyhow::Error::new)
            .context("applying recovered sound-clip playback semantics")
    }

    fn submit(&mut self, submission: AudioStreamSubmission) {
        match submission.kind {
            AudioStreamSubmissionKind::Start | AudioStreamSubmissionKind::Restart => {
                for buffer in &mut self.playback.stream_buffers {
                    buffer.status = AudioStreamBufferStatus::Free;
                }
                self.playback.stream_buffers[submission.buffer_index].status =
                    AudioStreamBufferStatus::ReadyAndDriverOwned;
                self.cursor = Some(StreamBufferCursor {
                    buffer_index: submission.buffer_index,
                    fractional_position: u64::MIN,
                    fractional_step: self.fractional_step(),
                    total_fractional_position: u64::MIN,
                });
            }
            AudioStreamSubmissionKind::Service => {
                self.playback.stream_buffers[submission.buffer_index].status =
                    AudioStreamBufferStatus::DriverOwned;
            }
        }
    }

    fn stop(&mut self) {
        self.cursor = None;
        self.playback.driver_requests = AudioDriverRequests::default();
        for buffer in &mut self.playback.stream_buffers {
            buffer.status = AudioStreamBufferStatus::Free;
        }
    }

    fn set_channel_active(&mut self, active: bool) {
        if self.stream.channel_active == active {
            return;
        }
        self.stop();
        self.stream.channel_active = active;
        self.stream.source = None;
        self.source_rate_hz = None;
    }

    const fn channel_active(&self) -> bool {
        self.stream.channel_active
    }

    fn discard_pending(&mut self) -> bool {
        if self.cursor.is_some() || !self.playback.driver_requests.stream_start_requested {
            return false;
        }
        self.playback.driver_requests = AudioDriverRequests::default();
        self.source_rate_hz = None;
        self.stream.source.take().is_some()
    }

    fn pending(&self) -> bool {
        self.playback.driver_requests.stream_start_requested
    }

    fn source_position(&self) -> Option<u64> {
        self.cursor
            .as_ref()
            .map(|cursor| cursor.total_fractional_position >> PCM_FRACTIONAL_BITS)
    }

    fn playback_position(&self) -> AudioStreamPlaybackPosition {
        let Some(cursor) = self.cursor.as_ref() else {
            return AudioStreamPlaybackPosition::Stopped;
        };
        let buffer = &self.playback.stream_buffers[cursor.buffer_index];
        let source_position = cursor.fractional_position >> PCM_FRACTIONAL_BITS;
        let remaining = (buffer.samples.len() as u64).saturating_sub(source_position);
        match u16::try_from(remaining) {
            Ok(u16::MIN) => AudioStreamPlaybackPosition::Stopped,
            Ok(remaining) => AudioStreamPlaybackPosition::Playing(remaining),
            Err(_) => AudioStreamPlaybackPosition::Unavailable,
        }
    }

    fn playback_remaining(&self) -> Option<u16> {
        match self.playback_position() {
            AudioStreamPlaybackPosition::Playing(remaining) => Some(remaining),
            AudioStreamPlaybackPosition::Stopped | AudioStreamPlaybackPosition::Unavailable => None,
        }
    }

    fn render_unsigned(&mut self, output: &mut [u8]) -> bool {
        let was_active = self.cursor.is_some();
        for destination in output {
            *destination = self.next_sample().unwrap_or(UNSIGNED_PCM_SILENCE);
        }
        was_active
    }

    fn next_sample(&mut self) -> Option<u8> {
        loop {
            let cursor = self.cursor.as_mut()?;
            let buffer_index = cursor.buffer_index;
            let sample_count = self.playback.stream_buffers[buffer_index].samples.len() as u64;
            let source_position = cursor.fractional_position >> PCM_FRACTIONAL_BITS;
            if source_position < sample_count {
                let sample =
                    self.playback.stream_buffers[buffer_index].samples[source_position as usize];
                cursor.fractional_position = cursor
                    .fractional_position
                    .wrapping_add(cursor.fractional_step);
                cursor.total_fractional_position = cursor
                    .total_fractional_position
                    .wrapping_add(cursor.fractional_step);
                return Some(sample);
            }

            let overflow = cursor
                .fractional_position
                .saturating_sub(sample_count << PCM_FRACTIONAL_BITS);
            self.playback.stream_buffers[buffer_index].status = AudioStreamBufferStatus::Free;
            let next_buffer_index = LAST_STREAM_BUFFER_INDEX - buffer_index;
            if !matches!(
                self.playback.stream_buffers[next_buffer_index].status,
                AudioStreamBufferStatus::DriverOwned | AudioStreamBufferStatus::ReadyAndDriverOwned
            ) {
                self.cursor = None;
                return None;
            }
            self.playback.stream_buffers[next_buffer_index].status =
                AudioStreamBufferStatus::ReadyAndDriverOwned;
            cursor.buffer_index = next_buffer_index;
            cursor.fractional_position = overflow;
        }
    }

    fn fractional_step(&self) -> u64 {
        (u64::from(self.source_rate_hz.unwrap_or(self.output_rate_hz)) << PCM_FRACTIONAL_BITS)
            / u64::from(self.output_rate_hz)
    }
}

/// Device-independent mixer implementing the game's unsigned-PCM layering rule.
#[derive(Clone, Debug)]
pub struct RuntimePcmMixer {
    output_rate_hz: u32,
    background: Option<PcmCursor>,
    foreground: Option<PcmCursor>,
    speaker: Option<PcmCursor>,
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
            speaker: None,
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

    /// Start the looped replacement for the PIT channel-two PC-speaker tone.
    pub fn enable_speaker(&mut self, clip: RuntimePcmClip) {
        self.speaker = Some(PcmCursor::new(clip, self.output_rate_hz, true));
    }

    /// Stop the PC-speaker replacement without disturbing digital audio.
    pub fn disable_speaker(&mut self) {
        self.speaker = None;
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
        self.speaker = None;
    }

    /// Stop digital PCM sources without changing the independent PC-speaker gate.
    pub fn stop_digital(&mut self) {
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
            let speaker = self.speaker.as_mut().and_then(PcmCursor::next_sample);
            let foreground_and_speaker = mix_optional_unsigned_pcm(foreground, speaker);
            *destination = mix_optional_unsigned_pcm(background, foreground_and_speaker)
                .unwrap_or(UNSIGNED_PCM_SILENCE);
        }
        if self
            .foreground
            .as_ref()
            .is_some_and(|cursor| cursor.source_position() >= cursor.clip.samples.len() as u64)
        {
            self.foreground = None;
        }
    }

    fn render_auxiliary_unsigned(&mut self, output: &mut [u8]) -> bool {
        let was_active = self.foreground.is_some() || self.speaker.is_some();
        for destination in output {
            let foreground = self.foreground.as_mut().and_then(PcmCursor::next_sample);
            let speaker = self.speaker.as_mut().and_then(PcmCursor::next_sample);
            *destination =
                mix_optional_unsigned_pcm(foreground, speaker).unwrap_or(UNSIGNED_PCM_SILENCE);
        }
        if self
            .foreground
            .as_ref()
            .is_some_and(|cursor| cursor.source_position() >= cursor.clip.samples.len() as u64)
        {
            self.foreground = None;
        }
        was_active
    }
}

impl Default for RuntimePcmMixer {
    fn default() -> Self {
        Self::new(RUNTIME_AUDIO_OUTPUT_RATE_HZ).expect("the fixed output rate is nonzero")
    }
}

#[derive(Debug)]
struct SharedAudioState {
    mixer: RuntimePcmMixer,
    music_stream: RuntimeMusicStream,
    callback_error: Option<String>,
}

impl Default for SharedAudioState {
    fn default() -> Self {
        Self {
            mixer: RuntimePcmMixer::default(),
            music_stream: RuntimeMusicStream::new(RUNTIME_AUDIO_OUTPUT_RATE_HZ),
            callback_error: None,
        }
    }
}

struct RuntimeAudioCallback {
    shared: Arc<Mutex<SharedAudioState>>,
    unsigned_samples: Vec<u8>,
    foreground_samples: Vec<u8>,
    output_samples: Vec<f32>,
}

impl RuntimeAudioCallback {
    fn new(shared: Arc<Mutex<SharedAudioState>>) -> Self {
        Self {
            shared,
            unsigned_samples: Vec::new(),
            foreground_samples: Vec::new(),
            output_samples: Vec::new(),
        }
    }

    /// Render the exact host-rate `f32` slice passed to SDL by the callback.
    fn render_for_sdl(&mut self, requested: usize) -> &[f32] {
        self.unsigned_samples
            .resize(requested, UNSIGNED_PCM_SILENCE);
        self.foreground_samples
            .resize(requested, UNSIGNED_PCM_SILENCE);
        self.output_samples
            .resize(requested, <f32 as AudioFormatNum>::SILENCE);
        {
            let mut shared = lock_shared(&self.shared);
            if shared
                .music_stream
                .render_unsigned(&mut self.unsigned_samples)
            {
                let foreground_active = shared
                    .mixer
                    .render_auxiliary_unsigned(&mut self.foreground_samples);
                if foreground_active {
                    for (destination, foreground) in self
                        .unsigned_samples
                        .iter_mut()
                        .zip(self.foreground_samples.iter().copied())
                    {
                        *destination = average_unsigned_pcm(foreground, *destination);
                    }
                }
            } else {
                shared.mixer.render_unsigned(&mut self.unsigned_samples);
            }
        }
        for (destination, sample) in self
            .output_samples
            .iter_mut()
            .zip(self.unsigned_samples.iter().copied())
        {
            *destination = (f32::from(sample) - UNSIGNED_PCM_SCALE) / UNSIGNED_PCM_SCALE;
        }
        &self.output_samples
    }
}

impl AudioCallback<f32> for RuntimeAudioCallback {
    fn callback(&mut self, stream: &mut AudioStream, requested: i32) {
        let requested = usize::try_from(requested).unwrap_or(usize::MIN);
        let submission = self.render_for_sdl(requested);
        if let Err(error) = stream.put_data_f32(submission) {
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
        let callback = RuntimeAudioCallback::new(Arc::clone(&shared));
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
        let mut shared = lock_shared(&self.shared);
        shared.music_stream.stop();
        shared.mixer.play_background(clip);
        Ok(())
    }

    /// Load one validated VOC into the recovered flat stream lifecycle.
    pub fn load_background_stream(
        &mut self,
        encoded_voc: &[u8],
        source_rate_hz: u32,
    ) -> Result<Option<&'static [u8]>> {
        self.check_callback()?;
        let mut shared = lock_shared(&self.shared);
        shared.mixer.stop_background();
        let wait_prompt = shared.music_stream.load(encoded_voc, source_rate_hz)?;
        if wait_prompt.is_none() {
            return Ok(None);
        }
        drop(shared);
        self.stream
            .clear()
            .map_err(|error| anyhow!("clearing replaced SDL3 music stream: {error}"))?;
        Ok(wait_prompt)
    }

    /// Load normalized unsigned 8-bit PCM into the recovered flat stream lifecycle.
    pub fn load_background_pcm_stream(
        &mut self,
        samples: &[u8],
        source_rate_hz: u32,
        sample_rate_code: u8,
    ) -> Result<Option<&'static [u8]>> {
        self.check_callback()?;
        let mut shared = lock_shared(&self.shared);
        shared.mixer.stop_background();
        let wait_prompt =
            shared
                .music_stream
                .load_pcm(samples, source_rate_hz, sample_rate_code)?;
        if wait_prompt.is_none() {
            return Ok(None);
        }
        drop(shared);
        self.stream
            .clear()
            .map_err(|error| anyhow!("clearing replaced SDL3 music stream: {error}"))?;
        Ok(wait_prompt)
    }

    /// Start the VOC retained by [`Self::load_background_stream`].
    pub fn start_background_stream(&mut self) -> Result<()> {
        self.check_callback()?;
        lock_shared(&self.shared).music_stream.start()
    }

    /// Return the persistent recovered streamed-audio enable latch.
    pub fn background_channel_active(&self) -> bool {
        lock_shared(&self.shared).music_stream.channel_active()
    }

    /// Change the recovered streamed-audio enable latch used by every VOC gate.
    pub fn set_background_channel_active(&mut self, active: bool) -> Result<()> {
        self.check_callback()?;
        let mut shared = lock_shared(&self.shared);
        shared.music_stream.set_channel_active(active);
        if !active {
            shared.mixer.stop_background();
        }
        drop(shared);
        if !active {
            self.stream
                .clear()
                .map_err(|error| anyhow!("clearing disabled SDL3 music stream: {error}"))?;
        }
        self.check_callback()
    }

    /// Queue at most one recovered 16 KiB stream page.
    pub fn refill_background_stream(&mut self) -> Result<AudioStreamRefillOutcome> {
        self.check_callback()?;
        lock_shared(&self.shared).music_stream.refill()
    }

    /// Report whether a loaded stream is waiting for its explicit start call.
    pub fn background_stream_pending(&self) -> bool {
        lock_shared(&self.shared).music_stream.pending()
    }

    /// Return the native driver callback's remaining samples in the active stream page.
    pub fn background_stream_remaining(&self) -> Option<u16> {
        lock_shared(&self.shared).music_stream.playback_remaining()
    }

    /// Discard a loaded stream that has not started.
    pub fn discard_pending_background_stream(&mut self) -> bool {
        lock_shared(&self.shared).music_stream.discard_pending()
    }

    /// Apply one recovered SND request to direct playback or the live music buffers.
    pub fn play_sound_request(
        &mut self,
        request: crate::native::bloodprg::AudioClipRequest,
        banks: AudioPlaybackBanks<'_>,
    ) -> Result<AudioPlaybackOutcome> {
        self.check_callback()?;
        let mut shared = lock_shared(&self.shared);
        let outcome = shared.music_stream.update_clip(request, banks)?;
        let direct_playback = match &outcome {
            AudioPlaybackOutcome::PlaybackDisabled => false,
            AudioPlaybackOutcome::StopAndPlay(playback) => {
                let clip = RuntimePcmClip::from_encoded_snd(&playback.encoded_clip)?;
                shared.music_stream.stop();
                shared.mixer.play_exclusive(clip);
                true
            }
            AudioPlaybackOutcome::StreamMix(_) => false,
        };
        drop(shared);
        if direct_playback {
            self.stream
                .clear()
                .map_err(|error| anyhow!("clearing SDL3 stream before direct playback: {error}"))?;
        }
        Ok(outcome)
    }

    /// Start one foreground clip over active music.
    pub fn play_foreground(&mut self, clip: RuntimePcmClip) -> Result<()> {
        self.check_callback()?;
        lock_shared(&self.shared).mixer.play_foreground(clip);
        Ok(())
    }

    /// Apply one exact enable/disable transition from the recovered speaker gate.
    pub fn apply_speaker_gate(&mut self, action: SpeakerGateAction) -> Result<()> {
        self.check_callback()?;
        let mut shared = lock_shared(&self.shared);
        match action {
            SpeakerGateAction::Enable => shared.mixer.enable_speaker(pc_speaker_tone_clip()?),
            SpeakerGateAction::Disable => shared.mixer.disable_speaker(),
        }
        Ok(())
    }

    /// Stop prior sound and start one exclusive clip.
    pub fn play_exclusive(&mut self, clip: RuntimePcmClip) -> Result<()> {
        self.check_callback()?;
        let mut shared = lock_shared(&self.shared);
        shared.music_stream.stop();
        shared.mixer.play_exclusive(clip);
        drop(shared);
        self.stream
            .clear()
            .map_err(|error| anyhow!("clearing SDL3 audio stream: {error}"))?;
        Ok(())
    }

    /// Stop every source and clear samples already queued in SDL.
    pub fn stop_all(&mut self) -> Result<()> {
        let mut shared = lock_shared(&self.shared);
        shared.music_stream.stop();
        shared.mixer.stop_all();
        drop(shared);
        self.stream
            .clear()
            .map_err(|error| anyhow!("clearing SDL3 audio stream: {error}"))?;
        self.check_callback()
    }

    /// Stop digital stream and PCM playback while preserving the PC-speaker channel.
    pub fn stop_digital(&mut self) -> Result<()> {
        let mut shared = lock_shared(&self.shared);
        shared.music_stream.stop();
        shared.mixer.stop_digital();
        drop(shared);
        self.stream
            .clear()
            .map_err(|error| anyhow!("clearing SDL3 digital audio stream: {error}"))?;
        self.check_callback()
    }

    /// Stop background music while allowing a foreground clip to finish.
    pub fn stop_background(&mut self) -> Result<()> {
        let mut shared = lock_shared(&self.shared);
        shared.music_stream.stop();
        shared.mixer.stop_background();
        drop(shared);
        self.stream
            .clear()
            .map_err(|error| anyhow!("clearing SDL3 audio stream: {error}"))?;
        self.check_callback()
    }

    /// Return the current source-sample position of looping music.
    pub fn background_position(&self) -> Option<u64> {
        let shared = lock_shared(&self.shared);
        shared
            .music_stream
            .source_position()
            .or_else(|| shared.mixer.background_position())
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

fn mix_optional_unsigned_pcm(first: Option<u8>, second: Option<u8>) -> Option<u8> {
    match (first, second) {
        (Some(first), Some(second)) => Some(average_unsigned_pcm(first, second)),
        (Some(sample), None) | (None, Some(sample)) => Some(sample),
        (None, None) => None,
    }
}

fn pc_speaker_tone_clip() -> Result<RuntimePcmClip> {
    let half_period = PC_SPEAKER_PIT_DIVISOR / 2;
    let mut samples = vec![PC_SPEAKER_LOW_SAMPLE; PC_SPEAKER_PIT_DIVISOR];
    samples[half_period..].fill(PC_SPEAKER_HIGH_SAMPLE);
    RuntimePcmClip::new(PC_SPEAKER_PIT_CLOCK_HZ, samples)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use commander_blood_formats::snd::SndBank;
    use sha2::{Digest, Sha256};

    use crate::native::bloodprg::{
        AUDIO_STREAM_PAGE_BYTE_COUNT, AudioClipRequest, AudioMixOperation, AudioMixStatus,
        CREATIVE_VOICE_FILE_HEADER_BYTE_COUNT,
    };

    const TEST_OUTPUT_RATE_HZ: u32 = 10;
    const HALF_OUTPUT_RATE_HZ: u32 = TEST_OUTPUT_RATE_HZ / 2;
    const TEST_STREAM_RATE_HZ: u32 = 11_111;
    const TEST_STREAM_RATE_CODE: u8 = 166;
    const PHONE_COMPLETION_CLIP_INDEX: u16 = 2;
    const SHIPPED_BRIDGE_SOUND_BANK_BYTE_COUNT: usize = 30_960;
    const SHIPPED_BRIDGE_SOUND_BANK_CLIP_COUNT: u16 = 17;
    const SHIPPED_BRIDGE_SOUND_BANK_SHA256: &str =
        "8823a3f57c9075e21b36a147a00b9248b729da8316cf76018a8abe526867fb8f";
    const PHONE_COMPLETION_CLIP_START: usize = 3_683;
    const PHONE_COMPLETION_CLIP_END: usize = 6_114;
    const PHONE_COMPLETION_CLIP_HEADER: [u8; SND_CLIP_HEADER_BYTE_COUNT] =
        [1, 122, 9, 0, TEST_STREAM_RATE_CODE, 0];
    const PHONE_COMPLETION_CLIP_SHA256: &str =
        "5d1a3f368242b2e05d909325f0069b1091caee4324be47a3090f06dc4d83a89c";
    const PHONE_COMPLETION_PCM_SHA256: &str =
        "597f5daf1f52a87343cb983959b06a4495220c85b003b55d5dd806366532a6cb";
    const PHONE_COMPLETION_PCM_SAMPLE_COUNT: usize = 2_425;
    const PHONE_COMPLETION_NON_SILENT_SAMPLE_COUNT: usize = 2_371;
    const PHONE_COMPLETION_MIX_OUTPUT_SAMPLE_COUNT: u16 = 2_430;
    const PHONE_COMPLETION_MIXED_SAMPLE_COUNT: usize = 2_429;
    const PHONE_COMPLETION_SDL_PREFIX: [f32; 24] = [
        -0.0234375, -0.0234375, -0.0234375, -0.0234375, -0.0234375, 0.421875, 0.421875, 0.421875,
        0.421875, -0.375, -0.375, -0.375, -0.375, -0.0625, -0.0625, -0.0625, -0.0625, -0.0625,
        0.4375, 0.4375, 0.4375, 0.4375, -0.328125, -0.328125,
    ];

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

    #[test]
    fn recovered_pc_speaker_gate_starts_and_stops_the_programmed_square_wave() {
        let tone = pc_speaker_tone_clip().unwrap();
        assert_eq!(tone.sample_rate_hz(), PC_SPEAKER_PIT_CLOCK_HZ);
        assert_eq!(tone.samples().len(), PC_SPEAKER_PIT_DIVISOR);
        assert!(
            tone.samples()[..PC_SPEAKER_PIT_DIVISOR / 2]
                .iter()
                .all(|sample| *sample == PC_SPEAKER_LOW_SAMPLE)
        );
        assert!(
            tone.samples()[PC_SPEAKER_PIT_DIVISOR / 2..]
                .iter()
                .all(|sample| *sample == PC_SPEAKER_HIGH_SAMPLE)
        );

        let mut mixer = RuntimePcmMixer::new(PC_SPEAKER_PIT_CLOCK_HZ).unwrap();
        mixer.enable_speaker(tone);
        mixer.play_foreground(
            RuntimePcmClip::new(PC_SPEAKER_PIT_CLOCK_HZ, [PC_SPEAKER_HIGH_SAMPLE]).unwrap(),
        );
        mixer.stop_digital();
        let mut enabled = [UNSIGNED_PCM_SILENCE; 2];
        mixer.render_unsigned(&mut enabled);
        assert_eq!(enabled, [PC_SPEAKER_LOW_SAMPLE; 2]);

        mixer.disable_speaker();
        let mut disabled = [u8::MIN; 2];
        mixer.render_unsigned(&mut disabled);
        assert_eq!(disabled, [UNSIGNED_PCM_SILENCE; 2]);
    }

    #[test]
    fn recovered_stream_channel_latch_gates_load_start_and_survives_driver_stops() {
        let payload = generated_stream_payload(AUDIO_STREAM_PAGE_BYTE_COUNT);
        let encoded = encoded_voc_stream(&payload);
        let mut stream = RuntimeMusicStream::new(TEST_STREAM_RATE_HZ);

        stream.set_channel_active(false);
        assert_eq!(stream.load(&encoded, TEST_STREAM_RATE_HZ).unwrap(), None);
        stream.start().unwrap();
        assert!(!stream.channel_active());
        assert!(stream.stream.source.is_none());
        assert!(stream.cursor.is_none());

        stream.stop();
        assert!(!stream.channel_active());
        stream.set_channel_active(true);
        assert_eq!(
            stream.load(&encoded, TEST_STREAM_RATE_HZ).unwrap(),
            Some(crate::native::bloodprg::AUDIO_STREAM_WAIT_PROMPT)
        );
        stream.start().unwrap();
        assert!(stream.channel_active());
        assert!(stream.cursor.is_some());

        stream.stop();
        assert!(stream.channel_active());
    }

    #[test]
    fn recovered_stream_hands_off_between_original_sized_buffers() {
        let payload = generated_stream_payload(AUDIO_STREAM_PAGE_BYTE_COUNT * 2);
        let encoded = encoded_voc_stream(&payload);
        let mut stream = RuntimeMusicStream::new(TEST_STREAM_RATE_HZ);
        stream
            .load(&encoded, TEST_STREAM_RATE_HZ)
            .expect("synthetic stream loads");
        stream.start().expect("synthetic stream starts");
        let initial_remaining = stream.playback_remaining().unwrap();
        assert_eq!(usize::from(initial_remaining), AUDIO_STREAM_PAGE_BYTE_COUNT);
        assert_eq!(
            stream.playback.stream_buffers[0].status,
            AudioStreamBufferStatus::ReadyAndDriverOwned
        );

        assert!(matches!(
            stream.refill().expect("second page queues"),
            AudioStreamRefillOutcome::Submitted(AudioStreamSubmission {
                kind: AudioStreamSubmissionKind::Service,
                buffer_index: 1,
                source_page_index: 1,
                ..
            })
        ));
        let mut output = vec![u8::MIN; AUDIO_STREAM_PAGE_BYTE_COUNT + 1];
        assert!(stream.render_unsigned(&mut output));
        assert!(stream.playback_remaining().unwrap() < initial_remaining);

        let first_page_pcm_count = AUDIO_STREAM_PAGE_BYTE_COUNT - SND_CLIP_HEADER_BYTE_COUNT;
        assert_eq!(
            &output[..first_page_pcm_count],
            &payload[SND_CLIP_HEADER_BYTE_COUNT..AUDIO_STREAM_PAGE_BYTE_COUNT]
        );
        assert_eq!(
            &output[first_page_pcm_count..AUDIO_STREAM_PAGE_BYTE_COUNT],
            &[u8::MIN; SND_CLIP_HEADER_BYTE_COUNT]
        );
        assert_eq!(
            output[AUDIO_STREAM_PAGE_BYTE_COUNT],
            payload[AUDIO_STREAM_PAGE_BYTE_COUNT]
        );
        assert_eq!(
            stream.playback.stream_buffers[0].status,
            AudioStreamBufferStatus::Free
        );
        assert_eq!(
            stream.playback.stream_buffers[1].status,
            AudioStreamBufferStatus::ReadyAndDriverOwned
        );
    }

    #[test]
    fn recovered_dialogue_mix_mutates_samples_consumed_by_stream() {
        let payload = generated_stream_payload(AUDIO_STREAM_PAGE_BYTE_COUNT * 2);
        let encoded = encoded_voc_stream(&payload);
        let mut stream = RuntimeMusicStream::new(TEST_STREAM_RATE_HZ);
        stream.load(&encoded, TEST_STREAM_RATE_HZ).unwrap();
        stream.start().unwrap();

        let voice_samples = [200, 180, 160, 140];
        let streamed_dialogue = bank_with_one_clip(&voice_samples);
        let resident_effects = empty_bank();
        let outcome = stream
            .update_clip(
                AudioClipRequest::StreamedDialogue { index: 0 },
                AudioPlaybackBanks {
                    resident_effects: &resident_effects,
                    resident_effects_memory: resident_effects.payload(),
                    streamed_dialogue: &streamed_dialogue,
                },
            )
            .unwrap();
        assert!(matches!(
            outcome,
            AudioPlaybackOutcome::StreamMix(report)
                if report.status == AudioMixStatus::Mixed
        ));

        let original = &payload[SND_CLIP_HEADER_BYTE_COUNT..][..voice_samples.len()];
        let mut output = [u8::MIN; 4];
        assert!(stream.render_unsigned(&mut output));
        assert_eq!(
            output,
            [
                average_unsigned_pcm(voice_samples[0], original[0]),
                average_unsigned_pcm(voice_samples[1], original[1]),
                average_unsigned_pcm(voice_samples[2], original[2]),
                original[3],
            ]
        );
    }

    #[test]
    fn shipped_phone_completion_boing_decodes_mixes_and_reaches_sdl_in_order() {
        let Some(bank_path) = shipped_bridge_sound_bank_path() else {
            assert!(
                std::env::var_os("CBLOOD_REQUIRE_ACCURACY_TESTS").is_none(),
                "CBLOOD_REQUIRE_ACCURACY_TESTS=1 requires the shipped SN/TB.SND bank"
            );
            return;
        };
        let encoded_bank = std::fs::read(&bank_path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", bank_path.display()));
        assert_eq!(encoded_bank.len(), SHIPPED_BRIDGE_SOUND_BANK_BYTE_COUNT);
        assert_eq!(
            format!("{:x}", Sha256::digest(&encoded_bank)),
            SHIPPED_BRIDGE_SOUND_BANK_SHA256
        );

        let resident_effects = SndBank::decode(&encoded_bank).unwrap();
        assert_eq!(
            resident_effects.header().clip_count,
            SHIPPED_BRIDGE_SOUND_BANK_CLIP_COUNT
        );
        assert_eq!(
            resident_effects.offsets()[usize::from(PHONE_COMPLETION_CLIP_INDEX)],
            PHONE_COMPLETION_CLIP_START
        );
        assert_eq!(
            resident_effects.offsets()[usize::from(PHONE_COMPLETION_CLIP_INDEX) + 1],
            PHONE_COMPLETION_CLIP_END
        );

        let shipped_clip = resident_effects
            .clip(usize::from(PHONE_COMPLETION_CLIP_INDEX))
            .unwrap();
        assert_eq!(
            &shipped_clip.encoded()[..SND_CLIP_HEADER_BYTE_COUNT],
            &PHONE_COMPLETION_CLIP_HEADER
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(shipped_clip.encoded())),
            PHONE_COMPLETION_CLIP_SHA256
        );
        assert_eq!(shipped_clip.sample_rate_hz(), Some(TEST_STREAM_RATE_HZ));
        let shipped_pcm = shipped_clip.pcm().unwrap();
        assert_eq!(shipped_pcm.len(), PHONE_COMPLETION_PCM_SAMPLE_COUNT);
        assert_eq!(
            format!("{:x}", Sha256::digest(shipped_pcm)),
            PHONE_COMPLETION_PCM_SHA256
        );
        assert_eq!(
            shipped_pcm
                .iter()
                .filter(|sample| **sample != UNSIGNED_PCM_SILENCE)
                .count(),
            PHONE_COMPLETION_NON_SILENT_SAMPLE_COUNT
        );
        let decoded_clip = RuntimePcmClip::from_snd_clip(shipped_clip).unwrap();
        assert_eq!(decoded_clip.sample_rate_hz(), TEST_STREAM_RATE_HZ);
        assert_eq!(decoded_clip.samples(), shipped_pcm);

        let shared = Arc::new(Mutex::new(SharedAudioState::default()));
        let mut stream_payload = vec![UNSIGNED_PCM_SILENCE; AUDIO_STREAM_PAGE_BYTE_COUNT];
        stream_payload[SND_CLIP_RATE_CODE_INDEX] = TEST_STREAM_RATE_CODE;
        let encoded_stream = encoded_voc_stream(&stream_payload);
        let streamed_dialogue = empty_bank();

        let outcome = {
            let mut state = lock_shared(&shared);
            state
                .music_stream
                .load(&encoded_stream, TEST_STREAM_RATE_HZ)
                .unwrap();
            state.music_stream.start().unwrap();
            state
                .music_stream
                .update_clip(
                    AudioClipRequest::VoiceReaction {
                        bank_index: PHONE_COMPLETION_CLIP_INDEX,
                    },
                    AudioPlaybackBanks {
                        resident_effects: &resident_effects,
                        resident_effects_memory: resident_effects.payload(),
                        streamed_dialogue: &streamed_dialogue,
                    },
                )
                .unwrap()
        };
        assert!(matches!(
            outcome,
            AudioPlaybackOutcome::StreamMix(report)
                if report.status == AudioMixStatus::Mixed
                    && report.source_output_sample_count == PHONE_COMPLETION_MIX_OUTPUT_SAMPLE_COUNT
                    && report.source_byte_count_consumed == PHONE_COMPLETION_MIXED_SAMPLE_COUNT
                    && report.operations.as_ref() == [AudioMixOperation {
                        buffer_index: 0,
                        sample_count: PHONE_COMPLETION_MIXED_SAMPLE_COUNT,
                    }]
        ));

        let mut callback = RuntimeAudioCallback::new(shared);
        let submission = callback.render_for_sdl(PHONE_COMPLETION_SDL_PREFIX.len());
        assert_eq!(submission, PHONE_COMPLETION_SDL_PREFIX);
        assert!(submission.iter().any(|sample| *sample != 0.0));
    }

    fn encoded_voc_stream(payload: &[u8]) -> Vec<u8> {
        let mut encoded = vec![u8::MIN; CREATIVE_VOICE_FILE_HEADER_BYTE_COUNT];
        encoded.extend_from_slice(payload);
        encoded
    }

    fn generated_stream_payload(byte_count: usize) -> Vec<u8> {
        let mut payload = (0..byte_count)
            .map(|index| (index.wrapping_mul(29).wrapping_add(17)) as u8)
            .collect::<Vec<_>>();
        payload[SND_CLIP_RATE_CODE_INDEX] = TEST_STREAM_RATE_CODE;
        payload
    }

    fn bank_with_one_clip(samples: &[u8]) -> SndBank {
        let mut clip = vec![u8::MIN; SND_CLIP_HEADER_BYTE_COUNT];
        clip[SND_CLIP_RATE_CODE_INDEX] = TEST_STREAM_RATE_CODE;
        clip.extend_from_slice(samples);
        let mut encoded = vec![1, 0, 0, 0];
        encoded.extend_from_slice(&0_u32.to_le_bytes());
        encoded.extend_from_slice(&(clip.len() as u32).to_le_bytes());
        encoded.extend_from_slice(&clip);
        SndBank::decode(&encoded).unwrap()
    }

    fn shipped_bridge_sound_bank_path() -> Option<PathBuf> {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut candidates = Vec::new();
        if let Some(root) = std::env::var_os("CBLOOD_ORIGINAL_ARCHIVE_ROOT") {
            candidates.push(PathBuf::from(root).join("resources/SN/TB.SND"));
        }
        if let Some(root) = std::env::var_os("CBLOOD_ASSET_CACHE") {
            candidates.push(PathBuf::from(root).join("resources/SN/TB.SND"));
        }
        candidates.extend([
            workspace_root.join("output/_tmp_iso/resources/SN/TB.SND"),
            workspace_root.join("commander-blood-audio/_tmp_iso/resources/SN/TB.SND"),
            workspace_root.join("accuracy/cblood_install/cblood/resources/SN/TB.SND"),
        ]);
        candidates.into_iter().find(|path| path.is_file())
    }

    fn empty_bank() -> SndBank {
        SndBank::decode(&[u8::MIN; 8]).unwrap()
    }
}
