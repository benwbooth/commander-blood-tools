//! Flushed JSONL semantic snapshots for normal interactive runtime diagnostics.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};

/// The runtime boundary represented by one live trace record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LiveTraceBoundary {
    /// The ordinary game-loop frame tail completed.
    Game,
    /// A blocking presentation frame was paced and presented.
    BlockingPresentation,
}

impl LiveTraceBoundary {
    fn as_str(self) -> &'static str {
        match self {
            Self::Game => "game",
            Self::BlockingPresentation => "blocking_presentation",
        }
    }
}

/// Owns the flushed JSONL stream and monotonic sequence clock for live tracing.
pub(super) struct LiveTraceWriter {
    output: BufWriter<File>,
    started_at: Instant,
    next_frame: u64,
}

impl LiveTraceWriter {
    pub(super) fn create(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating live trace directory {}", parent.display()))?;
        }
        let output = File::create(path)
            .with_context(|| format!("creating live trace {}", path.display()))?;
        Ok(Self {
            output: BufWriter::new(output),
            started_at: Instant::now(),
            next_frame: u64::MIN,
        })
    }

    pub(super) fn record(
        &mut self,
        boundary: LiveTraceBoundary,
        semantic: serde_json::Value,
    ) -> Result<()> {
        let next_frame = self
            .next_frame
            .checked_add(1)
            .context("live trace frame number overflow")?;
        let record = serde_json::json!({
            "schema": 1,
            "executable": "modern-rust",
            "frame": self.next_frame,
            "elapsed_ns": u64::try_from(self.started_at.elapsed().as_nanos()).unwrap_or(u64::MAX),
            "boundary": boundary.as_str(),
            "semantic": semantic,
        });
        serde_json::to_writer(&mut self.output, &record).context("writing live trace record")?;
        self.output
            .write_all(b"\n")
            .context("terminating live trace record")?;
        self.output.flush().context("flushing live trace record")?;
        self.next_frame = next_frame;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn test_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "commander-blood-live-trace-{}-{}-{}.jsonl",
            std::process::id(),
            std::thread::current().name().unwrap_or("writer"),
            label
        ))
    }

    #[test]
    fn records_are_flushed_jsonl_with_monotonic_frame_metadata() {
        let path = test_path("records");
        let mut writer = LiveTraceWriter::create(&path).unwrap();
        writer
            .record(
                LiveTraceBoundary::Game,
                serde_json::json!({"camera": "bridge"}),
            )
            .unwrap();
        let first = std::fs::read_to_string(&path).unwrap();
        assert_eq!(first.lines().count(), 1);
        writer
            .record(
                LiveTraceBoundary::BlockingPresentation,
                serde_json::json!({"caption": "visible"}),
            )
            .unwrap();
        drop(writer);

        let records: Vec<serde_json::Value> = std::fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(records[0]["frame"], 0);
        assert_eq!(records[1]["frame"], 1);
        assert!(
            records[1]["elapsed_ns"].as_u64().unwrap()
                >= records[0]["elapsed_ns"].as_u64().unwrap()
        );
        assert_eq!(records[0]["boundary"], "game");
        assert_eq!(records[1]["boundary"], "blocking_presentation");
        assert_eq!(records[1]["semantic"]["caption"], "visible");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn writer_surfaces_path_errors() {
        let blocker = std::env::temp_dir().join(format!(
            "commander-blood-live-trace-blocker-{}",
            std::process::id()
        ));
        std::fs::write(&blocker, b"file").unwrap();
        let path = blocker.join("trace.jsonl");
        let error = match LiveTraceWriter::create(&path) {
            Ok(_) => panic!("a file cannot be used as a trace directory"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("creating live trace directory"));
        std::fs::remove_file(blocker).unwrap();
    }
}
