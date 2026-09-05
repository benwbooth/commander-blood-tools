//! Game identity shared by asset imports, script dialects and persistent storage.

use anyhow::{Context, Result, bail};
use commander_blood_formats::code::ScriptDialect;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::native::bloodprg::{OriginalResourceCatalog, OriginalScriptProfileCatalog};

const SUPPORTED_SEQUEL_EXECUTABLE_SHA256: &str =
    "4b65ffca3e113a1826371e3436177861640a1b7aae24caafebb4c2f7aa467834";

const COMMANDER_COMPANIONS: [&str; 5] = [
    "BLOODPRG.EXE",
    "BLOOD.LBM",
    "TB.BIG",
    "DESCRIPT.DES",
    "BLOOD.SAV",
];
// The original sequel disc supplies no BLOOD.SAV. Saves are runtime-owned data,
// not a missing asset that the importer may borrow from Commander Blood.
const SEQUEL_COMPANIONS: [&str; 4] = ["BLOOD2PG.EXE", "BLOOD2.LBM", "TB.BIG", "DESCRIPT.DES"];

/// Authored game identity, independent of language and executable build revision.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GameVariant {
    /// The original Commander Blood game. Also identifies legacy import manifests.
    #[default]
    CommanderBlood,
    /// Big Bug Bang, with its extended simulation VM and distinct native catalogs.
    BigBugBang,
}

impl GameVariant {
    /// Human-readable game name.
    pub const fn title(self) -> &'static str {
        match self {
            Self::CommanderBlood => "Commander Blood",
            Self::BigBugBang => "Big Bug Bang",
        }
    }

    /// Stable namespace for caches, saves and other game-specific host data.
    pub const fn storage_name(self) -> &'static str {
        match self {
            Self::CommanderBlood => "commander-blood",
            Self::BigBugBang => "big-bug-bang",
        }
    }

    /// Original executable containing this game's native resource tables.
    pub const fn executable_filename(self) -> &'static str {
        match self {
            Self::CommanderBlood => "BLOODPRG.EXE",
            Self::BigBugBang => "BLOOD2PG.EXE",
        }
    }

    /// Loose title artwork supplied with this game.
    pub const fn title_filename(self) -> &'static str {
        match self {
            Self::CommanderBlood => "BLOOD.LBM",
            Self::BigBugBang => "BLOOD2.LBM",
        }
    }

    /// Instruction framing and state-record layout used by this game.
    pub const fn script_dialect(self) -> ScriptDialect {
        match self {
            Self::CommanderBlood => ScriptDialect::CommanderBlood,
            Self::BigBugBang => ScriptDialect::BigBugBang,
        }
    }

    /// Decode this game's executable-resident resource identities.
    pub fn decode_resource_catalog(self, executable: &[u8]) -> Result<OriginalResourceCatalog> {
        self.validate_catalog_build(executable)?;
        match self {
            Self::CommanderBlood => OriginalResourceCatalog::decode_bloodprg(executable),
            Self::BigBugBang => OriginalResourceCatalog::decode_blood2pg(executable),
        }
        .context("decoding game resource catalog")
    }

    /// Decode this game's executable-resident profile table and VM dialect.
    pub fn decode_profile_catalog(self, executable: &[u8]) -> Result<OriginalScriptProfileCatalog> {
        self.validate_catalog_build(executable)?;
        match self {
            Self::CommanderBlood => OriginalScriptProfileCatalog::decode_bloodprg(executable),
            Self::BigBugBang => OriginalScriptProfileCatalog::decode_blood2pg(executable),
        }
        .context("decoding game script-profile catalog")
    }

    fn validate_catalog_build(self, executable: &[u8]) -> Result<()> {
        // Preserve Commander's existing decoder contract. Only one sequel build
        // has been analyzed; a filename alone cannot authorize its fixed offsets.
        if self == Self::BigBugBang {
            let actual = format!("{:x}", Sha256::digest(executable));
            if actual != SUPPORTED_SEQUEL_EXECUTABLE_SHA256 {
                bail!(
                    "unrecognized Big Bug Bang executable SHA-256 {actual}; native catalog offsets are not verified for this build"
                );
            }
        }
        Ok(())
    }

    pub(crate) const fn required_companions(self) -> &'static [&'static str] {
        match self {
            Self::CommanderBlood => &COMMANDER_COMPANIONS,
            Self::BigBugBang => &SEQUEL_COMPANIONS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn games_have_distinct_native_inputs_and_storage() {
        let commander = GameVariant::CommanderBlood;
        let sequel = GameVariant::BigBugBang;
        assert_ne!(commander.storage_name(), sequel.storage_name());
        assert_ne!(
            commander.executable_filename(),
            sequel.executable_filename()
        );
        assert_ne!(commander.title_filename(), sequel.title_filename());
        assert_eq!(sequel.script_dialect(), ScriptDialect::BigBugBang);
        assert!(!sequel.required_companions().contains(&"BLOOD.SAV"));
        for game in [commander, sequel] {
            assert!(
                game.required_companions()
                    .contains(&game.executable_filename())
            );
            assert!(game.required_companions().contains(&game.title_filename()));
        }
    }

    #[test]
    fn unknown_sequel_build_cannot_use_verified_catalog_offsets() {
        let bytes = vec![0; 98190];
        assert!(
            GameVariant::BigBugBang
                .decode_resource_catalog(&bytes)
                .is_err()
        );
        assert!(
            GameVariant::BigBugBang
                .decode_profile_catalog(&bytes)
                .is_err()
        );
    }
}
