//! Lossless files from the native offline presentation runner.

use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, ensure};
use serde_json::json;
use sha2::{Digest, Sha256};

use super::game_lifecycle::native_scene_link_target;
use super::{
    ModernGameServices, OfflinePresentationInterval, OfflinePresentationSink, OriginalGameData,
    OriginalGameDataPaths, PRESENTATION_FRAME_DURATION, capture_offline_presentation,
};
use crate::asset_import::ImportedAssetManifest;
use crate::native::bloodprg::{GameSceneLink, PresentationResourceId, ScriptClock};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const SAMPLE_RATE: u64 = 48_000;

/// Export one complete native opening or credits sequence to a new directory.
/// Requires `ffmpeg` and `ffprobe` on PATH. Failed exports retain staging files.
pub fn export_presentation(
    assets: &Path,
    line: PresentationResourceId,
    output: &Path,
    max_frames: u64,
) -> Result<()> {
    ensure!(
        matches!(line.get(), 0 | 1),
        "expected opening or credits line"
    );
    ensure!(
        !output.exists(),
        "output already exists: {}",
        output.display()
    );
    let manifest = ImportedAssetManifest::load(assets)?;
    manifest.validate(assets, true)?;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let stage = parent.join(format!(
        ".offline-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    fs::create_dir(&stage)?;
    eprintln!(
        "Staging {} line {} at {}",
        manifest.game.title(),
        line.get(),
        stage.display()
    );
    let data = OriginalGameData::load_with_writable_root(
        OriginalGameDataPaths::from_root(assets)?,
        stage.join("userdata"),
    )?;
    let mut services = ModernGameServices::new_offline(
        (WIDTH, HEIGHT),
        data,
        ScriptClock {
            hour: 12,
            day: 2,
            month: 1,
        },
    )?;
    services.prepare_startup_resources()?;
    services.load_manu3_overlay()?;
    services.initialize_logical_viewport()?;
    services.load_initial_cartography_resource()?;
    services.submit_indexed_frame()?;
    services.present_artwork()?;
    let mut sink = FileSink::new(&stage)?;
    let report = capture_offline_presentation(
        &mut services,
        line,
        native_scene_link_target(GameSceneLink::Initial),
        max_frames,
        &mut sink,
    )?;
    sink.finish()?;
    ensure!(
        report.presented_frames == sink.frames
            && report.audio_samples == sink.samples
            && report.duration_ns == sink.elapsed_ns,
        "runner/sink accounting mismatch"
    );
    let endpoint = services.read_offline_rgba()?;
    fs::write(stage.join("endpoint.rgba"), &endpoint)?;
    let video_hash = format!("{:x}", sink.video_hash.clone().finalize());
    let audio_hash = format!("{:x}", sink.audio_hash.clone().finalize());
    let status = Command::new("ffmpeg")
        .args(["-nostdin", "-v", "error", "-n", "-i"])
        .arg(stage.join("video.mkv"))
        .args(["-f", "f32le", "-ar", "48000", "-ac", "1", "-i"])
        .arg(stage.join("audio.f32le"))
        .args([
            "-map",
            "0:v:0",
            "-map",
            "1:a:0",
            "-c:v",
            "copy",
            "-c:a",
            "pcm_f32le",
        ])
        .arg(stage.join("master.mkv"))
        .status()?;
    ensure!(status.success(), "ffmpeg mux failed: {status}");
    let video = decoded_hash(&stage.join("master.mkv"), true)?;
    let audio = decoded_hash(&stage.join("master.mkv"), false)?;
    ensure!(
        video
            == (
                video_hash.clone(),
                sink.frames * u64::from(WIDTH * HEIGHT * 4)
            ),
        "lossless video decode verification failed"
    );
    ensure!(
        audio == (audio_hash.clone(), sink.samples * 4),
        "lossless audio decode verification failed"
    );
    let timing = verify_timestamps(&stage.join("master.mkv"), sink.frames)?;
    fs::write(
        stage.join("timestamps.json"),
        serde_json::to_vec_pretty(&timing)?,
    )?;
    manifest.validate(assets, true)?;
    ensure!(
        ImportedAssetManifest::load(assets)? == manifest,
        "source manifest changed"
    );
    fs::write(
        stage.join("source-manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    let executable_hash = file_hash(&std::env::current_exe()?)?;
    fs::write(
        stage.join("report.json"),
        serde_json::to_vec_pretty(&json!({
            "schema": 1, "game": manifest.game, "presentation_line": line.get(),
            "complete_native_presentation": true, "complete_game": false,
            "runner": report, "width": WIDTH, "height": HEIGHT,
            "frame_duration_ns": PRESENTATION_FRAME_DURATION.as_nanos(),
            "sample_rate": SAMPLE_RATE, "audio_nonzero_samples": sink.nonzero_samples,
            "rgba_sha256": video_hash, "audio_f32le_sha256": audio_hash,
            "endpoint_rgba_sha256": format!("{:x}", Sha256::digest(&endpoint)),
            "exporter_sha256": executable_hash,
            "decoded_master_verified": true,
            "video_timestamps_verified": true,
            "timing": "Production 68ms presentation waits and shared PIT accumulator",
            "endpoint_policy": "Half-open capture; final flip retained separately without invented hold",
            "evidence_scope": "Native Rust presentation path, not whole-game DOS parity",
            "scene_link": "Initial", "script_clock": { "hour": 12, "day": 2, "month": 1 }
        }))?,
    )?;
    ensure!(!output.exists(), "output appeared during export");
    fs::rename(&stage, output)?;
    eprintln!("Verified export: {}", output.display());
    Ok(())
}

struct FileSink {
    encoder: Child,
    input: Option<ChildStdin>,
    audio: BufWriter<File>,
    timeline: BufWriter<File>,
    frames: u64,
    samples: u64,
    nonzero_samples: u64,
    elapsed_ns: u64,
    video_hash: Sha256,
    audio_hash: Sha256,
}

impl FileSink {
    fn new(stage: &Path) -> Result<Self> {
        let audio = BufWriter::new(File::create(stage.join("audio.f32le"))?);
        let timeline = BufWriter::new(File::create(stage.join("timeline.jsonl"))?);
        let mut encoder = Command::new("ffmpeg")
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
                "640x480",
                "-framerate",
            ])
            .arg(format!(
                "1000000000/{}",
                PRESENTATION_FRAME_DURATION.as_nanos()
            ))
            .args([
                "-i", "pipe:0", "-an", "-c:v", "ffv1", "-level", "3", "-pix_fmt", "bgra",
            ])
            .arg(stage.join("video.mkv"))
            .stdin(Stdio::piped())
            .spawn()
            .context("starting ffmpeg lossless encoder")?;
        let input = encoder.stdin.take();
        Ok(Self {
            encoder,
            input,
            audio,
            timeline,
            frames: 0,
            samples: 0,
            nonzero_samples: 0,
            elapsed_ns: 0,
            video_hash: Sha256::new(),
            audio_hash: Sha256::new(),
        })
    }

    fn finish(&mut self) -> Result<()> {
        self.input.take();
        let status = self.encoder.wait()?;
        ensure!(status.success(), "ffmpeg encoder failed: {status}");
        self.audio.flush()?;
        self.timeline.flush()?;
        Ok(())
    }
}

impl Drop for FileSink {
    fn drop(&mut self) {
        self.input.take();
        if !matches!(self.encoder.try_wait(), Ok(Some(_))) {
            let _ = self.encoder.kill();
            let _ = self.encoder.wait();
        }
    }
}

impl OfflinePresentationSink for FileSink {
    fn write_interval(&mut self, interval: OfflinePresentationInterval<'_>) -> Result<()> {
        ensure!(
            interval.start_ns == self.elapsed_ns,
            "noncontiguous frame timeline"
        );
        ensure!(
            u128::from(interval.duration_ns) == PRESENTATION_FRAME_DURATION.as_nanos(),
            "unexpected frame duration"
        );
        ensure!(
            interval.rgba.len() == (WIDTH * HEIGHT * 4) as usize,
            "unexpected frame size"
        );
        let end = self
            .elapsed_ns
            .checked_add(interval.duration_ns)
            .context("timeline overflow")?;
        let sample_end = (u128::from(end) * u128::from(SAMPLE_RATE) / 1_000_000_000) as u64;
        ensure!(
            self.samples + interval.audio.len() as u64 == sample_end,
            "audio timeline mismatch"
        );
        self.input
            .as_mut()
            .context("encoder already closed")?
            .write_all(interval.rgba)?;
        self.video_hash.update(interval.rgba);
        let mut pcm = Vec::with_capacity(interval.audio.len() * 4);
        for &sample in interval.audio {
            ensure!(sample.is_finite(), "nonfinite mixer output");
            self.nonzero_samples += u64::from(sample != 0.0);
            pcm.extend_from_slice(&sample.to_le_bytes());
        }
        self.audio.write_all(&pcm)?;
        self.audio_hash.update(&pcm);
        serde_json::to_writer(
            &mut self.timeline,
            &json!({
                "frame": self.frames, "start_ns": interval.start_ns, "duration_ns": interval.duration_ns,
                "sample_start": self.samples, "sample_count": interval.audio.len(),
                "rgba_sha256": format!("{:x}", Sha256::digest(interval.rgba)),
                "audio_sha256": format!("{:x}", Sha256::digest(&pcm))
            }),
        )?;
        self.timeline.write_all(b"\n")?;
        self.frames += 1;
        self.samples = sample_end;
        self.elapsed_ns = end;
        if self.frames % 500 == 0 {
            eprintln!("Captured {} frames ({:.3}s)", self.frames, end as f64 / 1e9);
        }
        Ok(())
    }
}

fn hash_reader(mut reader: impl Read) -> Result<(String, u64)> {
    let mut hash = Sha256::new();
    let mut bytes = 0;
    let mut buffer = [0; 65536];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
        bytes += n as u64;
    }
    Ok((format!("{:x}", hash.finalize()), bytes))
}

fn file_hash(path: &Path) -> Result<String> {
    Ok(hash_reader(File::open(path)?)?.0)
}

fn decoded_hash(path: &Path, video: bool) -> Result<(String, u64)> {
    let mut command = Command::new("ffmpeg");
    command.args(["-nostdin", "-v", "error", "-i"]).arg(path);
    if video {
        command.args([
            "-map",
            "0:v:0",
            "-fps_mode",
            "passthrough",
            "-pix_fmt",
            "rgba",
            "-f",
            "rawvideo",
        ]);
    } else {
        command.args(["-map", "0:a:0", "-c:a", "pcm_f32le", "-f", "f32le"]);
    }
    let mut child = command.arg("pipe:1").stdout(Stdio::piped()).spawn()?;
    let result = hash_reader(child.stdout.take().context("decoder stdout missing")?);
    if result.is_err() {
        let _ = child.kill();
    }
    let status = child.wait()?;
    ensure!(
        status.success(),
        "ffmpeg verification decoder failed: {status}"
    );
    result
}

fn verify_timestamps(path: &Path, frames: u64) -> Result<serde_json::Value> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_frames",
            "-show_streams",
            "-show_entries",
            "frame=best_effort_timestamp,duration:stream=time_base",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .context("probing encoded frame timestamps")?;
    ensure!(
        output.status.success(),
        "ffprobe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let timing: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    validate_timestamps(&timing, frames)?;
    Ok(timing)
}

fn validate_timestamps(timing: &serde_json::Value, count: u64) -> Result<()> {
    ensure!(
        timing["streams"][0]["time_base"] == "1/1000",
        "unexpected Matroska time base"
    );
    let frames = timing["frames"]
        .as_array()
        .context("ffprobe omitted frames")?;
    ensure!(
        frames.len() as u64 == count,
        "encoded timestamp count mismatch"
    );
    let duration_ms = u64::try_from(PRESENTATION_FRAME_DURATION.as_millis())?;
    ensure!(
        PRESENTATION_FRAME_DURATION.as_nanos() == u128::from(duration_ms) * 1_000_000,
        "frame interval cannot be represented exactly in Matroska milliseconds"
    );
    for (index, frame) in frames.iter().enumerate() {
        ensure!(
            frame["best_effort_timestamp"].as_u64() == Some(index as u64 * duration_ms),
            "frame {index} timestamp mismatch"
        );
        ensure!(
            frame["duration"].as_u64() == Some(duration_ms),
            "frame {index} duration mismatch"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_export_rejects_missing_shifted_or_retimed_frames() {
        let correct = json!({"streams": [{"time_base": "1/1000"}], "frames": [
            {"best_effort_timestamp": 0, "duration": 68},
            {"best_effort_timestamp": 68, "duration": 68}
        ]});
        validate_timestamps(&correct, 2).unwrap();
        assert!(validate_timestamps(&correct, 3).is_err());
        let mut shifted = correct.clone();
        shifted["frames"][1]["best_effort_timestamp"] = json!(69);
        assert!(validate_timestamps(&shifted, 2).is_err());
        let mut retimed = correct;
        retimed["frames"][0]["duration"] = json!(40);
        assert!(validate_timestamps(&retimed, 2).is_err());
    }
}
