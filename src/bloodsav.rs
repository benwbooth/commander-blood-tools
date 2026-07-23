//! The DOS `blood.sav` save-file format — the byte-exact layout the original
//! `BLOODPRG.EXE` reads and writes.
//!
//! Decoded from the binary's save/load routines (`vm_state_save` @0x1C3F,
//! `vm_state_load` @0x1CBD; see `re/REVERSE.md`). Both serialize the live VM
//! state in the same field order:
//!
//! | field        | size            | source (game global) | meaning |
//! |--------------|-----------------|----------------------|---------|
//! | `profile`    | 2 bytes (u16 LE)| `[0x6780]`           | current script profile index (which SCRIPT set was active) |
//! | `flags`      | 512 bytes       | `[0x6ADE]`           | the global flag/progression block (persistent world state) |
//! | `state`      | 96 bytes        | `[0x6CDE]`           | a secondary state block |
//! | `object_block` | variable      | far `[0x6724]`       | the runtime VM object/state table |
//! | `work_buffer`  | variable      | far `[0xABC]`        | the object work buffer |
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

/// Byte offset/size constants of the fixed header (all little-endian).
pub const PROFILE_SIZE: usize = 2;
pub const FLAGS_SIZE: usize = 0x200; // 512
pub const STATE_SIZE: usize = 0x60; // 96
pub const HEADER_SIZE: usize = PROFILE_SIZE + FLAGS_SIZE + STATE_SIZE;

impl BloodSave {
    /// Parse a `blood.sav` image. Returns `None` if it is shorter than the fixed
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
        assert_eq!(save.flags.len(), FLAGS_SIZE);
        assert_eq!(save.state.len(), STATE_SIZE);
        // The profile is a small index or the 0xFFFF sentinel.
        assert!(save.profile <= 16 || save.profile == 0xFFFF);
    }
}
