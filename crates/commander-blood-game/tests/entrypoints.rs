use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

use commander_blood_game::game::GameVariant;

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "blood-entrypoints-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn command(&self, game: GameVariant) -> Command {
        let binary = match game {
            GameVariant::CommanderBlood => env!("CARGO_BIN_EXE_commander-blood"),
            GameVariant::BigBugBang => env!("CARGO_BIN_EXE_big-bug-bang"),
        };
        let mut command = Command::new(binary);
        command
            .current_dir(&self.0)
            .env_remove("CBLOOD_DATA")
            .env_remove("BBB_DATA")
            .env_remove("CBLOOD_ASSET_CACHE")
            .env_remove("CBLOOD_WRITE_DATA")
            .env("XDG_DATA_HOME", &self.0)
            .env("HOME", &self.0);
        command
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

const GAMES: [GameVariant; 2] = [GameVariant::CommanderBlood, GameVariant::BigBugBang];

#[test]
fn help_identifies_the_selected_game_and_data_override() {
    let temporary = TemporaryDirectory::new();
    for game in GAMES {
        let output = temporary.command(game).arg("--help").output().unwrap();
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.starts_with(&format!("Usage: {} ", game.storage_name())));
        assert!(help.contains(&format!("{} assets", game.title())));
        assert!(help.contains(&format!("{} may point", game.data_environment_variable())));
    }
}

#[test]
fn wrong_game_is_rejected_before_launch_or_import_writes() {
    for game in GAMES {
        let temporary = TemporaryDirectory::new();
        let other = GAMES.into_iter().find(|other| *other != game).unwrap();
        // Identity must be checked before parsing or copying the executable.
        fs::write(temporary.0.join(other.executable_filename()), []).unwrap();
        let destination = temporary.0.join("imported");
        for import_only in [false, true] {
            let mut command = temporary.command(game);
            command.arg("--data").arg(&temporary.0);
            if import_only {
                command.arg("--import-assets").arg(&destination);
            }
            let output = command.output().unwrap();
            assert!(!output.status.success());
            let error = String::from_utf8(output.stderr).unwrap();
            assert!(
                error.contains(&format!("contains {} data", other.title())),
                "{error}"
            );
            assert!(
                error.contains(&format!("use {} instead", other.storage_name())),
                "{error}"
            );
            assert!(!destination.exists());
        }
    }
}

#[test]
fn data_environment_overrides_are_game_specific() {
    for game in GAMES {
        let temporary = TemporaryDirectory::new();
        let other = GAMES.into_iter().find(|other| *other != game).unwrap();
        let missing = temporary.0.join("missing");
        let output = temporary
            .command(game)
            .env(other.data_environment_variable(), &missing)
            .output()
            .unwrap();
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(!output.status.success());
        assert!(
            error.contains(&format!("complete {} data set not found", game.title())),
            "{error}"
        );

        let output = temporary
            .command(game)
            .env(game.data_environment_variable(), &missing)
            .output()
            .unwrap();
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(!output.status.success());
        assert!(
            error.contains("game import source is not a directory"),
            "{error}"
        );
        assert!(error.contains(missing.to_str().unwrap()), "{error}");
    }
}

#[test]
fn each_launcher_reads_only_its_own_default_cache() {
    for game in GAMES {
        let temporary = TemporaryDirectory::new();
        let other = GAMES.into_iter().find(|other| *other != game).unwrap();
        let other_cache = temporary.0.join(other.storage_name()).join("assets-v1");
        fs::create_dir_all(&other_cache).unwrap();
        fs::write(other_cache.join("manifest.json"), "invalid manifest").unwrap();
        let output = temporary.command(game).output().unwrap();
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(!output.status.success());
        assert!(
            error.contains(&format!("complete {} data set not found", game.title())),
            "{error}"
        );

        let own_cache = temporary.0.join(game.storage_name()).join("assets-v1");
        fs::create_dir_all(&own_cache).unwrap();
        fs::write(own_cache.join("manifest.json"), "invalid manifest").unwrap();
        let output = temporary.command(game).output().unwrap();
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(!output.status.success());
        assert!(error.contains(own_cache.to_str().unwrap()), "{error}");
    }
}
