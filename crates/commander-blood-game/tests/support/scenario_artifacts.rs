use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

const ARTIFACT_ROOT_ENV: &str = "CBLOOD_FIDELITY_ARTIFACT_ROOT";
const TIMEOUT_ENV: &str = "CBLOOD_SCENARIO_TIMEOUT_SECONDS";
const DEFAULT_TIMEOUT_SECONDS: u64 = 600;
const HASH_BUFFER_BYTES: usize = 65_536;
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Keep successful captures too: assertions run after the process runner returns.
pub struct ScenarioArtifacts(pub PathBuf);

impl ScenarioArtifacts {
    pub fn create(workspace: &Path, trace_name: &str) -> io::Result<Self> {
        let root = std::env::var_os(ARTIFACT_ROOT_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|| workspace.join("output/fidelity"));
        fs::create_dir_all(&root)?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = root.join(format!(
            "{trace_name}-{timestamp}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path)?;
        eprintln!("Production scenario artifacts: {}", path.display());
        Ok(Self(path))
    }

    pub fn record_inputs(
        &self,
        command: &Command,
        scenario: &Path,
        asset_cache: &Path,
        writable: &Path,
        timeout: Duration,
    ) -> io::Result<()> {
        fs::copy(scenario, self.0.join("scenario.tsv"))?;
        let mut inputs = Vec::new();
        for path in [
            PathBuf::from(command.get_program()),
            scenario.to_owned(),
            asset_cache.join("manifest.json"),
        ] {
            inputs.push(serde_json::json!({"path": path, "sha256": sha256_file(&path)?}));
        }
        let initial_writable = self.0.join("initial-writable");
        fs::create_dir(&initial_writable)?;
        if writable.exists() {
            for entry in fs::read_dir(writable)? {
                let entry = entry?;
                if !entry.file_type()?.is_file() {
                    return Err(io::Error::other(
                        "scenario setup must contain regular files only",
                    ));
                }
                let snapshot = initial_writable.join(entry.file_name());
                fs::copy(entry.path(), &snapshot)?;
                inputs
                    .push(serde_json::json!({"path": snapshot, "sha256": sha256_file(&snapshot)?}));
            }
        }
        let metadata = serde_json::json!({
            "command": format!("{command:?}"),
            "timeout_seconds": timeout.as_secs(),
            "asset_cache": asset_cache,
            "display": std::env::var_os("DISPLAY").map(|v| v.to_string_lossy().into_owned()),
            "inputs": inputs,
        });
        fs::write(
            self.0.join("inputs.json"),
            serde_json::to_vec_pretty(&metadata)?,
        )
    }
}

pub fn timeout() -> io::Result<Duration> {
    let seconds = match std::env::var(TIMEOUT_ENV) {
        Ok(value) => value
            .parse::<u64>()
            .ok()
            .filter(|seconds| *seconds > 0)
            .ok_or_else(|| io::Error::other(format!("{TIMEOUT_ENV} must be a positive integer")))?,
        Err(std::env::VarError::NotPresent) => DEFAULT_TIMEOUT_SECONDS,
        Err(error) => return Err(io::Error::other(error)),
    };
    Ok(Duration::from_secs(seconds))
}

fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; HASH_BUFFER_BYTES];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            return Ok(format!("{:x}", digest.finalize()));
        }
        digest.update(&buffer[..count]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_artifacts_identify_inputs_and_preserve_initial_saves() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "cblood-artifact-test-{}-{timestamp}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let cache = root.join("assets");
        let writable = root.join("writable");
        let retained = root.join("capture");
        for path in [&cache, &writable, &retained] {
            fs::create_dir(path).unwrap();
        }
        fs::write(cache.join("manifest.json"), b"{}").unwrap();
        fs::write(writable.join("GAME1.SAV"), b"abc").unwrap();
        let scenario = root.join("input.tsv");
        fs::write(&scenario, b"frames 1\n").unwrap();
        let artifacts = ScenarioArtifacts(retained.clone());
        artifacts
            .record_inputs(
                &Command::new(std::env::current_exe().unwrap()),
                &scenario,
                &cache,
                &writable,
                Duration::from_secs(2),
            )
            .unwrap();
        drop(artifacts);
        fs::write(writable.join("GAME1.SAV"), b"changed").unwrap();
        assert_eq!(
            fs::read(retained.join("scenario.tsv")).unwrap(),
            b"frames 1\n"
        );
        assert_eq!(
            fs::read(retained.join("initial-writable/GAME1.SAV")).unwrap(),
            b"abc"
        );
        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(retained.join("inputs.json")).unwrap()).unwrap();
        assert_eq!(metadata["timeout_seconds"], 2);
        assert_eq!(metadata["inputs"].as_array().unwrap().len(), 4);
        assert_eq!(
            metadata["inputs"][3]["sha256"],
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
