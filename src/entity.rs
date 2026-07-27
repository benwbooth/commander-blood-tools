//! Object/entity instance model — a port of the decoded runtime object system
//! (`entity_object_table` at `DS:0x6212`, see `re/REVERSE.md` + `labels.csv`). Each
//! object is a 32-byte record; this ports the record's flag word and its decoded state
//! machine (the flag getter/setter/toggle family at `0x41c3`/`0x41d1`/`0x420d`/`0x428c`).

/// Flag bits in the entity record's `+0x00` word (decoded from the toggle-routine family).
pub mod flag {
    /// `0x80` — the object is active (gates every state change).
    /// `0x80` — bit 7, and it NEVER appears as an immediate: it is read by SIGN
    /// (`or al,al / jns 0x41EA` @`0x41DE`) and by the PAIR `test al,0x81`
    /// @`0x421D`/`0x42DD`, which tests it together with [`STATE0`]. #511 records
    /// this as the binary's habit for bit-7 flags (audit-fixes #539).
    ///
    /// SAME BIT as `ship3d::SHIP_3D_OBJECT_VISIBLE_FLAG` — see [`INIT`].
    pub const ACTIVE: u16 = 0x80;
    /// `0x01` — state bit 0 (advances to [`STATE1`]).
    /// `0x01` — `test al,1` @`0x41E2`, and again @`0x4201`, `0x4280`, `0x429F`,
    /// `0x42C1`: five sites across the flag family. Same bit as
    /// `ship3d::SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG` (audit-fixes #539).
    pub const STATE0: u16 = 0x01;
    /// `0x02` — state bit 1 (set when STATE0 advances).
    /// `0x02` — `or al,2` @`0x41E8`, set as `and al,0xfe` @`0x41E6` clears
    /// [`STATE0`], and again @`0x4227`. The clear-and-set is one handoff, not two
    /// edits (#505). Same bit as `ship3d::SHIP_3D_SPRITE_SLOT_DIRTY_FLAG`
    /// (audit-fixes #539).
    pub const STATE1: u16 = 0x02;
    /// `0x20` — a toggle state (bit 5).
    /// `0x20` — `xor al,0x20` @`0x427E`. An XOR, so this really does TOGGLE;
    /// unlike the state bits above it is never set or cleared outright
    /// (audit-fixes #539).
    pub const TOGGLE5: u16 = 0x20;
    /// `0x40` — a toggle state (bit 6).
    /// `0x40` — `xor al,0x40` @`0x429D`, the twin of [`TOGGLE5`]
    /// (audit-fixes #539).
    pub const TOGGLE6: u16 = 0x40;
    /// `0x04` — carried from the source data during populate.
    pub const SOURCE: u16 = 0x04;
    /// The initial flags an object is populated with (`0x83` = active + state0 + state1).
    /// `0x83` = active + state0 + state1.
    ///
    /// THIS WORD IS SHARED WITH `ship3d.rs` (audit-fixes #539). The record is the
    /// same one #503/#505 decoded from the other side: `shl ax,5 / mov bx,0x6212 /
    /// add bx,ax` opens `0x420D`, `0x428C` and `0x41D1` alike, so `entity.rs`'s
    /// "object flags" and `ship3d.rs`'s "sprite slot flags" are ONE 32-byte
    /// record's `+0x00` word under two sets of names:
    ///
    /// ```text
    ///   ACTIVE 0x80  =  SHIP_3D_OBJECT_VISIBLE_FLAG
    ///   STATE0 0x01  =  SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG
    ///   STATE1 0x02  =  SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
    /// ```
    ///
    /// The names disagree about what the bits MEAN — "visible" versus "active",
    /// "active" versus "state 0" — while agreeing on every value. Neither naming
    /// is wrong for its own subsystem; both are recorded here so a reader of
    /// either file knows the other exists.
    pub const INIT: u16 = ACTIVE | STATE0 | STATE1;
}

/// A runtime object instance — the decoded 32-byte `entity_object_table` record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct EntityObject {
    /// `+0x00` flags word (see [`flag`]).
    pub flags: u16,
    /// `+0x04`/`+0x06` far pointer to the object's data (segment, offset) in a resource.
    pub data_ptr: (u16, u16),
    /// `+0x08` comparable id/group/target.
    pub group: u16,
    /// `+0x0c`/`+0x0e` two data words (position).
    pub pos: (u16, u16),
    /// `+0x14`/`+0x16` initial backups of `pos` (reset-to values).
    pub init_pos: (u16, u16),
}

impl EntityObject {
    /// Populate an object as the decoded `entity_object_populate` routines do: flags =
    /// `(source & 0x04) | 0x83`, and `init_pos` backs up `pos`.
    pub fn populate(source_flags: u16, data_ptr: (u16, u16), group: u16, pos: (u16, u16)) -> Self {
        Self {
            flags: (source_flags & flag::SOURCE) | flag::INIT,
            data_ptr,
            group,
            pos,
            init_pos: pos,
        }
    }

    /// Whether the object is active (`+0x00 & 0x80`).
    pub fn is_active(&self) -> bool {
        self.flags & flag::ACTIVE != 0
    }

    /// The decoded state advance (`0x41d1`): only when active and STATE0 is set, clear
    /// STATE0 and set STATE1.
    pub fn advance_state(&mut self) {
        if self.is_active() && self.flags & flag::STATE0 != 0 {
            self.flags = (self.flags & !flag::STATE0) | flag::STATE1;
        }
    }

    /// The decoded toggle-family behaviour (`0x428C`, and its siblings `0x4270`,
    /// `0x42AB`, ...): only when active, toggle `mask`, then if STATE0 is set —
    /// tested AFTER the toggle, as the original does — also set STATE1.
    ///
    /// ```text
    ///   0x4299  or al,al / jns          inactive -> skip, and do not even store
    ///   0x429D  xor al,0x40             the family member's own bit
    ///   0x429F  test al,1 / je          the shared state-advance...
    ///   0x42A3  or al,2                 ...only when STATE0 survived the toggle
    /// ```
    ///
    /// THE CITATION USED TO SAY `0x420D`, which is not a toggle at all:
    /// `re/labels.csv` corrected it to `sprite_slot_set_draw_position` (the nav
    /// projector's `AX=id, BX=x, CX=y` setter). The correction never reached here.
    ///
    /// The original toggles a LOW-BYTE bit (`xor al`); this takes an arbitrary
    /// `mask`, which is a port generalisation — no caller in the game sets a high
    /// bit this way.
    pub fn toggle(&mut self, mask: u16) {
        if !self.is_active() {
            return;
        }
        self.flags ^= mask;
        if self.flags & flag::STATE0 != 0 {
            self.flags |= flag::STATE1;
        }
    }

    /// Reset the object's position to its populated initial backup (`+0x14/+0x16`).
    pub fn reset_position(&mut self) {
        self.pos = self.init_pos;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn populate_sets_init_flags_and_backs_up_position() {
        let e = EntityObject::populate(0xFFFF, (0x1234, 0x0040), 7, (134, 117));
        // flags = (source & 0x04) | 0x83
        assert_eq!(e.flags, flag::SOURCE | flag::INIT);
        assert_eq!(e.flags, 0x87);
        assert!(e.is_active());
        assert_eq!(e.init_pos, (134, 117));
        // source without bit 0x04 -> flags 0x83
        assert_eq!(EntityObject::populate(0, (0, 0), 0, (0, 0)).flags, 0x83);
    }

    #[test]
    fn advance_state_matches_0x41d1() {
        // active + state0 -> clears state0, sets state1.
        let mut e = EntityObject { flags: flag::ACTIVE | flag::STATE0, ..Default::default() };
        e.advance_state();
        assert_eq!(e.flags & flag::STATE0, 0);
        assert_eq!(e.flags & flag::STATE1, flag::STATE1);
        // inactive object: no change.
        let mut n = EntityObject { flags: flag::STATE0, ..Default::default() };
        n.advance_state();
        assert_eq!(n.flags, flag::STATE0);
    }

    #[test]
    fn toggle_family_gated_on_active() {
        let mut e = EntityObject { flags: flag::ACTIVE | flag::STATE0, ..Default::default() };
        e.toggle(flag::TOGGLE6);
        assert_eq!(e.flags & flag::TOGGLE6, flag::TOGGLE6, "bit toggled on");
        assert_eq!(e.flags & flag::STATE1, flag::STATE1, "state advanced");
        e.toggle(flag::TOGGLE6);
        assert_eq!(e.flags & flag::TOGGLE6, 0, "bit toggled off");
        // inactive: no toggle.
        let mut n = EntityObject { flags: 0, ..Default::default() };
        n.toggle(flag::TOGGLE6);
        assert_eq!(n.flags, 0);
    }

    #[test]
    fn reset_position_restores_the_init_backup() {
        let mut e = EntityObject::populate(0, (0, 0), 0, (100, 50));
        e.pos = (200, 80);
        e.reset_position();
        assert_eq!(e.pos, (100, 50));
    }
}
