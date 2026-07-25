//! Cross-platform audio output for the runnable engine, built on cpal (ALSA /
//! WASAPI / CoreAudio). Plays the game's unsigned 8-bit mono PCM (SND clips, VOC
//! music) by resampling into the device's native stream format on the fly.
//!
//! MIXING FOLLOWS THE GAME, not the OS. The original never hands several sounds
//! to the hardware at once: the loader writes one into a voice buffer
//! (`AH=3Fh` @`0x4049`) and the streamer AVERAGES later ones into it
//! (`lodsb / add al,es:[di] / rcr al,1` @`0xBB6D`). Three independent device
//! streams — what this module used to open — get summed by the OS at full
//! amplitude and can clip, which the game cannot do.
//!
//! So every [`MusicPlayer`] is now a handle on ONE shared output stream, and the
//! callback folds the active sources with [`crate::snd::mix_unsigned_pcm_layered`].
//! The public API is unchanged, so callers gained the game's mixing without
//! knowing about it.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

/// One sound feeding the shared mixer.
struct MixSource {
    id: usize,
    pcm: Arc<Vec<u8>>,
    /// 16.16 fixed-point read cursor, stepped per output sample.
    pos: usize,
    step: usize,
    looped: bool,
}

impl MixSource {
    /// Render this source's next `n` samples as unsigned 8-bit PCM, returning
    /// how many were real (a play-once source that has ended returns fewer).
    fn render(&mut self, out: &mut [u8]) -> usize {
        let len = self.pcm.len();
        if len == 0 {
            return 0;
        }
        let mut produced = 0usize;
        for slot in out.iter_mut() {
            let raw = self.pos >> 16;
            if self.looped {
                *slot = self.pcm[raw % len];
            } else if raw < len {
                *slot = self.pcm[raw];
            } else {
                break; // silence past the end; the source is finished
            }
            self.pos = self.pos.wrapping_add(self.step);
            produced += 1;
        }
        produced
    }

    fn finished(&self) -> bool {
        !self.looped && (self.pos >> 16) >= self.pcm.len()
    }
}

/// The shared mixer: every playing sound, folded with the game's rule.
#[derive(Default)]
pub struct AudioMixer {
    sources: Vec<MixSource>,
    next_id: usize,
}

impl AudioMixer {
    /// Fill `out` with unsigned 8-bit PCM, layering every active source, and drop
    /// the ones that have ended.
    ///
    /// Sources are folded in insertion order, which matters: the layered average
    /// weights the most recently added source most heavily (see
    /// [`crate::snd::mix_unsigned_pcm_layered`]). Silence when nothing plays.
    pub fn render(&mut self, out: &mut [u8]) {
        out.fill(crate::snd::SILENCE);
        if self.sources.is_empty() {
            return;
        }
        let mut rendered: Vec<Vec<u8>> = Vec::with_capacity(self.sources.len());
        for source in self.sources.iter_mut() {
            let mut buffer = vec![crate::snd::SILENCE; out.len()];
            let produced = source.render(&mut buffer);
            if produced > 0 {
                buffer.truncate(produced);
                rendered.push(buffer);
            }
        }
        let slices: Vec<&[u8]> = rendered.iter().map(Vec::as_slice).collect();
        crate::snd::mix_unsigned_pcm_layered(&slices, out);
        self.sources.retain(|s| !s.finished());
    }

    /// Add a source; returns its handle id.
    pub fn add(&mut self, pcm: Arc<Vec<u8>>, step: usize, looped: bool) -> usize {
        self.next_id += 1;
        let id = self.next_id;
        self.sources.push(MixSource { id, pcm, pos: 0, step, looped });
        id
    }

    pub fn remove(&mut self, id: usize) {
        self.sources.retain(|s| s.id != id);
    }

    pub fn active(&self) -> usize {
        self.sources.len()
    }
}

/// The process-wide mixer. One device stream, however many sounds.
static MIXER: std::sync::OnceLock<Arc<Mutex<AudioMixer>>> = std::sync::OnceLock::new();
static OUTPUT: Mutex<Option<cpal::Stream>> = Mutex::new(None);
/// The device rate the shared stream opened at, needed to compute a source step.
static DEVICE_RATE: AtomicUsize = AtomicUsize::new(0);

pub fn mixer() -> Arc<Mutex<AudioMixer>> {
    Arc::clone(MIXER.get_or_init(|| Arc::new(Mutex::new(AudioMixer::default()))))
}

/// Open the shared output stream once. Returns the device sample rate, or `None`
/// when there is no device (the engine stays silent — audio is never a hard
/// dependency).
fn ensure_output() -> Option<u32> {
    let rate = DEVICE_RATE.load(Ordering::Relaxed);
    if rate != 0 {
        return Some(rate as u32);
    }
    let mut slot = OUTPUT.lock().ok()?;
    if slot.is_some() {
        return Some(DEVICE_RATE.load(Ordering::Relaxed) as u32);
    }
    let device = cpal::default_host().default_output_device()?;
    let config = device.default_output_config().ok()?;
    let dev_rate = config.sample_rate().0.max(1);
    let channels = config.channels() as usize;
    let shared = mixer();
    let stream = device
        .build_output_stream(
            &config.config(),
            move |out: &mut [f32], _| {
                let frames = out.len() / channels.max(1);
                let mut pcm = vec![crate::snd::SILENCE; frames];
                if let Ok(mut m) = shared.lock() {
                    m.render(&mut pcm);
                }
                for (frame, sample) in out.chunks_mut(channels).zip(pcm) {
                    // u8 unsigned PCM -> f32 in [-1, 1].
                    let v = (sample as f32 - 128.0) / 128.0;
                    for slot in frame.iter_mut() {
                        *slot = v;
                    }
                }
            },
            |_err| {},
            None,
        )
        .ok()?;
    stream.play().ok()?;
    *slot = Some(stream);
    DEVICE_RATE.store(dev_rate as usize, Ordering::Relaxed);
    Some(dev_rate)
}

/// Looping background-music player: streams u8 mono PCM at `src_rate` Hz to the
/// default output device until dropped/stopped. Playback position advances by a
/// fixed-point step so any device rate works (nearest-sample resampling — the
/// source material is 11 kHz 8-bit, so this is transparent for it).
pub struct MusicPlayer {
    /// This sound's handle in the shared mixer.
    id: usize,
    stop: Arc<AtomicBool>,
}

impl MusicPlayer {
    /// Start looping playback. Returns `None` when no output device is available
    /// (the engine stays silent — audio is never a hard dependency).
    pub fn start(pcm: Vec<u8>, src_rate: u32) -> Option<Self> {
        Self::start_inner(pcm, src_rate, true)
    }

    /// Play once (voice clips): the stream goes silent at the end of the data and
    /// idles until dropped.
    pub fn start_once(pcm: Vec<u8>, src_rate: u32) -> Option<Self> {
        Self::start_inner(pcm, src_rate, false)
    }

    fn start_inner(pcm: Vec<u8>, src_rate: u32, looped: bool) -> Option<Self> {
        if pcm.is_empty() || src_rate == 0 {
            return None;
        }
        let dev_rate = ensure_output()?;
        // Fixed-point (16.16) source-position stepping, as before -- only the
        // DESTINATION changed, from a private stream to the shared mixer.
        let step = (((src_rate as u64) << 16) / dev_rate as u64) as usize;
        let id = mixer().lock().ok()?.add(Arc::new(pcm), step, looped);
        Some(Self { id, stop: Arc::new(AtomicBool::new(false)) })
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Ok(mut m) = mixer().lock() {
            m.remove(self.id); // the shared stream stays open for other sounds
        }
    }
}

impl Drop for MusicPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The 8253/8254 PIT input clock (Hz) — the base frequency the PC-speaker channel-2
/// square wave is divided down from.
pub const PIT_CLOCK_HZ: f32 = 1_193_182.0;

/// Convert a PIT frequency divisor (the value written to port 0x42) to the PC-speaker
/// tone frequency in Hz, exactly as the hardware does: `1193182 / divisor`. The game's
/// beep handler (`cmd_handler_pc_speaker_beep` 0x6c0) writes divisor `0x2e9c` → ~100 Hz.
pub fn pit_divisor_to_hz(divisor: u16) -> f32 {
    if divisor == 0 {
        return 0.0;
    }
    PIT_CLOCK_HZ / divisor as f32
}

/// Synthesize the PC-speaker beep as `secs` of unsigned-8-bit mono square wave at `hz`
/// (sampled at `rate`) — the waveform the speaker gate produces. Returns the PCM buffer,
/// playable through [`MusicPlayer::start_once`]. This reproduces the game's decoded
/// PC-speaker SFX (distinct from its VOC audio) in the cross-platform audio path.
pub fn square_wave_pcm(hz: f32, secs: f32, rate: u32) -> Vec<u8> {
    let n = ((rate as f32) * secs).max(0.0) as usize;
    if hz <= 0.0 || n == 0 {
        return vec![0x80; n]; // silence (unsigned-8 midpoint)
    }
    let period = rate as f32 / hz; // samples per full cycle
    (0..n)
        .map(|i| {
            // First half of each period high, second half low — a 50% square wave.
            if (i as f32 % period) < period / 2.0 {
                0xC0
            } else {
                0x40
            }
        })
        .collect()
}

/// Play the decoded PC-speaker beep (PIT `divisor`) for `secs` on the default device,
/// returning a play-once player (or `None` if no device). Convenience wrapper tying the
/// decoded divisor→frequency to the square-wave synth + cpal output.
pub fn beep(divisor: u16, secs: f32) -> Option<MusicPlayer> {
    let rate = 22_050u32;
    let pcm = square_wave_pcm(pit_divisor_to_hz(divisor), secs, rate);
    MusicPlayer::start_once(pcm, rate)
}

#[cfg(test)]
mod tests {

    /// The mixer is device-free by construction, so the game's mixing rule is
    /// testable here even though playback is not.
    #[test]
    fn the_shared_mixer_layers_sources_the_way_the_game_does() {
        use std::sync::Arc;
        let mut mix = AudioMixer::default();
        let mut out = vec![0u8; 4];

        // Nothing playing is silence, not noise.
        mix.render(&mut out);
        assert_eq!(out, vec![crate::snd::SILENCE; 4]);

        // One source plays UNATTENUATED -- the loader's overwrite (0x4049), not
        // an average against silence.
        let a = Arc::new(vec![0x00u8, 0x40, 0xFF, 0x80]);
        let id_a = mix.add(Arc::clone(&a), 1 << 16, true);
        mix.render(&mut out);
        assert_eq!(out, *a, "a lone sound is not halved");

        // A second source averages in (0xBB6D). Both restart at 0 each render
        // only because the first render advanced them; use fresh ones.
        let mut mix2 = AudioMixer::default();
        let b = Arc::new(vec![0xFFu8, 0xC0, 0x00, 0x80]);
        mix2.add(Arc::clone(&a), 1 << 16, true);
        mix2.add(Arc::clone(&b), 1 << 16, true);
        let mut out2 = vec![0u8; 4];
        mix2.render(&mut out2);
        for i in 0..4 {
            assert_eq!(out2[i], crate::snd::snd_mix_average(b[i], a[i]), "sample {i}");
        }

        // Removing a source stops it contributing.
        mix.remove(id_a);
        assert_eq!(mix.active(), 0);
        mix.render(&mut out);
        assert_eq!(out, vec![crate::snd::SILENCE; 4]);
    }

    /// A play-once source ends and is reaped; a looping one never is.
    #[test]
    fn play_once_sources_are_reaped_and_loops_are_not() {
        use std::sync::Arc;
        let mut mix = AudioMixer::default();
        mix.add(Arc::new(vec![0x10u8, 0x20]), 1 << 16, false);
        let mut out = vec![0u8; 8];
        mix.render(&mut out);
        assert_eq!(&out[..2], &[0x10, 0x20], "the data plays");
        assert_eq!(&out[2..], &[crate::snd::SILENCE; 6], "then silence past the end");
        assert_eq!(mix.active(), 0, "a finished play-once source is dropped");

        let mut looping = AudioMixer::default();
        looping.add(Arc::new(vec![0x10u8, 0x20]), 1 << 16, true);
        let mut out2 = vec![0u8; 8];
        looping.render(&mut out2);
        assert_eq!(out2, vec![0x10, 0x20, 0x10, 0x20, 0x10, 0x20, 0x10, 0x20]);
        assert_eq!(looping.active(), 1, "a loop is never reaped");
    }
    use super::*;

    #[test]
    fn pit_divisor_matches_the_hardware_formula() {
        // The game's beep divisor 0x2e9c -> ~100 Hz.
        let hz = pit_divisor_to_hz(0x2e9c);
        assert!((hz - 100.0).abs() < 1.0, "0x2e9c -> ~100 Hz, got {hz}");
        // A440 would need divisor ~2712.
        assert!((pit_divisor_to_hz(2712) - 440.0).abs() < 1.0);
        // Divisor 0 is treated as silence (avoid div-by-zero).
        assert_eq!(pit_divisor_to_hz(0), 0.0);
    }

    #[test]
    fn square_wave_has_the_right_period_and_swing() {
        // 100 Hz at 22050 -> 220.5 samples/period; first half high, second half low.
        let pcm = square_wave_pcm(100.0, 0.05, 22050);
        assert_eq!(pcm.len(), (22050.0 * 0.05) as usize);
        let period = 22050.0f32 / 100.0;
        assert!(pcm[0] > 0x80, "cycle starts high");
        assert!(pcm[(period * 0.75) as usize] < 0x80, "second half is low");
        // It oscillates (both high and low samples present).
        assert!(pcm.iter().any(|&s| s > 0x80) && pcm.iter().any(|&s| s < 0x80));
    }

    #[test]
    fn zero_frequency_is_silence() {
        let pcm = square_wave_pcm(0.0, 0.01, 22050);
        assert!(pcm.iter().all(|&s| s == 0x80));
    }
}
