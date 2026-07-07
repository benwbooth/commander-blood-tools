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
