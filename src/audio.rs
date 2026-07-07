//! Cross-platform audio output for the runnable engine, built on cpal (ALSA /
//! WASAPI / CoreAudio). Plays the game's unsigned 8-bit mono PCM (SND clips, VOC
//! music) by resampling into the device's native stream format on the fly.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Looping background-music player: streams u8 mono PCM at `src_rate` Hz to the
/// default output device until dropped/stopped. Playback position advances by a
/// fixed-point step so any device rate works (nearest-sample resampling — the
/// source material is 11 kHz 8-bit, so this is transparent for it).
pub struct MusicPlayer {
    stream: Option<cpal::Stream>,
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
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;
        let dev_rate = config.sample_rate().0.max(1);
        let channels = config.channels() as usize;
        let stop = Arc::new(AtomicBool::new(false));

        // Fixed-point (16.16) source-position stepping.
        let step = ((src_rate as u64) << 16) / dev_rate as u64;
        let pos = Arc::new(AtomicUsize::new(0));
        let pcm = Arc::new(pcm);

        let build = |device: &cpal::Device, config: &cpal::SupportedStreamConfig| {
            let pcm = Arc::clone(&pcm);
            let pos = Arc::clone(&pos);
            let stop = Arc::clone(&stop);
            device.build_output_stream(
                &config.config(),
                move |out: &mut [f32], _| {
                    if stop.load(Ordering::Relaxed) {
                        out.fill(0.0);
                        return;
                    }
                    let mut p = pos.load(Ordering::Relaxed);
                    for frame in out.chunks_mut(channels) {
                        let raw = p >> 16;
                        let s = if looped {
                            (pcm[raw % pcm.len()] as f32 - 128.0) / 128.0
                        } else if raw < pcm.len() {
                            // u8 unsigned PCM -> f32 in [-1, 1].
                            (pcm[raw] as f32 - 128.0) / 128.0
                        } else {
                            0.0 // play-once: silence past the end
                        };
                        for slot in frame.iter_mut() {
                            *slot = s;
                        }
                        p = p.wrapping_add(step as usize);
                    }
                    pos.store(p, Ordering::Relaxed);
                },
                |_err| {},
                None,
            )
        };

        let stream = build(&device, &config).ok()?;
        stream.play().ok()?;
        Some(Self {
            stream: Some(stream),
            stop,
        })
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.stream.take(); // drop closes the device stream
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
