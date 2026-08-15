//! Build byte-exact Commander Blood VM resource bundles and runnable trees.

use std::fs;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};

use crate::bloodscript;

const SCRIPT_COUNT: usize = 5;
const COMPILED_EXTENSIONS: [&str; 2] = ["COD", "BAS"];
const COMPANION_EXTENSIONS: [&str; 3] = ["DEB", "DIC", "VAR"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleEntry {
    pub script: String,
    pub extension: String,
    pub bytes: usize,
    pub origin: &'static str,
    pub comparison: &'static str,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuntimeTreeStats {
    pub hardlinked_files: usize,
    pub copied_files: usize,
    pub directories: usize,
}

/// Compile all ten checked-in BloodScript sources and copy their fifteen data
/// companions. Every output is compared with the shipped resource before it is
/// written; a mismatch aborts the bundle.
pub fn compile_bundle(
    source_dir: &Path,
    original_dir: &Path,
    output_dir: &Path,
) -> Result<Vec<BundleEntry>> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("creating VM bundle directory {}", output_dir.display()))?;
    let mut entries = Vec::new();

    for script in 1..=SCRIPT_COUNT {
        let script_name = format!("SCRIPT{script}");
        for extension in COMPILED_EXTENSIONS {
            let lower = extension.to_ascii_lowercase();
            let source_path = source_dir.join(format!("script{script}.{lower}.blood"));
            let source = fs::read_to_string(&source_path)
                .with_context(|| format!("reading BloodScript source {}", source_path.display()))?;
            let compiled = bloodscript::compile(&source)
                .with_context(|| format!("compiling {}", source_path.display()))?;
            let file_name = format!("{script_name}.{extension}");
            let original_path = original_dir.join(&file_name);
            let original = fs::read(&original_path)
                .with_context(|| format!("reading shipped VM image {}", original_path.display()))?;
            require_equal(&file_name, &compiled, &original)?;
            fs::write(output_dir.join(&file_name), &compiled)
                .with_context(|| format!("writing compiled VM image {file_name}"))?;
            entries.push(BundleEntry {
                script: script_name.clone(),
                extension: extension.to_string(),
                bytes: compiled.len(),
                origin: "compiled",
                comparison: "byte_exact",
            });
        }

        for extension in COMPANION_EXTENSIONS {
            let file_name = format!("{script_name}.{extension}");
            let original_path = original_dir.join(&file_name);
            let bytes = fs::read(&original_path)
                .with_context(|| format!("reading VM companion {}", original_path.display()))?;
            fs::write(output_dir.join(&file_name), &bytes)
                .with_context(|| format!("writing VM companion {file_name}"))?;
            entries.push(BundleEntry {
                script: script_name.clone(),
                extension: extension.to_string(),
                bytes: bytes.len(),
                origin: "preserved",
                comparison: "byte_exact",
            });
        }
    }
    Ok(entries)
}

/// Materialize a runnable extracted-CD tree, omitting the 25 script resources
/// while cloning assets, then install a freshly compiled exact script bundle.
/// Files are hard-linked when possible and copied only across filesystem
/// boundaries.
pub fn build_runtime_tree(
    source_dir: &Path,
    original_dir: &Path,
    output_dir: &Path,
) -> Result<(Vec<BundleEntry>, RuntimeTreeStats)> {
    require_empty_or_missing(output_dir)?;
    fs::create_dir_all(output_dir)
        .with_context(|| format!("creating runtime tree {}", output_dir.display()))?;
    let mut stats = RuntimeTreeStats::default();
    clone_asset_tree(original_dir, output_dir, original_dir, &mut stats)?;
    let entries = compile_bundle(source_dir, original_dir, output_dir)?;
    Ok((entries, stats))
}

pub fn manifest(entries: &[BundleEntry]) -> String {
    let mut output = String::from("script\timage\tbytes\torigin\tcomparison\n");
    for entry in entries {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            entry.script, entry.extension, entry.bytes, entry.origin, entry.comparison
        ));
    }
    output
}

fn require_equal(name: &str, compiled: &[u8], original: &[u8]) -> Result<()> {
    if compiled == original {
        return Ok(());
    }
    let first_difference = compiled
        .iter()
        .zip(original)
        .position(|(left, right)| left != right)
        .unwrap_or(compiled.len().min(original.len()));
    bail!(
        "compiled {name} differs from the shipped image at 0x{first_difference:04X} (compiled {} bytes, shipped {} bytes)",
        compiled.len(),
        original.len()
    )
}

fn require_empty_or_missing(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let mut entries = fs::read_dir(path)
        .with_context(|| format!("reading runtime output directory {}", path.display()))?;
    if entries.next().transpose()?.is_some() {
        bail!("runtime output directory {} is not empty", path.display());
    }
    Ok(())
}

fn clone_asset_tree(
    source: &Path,
    destination: &Path,
    root: &Path,
    stats: &mut RuntimeTreeStats,
) -> Result<()> {
    for entry in fs::read_dir(source).with_context(|| format!("reading {}", source.display()))? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let source_path = entry.path();
        let relative = source_path
            .strip_prefix(root)
            .map_err(|_| anyhow!("{} is outside {}", source_path.display(), root.display()))?;
        let destination_path = destination.join(
            relative
                .file_name()
                .ok_or_else(|| anyhow!("asset path {} has no file name", relative.display()))?,
        );
        if file_type.is_dir() {
            fs::create_dir_all(&destination_path)?;
            stats.directories += 1;
            clone_asset_tree(&source_path, &destination_path, root, stats)?;
        } else if file_type.is_file() {
            if is_script_resource(relative) {
                continue;
            }
            match fs::hard_link(&source_path, &destination_path) {
                Ok(()) => stats.hardlinked_files += 1,
                Err(_) => {
                    fs::copy(&source_path, &destination_path).with_context(|| {
                        format!(
                            "copying runtime asset {} to {}",
                            source_path.display(),
                            destination_path.display()
                        )
                    })?;
                    stats.copied_files += 1;
                }
            }
        } else {
            bail!("unsupported asset entry {}", source_path.display());
        }
    }
    Ok(())
}

fn is_script_resource(relative: &Path) -> bool {
    if relative
        .parent()
        .is_some_and(|parent| !parent.as_os_str().is_empty())
    {
        return false;
    }
    let Some(name) = relative.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let upper = name.to_ascii_uppercase();
    (1..=SCRIPT_COUNT).any(|script| {
        COMPILED_EXTENSIONS
            .iter()
            .chain(COMPANION_EXTENSIONS.iter())
            .any(|extension| upper == format!("SCRIPT{script}.{extension}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_root(name: &str) -> PathBuf {
        let unique = format!(
            "cb-vm-bundle-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::env::temp_dir().join(unique)
    }

    fn fixture(root: &Path) -> (PathBuf, PathBuf) {
        let sources = root.join("sources");
        let original = root.join("original");
        fs::create_dir_all(&sources).unwrap();
        fs::create_dir_all(&original).unwrap();
        for script in 1..=SCRIPT_COUNT {
            for extension in COMPILED_EXTENSIONS {
                fs::write(
                    sources.join(format!(
                        "script{script}.{}.blood",
                        extension.to_ascii_lowercase()
                    )),
                    "; format: bloodscript-ir-v1\n00000000: END\n",
                )
                .unwrap();
                fs::write(original.join(format!("SCRIPT{script}.{extension}")), [0xFF]).unwrap();
            }
            for extension in COMPANION_EXTENSIONS {
                fs::write(
                    original.join(format!("SCRIPT{script}.{extension}")),
                    [script as u8, extension.as_bytes()[0]],
                )
                .unwrap();
            }
        }
        (sources, original)
    }

    #[test]
    fn compiles_all_ten_images_and_preserves_companions() {
        let root = test_root("bundle");
        let (sources, original) = fixture(&root);
        let output = root.join("bundle");
        let entries = compile_bundle(&sources, &original, &output).unwrap();
        assert_eq!(entries.len(), 25);
        assert_eq!(fs::read(output.join("SCRIPT3.COD")).unwrap(), [0xFF]);
        assert_eq!(fs::read(output.join("SCRIPT3.DEB")).unwrap(), [3, b'D']);
        assert!(entries.iter().all(|entry| entry.comparison == "byte_exact"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn runtime_tree_replaces_scripts_without_mutating_originals() {
        let root = test_root("runtime");
        let (sources, original) = fixture(&root);
        fs::write(original.join("BLOODPRG.EXE"), b"executable").unwrap();
        fs::create_dir(original.join("SQ")).unwrap();
        fs::write(original.join("SQ/INTRO.HNM"), b"asset").unwrap();
        let output = root.join("runtime");
        let (entries, stats) = build_runtime_tree(&sources, &original, &output).unwrap();
        assert_eq!(entries.len(), 25);
        assert_eq!(fs::read(output.join("SCRIPT1.BAS")).unwrap(), [0xFF]);
        assert_eq!(fs::read(output.join("SQ/INTRO.HNM")).unwrap(), b"asset");
        assert_eq!(fs::read(original.join("SCRIPT1.BAS")).unwrap(), [0xFF]);
        assert_eq!(stats.directories, 1);
        assert!(stats.hardlinked_files + stats.copied_files >= 2);
        fs::remove_dir_all(root).unwrap();
    }
}
