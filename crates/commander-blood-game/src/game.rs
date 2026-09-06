//! Game identity shared by asset imports, script dialects and persistent storage.

use anyhow::{Context, Result, bail};
use commander_blood_formats::bloodprg::{
    BloodprgBridgeResources, BloodprgFontResources, BloodprgPresentationCatalog,
    decode_big_bug_bang_inventory_cancel_label, decode_blood2pg_bridge_resources,
    decode_blood2pg_font_resources, decode_blood2pg_presentation_catalog,
    decode_bloodprg_bridge_resources, decode_bloodprg_font_resources,
    decode_bloodprg_presentation_catalog,
};
use commander_blood_formats::code::ScriptDialect;
use commander_blood_formats::name_area_effect::{
    NameAreaEffectSequence, decode_blood2pg_name_area_effect_sequences,
    decode_bloodprg_name_area_effect_sequences,
};
use commander_blood_formats::palette::{
    decode_blood2pg_default_vga_palette, decode_bloodprg_default_vga_palette,
};
use commander_blood_formats::world_art::{
    WorldArtworkLayout, decode_blood2pg_world_artwork_layout, decode_bloodprg_world_artwork_layout,
};
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
        self.validate_native_build(executable)?;
        match self {
            Self::CommanderBlood => OriginalResourceCatalog::decode_bloodprg(executable),
            Self::BigBugBang => OriginalResourceCatalog::decode_blood2pg(executable),
        }
        .context("decoding game resource catalog")
    }

    /// Decode this game's executable-resident profile table and VM dialect.
    pub fn decode_profile_catalog(self, executable: &[u8]) -> Result<OriginalScriptProfileCatalog> {
        self.validate_native_build(executable)?;
        match self {
            Self::CommanderBlood => OriginalScriptProfileCatalog::decode_bloodprg(executable),
            Self::BigBugBang => OriginalScriptProfileCatalog::decode_blood2pg(executable),
        }
        .context("decoding game script-profile catalog")
    }

    /// Decode the game's complete font maps, advances, and glyph bitmaps.
    pub fn decode_fonts(self, executable: &[u8]) -> Result<BloodprgFontResources> {
        self.validate_native_build(executable)?;
        match self {
            Self::CommanderBlood => decode_bloodprg_font_resources(executable),
            Self::BigBugBang => decode_blood2pg_font_resources(executable),
        }
        .context("decoding game font resources")
    }

    /// Decode the game's initial scene filenames and stream flags.
    pub fn decode_presentation_catalog(
        self,
        executable: &[u8],
    ) -> Result<BloodprgPresentationCatalog> {
        self.validate_native_build(executable)?;
        match self {
            Self::CommanderBlood => decode_bloodprg_presentation_catalog(executable),
            Self::BigBugBang => decode_blood2pg_presentation_catalog(executable),
        }
        .context("decoding game presentation catalog")
    }

    /// Decode the game's unexpanded six-bit default palette.
    pub fn decode_default_vga_palette(self, executable: &[u8]) -> Result<[[u8; 3]; 256]> {
        self.validate_native_build(executable)?;
        match self {
            Self::CommanderBlood => decode_bloodprg_default_vga_palette(executable),
            Self::BigBugBang => decode_blood2pg_default_vga_palette(executable),
        }
        .context("decoding game default palette")
    }

    /// Decode the game's authored name-area effect frames.
    pub fn decode_name_area_effect_sequences(
        self,
        executable: &[u8],
    ) -> Result<Box<[NameAreaEffectSequence]>> {
        self.validate_native_build(executable)?;
        match self {
            Self::CommanderBlood => decode_bloodprg_name_area_effect_sequences(executable),
            Self::BigBugBang => decode_blood2pg_name_area_effect_sequences(executable),
        }
        .context("decoding game name-area effects")
    }

    /// Decode the game's world-artwork identities and initial activation flags.
    pub fn decode_world_artwork_layout(
        self,
        executable: &[u8],
    ) -> Result<Box<[WorldArtworkLayout]>> {
        self.validate_native_build(executable)?;
        match self {
            Self::CommanderBlood => decode_bloodprg_world_artwork_layout(executable),
            Self::BigBugBang => decode_blood2pg_world_artwork_layout(executable),
        }
        .context("decoding game world-artwork layout")
    }

    /// Resolve the sequel's object-chooser cancel text from its verified build.
    pub fn decode_inventory_cancel_label(self, executable: &[u8]) -> Result<Box<[u8]>> {
        if self != Self::BigBugBang {
            bail!("object-backed inventory choices require Big Bug Bang resources");
        }
        self.validate_native_build(executable)?;
        decode_big_bug_bang_inventory_cancel_label(executable)
            .context("decoding inventory cancellation text")
    }

    /// Decode typed bridge projection tables and initial navigation actor records.
    pub fn decode_bridge_resources(self, executable: &[u8]) -> Result<BloodprgBridgeResources> {
        self.validate_native_build(executable)?;
        match self {
            Self::CommanderBlood => decode_bloodprg_bridge_resources(executable),
            Self::BigBugBang => decode_blood2pg_bridge_resources(executable),
        }
        .context("decoding game bridge resources")
    }

    fn validate_native_build(self, executable: &[u8]) -> Result<()> {
        // Preserve Commander's existing decoder contract. Only one sequel build
        // has been analyzed; a filename alone cannot authorize its fixed offsets.
        if self == Self::BigBugBang {
            let actual = format!("{:x}", Sha256::digest(executable));
            if actual != SUPPORTED_SEQUEL_EXECUTABLE_SHA256 {
                bail!(
                    "unrecognized Big Bug Bang executable SHA-256 {actual}; native table offsets are not verified for this build"
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
        assert!(GameVariant::BigBugBang.decode_fonts(&bytes).is_err());
        assert!(
            GameVariant::BigBugBang
                .decode_default_vga_palette(&bytes)
                .is_err()
        );
        assert!(
            GameVariant::BigBugBang
                .decode_name_area_effect_sequences(&bytes)
                .is_err()
        );
        assert!(
            GameVariant::BigBugBang
                .decode_world_artwork_layout(&bytes)
                .is_err()
        );
        assert!(
            GameVariant::BigBugBang
                .decode_presentation_catalog(&bytes)
                .is_err()
        );
        assert!(
            GameVariant::BigBugBang
                .decode_bridge_resources(&bytes)
                .is_err()
        );
    }
}
