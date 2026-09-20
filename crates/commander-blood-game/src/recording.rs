//! Opt-in capture of final GPU frames and the samples submitted to SDL.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc,
};
use std::thread::JoinHandle;
use std::time::Instant;

use anyhow::{Context, Result, bail};

const FPS: u64 = 30;
const NS: u64 = 1_000_000_000;

enum Event {
    Scene {
        time: u64,
        state: serde_json::Value,
    },
    Video {
        time: u64,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    Audio {
        time: u64,
        rate: u32,
        samples: Vec<f32>,
    },
    Finish(u64),
}

pub(crate) struct Recording {
    epoch: Instant,
    next_frame: AtomicU64,
    sender: mpsc::SyncSender<Event>,
    error: Mutex<Option<String>>,
    last_scene: Mutex<Option<serde_json::Value>>,
}

impl Recording {
    pub(crate) fn scene(&self, state: serde_json::Value) -> Result<()> {
        self.check()?;
        let mut last = self.last_scene.lock().unwrap();
        if last.as_ref() == Some(&state) {
            return Ok(());
        }
        self.sender
            .send(Event::Scene {
                time: self.elapsed_ns(),
                state: state.clone(),
            })
            .context("recording scene writer stopped")?;
        *last = Some(state);
        Ok(())
    }
    pub(crate) fn elapsed_ns(&self) -> u64 {
        self.epoch.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64
    }

    pub(crate) fn frame_due(&self) -> Result<Option<u64>> {
        self.check()?;
        let time = self.elapsed_ns();
        let frame = frame_at(time);
        if frame < self.next_frame.load(Ordering::Relaxed) {
            return Ok(None);
        }
        self.next_frame.store(frame + 1, Ordering::Relaxed);
        Ok(Some(time))
    }

    pub(crate) fn video(&self, time: u64, width: u32, height: u32, rgba: Vec<u8>) -> Result<()> {
        self.check()?;
        self.sender
            .send(Event::Video {
                time,
                width,
                height,
                rgba,
            })
            .context("recording encoder stopped")
    }

    pub(crate) fn audio(&self, samples: &[f32], rate: u32) {
        // Never block SDL behind GPU readback or video encoding. Overflow is fatal,
        // rather than silently producing a recording with missing sound.
        if let Err(error) = self.sender.try_send(Event::Audio {
            time: self.elapsed_ns(),
            rate,
            samples: samples.to_vec(),
        }) {
            *self.error.lock().unwrap() = Some(format!("recording audio queue: {error}"));
        }
    }

    fn check(&self) -> Result<()> {
        if let Some(error) = self.error.lock().unwrap().as_ref() {
            bail!("{error}");
        }
        Ok(())
    }
}

pub(crate) struct RecordingSession {
    pub(crate) handle: Arc<Recording>,
    worker: Option<JoinHandle<Result<()>>>,
}

impl RecordingSession {
    pub(crate) fn create(path: &Path) -> Result<Self> {
        fs::create_dir(path)
            .with_context(|| format!("recording directory must be new: {}", path.display()))?;
        let (sender, receiver) = mpsc::sync_channel(64);
        let path = path.to_owned();
        let handle = Arc::new(Recording {
            epoch: Instant::now(),
            next_frame: AtomicU64::new(0),
            sender,
            error: Mutex::new(None),
            last_scene: Mutex::new(None),
        });
        let worker = std::thread::spawn(move || write_recording(&path, receiver));
        Ok(Self {
            handle,
            worker: Some(worker),
        })
    }

    pub(crate) fn finish(mut self) -> Result<()> {
        self.close()
    }

    fn close(&mut self) -> Result<()> {
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        let _ = self
            .handle
            .sender
            .send(Event::Finish(self.handle.elapsed_ns()));
        worker
            .join()
            .map_err(|_| anyhow::anyhow!("recording writer panicked"))??;
        self.handle.check()
    }
}

impl Drop for RecordingSession {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

struct VideoEncoder {
    child: Child,
    input: Option<ChildStdin>,
}

impl Drop for VideoEncoder {
    fn drop(&mut self) {
        self.input.take();
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn frame_at(time: u64) -> u64 {
    (u128::from(time) * u128::from(FPS) / u128::from(NS)) as u64
}

fn write_recording(path: &Path, receiver: mpsc::Receiver<Event>) -> Result<()> {
    let mut encoder: Option<VideoEncoder> = None;
    let mut size = None;
    let mut last = Vec::new();
    let mut frames = 0;
    let mut audio = BufWriter::new(File::create(path.join("audio.f32le"))?);
    let mut audio_samples = 0u64;
    let mut audio_rate = None;
    let mut first_audio_ns = None;
    let mut timeline = BufWriter::new(File::create(path.join("timeline.jsonl"))?);
    let mut scenes = BufWriter::new(File::create(path.join("scenes.jsonl"))?);
    let end = loop {
        match receiver
            .recv()
            .context("recording ended without finalization")?
        {
            Event::Scene { time, state } => {
                writeln!(
                    scenes,
                    "{}",
                    serde_json::json!({"time_ns":time, "state":state})
                )?;
            }
            Event::Video {
                time,
                width,
                height,
                rgba,
            } => {
                anyhow::ensure!(
                    rgba.len() == width as usize * height as usize * 4,
                    "invalid captured frame size"
                );
                if size.is_none() {
                    let mut child = Command::new("ffmpeg")
                        .args([
                            "-nostdin",
                            "-v",
                            "error",
                            "-n",
                            "-f",
                            "rawvideo",
                            "-pixel_format",
                            "rgba",
                            "-video_size",
                        ])
                        .arg(format!("{width}x{height}"))
                        .args([
                            "-framerate",
                            "30",
                            "-i",
                            "pipe:0",
                            "-an",
                            "-c:v",
                            "ffv1",
                            "-level",
                            "3",
                        ])
                        .arg(path.join("video.mkv"))
                        .stdin(Stdio::piped())
                        .stdout(Stdio::null())
                        .stderr(File::create(path.join("encoder.log"))?)
                        .spawn()
                        .context("starting ffmpeg for lossless capture")?;
                    let input = child.stdin.take();
                    encoder = Some(VideoEncoder { child, input });
                    size = Some((width, height));
                    last = vec![0; rgba.len()];
                }
                anyhow::ensure!(
                    size == Some((width, height)),
                    "resizing during recording is unsupported"
                );
                let input = encoder.as_mut().unwrap().input.as_mut().unwrap();
                while frames < frame_at(time) {
                    input.write_all(&last)?;
                    frames += 1;
                }
                // Keep the new frame from this sampling instant onward.
                last = rgba;
                writeln!(
                    timeline,
                    "{}",
                    serde_json::json!({"kind":"video", "time_ns":time, "frame":frames})
                )?;
            }
            Event::Audio {
                time,
                rate,
                samples,
            } => {
                if audio_rate.is_none() {
                    audio_rate = Some(rate);
                    first_audio_ns = Some(time);
                    let leading = (u128::from(time) * u128::from(rate) / u128::from(NS)) as u64;
                    for _ in 0..leading {
                        audio.write_all(&0f32.to_le_bytes())?;
                    }
                    audio_samples = leading;
                }
                anyhow::ensure!(
                    audio_rate == Some(rate),
                    "audio rate changed during capture"
                );
                writeln!(
                    timeline,
                    "{}",
                    serde_json::json!({"kind":"audio", "time_ns":time, "sample":audio_samples, "samples":samples.len()})
                )?;
                for sample in &samples {
                    audio.write_all(&sample.to_le_bytes())?;
                }
                audio_samples += samples.len() as u64;
            }
            Event::Finish(time) => break time,
        }
    };
    let encoder = encoder
        .as_mut()
        .context("recording contains no rendered frames")?;
    let target = frame_at(end) + 1;
    while frames < target {
        encoder.input.as_mut().unwrap().write_all(&last)?;
        frames += 1;
    }
    encoder.input.take();
    anyhow::ensure!(
        encoder.child.wait()?.success(),
        "ffmpeg capture failed; see encoder.log"
    );
    audio.flush()?;
    timeline.flush()?;
    scenes.flush()?;
    let metadata = serde_json::json!({
        "schema":1, "width":size.unwrap().0, "height":size.unwrap().1, "fps":FPS,
        "frames":frames, "duration_ns":end, "audio_rate":audio_rate,
        "audio_samples":audio_samples, "first_audio_ns":first_audio_ns,
        "clock":"shared monotonic wall clock; audio measured at SDL submission",
        "video":"video.mkv", "audio":"audio.f32le"
    });
    fs::write(
        path.join("recording.json"),
        serde_json::to_vec_pretty(&metadata)?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timestamps_select_frames_without_float_drift() {
        assert_eq!(frame_at(0), 0);
        assert_eq!(frame_at(33_333_333), 0);
        assert_eq!(frame_at(33_333_334), 1);
        assert_eq!(frame_at(NS), 30);
        assert_eq!(frame_at(NS * 60 * 60), 108_000);
    }

    #[test]
    #[ignore = "requires ffmpeg on PATH"]
    fn lossless_writer_preserves_frames_audio_and_shared_start_offset() {
        let path = std::env::temp_dir().join(format!(
            "cb-recording-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        let (sender, receiver) = mpsc::sync_channel(8);
        let first = vec![255u8; 4 * 4 * 4];
        let second = vec![0u8; 4 * 4 * 4];
        sender
            .send(Event::Video {
                time: 0,
                width: 4,
                height: 4,
                rgba: first.clone(),
            })
            .unwrap();
        sender
            .send(Event::Audio {
                time: 10_000_000,
                rate: 1000,
                samples: vec![0.5, -0.5],
            })
            .unwrap();
        sender
            .send(Event::Video {
                time: 66_666_667,
                width: 4,
                height: 4,
                rgba: second.clone(),
            })
            .unwrap();
        sender
            .send(Event::Scene {
                time: 70_000_000,
                state: serde_json::json!({"subtitle":"hello"}),
            })
            .unwrap();
        sender.send(Event::Finish(100_000_000)).unwrap();
        write_recording(&path, receiver).unwrap();
        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(path.join("recording.json")).unwrap()).unwrap();
        assert_eq!(metadata["frames"], 4);
        assert_eq!(metadata["audio_samples"], 12);
        let samples: Vec<f32> = fs::read(path.join("audio.f32le"))
            .unwrap()
            .chunks_exact(4)
            .map(|v| f32::from_le_bytes(v.try_into().unwrap()))
            .collect();
        assert_eq!(&samples[..10], &[0.0; 10]);
        assert_eq!(&samples[10..], &[0.5, -0.5]);
        let decoded = Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(path.join("video.mkv"))
            .args(["-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"])
            .output()
            .unwrap();
        assert!(decoded.status.success());
        assert_eq!(
            decoded.stdout,
            [first.clone(), first, second.clone(), second].concat()
        );
        fs::remove_dir_all(path).unwrap();
    }
}
