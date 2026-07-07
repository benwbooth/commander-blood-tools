//! Object/entity instance model — a port of the decoded runtime object system
//! (`entity_object_table` at `DS:0x6212`, see `re/REVERSE.md` + `labels.csv`). Each
//! object is a 32-byte record; this ports the record's flag word and its decoded state
//! machine (the flag getter/setter/toggle family at `0x41c3`/`0x41d1`/`0x420d`/`0x428c`).

/// Flag bits in the entity record's `+0x00` word (decoded from the toggle-routine family).
pub mod flag {
    /// `0x80` — the object is active (gates every state change).
    pub const ACTIVE: u16 = 0x80;
    /// `0x01` — state bit 0 (advances to [`STATE1`]).
    pub const STATE0: u16 = 0x01;
    /// `0x02` — state bit 1 (set when STATE0 advances).
    pub const STATE1: u16 = 0x02;
    /// `0x20` — a toggle state (bit 5).
    pub const TOGGLE5: u16 = 0x20;
    /// `0x40` — a toggle state (bit 6).
    pub const TOGGLE6: u16 = 0x40;
    /// `0x04` — carried from the source data during populate.
    pub const SOURCE: u16 = 0x04;
    /// The initial flags an object is populated with (`0x83` = active + state0 + state1).
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

    /// The decoded toggle-family behaviour (`0x420d`/`0x428c`): only when active, toggle
    /// `mask`, and if STATE0 is set also set STATE1 (the shared state-advance side effect).
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
