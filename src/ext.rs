//! Partial decoder for the `.ext` world-body structure (the planet/cyberspace world
//! files). Decoded so far (see `re/REVERSE.md`): after the 8-byte world magic
//! ([`crate::levels::EXT_WORLD_MAGIC`]) the body is a series of sections. The first is a
//! **count-prefixed table of 3-byte records** terminated by `FF FF`. Validated across
//! 36/37 world files (sess 007): the count (body byte 8) is ~63 for most worlds
//! (occasionally 62/55/49/33/12), the section is `FF FF`-terminated, and every record's
//! three values index within the record count (`0` = no link) — a fixed-size adjacency/
//! index table.
//!
//! NOTE (corrected): an earlier claim that these are *triangle-mesh faces* was
//! **over-generalized from venusia alone**. The cross-world survey shows the
//! strictly-ascending-triple share varies wildly — venusia 79%, ekatomb3 71%,
//! venusia2 53%, but corpo/crazy/cyber/magnus/kortex/… are ~0%. So "ascending triangle
//! faces" is a per-world value pattern, not the table's universal semantic; the records
//! are a 3-link index/adjacency structure whose meaning is still under study. A
//! following section carries 16-bit values (e.g. 134,117) then a largely preallocated/
//! sparse region.

use crate::levels::EXT_WORLD_MAGIC;

/// The decoded framing of an `.ext` world body's first section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtWorld {
    /// The first section's 3-byte records (`[a,b,c]`), count taken from body byte 8.
    pub table1: Vec<[u8; 3]>,
    /// Whether the first section ended with the expected `FF FF` terminator.
    pub terminated: bool,
    /// Byte offset just past the first section (its terminator) — where the next
    /// section (16-bit coordinate records) begins.
    pub next_section: usize,
}

impl ExtWorld {
    /// The records that are strictly-ascending index triples (`a<b<c`). In some worlds
    /// (venusia 79%, ekatomb3 71%) these dominate — consistent with triangle-mesh face
    /// connectivity there — but in most worlds they are near 0%, so this is a per-world
    /// pattern, not the table's universal semantic (see the module note). Kept as a
    /// diagnostic, not an asserted interpretation.
    pub fn ascending_triple_records(&self) -> Vec<[u8; 3]> {
        self.table1
            .iter()
            .filter(|r| r[0] < r[1] && r[1] < r[2])
            .copied()
            .collect()
    }

    /// The fraction (0..100) of records that are strictly-ascending triples — the
    /// diagnostic that varies per world (venusia ~79%, magnus ~0%).
    pub fn ascending_triple_percent(&self) -> usize {
        if self.table1.is_empty() {
            return 0;
        }
        self.ascending_triple_records().len() * 100 / self.table1.len()
    }

    /// The highest index referenced by any record (`+1` = the index space size), for
    /// locating/validating the following section.
    pub fn max_index(&self) -> u8 {
        self.table1.iter().flatten().copied().max().unwrap_or(0)
    }

    /// The "no-link" sentinel for a node reference. Cross-validated across 35/36 clean
    /// worlds: every `b`/`c` field is either a valid node index (`< count`) or exactly
    /// `0x3F` (63), which marks the absence of a link — analogous to the `FF FF` section
    /// terminator but within a 6-bit index space. (The lone exception, CYBER3, has a
    /// smaller count of 33 and a different first-section layout.)
    pub const NO_LINK: u8 = 0x3F;

    /// The out-links of first-section record `i` — each 3-byte record holds up to three
    /// references to other nodes; a reference of `0` or [`Self::NO_LINK`] (0x3F) marks the
    /// absence of a link. Returns the referenced node indices (all three fields, since
    /// each — including `a` — stays in the index range or uses the 0x3F sentinel).
    ///
    /// The reference space is **directed, not symmetric**: across BLACK only ~4% of links
    /// are reciprocated, so this is a directed graph / tree (a traversal or containment
    /// hierarchy over the location's nodes), not an undirected room-adjacency graph as was
    /// earlier speculated. The structure is decoded and cross-validated; the precise
    /// gameplay role (nav order vs scene-object hierarchy) is still the open question.
    pub fn record_links(&self, i: usize) -> Vec<u8> {
        self.table1
            .get(i)
            .map(|r| {
                r.iter()
                    .copied()
                    .filter(|&v| v != 0 && v != Self::NO_LINK)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Whether every node reference is either a valid index (`< count`) or the
    /// [`Self::NO_LINK`] sentinel — the cross-world consistency check for the directed-node
    /// interpretation (holds for 35/36 clean worlds; the lone exception is CYBER3).
    pub fn links_are_valid(&self) -> bool {
        let n = self.table1.len() as u8;
        self.table1
            .iter()
            .all(|r| r.iter().all(|&v| v < n || v == Self::NO_LINK))
    }
}

/// A world object record — the 10-byte entries in the section after the first table
/// (`next_section`). Cross-validated across venusia/magnus/black: each world's initial
/// object is `id=1, type=4` at a world-specific screen position (venusia 134,117; magnus
/// 169,92; black 199,42). Most slots are zero (preallocated, filled at runtime). The
/// `x`/`y` are the object coordinates `entity_draw` (0x9240) scales + renders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtObject {
    pub id: u16,       // +0x00
    pub kind: u16,     // +0x02
    pub reserved: u16, // +0x04
    pub x: u16,        // +0x06
    pub y: u16,        // +0x08
}

impl ExtWorld {
    /// Parse the object records (10-byte `[id, type, reserved, x, y]`) starting at
    /// [`Self::next_section`] from `data`, returning the non-empty (any field set) ones.
    pub fn objects(&self, data: &[u8]) -> Vec<ExtObject> {
        let w = |o: usize| -> u16 {
            u16::from_le_bytes([
                data.get(o).copied().unwrap_or(0),
                data.get(o + 1).copied().unwrap_or(0),
            ])
        };
        let mut out = Vec::new();
        let mut o = self.next_section;
        // Bounded scan of the object-record region (stop at the dense payload / EOF).
        for _ in 0..64 {
            if o + 10 > data.len() {
                break;
            }
            let obj = ExtObject {
                id: w(o),
                kind: w(o + 2),
                reserved: w(o + 4),
                x: w(o + 6),
                y: w(o + 8),
            };
            if obj != (ExtObject { id: 0, kind: 0, reserved: 0, x: 0, y: 0 }) {
                out.push(obj);
            }
            o += 10;
        }
        out
    }
}

/// Parse the framing of an `.ext` world body. Returns `None` if it isn't a world file.
/// The first-section record count is body byte 8; records are 3 bytes each and the
/// section is expected to end with `FF FF` (holds for venusia/magnus/black/cyber; some
/// worlds like eden use a different first-section layout, reported via `terminated`).
pub fn parse_ext(data: &[u8]) -> Option<ExtWorld> {
    if data.len() < EXT_WORLD_MAGIC.len() || data[..EXT_WORLD_MAGIC.len()] != EXT_WORLD_MAGIC {
        return None;
    }
    let count = *data.get(8)? as usize;
    let mut table1 = Vec::with_capacity(count);
    let mut o = 9usize;
    for _ in 0..count {
        let rec = data.get(o..o + 3)?;
        table1.push([rec[0], rec[1], rec[2]]);
        o += 3;
    }
    let terminated = data.get(o..o + 2) == Some(&[0xFF, 0xFF]);
    let next_section = if terminated { o + 2 } else { o };
    Some(ExtWorld {
        table1,
        terminated,
        next_section,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(name: &str) -> Option<Vec<u8>> {
        ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(|d| std::path::Path::new(d).join(name))
            .find(|p| p.exists())
            .and_then(|p| std::fs::read(p).ok())
    }

    /// Every primary world's `.ext` must parse and expose an initial object `id=1, type=4`
    /// at an on-screen position (0..=320, 0..=200) - the actor/room anchor. Extends the
    /// venusia/magnus/black cross-validation to all 16 primary worlds. Skips absent files.
    #[test]
    fn all_primary_worlds_parse_with_valid_initial_object() {
        const WORLDS: &[&str] = &[
            "BLACK", "KULT", "VENUSIA", "ERAZOR", "MASTACHO", "MAGNUS", "EKATOMB", "CRAZY",
            "KORTEX", "VISTA", "MOSKITO", "PTERRA", "CYBER", "CORPO", "MENHIR", "VULCAN",
        ];
        let mut checked = 0;
        for w in WORLDS {
            let Some(data) = load(&format!("{w}.EXT")) else {
                continue;
            };
            let ext = parse_ext(&data).unwrap_or_else(|| panic!("{w}.EXT parses"));
            let objs = ext.objects(&data);
            let first = objs.first().unwrap_or_else(|| panic!("{w}.EXT has an object"));
            assert_eq!(first.id, 1, "{w} initial object id");
            assert_eq!(first.kind, 4, "{w} initial object type");
            assert!(first.x <= 320 && first.y <= 200, "{w} object pos ({},{})", first.x, first.y);
            checked += 1;
        }
        if checked > 0 {
            assert_eq!(checked, WORLDS.len(), "all present worlds validated");
        }
    }

    #[test]
    fn parses_first_section_framing_of_real_worlds() {
        // venusia/magnus/black/cyber: count-prefixed 3-byte records + FF FF terminator.
        for (name, count) in [
            ("VENUSIA.EXT", 63usize),
            ("MAGNUS.EXT", 62),
            ("BLACK.EXT", 63),
            ("CYBER.EXT", 63),
        ] {
            let Some(data) = load(name) else { continue };
            let ext = parse_ext(&data).expect("parses world");
            assert_eq!(ext.table1.len(), count, "{name} first-section record count");
            assert!(ext.terminated, "{name} first section ends with FF FF");
            // The 3-byte record values stay within the record-count index range.
            for r in &ext.table1 {
                assert!(r.iter().all(|&v| (v as usize) <= count));
            }
            // The next section begins right after the terminator.
            assert_eq!(ext.next_section, 9 + count * 3 + 2);
        }
    }

    #[test]
    fn first_section_is_a_valid_adjacency_table() {
        let Some(data) = load("VENUSIA.EXT") else { return };
        let ext = parse_ext(&data).unwrap();
        // All record links reference valid record indices.
        assert!(ext.links_are_valid());
        // record_links drops the zero (no-link) entries. venusia record 1 = (8,10,14).
        assert_eq!(ext.record_links(1), vec![8, 10, 14]);
        // A (0,0,c) record has a single link.
        assert_eq!(ext.record_links(0), vec![8]); // (0,0,8)
        // Out-of-range record -> no links.
        assert!(ext.record_links(9999).is_empty());
    }

    #[test]
    fn node_refs_are_index_or_0x3f_sentinel_across_worlds() {
        // Cross-world decode: every node reference (all three record fields) is either a
        // valid index (< count) or exactly 0x3F, the "no-link" sentinel. Verified against
        // every clean count+FF-FF world present; the lone known exception is CYBER3 (a
        // 33-node world with a different first-section layout), which we exclude.
        let names = [
            "VENUSIA.EXT",
            "MAGNUS.EXT",
            "BLACK.EXT",
            "CYBER.EXT",
            "KORTEX.EXT",
            "VULCAN.EXT",
            "FOREST.EXT",
            "MENHIR.EXT",
        ];
        let mut checked = 0;
        for name in names {
            let Some(data) = load(name) else { continue };
            let Some(ext) = parse_ext(&data) else { continue };
            if !ext.terminated || ext.table1.is_empty() {
                continue;
            }
            let n = ext.table1.len() as u8;
            for r in &ext.table1 {
                for &v in r {
                    assert!(
                        v < n || v == ExtWorld::NO_LINK,
                        "{name}: node ref {v} is neither a valid index (<{n}) nor the 0x3F sentinel",
                    );
                }
            }
            assert!(ext.links_are_valid(), "{name} links_are_valid");
            checked += 1;
        }
        assert!(checked > 0, "no world files available to check");
    }

    #[test]
    fn ascending_triple_share_varies_by_world_not_universal() {
        // The corrected finding: the ascending-triple share is per-world, NOT a
        // universal mesh signature. venusia is high, magnus/cyber ~0.
        if let Some(v) = load("VENUSIA.EXT") {
            let ext = parse_ext(&v).unwrap();
            assert!(ext.ascending_triple_percent() >= 60, "venusia is highly ascending");
            // Ascending records are in-range and strictly ordered.
            for f in ext.ascending_triple_records() {
                assert!(f[0] < f[1] && f[1] < f[2] && f[2] <= ext.max_index());
            }
        }
        for low in ["MAGNUS.EXT", "CYBER.EXT"] {
            if let Some(d) = load(low) {
                let ext = parse_ext(&d).unwrap();
                assert!(
                    ext.ascending_triple_percent() < 30,
                    "{low} is not ascending-dominated ({}%)",
                    ext.ascending_triple_percent()
                );
            }
        }
    }

    #[test]
    fn object_records_decode_the_initial_world_object() {
        // Each world's first object record is id=1, type=4 at a world-specific position.
        for (name, x, y) in [("VENUSIA.EXT", 134, 117), ("MAGNUS.EXT", 169, 92), ("BLACK.EXT", 199, 42)] {
            let Some(data) = load(name) else { continue };
            let ext = parse_ext(&data).unwrap();
            let objs = ext.objects(&data);
            assert!(!objs.is_empty(), "{name} has an initial object");
            let first = objs[0];
            assert_eq!(first.id, 1, "{name} object id");
            assert_eq!(first.kind, 4, "{name} object type");
            assert_eq!((first.x, first.y), (x, y), "{name} object position");
        }
    }

    #[test]
    fn rejects_non_world_data() {
        assert!(parse_ext(b"not a world file at all").is_none());
    }
}
