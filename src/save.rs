//! Port-native save/load of the resumable game state.
//!
//! This is the PORT's own save format — a small, human-readable key/value file — NOT the
//! original DOS `blood.sav` byte layout, which is a separate reverse-engineering target
//! (the save routine writes it, but its field layout is still undecoded; see
//! `re/REVERSE.md`). It persists exactly what the port needs to resume play: the active
//! screen, the nav heading, the current location/dialogue script, dialogue progress, the
//! video-phone selection, and the text-speed setting.
//!
//! [`crate::engine::EngineState::capture_save`] builds a [`SaveState`] from the live
//! engine and [`crate::engine::EngineState::restore_save`] applies one back.

use std::io::{self, Write};
use std::path::Path;

/// The screen the player was on when the game was saved, so loading resumes the same view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveScreen {
    /// The navigation star-map.
    Nav,
    /// The ship-bridge console hub.
    Bridge,
    /// The comms / "Hate TV" screen.
    Comms,
    /// The cyberspace hyperspace-tunnel screen.
    Cyberspace,
    /// The cryo-chamber (console CRYOBOX option).
    Cryobox,
    /// The video-phone call screen (console TELEPHONE option).
    Telephone,
    /// A dialogue/cutscene scene at a location.
    Dialogue,
}

impl SaveScreen {
    /// The stable tag written to the save file for this screen.
    const fn tag(self) -> &'static str {
        match self {
            SaveScreen::Nav => "nav",
            SaveScreen::Bridge => "bridge",
            SaveScreen::Comms => "comms",
            SaveScreen::Cyberspace => "cyberspace",
            SaveScreen::Cryobox => "cryobox",
            SaveScreen::Telephone => "telephone",
            SaveScreen::Dialogue => "dialogue",
        }
    }

    /// Parse a screen tag back from the save file.
    fn from_tag(tag: &str) -> Option<Self> {
        Some(match tag {
            "nav" => SaveScreen::Nav,
            "bridge" => SaveScreen::Bridge,
            "comms" => SaveScreen::Comms,
            "cyberspace" => SaveScreen::Cyberspace,
            "cryobox" => SaveScreen::Cryobox,
            "telephone" => SaveScreen::Telephone,
            "dialogue" => SaveScreen::Dialogue,
            _ => return None,
        })
    }
}

/// The resumable game state persisted by the port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveState {
    /// The screen the player was on.
    pub screen: SaveScreen,
    /// The current location/dialogue script number (0 = none, on the nav).
    pub script: u32,
    /// The nav compass heading (0..179).
    pub compass_angle: u16,
    /// The player's progress through the current dialogue (line index).
    pub dialogue_cursor: usize,
    /// The video-phone's selected contact index.
    pub phone_contact: usize,
    /// Whether a video-phone call was connected.
    pub phone_connected: bool,
    /// The decoded text-speed reveal step (subtitle characters per tick).
    pub text_speed_step: u16,
}

/// The save file's first line — a magic + format version, so a foreign or future file is
/// rejected rather than misread.
const SAVE_MAGIC: &str = "COMMANDER-BLOOD-SAVE 1";

impl SaveState {
    /// Serialize to the port's line-based save text.
    pub fn to_text(&self) -> String {
        format!(
            "{SAVE_MAGIC}\n\
             screen={}\n\
             script={}\n\
             compass_angle={}\n\
             dialogue_cursor={}\n\
             phone_contact={}\n\
             phone_connected={}\n\
             text_speed_step={}\n",
            self.screen.tag(),
            self.script,
            self.compass_angle,
            self.dialogue_cursor,
            self.phone_contact,
            self.phone_connected as u8,
            self.text_speed_step,
        )
    }

    /// Parse the port's save text. Returns `None` if the magic is missing or a field is
    /// malformed — a corrupt or foreign file simply doesn't load (the game starts fresh).
    pub fn from_text(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        if lines.next()?.trim() != SAVE_MAGIC {
            return None;
        }
        let mut screen = None;
        let mut script = None;
        let mut compass_angle = None;
        let mut dialogue_cursor = None;
        let mut phone_contact = None;
        let mut phone_connected = None;
        let mut text_speed_step = None;
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let (key, value) = line.split_once('=')?;
            match key.trim() {
                "screen" => screen = SaveScreen::from_tag(value.trim()),
                "script" => script = value.trim().parse().ok(),
                "compass_angle" => compass_angle = value.trim().parse().ok(),
                "dialogue_cursor" => dialogue_cursor = value.trim().parse().ok(),
                "phone_contact" => phone_contact = value.trim().parse().ok(),
                "phone_connected" => phone_connected = value.trim().parse::<u8>().ok().map(|v| v != 0),
                "text_speed_step" => text_speed_step = value.trim().parse().ok(),
                _ => {} // ignore unknown keys for forward compatibility
            }
        }
        Some(SaveState {
            screen: screen?,
            script: script?,
            compass_angle: compass_angle?,
            dialogue_cursor: dialogue_cursor?,
            phone_contact: phone_contact?,
            phone_connected: phone_connected?,
            text_speed_step: text_speed_step?,
        })
    }

    /// Write the save to `path` (the port's `blood.sav`).
    pub fn write(&self, path: &Path) -> io::Result<()> {
        let mut file = std::fs::File::create(path)?;
        file.write_all(self.to_text().as_bytes())
    }

    /// Read a save from `path`, or `None` if it is absent/corrupt/foreign.
    pub fn read(path: &Path) -> Option<Self> {
        Self::from_text(&std::fs::read_to_string(path).ok()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_state_round_trips_through_text() {
        let s = SaveState {
            screen: SaveScreen::Telephone,
            script: 3,
            compass_angle: 137,
            dialogue_cursor: 42,
            phone_contact: 2,
            phone_connected: true,
            text_speed_step: 5,
        };
        let parsed = SaveState::from_text(&s.to_text()).expect("parses");
        assert_eq!(parsed, s);
    }

    #[test]
    fn every_screen_tag_round_trips() {
        for screen in [
            SaveScreen::Nav,
            SaveScreen::Bridge,
            SaveScreen::Comms,
            SaveScreen::Cyberspace,
            SaveScreen::Cryobox,
            SaveScreen::Telephone,
            SaveScreen::Dialogue,
        ] {
            assert_eq!(SaveScreen::from_tag(screen.tag()), Some(screen));
        }
    }

    #[test]
    fn rejects_foreign_or_corrupt_saves() {
        assert!(SaveState::from_text("not a save file").is_none());
        assert!(SaveState::from_text("").is_none());
        // Right magic but a missing required field.
        assert!(SaveState::from_text(&format!("{SAVE_MAGIC}\nscreen=nav\n")).is_none());
        // Right magic but an unknown screen tag.
        assert!(SaveState::from_text(&format!("{SAVE_MAGIC}\nscreen=warp\n")).is_none());
    }

    #[test]
    fn write_then_read_a_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("cb_save_test_blood.sav");
        let s = SaveState {
            screen: SaveScreen::Nav,
            script: 0,
            compass_angle: 90,
            dialogue_cursor: 0,
            phone_contact: 0,
            phone_connected: false,
            text_speed_step: 3,
        };
        s.write(&path).expect("writes");
        assert_eq!(SaveState::read(&path), Some(s));
        let _ = std::fs::remove_file(&path);
    }
}
