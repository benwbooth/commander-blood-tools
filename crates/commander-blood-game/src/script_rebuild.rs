//! Startup compilation and byte-exact verification of editable game scripts.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result, bail};
use commander_blood_formats::archive::BloodResourceName;
use commander_blood_script_compiler::{ProfileDialect, compile_descript, compile_profile};
use sha2::{Digest, Sha256};

use crate::game::GameVariant;

const SCRIPT_SOURCE_ENVIRONMENT_VARIABLE: &str = "CBLOOD_SCRIPT_SOURCE";
const COMPILED_SCRIPT_CACHE_DIRECTORY: &str = "compiled-scripts-v1";
const DESCRIPT_SOURCE_RELATIVE_PATH: &str = "descript/DESCRIPT.descript";
const COMMANDER_VM_SOURCE_DIRECTORY_RELATIVE_PATH: &str = "vm/profiles";
const SEQUEL_VM_SOURCE_DIRECTORY_RELATIVE_PATH: &str = "vm/big-bug-bang-profiles";
const DESCRIPT_COMPILED_FILENAME: &str = "DESCRIPT.DES";
const COMMANDER_SCRIPT_EXTENSIONS: [&str; 5] = ["COD", "BAS", "DEB", "DIC", "VAR"];
const SEQUEL_SCRIPT_EXTENSIONS: [&str; 4] = ["COD", "DEB", "DIC", "VAR"];

type CompiledImage = (String, Vec<u8>);
type PreparedUnit = (Vec<CompiledImage>, bool);

pub(crate) struct VerifiedScriptArtifacts {
    pub(crate) descript: Option<Box<[u8]>>,
    pub(crate) resources: BTreeMap<BloodResourceName, Box<[u8]>>,
    pub(crate) rebuilt_unit_count: usize,
}

#[derive(Clone, Debug)]
struct ScriptSourcePaths {
    descript: Option<PathBuf>,
    profiles: PathBuf,
    layout: ScriptSourceLayout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ScriptSourceLayout {
    descript: Option<&'static str>,
    profiles: &'static str,
    profile_count: usize,
    profile_dialect: ProfileDialect,
    profile_extensions: &'static [&'static str],
}

#[derive(Debug)]
struct ArtifactSpec {
    filename: String,
    canonical_path: PathBuf,
}

impl ScriptSourcePaths {
    fn discover(game: GameVariant) -> Result<Self> {
        if let Some(explicit_root) = std::env::var_os(SCRIPT_SOURCE_ENVIRONMENT_VARIABLE) {
            let root = PathBuf::from(explicit_root);
            return Self::from_root(&root, game).with_context(|| {
                format!(
                    "resolving {SCRIPT_SOURCE_ENVIRONMENT_VARIABLE} source root {} for {}",
                    root.display(),
                    game.title()
                )
            });
        }

        let mut candidates = Vec::new();
        if let Ok(executable) = std::env::current_exe()
            && let Some(prefix) = executable.parent().and_then(Path::parent)
        {
            candidates.push(prefix.join("share/commander-blood/re"));
        }
        candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../re"));

        for candidate in &candidates {
            if let Ok(paths) = Self::from_root(candidate, game) {
                return Ok(paths);
            }
        }

        let searched = candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "editable {} script sources were not found; set {SCRIPT_SOURCE_ENVIRONMENT_VARIABLE} to the re source directory (searched {searched})",
            game.title()
        )
    }

    fn from_root(root: &Path, game: GameVariant) -> Result<Self> {
        let root = if root.join("re").is_dir() {
            root.join("re")
        } else {
            root.to_owned()
        };
        let layout = ScriptSourceLayout::for_game(game);
        let descript = layout.descript.map(|path| root.join(path));
        if let Some(descript) = &descript
            && !descript.is_file()
        {
            bail!("DESCRIPT source is missing: {}", descript.display());
        }
        let profiles = root.join(layout.profiles);
        if !profiles.is_dir() {
            bail!(
                "BloodScript source directory is missing: {}",
                profiles.display()
            );
        }
        for script in 1..=layout.profile_count {
            let source = profiles.join(format!("script{script}.blood"));
            if !source.is_file() {
                bail!("BloodScript source is missing: {}", source.display());
            }
        }
        Ok(Self {
            descript,
            profiles,
            layout,
        })
    }
}

impl ScriptSourceLayout {
    const fn for_game(game: GameVariant) -> Self {
        match game {
            GameVariant::CommanderBlood => Self {
                descript: Some(DESCRIPT_SOURCE_RELATIVE_PATH),
                profiles: COMMANDER_VM_SOURCE_DIRECTORY_RELATIVE_PATH,
                profile_count: 5,
                profile_dialect: ProfileDialect::CommanderBlood,
                profile_extensions: &COMMANDER_SCRIPT_EXTENSIONS,
            },
            GameVariant::BigBugBang => Self {
                descript: None,
                profiles: SEQUEL_VM_SOURCE_DIRECTORY_RELATIVE_PATH,
                profile_count: 17,
                profile_dialect: ProfileDialect::BigBugBang,
                profile_extensions: &SEQUEL_SCRIPT_EXTENSIONS,
            },
        }
    }
}

pub(crate) fn prepare_verified_script_artifacts(
    game: GameVariant,
    canonical_descript: &Path,
    canonical_resource_root: &Path,
    writable_root: &Path,
) -> Result<VerifiedScriptArtifacts> {
    let sources = ScriptSourcePaths::discover(game)?;
    prepare_verified_script_artifacts_from_sources(
        &sources,
        canonical_descript,
        canonical_resource_root,
        writable_root,
    )
}

fn prepare_verified_script_artifacts_from_sources(
    sources: &ScriptSourcePaths,
    canonical_descript: &Path,
    canonical_resource_root: &Path,
    writable_root: &Path,
) -> Result<VerifiedScriptArtifacts> {
    let cache_root = writable_root.join(COMPILED_SCRIPT_CACHE_DIRECTORY);
    let mut rebuilt_unit_count = 0;

    let mut descript = None;
    if let Some(source_path) = &sources.descript {
        let descript_specs = [ArtifactSpec {
            filename: DESCRIPT_COMPILED_FILENAME.to_owned(),
            canonical_path: canonical_descript.to_owned(),
        }];
        let (descript_images, descript_rebuilt) =
            prepare_unit(source_path, &descript_specs, &cache_root, || {
                let source = fs::read_to_string(source_path)
                    .with_context(|| format!("reading {}", source_path.display()))?;
                let image = compile_descript(&source)
                    .with_context(|| format!("compiling {}", source_path.display()))?;
                Ok(vec![(DESCRIPT_COMPILED_FILENAME.to_owned(), image)])
            })?;
        rebuilt_unit_count += usize::from(descript_rebuilt);
        descript = descript_images
            .into_iter()
            .next()
            .map(|(_, bytes)| bytes.into_boxed_slice());
    }

    let mut resources = BTreeMap::new();
    for script in 1..=sources.layout.profile_count {
        let source_path = sources.profiles.join(format!("script{script}.blood"));
        let script_name = format!("SCRIPT{script}");
        let specs = sources
            .layout
            .profile_extensions
            .iter()
            .map(|extension| {
                let filename = format!("{script_name}.{extension}");
                ArtifactSpec {
                    canonical_path: canonical_resource_root.join(&filename),
                    filename,
                }
            })
            .collect::<Vec<_>>();
        let (images, rebuilt) = prepare_unit(&source_path, &specs, &cache_root, || {
            let source = fs::read_to_string(&source_path)
                .with_context(|| format!("reading {}", source_path.display()))?;
            let profile = compile_profile(&source)
                .with_context(|| format!("compiling {}", source_path.display()))?;
            if !profile.name.eq_ignore_ascii_case(&script_name) {
                bail!(
                    "{} declares profile {:?}, expected {script_name}",
                    source_path.display(),
                    profile.name
                );
            }
            if profile.dialect != sources.layout.profile_dialect {
                bail!(
                    "{} declares {:?}, expected {:?}",
                    source_path.display(),
                    profile.dialect,
                    sources.layout.profile_dialect
                );
            }
            if profile.extensions() != sources.layout.profile_extensions {
                bail!(
                    "{} owns {:?}, expected {:?}",
                    source_path.display(),
                    profile.extensions(),
                    sources.layout.profile_extensions
                );
            }
            Ok(sources
                .layout
                .profile_extensions
                .iter()
                .map(|extension| {
                    (
                        format!("{script_name}.{extension}"),
                        profile
                            .image(extension)
                            .expect("known BloodScript image extension")
                            .to_vec(),
                    )
                })
                .collect())
        })?;
        rebuilt_unit_count += usize::from(rebuilt);
        for (filename, bytes) in images {
            let name = BloodResourceName::new(filename.as_bytes())
                .with_context(|| format!("validating rebuilt resource name {filename}"))?;
            resources.insert(name, bytes.into_boxed_slice());
        }
    }

    Ok(VerifiedScriptArtifacts {
        descript,
        resources,
        rebuilt_unit_count,
    })
}

fn prepare_unit<F>(
    source_path: &Path,
    artifacts: &[ArtifactSpec],
    cache_root: &Path,
    compile: F,
) -> Result<PreparedUnit>
where
    F: FnOnce() -> Result<Vec<CompiledImage>>,
{
    let canonical = artifacts
        .iter()
        .map(|artifact| {
            fs::read(&artifact.canonical_path)
                .with_context(|| format!("reading {}", artifact.canonical_path.display()))
                .map(|bytes| (artifact.filename.clone(), bytes))
        })
        .collect::<Result<Vec<_>>>()?;

    let cached = artifacts
        .iter()
        .map(|artifact| {
            let path = cache_root.join(&artifact.filename);
            if !path.is_file() {
                return Ok(None);
            }
            let bytes = fs::read(&path)
                .with_context(|| format!("reading compiled script cache {}", path.display()))?;
            let expected = canonical_image(&canonical, &artifact.filename);
            require_exact_checksum(source_path, &artifact.filename, &bytes, expected, "cached")?;
            Ok(Some((artifact.filename.clone(), bytes)))
        })
        .collect::<Result<Vec<_>>>()?;
    let complete_cache = cached.iter().all(Option::is_some);
    let comparison_paths = artifacts
        .iter()
        .map(|artifact| {
            let cached_path = cache_root.join(&artifact.filename);
            if complete_cache {
                cached_path
            } else {
                artifact.canonical_path.clone()
            }
        })
        .collect::<Vec<_>>();

    if !source_is_newer(source_path, &comparison_paths)? {
        let images = if complete_cache {
            cached.into_iter().flatten().collect()
        } else {
            Vec::new()
        };
        return Ok((images, false));
    }

    let compiled = compile()?;
    if compiled.len() != artifacts.len() {
        bail!(
            "compiler for {} produced {} artifacts, expected {}",
            source_path.display(),
            compiled.len(),
            artifacts.len()
        );
    }
    for (filename, bytes) in &compiled {
        let expected = canonical_image(&canonical, filename);
        require_exact_checksum(source_path, filename, bytes, expected, "compiled")?;
    }
    fs::create_dir_all(cache_root)
        .with_context(|| format!("creating compiled script cache {}", cache_root.display()))?;
    for (filename, bytes) in &compiled {
        write_atomically(&cache_root.join(filename), bytes)?;
    }
    Ok((compiled, true))
}

fn canonical_image<'a>(canonical: &'a [CompiledImage], filename: &str) -> &'a [u8] {
    canonical
        .iter()
        .find(|(candidate, _)| candidate == filename)
        .map(|(_, bytes)| bytes.as_slice())
        .unwrap_or_else(|| panic!("compiler produced unexpected artifact {filename}"))
}

fn source_is_newer(source_path: &Path, compiled_paths: &[PathBuf]) -> Result<bool> {
    let source_time = modified_time(source_path)?;
    for path in compiled_paths {
        if source_time > modified_time(path)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn modified_time(path: &Path) -> Result<SystemTime> {
    fs::metadata(path)
        .with_context(|| format!("reading timestamp for {}", path.display()))?
        .modified()
        .with_context(|| format!("reading modification time for {}", path.display()))
}

fn require_exact_checksum(
    source_path: &Path,
    filename: &str,
    actual: &[u8],
    expected: &[u8],
    origin: &str,
) -> Result<()> {
    if actual == expected {
        return Ok(());
    }
    let first_difference = actual
        .iter()
        .zip(expected)
        .position(|(left, right)| left != right)
        .unwrap_or(actual.len().min(expected.len()));
    bail!(
        "{origin} {filename} from {} failed byte-exact checksum verification at byte {first_difference}: expected SHA-256 {}, got SHA-256 {}; the game will not start with unverified script output",
        source_path.display(),
        sha256(expected),
        sha256(actual)
    )
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = path.with_extension(format!(
        "{}.{}.tmp",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("script"),
        std::process::id()
    ));
    fs::write(&temporary, bytes)
        .with_context(|| format!("writing temporary compiled script {}", temporary.display()))?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error).with_context(|| {
            format!(
                "installing verified compiled script {} into {}",
                temporary.display(),
                path.display()
            )
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::Duration;

    use super::*;

    static TEMPORARY_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    const TIMESTAMP_ADVANCE: Duration = Duration::from_millis(20);

    struct TemporaryDirectory(PathBuf);

    impl TemporaryDirectory {
        fn create() -> Self {
            let sequence = TEMPORARY_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "commander-blood-script-rebuild-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TemporaryDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn artifact(root: &Path, filename: &str, bytes: &[u8]) -> ArtifactSpec {
        let canonical_path = root.join(filename);
        fs::write(&canonical_path, bytes).unwrap();
        ArtifactSpec {
            filename: filename.to_owned(),
            canonical_path,
        }
    }

    #[test]
    fn game_specific_source_layouts_separate_profile_ownership() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let commander = ScriptSourcePaths::from_root(&root, GameVariant::CommanderBlood).unwrap();
        assert!(commander.descript.as_ref().unwrap().is_file());
        assert_eq!(commander.layout.profile_count, 5);
        assert_eq!(
            commander.layout.profile_extensions,
            COMMANDER_SCRIPT_EXTENSIONS
        );
        assert_eq!(
            commander.layout.profile_dialect,
            ProfileDialect::CommanderBlood
        );

        let sequel = ScriptSourcePaths::from_root(&root, GameVariant::BigBugBang).unwrap();
        assert!(sequel.descript.is_none());
        assert_eq!(sequel.layout.profile_count, 17);
        assert_eq!(sequel.layout.profile_extensions, SEQUEL_SCRIPT_EXTENSIONS);
        assert_eq!(sequel.layout.profile_dialect, ProfileDialect::BigBugBang);
        assert_ne!(commander.profiles, sequel.profiles);
    }

    #[test]
    #[ignore = "requires original Big Bug Bang imported resources"]
    fn sequel_rebuild_emits_exactly_its_68_active_profile_resources() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let canonical = workspace.join("output/big-bug-bang/imported-assets/resources");
        let source_root = workspace.join("re");
        let sources = ScriptSourcePaths::from_root(&source_root, GameVariant::BigBugBang).unwrap();
        let writable = TemporaryDirectory::create();

        let artifacts = prepare_verified_script_artifacts_from_sources(
            &sources,
            &canonical.join(DESCRIPT_COMPILED_FILENAME),
            &canonical,
            &writable.0,
        )
        .unwrap();
        assert!(artifacts.descript.is_none());
        assert_eq!(artifacts.resources.len(), 68);
        assert_eq!(artifacts.rebuilt_unit_count, 17);
        assert!(
            artifacts
                .resources
                .keys()
                .all(|name| !name.as_bytes().ends_with(b".BAS"))
        );

        let cache = writable.0.join(COMPILED_SCRIPT_CACHE_DIRECTORY);
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 68);
        assert!(!cache.join(DESCRIPT_COMPILED_FILENAME).exists());
        assert!(!cache.join("SCRIPT2.BAS").exists());

        let cached = prepare_verified_script_artifacts_from_sources(
            &sources,
            &canonical.join(DESCRIPT_COMPILED_FILENAME),
            &canonical,
            &writable.0,
        )
        .unwrap();
        assert!(cached.descript.is_none());
        assert_eq!(cached.resources, artifacts.resources);
        assert_eq!(cached.rebuilt_unit_count, 0);
    }

    #[test]
    fn rebuilds_newer_sources_once_and_preserves_verified_cache_on_mismatch() {
        let root = TemporaryDirectory::create();
        let canonical_root = root.0.join("canonical");
        let cache_root = root.0.join("cache");
        fs::create_dir_all(&canonical_root).unwrap();
        let specs = [
            artifact(&canonical_root, "SCRIPT1.COD", b"exact cod"),
            artifact(&canonical_root, "SCRIPT1.BAS", b"exact bas"),
        ];
        thread::sleep(TIMESTAMP_ADVANCE);
        let source = root.0.join("script1.blood");
        fs::write(&source, "first source").unwrap();

        let expected = vec![
            ("SCRIPT1.COD".to_owned(), b"exact cod".to_vec()),
            ("SCRIPT1.BAS".to_owned(), b"exact bas".to_vec()),
        ];
        let (rebuilt, did_rebuild) =
            prepare_unit(&source, &specs, &cache_root, || Ok(expected.clone())).unwrap();
        assert!(did_rebuild);
        assert_eq!(rebuilt, expected);

        let (cached, did_rebuild) = prepare_unit(&source, &specs, &cache_root, || {
            panic!("unchanged source must not invoke the compiler")
        })
        .unwrap();
        assert!(!did_rebuild);
        assert_eq!(cached, expected);

        thread::sleep(TIMESTAMP_ADVANCE);
        fs::write(&source, "edited source").unwrap();
        let error = prepare_unit(&source, &specs, &cache_root, || {
            Ok(vec![
                ("SCRIPT1.COD".to_owned(), b"wrong cod".to_vec()),
                ("SCRIPT1.BAS".to_owned(), b"exact bas".to_vec()),
            ])
        })
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("failed byte-exact checksum verification"));
        assert!(message.contains("expected SHA-256"));
        assert_eq!(
            fs::read(cache_root.join("SCRIPT1.COD")).unwrap(),
            b"exact cod"
        );
        assert_eq!(
            fs::read(cache_root.join("SCRIPT1.BAS")).unwrap(),
            b"exact bas"
        );
    }

    #[test]
    fn rejects_a_corrupt_compiled_cache_before_loading_it() {
        let root = TemporaryDirectory::create();
        let canonical_root = root.0.join("canonical");
        let cache_root = root.0.join("cache");
        fs::create_dir_all(&canonical_root).unwrap();
        fs::create_dir_all(&cache_root).unwrap();
        let specs = [artifact(
            &canonical_root,
            DESCRIPT_COMPILED_FILENAME,
            b"verified descript",
        )];
        let source = root.0.join("DESCRIPT.descript");
        fs::write(&source, "older source").unwrap();
        thread::sleep(TIMESTAMP_ADVANCE);
        fs::write(
            cache_root.join(DESCRIPT_COMPILED_FILENAME),
            b"corrupt descript",
        )
        .unwrap();

        let error = prepare_unit(&source, &specs, &cache_root, || {
            panic!("a corrupt cache must fail before compilation")
        })
        .unwrap_err();
        assert!(format!("{error:#}").contains("cached DESCRIPT.DES"));
    }
}
