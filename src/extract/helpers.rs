use super::*;

// ===========================================================================
// Helpers
// ===========================================================================

pub(super) fn run_ffmpeg(input: &Path, output: &Path, extra: &[&str]) -> bool {
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-i"]).arg(input);
    for arg in extra {
        cmd.arg(arg);
    }
    cmd.args(["-v", "warning"]).arg(output);
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

pub(super) fn find_file_recursive(dir: &Path, target: &str) -> Option<PathBuf> {
    let target_lower = target.to_lowercase();
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file_recursive(&path, target) {
                return Some(found);
            }
        } else if path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase() == target_lower)
            .unwrap_or(false)
        {
            return Some(path);
        }
    }
    None
}

pub(super) fn walk_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_files(&path));
            } else {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

pub(super) fn require(cmd: &str) {
    if !cmd_exists(cmd) {
        eprintln!("Required tool not found: {cmd}");
        std::process::exit(1);
    }
}

pub(super) fn require_any(cmds: &[&str]) -> String {
    for cmd in cmds {
        if cmd_exists(cmd) {
            return cmd.to_string();
        }
    }
    eprintln!("Required tool not found (need one of: {})", cmds.join(", "));
    std::process::exit(1)
}

pub(super) fn cmd_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
