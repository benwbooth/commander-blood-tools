//! Lossless files from the native offline presentation runner.

use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, ensure};
use serde_json::json;
use sha2::{Digest, Sha256};

use super::game_lifecycle::native_scene_link_target;
use super::offline_game::{
    OfflineDialogueChapter, OfflineGameSink, capture_dialogue_chapter, capture_startup_cinematic,
};
use super::offline_video::OfflineVideoWriter;
use super::{
    ModernGameServices, OfflinePresentationInterval, OfflinePresentationSink, OriginalGameData,
    OriginalGameDataPaths, capture_offline_presentation,
};
use crate::asset_import::ImportedAssetManifest;
use crate::native::bloodprg::{GameSceneLink, PresentationResourceId, ScriptClock};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const SAMPLE_RATE: u64 = 48_000;

enum ExportTarget {
    Presentation(PresentationResourceId),
    StartupCinematic,
    Sequence(String),
    Dialogue(OfflineDialogueChapter),
}

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
    export(assets, ExportTarget::Presentation(line), output, max_frames)
}

/// Export the first complete authored cinematic list through the main lifecycle.
/// The logo reel is executed during bootstrap, but is not included in this capture.
pub fn export_startup_cinematic(assets: &Path, output: &Path, max_frames: u64) -> Result<()> {
    export(assets, ExportTarget::StartupCinematic, output, max_frames)
}

/// Render an authored DESCRIPT sequence using the native startup panel player.
/// Selecting a chapter does not establish a gameplay route to that record.
pub fn export_sequence(assets: &Path, record: &str, output: &Path, max_frames: u64) -> Result<()> {
    export(
        assets,
        ExportTarget::Sequence(record.to_owned()),
        output,
        max_frames,
    )
}

/// Render an explicitly source-bound native dialogue chapter without pointer input.
pub fn export_dialogue(assets: &Path, plan: &Path, output: &Path, max_frames: u64) -> Result<()> {
    let chapter =
        serde_json::from_slice(&fs::read(plan)?).context("reading the dialogue chapter plan")?;
    export(assets, ExportTarget::Dialogue(chapter), output, max_frames)
}

fn export(assets: &Path, target: ExportTarget, output: &Path, max_frames: u64) -> Result<()> {
    ensure!(max_frames > 0, "offline frame cap must be nonzero");
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
    eprintln!("Staging {} at {}", manifest.game.title(), stage.display());
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
    let mut sink = FileSink::new(&stage)?;
    let (report, endpoint, target_name, line) = match target {
        ExportTarget::Dialogue(chapter) => {
            let (report, endpoint) =
                capture_dialogue_chapter(services, &chapter, max_frames, &mut sink)?;
            (report, endpoint, "dialogue_chapter", None)
        }
        ExportTarget::Presentation(line) => {
            services.prepare_startup_resources()?;
            services.load_manu3_overlay()?;
            services.initialize_logical_viewport()?;
            services.load_initial_cartography_resource()?;
            services.submit_indexed_frame()?;
            services.present_artwork()?;
            let report = capture_offline_presentation(
                &mut services,
                line,
                native_scene_link_target(GameSceneLink::Initial),
                max_frames,
                &mut sink,
            )?;
            (
                serde_json::to_value(report)?,
                services.read_offline_rgba()?,
                "blocking_presentation",
                Some(line.get()),
            )
        }
        ExportTarget::StartupCinematic | ExportTarget::Sequence(_) => {
            let record = match &target {
                ExportTarget::Sequence(name) => Some(name.as_str()),
                _ => None,
            };
            let (report, endpoint) =
                capture_startup_cinematic(services, record, max_frames, &mut sink)?;
            (
                serde_json::to_value(report)?,
                endpoint,
                if record.is_some() {
                    "descript_sequence"
                } else {
                    "startup_cinematic"
                },
                None,
            )
        }
    };
    sink.finish()?;
    ensure!(
        report["presented_frames"].as_u64() == Some(sink.frames)
            && report["audio_samples"].as_u64() == Some(sink.samples)
            && report["duration_ns"].as_u64() == Some(sink.elapsed_ns),
        "runner/sink accounting mismatch"
    );
    fs::write(stage.join("endpoint.rgba"), &endpoint)?;
    let video_hash = format!("{:x}", sink.video_hash.clone().finalize());
    let audio_hash = format!("{:x}", sink.audio_hash.clone().finalize());
    mux_master(&stage, &sink.intervals)?;
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
    let timing = verify_timestamps(&stage.join("master.mkv"), &sink.intervals)?;
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
            "schema": 2, "game": manifest.game, "presentation_line": line, "target": target_name,
            "complete_native_presentation": true, "complete_game": false,
            "runner": report, "width": WIDTH, "height": HEIGHT,
            "video_codec": "lossless VP9 profile 1, full-range RGB",
            "frame_intervals": "Native game/presentation waits; see timeline.jsonl",
            "sample_rate": SAMPLE_RATE, "audio_nonzero_samples": sink.nonzero_samples,
            "rgba_sha256": video_hash, "audio_f32le_sha256": audio_hash,
            "endpoint_rgba_sha256": format!("{:x}", Sha256::digest(&endpoint)),
            "exporter_sha256": executable_hash,
            "decoded_master_verified": true,
            "video_timestamps_verified": true,
            "timing": "Production 46ms/68ms waits and shared PIT accumulator, without render-only interpolation",
            "endpoint_policy": "Half-open capture; final flip retained separately without invented hold",
            "evidence_scope": "Native Rust presentation path, not whole-game DOS parity",
            "initial_scene_link": "Initial", "script_clock": { "hour": 12, "day": 2, "month": 1 },
            "packed_clock_seed": if line.is_none() { Some(39) } else { None }
        }))?,
    )?;
    ensure!(!output.exists(), "output appeared during export");
    fs::rename(&stage, output)?;
    eprintln!("Verified export: {}", output.display());
    Ok(())
}

struct FileSink {
    video: OfflineVideoWriter,
    audio: BufWriter<File>,
    timeline: BufWriter<File>,
    states: BufWriter<File>,
    intervals: Vec<(u64, u64)>,
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
        let video = OfflineVideoWriter::new(&stage.join("video.mkv"), WIDTH, HEIGHT)?;
        Ok(Self {
            video,
            audio,
            timeline,
            states: BufWriter::new(File::create(stage.join("native-state.jsonl"))?),
            intervals: Vec::new(),
            frames: 0,
            samples: 0,
            nonzero_samples: 0,
            elapsed_ns: 0,
            video_hash: Sha256::new(),
            audio_hash: Sha256::new(),
        })
    }

    fn finish(&mut self) -> Result<()> {
        self.video.finish()?;
        self.audio.flush()?;
        self.timeline.flush()?;
        self.states.flush()?;
        Ok(())
    }
}

impl OfflinePresentationSink for FileSink {
    fn write_interval(&mut self, interval: OfflinePresentationInterval<'_>) -> Result<()> {
        ensure!(
            interval.start_ns == self.elapsed_ns,
            "noncontiguous frame timeline"
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
        self.video
            .write(interval.start_ns, interval.duration_ns, interval.rgba)?;
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
        self.intervals
            .push((interval.start_ns, interval.duration_ns));
        self.frames += 1;
        self.samples = sample_end;
        self.elapsed_ns = end;
        if self.frames % 500 == 0 {
            eprintln!("Captured {} frames ({:.3}s)", self.frames, end as f64 / 1e9);
        }
        Ok(())
    }
}

impl OfflineGameSink for FileSink {
    fn write_native_state(&mut self, time_ns: u64, state: &serde_json::Value) -> Result<()> {
        serde_json::to_writer(
            &mut self.states,
            &json!({"time_ns": time_ns, "state": state}),
        )?;
        self.states.write_all(b"\n")?;
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

fn mux_master(stage: &Path, intervals: &[(u64, u64)]) -> Result<()> {
    let (_, last_duration) = intervals
        .last()
        .context("cannot mux an empty native timeline")?;
    // WebM SimpleBlocks retain PTS, not frame durations. Do not let the demuxer's
    // guessed frame rate extend a short final wait during stream copy.
    let packet_durations = format!(
        "setts=duration='if(eq(N,{}),{}/(1000000000*TB),NEXT_PTS-PTS)'",
        intervals.len() - 1,
        last_duration,
    );
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
        .args(["-bsf:v", &packet_durations])
        .arg(stage.join("master.mkv"))
        .status()?;
    ensure!(status.success(), "ffmpeg mux failed: {status}");
    Ok(())
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
            "-enc_time_base:v",
            "1:1000",
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

fn verify_timestamps(path: &Path, intervals: &[(u64, u64)]) -> Result<serde_json::Value> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_frames",
            "-show_streams",
            "-show_format",
            "-show_entries",
            "frame=best_effort_timestamp:stream=time_base:format=duration",
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
    validate_timestamps(&timing, intervals)?;
    Ok(timing)
}

fn validate_timestamps(timing: &serde_json::Value, intervals: &[(u64, u64)]) -> Result<()> {
    ensure!(
        timing["streams"][0]["time_base"] == "1/1000",
        "unexpected Matroska time base"
    );
    let frames = timing["frames"]
        .as_array()
        .context("ffprobe omitted frames")?;
    ensure!(
        frames.len() == intervals.len(),
        "encoded timestamp count mismatch"
    );
    let mut end_ns = 0;
    for (index, (frame, &(start_ns, duration_ns))) in frames.iter().zip(intervals).enumerate() {
        ensure!(
            start_ns == end_ns && duration_ns > 0,
            "invalid reference interval"
        );
        ensure!(
            start_ns % 1_000_000 == 0 && duration_ns % 1_000_000 == 0,
            "reference interval is not representable in Matroska milliseconds"
        );
        ensure!(
            frame["best_effort_timestamp"].as_u64() == Some(start_ns / 1_000_000),
            "frame {index} timestamp mismatch"
        );
        end_ns = start_ns
            .checked_add(duration_ns)
            .context("reference timeline overflow")?;
    }
    let duration: f64 = timing["format"]["duration"]
        .as_str()
        .context("ffprobe omitted duration")?
        .parse()?;
    ensure!(
        duration.is_finite() && (duration * 1e9 - end_ns as f64).abs() < 1000.0,
        "encoded duration {duration} does not match native endpoint {}",
        end_ns as f64 / 1e9
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires FFmpeg and FFprobe on PATH"]
    fn offline_export_preserves_variable_intervals_pixels_and_audio() {
        verify_export_for_intervals(&[68_000_000, 46_000_000, 68_000_000]);
        let mut short_tail = vec![68_000_000; 48];
        short_tail.extend([46_000_000, 68_000_000, 46_000_000]);
        verify_export_for_intervals(&short_tail);
        verify_export_for_intervals(&[46_000_000]);
    }

    fn verify_export_for_intervals(durations: &[u64]) {
        let root = std::env::temp_dir().join(format!(
            "offline-vfr-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let mut sink = FileSink::new(&root).unwrap();
        let mut rgba = vec![0; (WIDTH * HEIGHT * 4) as usize];
        for (index, pixel) in rgba.chunks_exact_mut(4).enumerate() {
            pixel.copy_from_slice(&[index as u8, (index / WIDTH as usize) as u8, 211, 255]);
        }
        for (index, &duration_ns) in durations.iter().enumerate() {
            rgba[0] = index as u8;
            let audio = vec![
                if index == 1 { -0.25 } else { 0.5 };
                (duration_ns * SAMPLE_RATE / 1_000_000_000) as usize
            ];
            sink.write_interval(OfflinePresentationInterval {
                start_ns: sink.elapsed_ns,
                duration_ns,
                rgba: &rgba,
                audio: &audio,
            })
            .unwrap();
        }
        sink.finish().unwrap();
        mux_master(&root, &sink.intervals).unwrap();
        assert_eq!(
            decoded_hash(&root.join("master.mkv"), true).unwrap(),
            (
                format!("{:x}", sink.video_hash.clone().finalize()),
                sink.frames * u64::from(WIDTH * HEIGHT * 4)
            )
        );
        assert_eq!(
            decoded_hash(&root.join("master.mkv"), false).unwrap(),
            (
                format!("{:x}", sink.audio_hash.clone().finalize()),
                sink.samples * 4
            )
        );
        verify_timestamps(&root.join("master.mkv"), &sink.intervals).unwrap();
        drop(sink);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn offline_export_rejects_missing_shifted_or_retimed_frames() {
        let correct = json!({"streams": [{"time_base": "1/1000"}], "frames": [
            {"best_effort_timestamp": 0}, {"best_effort_timestamp": 68}
        ], "format": {"duration": "0.114000"}});
        let intervals = [(0, 68_000_000), (68_000_000, 46_000_000)];
        validate_timestamps(&correct, &intervals).unwrap();
        assert!(validate_timestamps(&correct, &intervals[..1]).is_err());
        let mut shifted = correct.clone();
        shifted["frames"][1]["best_effort_timestamp"] = json!(69);
        assert!(validate_timestamps(&shifted, &intervals).is_err());
        let mut retimed = correct;
        retimed["format"]["duration"] = json!("0.115000");
        assert!(validate_timestamps(&retimed, &intervals).is_err());
    }
}
