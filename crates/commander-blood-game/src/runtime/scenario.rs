//! Deterministic action driver shared with the original-game runtime oracle.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;

const LOGICAL_SCREEN_WIDTH: i16 = 320;
const LOGICAL_SCREEN_HEIGHT: i16 = 200;
const BRIDGE_VIEW_FRAME_COUNT: u16 = 180;
const PARK_FRAME_TOLERANCE: u16 = 2;
const MAXIMUM_PARK_FRAME_COUNT: u16 = 600;
const CLICK_RELEASE_SETTLE_FRAMES: u16 = 2;
/// One runtime_boot wait unit advances about two recovered presentation loops.
///
/// The clean-boot oracle's 150-unit opening span reaches the authored terminal
/// frame of the 263-frame MIND.HNM stream. Keeping this conversion explicit
/// makes the same checked-in action timeline meaningful in the flat runtime.
const RUST_TICKS_PER_ORACLE_WAIT_UNIT: u16 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RuntimeScenarioKey {
    Character(char),
    Enter,
    Escape,
    Backspace,
    Space,
    Delete,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct RuntimeScenarioFrameInput {
    pub pointer_position: Option<[i16; 2]>,
    pub primary_pressed: bool,
    pub key: Option<RuntimeScenarioKey>,
    pub request_shutdown: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeScenarioActionKind {
    Move { position: [i16; 2] },
    Click { position: [i16; 2] },
    Key(RuntimeScenarioKey),
    Wait { frames: u16 },
    Park { edge_x: i16, target_frame: u16 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeScenarioAction {
    source: String,
    kind: RuntimeScenarioActionKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompletedRuntimeScenarioAction {
    index: usize,
    source: String,
}

/// Drives one checked-in DOS-oracle scenario through flat logical input.
pub(super) struct RuntimeScenarioDriver {
    scenario_path: PathBuf,
    actions: Vec<RuntimeScenarioAction>,
    trace: BufWriter<File>,
    action_index: usize,
    action_frame: u16,
    pending_trace: Option<CompletedRuntimeScenarioAction>,
    initial_trace_written: bool,
    frame_count: u64,
}

impl RuntimeScenarioDriver {
    pub(super) fn load(scenario_path: &Path, trace_path: &Path) -> Result<Self> {
        let source = std::fs::read_to_string(scenario_path)
            .with_context(|| format!("reading runtime scenario {}", scenario_path.display()))?;
        let actions = parse_scenario(&source, scenario_path)?;
        if actions.is_empty() {
            bail!("runtime scenario is empty: {}", scenario_path.display());
        }
        if let Some(parent) = trace_path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("creating runtime trace directory {}", parent.display())
            })?;
        }
        let trace = BufWriter::new(
            File::create(trace_path)
                .with_context(|| format!("creating runtime trace {}", trace_path.display()))?,
        );
        Ok(Self {
            scenario_path: scenario_path.to_owned(),
            actions,
            trace,
            action_index: usize::MIN,
            action_frame: u16::MIN,
            pending_trace: None,
            initial_trace_written: false,
            frame_count: u64::MIN,
        })
    }

    /// Write every state boundary that became observable before this frame.
    pub(super) fn record_due_boundaries(&mut self, semantic: &Value) -> Result<bool> {
        self.record_initial_boundary(semantic)?;
        if let Some(completed) = self.pending_trace.take() {
            self.write_record(
                completed.index + 1,
                "after",
                Some(&completed.source),
                semantic,
            )?;
        }
        Ok(self.action_index >= self.actions.len() && self.pending_trace.is_none())
    }

    pub(super) fn record_initial_boundary(&mut self, semantic: &Value) -> Result<()> {
        if !self.initial_trace_written {
            self.write_record(usize::MIN, "initial", None, semantic)?;
            self.initial_trace_written = true;
        }
        Ok(())
    }

    /// Advance exactly one translated game or blocking-presentation frame.
    pub(super) fn advance(
        &mut self,
        current_bridge_frame: Option<u16>,
    ) -> Result<RuntimeScenarioFrameInput> {
        self.frame_count = self.frame_count.wrapping_add(1);
        let Some(action) = self.actions.get(self.action_index) else {
            return Ok(RuntimeScenarioFrameInput {
                request_shutdown: true,
                ..RuntimeScenarioFrameInput::default()
            });
        };
        let mut input = RuntimeScenarioFrameInput::default();
        let complete = match action.kind {
            RuntimeScenarioActionKind::Move { position } => {
                input.pointer_position = Some(position);
                true
            }
            RuntimeScenarioActionKind::Click { position } => {
                input.pointer_position = Some(position);
                input.primary_pressed = self.action_frame == u16::MIN;
                self.action_frame >= CLICK_RELEASE_SETTLE_FRAMES
            }
            RuntimeScenarioActionKind::Key(key) => {
                input.key = Some(key);
                true
            }
            RuntimeScenarioActionKind::Wait { frames } => {
                self.action_frame + 1 >= frames.saturating_mul(RUST_TICKS_PER_ORACLE_WAIT_UNIT)
            }
            RuntimeScenarioActionKind::Park {
                edge_x,
                target_frame,
            } => {
                let current = current_bridge_frame.context(
                    "park action reached the runtime before a bridge frame was available",
                )?;
                let distance = current.abs_diff(target_frame);
                let circular_distance = distance.min(BRIDGE_VIEW_FRAME_COUNT - distance);
                if circular_distance <= PARK_FRAME_TOLERANCE {
                    // The DOS steering routine recenters its virtual ring cursor
                    // after every turn. Release modern edge pressure once the
                    // closed-loop target is reached so later waits do not drift.
                    input.pointer_position =
                        Some([LOGICAL_SCREEN_WIDTH / 2, LOGICAL_SCREEN_HEIGHT / 2]);
                    true
                } else if self.action_frame >= MAXIMUM_PARK_FRAME_COUNT {
                    bail!(
                        "park action could not reach bridge frame {target_frame}; stopped at {current}"
                    );
                } else {
                    input.pointer_position = Some([edge_x, LOGICAL_SCREEN_HEIGHT / 2]);
                    false
                }
            }
        };

        if complete {
            self.pending_trace = Some(CompletedRuntimeScenarioAction {
                index: self.action_index,
                source: action.source.clone(),
            });
            self.action_index += 1;
            self.action_frame = u16::MIN;
        } else {
            self.action_frame = self.action_frame.saturating_add(1);
        }
        Ok(input)
    }

    fn write_record(
        &mut self,
        action_index: usize,
        phase: &str,
        action: Option<&str>,
        semantic: &Value,
    ) -> Result<()> {
        let waiting_for_input = semantic
            .pointer("/presentation/waiting_for_input")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let record = serde_json::json!({
            "schema": 1,
            "executable": "modern-rust",
            "scenario": self.scenario_path,
            "action_index": action_index,
            "phase": phase,
            "action": action,
            "steps": self.frame_count,
            "machine": {
                "memory_model": "flat",
            },
            "guest_end": null,
            "liveness": if waiting_for_input { "waiting_for_input" } else { "progress" },
            "semantic": semantic,
        });
        serde_json::to_writer(&mut self.trace, &record).context("writing runtime trace record")?;
        writeln!(self.trace).context("terminating runtime trace record")?;
        self.trace.flush().context("flushing runtime trace record")
    }
}

fn parse_scenario(source: &str, path: &Path) -> Result<Vec<RuntimeScenarioAction>> {
    source
        .lines()
        .enumerate()
        .filter_map(|(line_index, line)| {
            let line = line.trim();
            (!line.is_empty() && !line.starts_with('#')).then_some((line_index + 1, line))
        })
        .map(|(line_number, line)| parse_action(line, path, line_number))
        .collect()
}

fn parse_action(line: &str, path: &Path, line_number: usize) -> Result<RuntimeScenarioAction> {
    let fields: Vec<_> = line.split_whitespace().collect();
    let fail = |message: &str| anyhow::anyhow!("{}:{line_number}: {message}", path.display());
    let position = |fields: &[&str]| -> Result<[i16; 2]> {
        if fields.len() != 3 {
            return Err(fail("pointer action requires x and y"));
        }
        let x = fields[1]
            .parse::<i16>()
            .map_err(|_| fail("invalid pointer x coordinate"))?;
        let y = fields[2]
            .parse::<i16>()
            .map_err(|_| fail("invalid pointer y coordinate"))?;
        if !(0..LOGICAL_SCREEN_WIDTH).contains(&x) || !(0..LOGICAL_SCREEN_HEIGHT).contains(&y) {
            return Err(fail("pointer coordinate is outside the 320 by 200 surface"));
        }
        Ok([x, y])
    };

    let kind = match fields.first().copied() {
        Some("move") => RuntimeScenarioActionKind::Move {
            position: position(&fields)?,
        },
        Some("click" | "sclick") => RuntimeScenarioActionKind::Click {
            position: position(&fields)?,
        },
        Some("key") => {
            if !(2..=3).contains(&fields.len()) {
                return Err(fail(
                    "key action requires a scan code and optional ASCII byte",
                ));
            }
            let scan_code = fields[1]
                .parse::<u8>()
                .map_err(|_| fail("invalid key scan code"))?;
            let ascii = fields
                .get(2)
                .map(|field| {
                    field
                        .parse::<u8>()
                        .map_err(|_| fail("invalid key ASCII byte"))
                })
                .transpose()?
                .unwrap_or(u8::MIN);
            RuntimeScenarioActionKind::Key(
                decode_scenario_key(scan_code, ascii)
                    .ok_or_else(|| fail("key action has no supported logical key translation"))?,
            )
        }
        Some("wait") => {
            if fields.len() != 2 {
                return Err(fail("wait action requires one frame count"));
            }
            let frames = fields[1]
                .parse::<u16>()
                .map_err(|_| fail("invalid wait frame count"))?;
            if frames == u16::MIN {
                return Err(fail("wait frame count must be positive"));
            }
            RuntimeScenarioActionKind::Wait { frames }
        }
        Some("park") => {
            if fields.len() != 3 {
                return Err(fail(
                    "park action requires an edge x and target bridge frame",
                ));
            }
            let edge_x = fields[1]
                .parse::<i16>()
                .map_err(|_| fail("invalid park edge x coordinate"))?;
            let target_frame = fields[2]
                .parse::<u16>()
                .map_err(|_| fail("invalid park target bridge frame"))?;
            if !(0..LOGICAL_SCREEN_WIDTH).contains(&edge_x) {
                return Err(fail("park edge x is outside the 320-pixel surface"));
            }
            if target_frame >= BRIDGE_VIEW_FRAME_COUNT {
                return Err(fail("park target is outside the 180-frame bridge ring"));
            }
            RuntimeScenarioActionKind::Park {
                edge_x,
                target_frame,
            }
        }
        Some(command) => return Err(fail(&format!("unsupported scenario command {command:?}"))),
        None => return Err(fail("empty scenario action")),
    };
    Ok(RuntimeScenarioAction {
        source: line.to_owned(),
        kind,
    })
}

fn decode_scenario_key(scan_code: u8, ascii: u8) -> Option<RuntimeScenarioKey> {
    match (scan_code, ascii) {
        (_, b' '..=b'~') => Some(RuntimeScenarioKey::Character(char::from(ascii))),
        (1, _) => Some(RuntimeScenarioKey::Escape),
        (14, _) => Some(RuntimeScenarioKey::Backspace),
        (28, _) => Some(RuntimeScenarioKey::Enter),
        (57, _) => Some(RuntimeScenarioKey::Space),
        (72, _) => Some(RuntimeScenarioKey::ArrowUp),
        (75, _) => Some(RuntimeScenarioKey::ArrowLeft),
        (77, _) => Some(RuntimeScenarioKey::ArrowRight),
        (80, _) => Some(RuntimeScenarioKey::ArrowDown),
        (83, _) => Some(RuntimeScenarioKey::Delete),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_checked_in_oracle_scenario_uses_the_supported_language() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../accuracy/scenarios");
        let mut parsed = usize::MIN;
        for entry in std::fs::read_dir(&root).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("tsv") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            assert!(!parse_scenario(&source, &path).unwrap().is_empty());
            parsed += 1;
        }
        assert!(parsed > 1);
    }

    #[test]
    fn click_is_pressed_for_one_frame_then_settled_after_release() {
        let action = parse_action("click 125 118", Path::new("scenario.tsv"), 1).unwrap();
        let trace_path = std::env::temp_dir().join(format!(
            "commander-blood-scenario-{}-{}.jsonl",
            std::process::id(),
            std::thread::current().name().unwrap_or("click")
        ));
        let mut driver = RuntimeScenarioDriver {
            scenario_path: PathBuf::from("scenario.tsv"),
            actions: vec![action],
            trace: BufWriter::new(File::create(&trace_path).unwrap()),
            action_index: 0,
            action_frame: 0,
            pending_trace: None,
            initial_trace_written: false,
            frame_count: 0,
        };

        assert_eq!(
            driver.advance(None).unwrap(),
            RuntimeScenarioFrameInput {
                pointer_position: Some([125, 118]),
                primary_pressed: true,
                ..RuntimeScenarioFrameInput::default()
            }
        );
        assert_eq!(
            driver.advance(None).unwrap(),
            RuntimeScenarioFrameInput {
                pointer_position: Some([125, 118]),
                ..RuntimeScenarioFrameInput::default()
            }
        );
        assert_eq!(driver.action_index, 0);
        assert_eq!(
            driver.advance(None).unwrap(),
            RuntimeScenarioFrameInput {
                pointer_position: Some([125, 118]),
                ..RuntimeScenarioFrameInput::default()
            }
        );
        assert_eq!(driver.action_index, 1);
        assert!(driver.pending_trace.is_some());
        let _ = std::fs::remove_file(trace_path);
    }

    #[test]
    fn unknown_commands_are_errors_instead_of_silent_no_ops() {
        let error = parse_action("poke 1234 ff", Path::new("scenario.tsv"), 7).unwrap_err();
        assert!(error.to_string().contains("scenario.tsv:7"));
        assert!(error.to_string().contains("unsupported scenario command"));
    }

    #[test]
    fn oracle_wait_units_use_the_calibrated_presentation_clock_conversion() {
        let action = parse_action("wait 1", Path::new("scenario.tsv"), 1).unwrap();
        let trace_path = std::env::temp_dir().join(format!(
            "commander-blood-wait-{}-{}.jsonl",
            std::process::id(),
            std::thread::current().name().unwrap_or("wait")
        ));
        let mut driver = RuntimeScenarioDriver {
            scenario_path: PathBuf::from("scenario.tsv"),
            actions: vec![action],
            trace: BufWriter::new(File::create(&trace_path).unwrap()),
            action_index: 0,
            action_frame: 0,
            pending_trace: None,
            initial_trace_written: false,
            frame_count: 0,
        };

        driver.advance(None).unwrap();
        assert_eq!(driver.action_index, 0);
        driver.advance(None).unwrap();
        assert_eq!(driver.action_index, 1);
        let _ = std::fs::remove_file(trace_path);
    }
}
