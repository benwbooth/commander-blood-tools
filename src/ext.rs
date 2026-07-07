//! Partial decoder for the `.ext` world-body structure (the planet/cyberspace world
//! files). Decoded so far (see `re/REVERSE.md`): after the 8-byte world magic
//! ([`crate::levels::EXT_WORLD_MAGIC`]) the body is a series of sections. The first is a
//! **count-prefixed table of 3-byte records** terminated by `FF FF`; the values index
//! within the record count (a lookup/adjacency table). A following section carries
//! 16-bit coordinate records (screen-space positions like 134,117). The full geometry/
//! object semantics past this framing are still under study — this ports the section
//! framing so the body can be walked.

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
    fn rejects_non_world_data() {
        assert!(parse_ext(b"not a world file at all").is_none());
    }
}
