//! The DOS save-file formats — the byte-exact layouts the original
//! `BLOODPRG.EXE` reads and writes. LIVE-VERIFIED round trip (save_option scenario):
//! the real game, driven through OPTION->SAVE with a typed name, wrote BOTH files and
//! this module parses them:
//! - `game<N>.sav` (slots 1..10): the state file described below (observed: 5887
//!   bytes, profile=1 at the post-tutorial hub, 5277-byte runtime region).
//! - `blood.sav`: the SLOT-NAME DIRECTORY — exactly ten 32-byte records
//!   {15-char name, NUL, "game<N>.sav" + padding} (the typed name landed in slot 1).
//!
//! Decoded from the binary's save/load routines (`vm_state_save` @0x1C3F,
//! `vm_state_load` @0x1CBD; see `re/REVERSE.md`). Both serialize the live VM
//! state in the same field order:
//!
//! | field        | size            | source (game global) | meaning |
//! |--------------|-----------------|----------------------|---------|
//! | `profile`    | 2 bytes (u16 LE)| `[0x677E]` (see below)| current script profile index (which SCRIPT set was active) |
//! | `flags`      | 512 bytes       | `[0x6ADE]`           | the global flag/progression block (persistent world state) |
//! | `state`      | 96 bytes        | `[0x6CDE]`           | a secondary state block |
//! | `object_block` | variable      | far `[0x6724]`       | the runtime VM object/state table |
//! | `work_buffer`  | variable      | far `[0xABC]`        | the object work buffer |
//!
//! SAVE AND LOAD USE DIFFERENT GLOBALS FOR THE PROFILE, deliberately. The writer
//! at `0x1C63` does `mov cx,2 / mov dx,0x677E` — `vm_resource_profile_index`, the
//! profile CURRENTLY selected. The reader at `0x1CEB` does `mov cx,2 / mov
//! dx,0x6780` — `vm_pending_resource_profile`, the slot the `0xD2` opcode posts a
//! REQUEST into. So loading a save does not switch script sets itself: it posts
//! the saved profile as pending, and the main loop's normal dispatch
//! (`0x108E` -> `0x10C5`) performs the switch on the next pass. (This module's
//! table previously named `[0x6780]` for both, which reads as a symmetric field
//! and misses the mechanism.)
//!
//! On load the game reads the profile first, reloads that script set, then reads
//! the four state blocks and rebuilds its derived pointers. The two variable
//! blocks are sized by the writer (from the resource id `[0x6716]` for the
//! object block, and `vm_context_pointer_setup` @0x1D94 for the work buffer), so
//! this reader takes the remaining bytes: the object block is everything up to
//! the last chunk, and — because the game only stores the two lengths implicitly
//! (they follow from the loaded profile's resource sizes) — a faithful *round
//! trip* needs the live game to supply the split. This module therefore exposes
//! the fixed header exactly, and the trailing bytes as one opaque `runtime`
//! region that a VM-aware caller (which knows the object-block length for the
//! loaded profile) can split precisely.

/// The fixed-layout portion of a `blood.sav` file, plus the trailing runtime
/// region (object block + work buffer, whose split depends on the loaded
/// profile's live object-table size).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BloodSave {
    /// Current script profile index (`[0x6780]`); `0xFFFF` = none.
    pub profile: u16,
    /// The 512-byte global flag/progression block (`[0x6ADE]`).
    pub flags: Vec<u8>,
    /// The 96-byte secondary state block (`[0x6CDE]`).
    pub state: Vec<u8>,
    /// The trailing runtime region: the VM object block (`[0x6724]`) followed by
    /// the work buffer (`[0xABC]`). Their boundary is the loaded profile's live
    /// object-table length; kept opaque here (see module docs).
    pub runtime: Vec<u8>,
}

/// One row of the SLOT-NAME DIRECTORY (`blood.sav`): a 32-byte record holding a
/// 15-character name padded with spaces, a NUL, then the slot's filename
/// (`game<N>.sav`) padded with NULs. Ten of them, one per save slot.
///
/// The game shows these through the ORDINARY LIST WIDGET: the save flow sets
/// `[0x2734]` to the record being renamed (`0x1BAB`, value `0x25ED`) and copies
/// 16 bytes of it into the edit buffer at `DS:0x273B` (`rep movsd cx=4`
/// @`0x1BBD`), and the widget then substitutes that buffer for the matching row
/// while drawing (`cmp si,[0x2734] / jne / mov si,0x273B` @`0x8573`). There is
/// no separate save screen: it is the ten slot names in the list, one of which
/// is being typed into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveSlot {
    /// The displayed name, trailing pad stripped. Empty for an unused slot.
    pub name: String,
    /// `game<N>.sav`, the file the slot's state lives in.
    pub file: String,
}

/// Bytes per directory record, and the record count.
pub const SLOT_RECORD_LEN: usize = 32;
pub const SLOT_NAME_LEN: usize = 16;
pub const SLOT_COUNT: usize = 10;

/// Parse `blood.sav`, the slot-name directory the save flow renames through
/// (`0x1BAB` points `[0x2734]` at a record, `0x1BBD` copies it to `DS:0x273B`).
/// Returns `None` unless the image is exactly the ten records the format defines.
pub fn parse_slot_directory(data: &[u8]) -> Option<Vec<SaveSlot>> {
    if data.len() != SLOT_COUNT * SLOT_RECORD_LEN {
        return None;
    }
    Some(
        data.chunks_exact(SLOT_RECORD_LEN)
            .map(|rec| {
                let field = |r: &[u8]| {
                    String::from_utf8_lossy(r.split(|&b| b == 0).next().unwrap_or_default())
                        .trim_end()
                        .to_string()
                };
                SaveSlot {
                    name: field(&rec[..SLOT_NAME_LEN]),
                    file: field(&rec[SLOT_NAME_LEN..]),
                }
            })
            .collect(),
    )
}

/// The DS globals the writer streams from (`0x1C63`, `0x1C6D`, `0x1C72`) and,
/// for the profile, the DIFFERENT one the reader streams into (`0x1CEB`).
pub const SAVE_PROFILE_SOURCE_DS: u16 = 0x677E;
pub const LOAD_PROFILE_DEST_DS: u16 = 0x6780;
pub const FLAGS_SOURCE_DS: u16 = 0x6ADE;
pub const STATE_SOURCE_DS: u16 = 0x6CDE;
/// File offsets of the three `int 21h` AH=0x40 write calls' `mov cx,imm` in
/// `vm_state_save`, so the sizes below can be checked against the code itself.
/// Note the third pair is emitted `mov dx` FIRST then `mov cx` (`0x1C72`/`0x1C75`),
/// where the first two are `mov cx` then `mov dx` — so the size immediate sits at
/// `0x1C76`, not where the earlier spacing would suggest.
pub const SAVE_WRITE_SIZE_IMMEDIATES: [(usize, usize); 3] = [
    (0x1C61, PROFILE_SIZE),
    (0x1C6B, FLAGS_SIZE),
    (0x1C76, STATE_SIZE),
];

/// Byte offset/size constants of the fixed header (all little-endian).
pub const PROFILE_SIZE: usize = 2;
pub const FLAGS_SIZE: usize = 0x200; // 512
pub const STATE_SIZE: usize = 0x60; // 96
pub const HEADER_SIZE: usize = PROFILE_SIZE + FLAGS_SIZE + STATE_SIZE;

impl BloodSave {
    /// Parse a save image in the field order `vm_state_save` (`0x1C3F`) writes:
    /// the 2-byte profile (`0x1C60`), 512 flag bytes (`0x1C6A`), 96 state bytes
    /// (`0x1C75`), then the two variable blocks as one opaque region.
    /// Returns `None` if it is shorter than the fixed
    /// header (profile + 512 flags + 96 state).
    pub fn parse(data: &[u8]) -> Option<BloodSave> {
        if data.len() < HEADER_SIZE {
            return None;
        }
        let profile = u16::from_le_bytes([data[0], data[1]]);
        let flags = data[PROFILE_SIZE..PROFILE_SIZE + FLAGS_SIZE].to_vec();
        let state =
            data[PROFILE_SIZE + FLAGS_SIZE..HEADER_SIZE].to_vec();
        let runtime = data[HEADER_SIZE..].to_vec();
        Some(BloodSave {
            profile,
            flags,
            state,
            runtime,
        })
    }

    /// Serialize back to the DOS byte layout (profile, flags, state, runtime).
    /// Byte-exact with [`BloodSave::parse`]'s input for the header; the runtime
    /// region round-trips verbatim.
    /// Re-serialise in the writer's order (`vm_state_save` `0x1C3F`: profile,
    /// flags, state, then the variable region), so a parsed real save round-trips
    /// byte-for-byte.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_SIZE + self.runtime.len());
        out.extend_from_slice(&self.profile.to_le_bytes());
        // Pad/truncate the two fixed blocks defensively so a hand-built save is
        // always the right size.
        let mut flags = self.flags.clone();
        flags.resize(FLAGS_SIZE, 0);
        out.extend_from_slice(&flags);
        let mut state = self.state.clone();
        state.resize(STATE_SIZE, 0);
        out.extend_from_slice(&state);
        out.extend_from_slice(&self.runtime);
        out
    }

    /// Whether a progression flag bit is set in the 512-byte flag block.
    /// `byte` indexes into `flags` (0..512), `bit` is 0..8. The block mirrors the
    /// game's `[0x6ADE]` region; the entity-progression bits the port tracks in
    /// [`crate::progress`] correspond to bits here (exact mapping is per-entity).
    pub fn flag_bit(&self, byte: usize, bit: u8) -> bool {
        self.flags
            .get(byte)
            .is_some_and(|b| b & (1 << bit) != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_synthetic_save() {
        let mut data = Vec::new();
        data.extend_from_slice(&0x0002u16.to_le_bytes()); // profile 2 (SCRIPT3)
        data.extend((0..FLAGS_SIZE).map(|i| (i & 0xFF) as u8));
        data.extend((0..STATE_SIZE).map(|i| (i * 3 & 0xFF) as u8));
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]); // opaque runtime
        let save = BloodSave::parse(&data).unwrap();
        assert_eq!(save.profile, 2);
        assert_eq!(save.flags.len(), FLAGS_SIZE);
        assert_eq!(save.state.len(), STATE_SIZE);
        assert_eq!(save.runtime, vec![0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(save.to_bytes(), data, "byte-exact round trip");
    }

    #[test]
    fn rejects_truncated_files() {
        assert!(BloodSave::parse(&[0u8; 10]).is_none());
        assert!(BloodSave::parse(&[0u8; HEADER_SIZE - 1]).is_none());
        assert!(BloodSave::parse(&[0u8; HEADER_SIZE]).is_some());
    }

    #[test]
    fn reads_flag_bits() {
        let mut data = vec![0u8; HEADER_SIZE];
        data[0] = 0xFF;
        data[1] = 0xFF; // profile = 0xFFFF (none)
        data[PROFILE_SIZE + 5] = 0b0010_0000; // flag byte 5, bit 5
        let save = BloodSave::parse(&data).unwrap();
        assert_eq!(save.profile, 0xFFFF);
        assert!(save.flag_bit(5, 5));
        assert!(!save.flag_bit(5, 4));
        assert!(!save.flag_bit(6, 0));
    }

    /// If the real game has been driven to save, parse it and sanity-check the fixed
    /// header. LIVE-OBSERVED (save_option scenario, OPTION->LOAD file-open trace):
    /// the real slot filenames are `game<N>.sav` (game1.sav for slot 1) — NOT
    /// blood.sav (that name is only opened at BOOT as a legacy/quick slot probe).
    #[test]
    fn parses_the_real_slot_directory() {
        let paths = ["accuracy/cdrive/cblood/blood.sav", "../accuracy/cdrive/cblood/blood.sav"];
        let Some(data) = paths.iter().find_map(|p| std::fs::read(p).ok()) else {
            return;
        };
        assert_eq!(data.len(), SLOT_COUNT * SLOT_RECORD_LEN, "ten 32-byte records");
        let slots = parse_slot_directory(&data).expect("the real directory parses");
        assert_eq!(slots.len(), SLOT_COUNT);
        // Every slot names its own file, in order.
        for (i, slot) in slots.iter().enumerate() {
            assert_eq!(slot.file, format!("game{}.sav", i + 1), "slot {i}");
        }
        // The live save flow typed a name into slot 1 and left the rest blank.
        assert_eq!(slots[0].name, "ab");
        assert!(slots[1..].iter().all(|s| s.name.is_empty()));
        // A short image is not a directory.
        assert!(parse_slot_directory(&data[..data.len() - 1]).is_none());
    }

    #[test]
    fn parses_a_real_save_if_present() {
        let paths = [
            "accuracy/cdrive/cblood/game1.sav",
            "../accuracy/cdrive/cblood/game1.sav",
            "accuracy/cdrive/cblood/blood.sav",
            "../accuracy/cdrive/cblood/blood.sav",
        ];
        let Some(data) = paths
            .iter()
            .find_map(|p| std::fs::read(p).ok())
        else {
            return;
        };
        let save = BloodSave::parse(&data).expect("real blood.sav parses");
        // Falsifiable against the REAL file: the fixed header plus the opaque
        // runtime region must account for every byte, and the header is the three
        // sizes the writer's `mov cx,imm` immediates carry.
        assert_eq!(
            PROFILE_SIZE + save.flags.len() + save.state.len() + save.runtime.len(),
            data.len(),
            "the parse must consume the whole file"
        );
        assert_eq!(HEADER_SIZE, 610);
        assert!(
            save.runtime.len() > HEADER_SIZE,
            "the variable region dwarfs the header in a real save ({} bytes)",
            save.runtime.len()
        );
        // The profile is a small index or the 0xFFFF sentinel.
        assert!(save.profile <= 16 || save.profile == 0xFFFF);
    }

    /// The header sizes are not free constants: they are the `mov cx,imm` values
    /// of the three `int 21h` AH=0x40 writes in `vm_state_save` (`0x1C3F`), and
    /// the globals they stream from are the `mov dx,imm` right beside them.
    #[test]
    fn the_header_sizes_are_the_writers_own_immediates() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        let imm16 = |at: usize| u16::from_le_bytes([exe[at], exe[at + 1]]) as usize;
        for (at, want) in SAVE_WRITE_SIZE_IMMEDIATES {
            assert_eq!(imm16(at), want, "mov cx,imm at {at:#x}");
        }
        // ...and the source/destination globals, including the deliberate
        // asymmetry: the writer takes the CURRENT profile, the reader posts it as
        // PENDING for the main loop to dispatch.
        assert_eq!(imm16(0x1C64), SAVE_PROFILE_SOURCE_DS as usize, "save mov dx");
        assert_eq!(imm16(0x1C6E), FLAGS_SOURCE_DS as usize, "flags mov dx");
        assert_eq!(imm16(0x1C73), STATE_SOURCE_DS as usize, "state mov dx");
        assert_eq!(imm16(0x1CEC), LOAD_PROFILE_DEST_DS as usize, "load mov dx");
        assert_ne!(
            SAVE_PROFILE_SOURCE_DS, LOAD_PROFILE_DEST_DS,
            "if these ever match, the mechanism note in the module docs is wrong"
        );
    }
}
