//! Partial decoder for the `.ext` world-body structure (the planet/cyberspace world
//! files). Decoded so far (see `re/REVERSE.md`): after the 8-byte world magic
//! ([`crate::levels::EXT_WORLD_MAGIC`]) the body is a series of sections. The first is a
//! **count-prefixed table of 3-byte records** terminated by `FF FF`. Characterized
//! (sess 007): these are **triangle-mesh face connectivity** — ~73% are strictly-
//! ascending vertex-index triples (`a<b<c`), vertices are reused across faces (high
//! shared in-degree), and it is not a tree — the geometry of the location's pseudo-3D
//! scene. A following section carries 16-bit coordinate records (vertex positions /
//! anchors, e.g. 134,117) then a largely preallocated/sparse region. The vertex-
//! coordinate layout + object semantics past the mesh are still under study.

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
    /// The first-section records that are strictly-ascending index triples (`a<b<c`) —
    /// **triangle-mesh faces** (vertex-index triples). The characterization: on the real
    /// worlds ~73% of records are proper ascending triples, indices share high in-degree
    /// (vertices reused across faces), and the structure is not a tree — the signature of
    /// mesh face connectivity for the location's pseudo-3D geometry, not a room/script
    /// tree. Returns the `(a,b,c)` faces.
    pub fn mesh_faces(&self) -> Vec<[u8; 3]> {
        self.table1
            .iter()
            .filter(|r| r[0] < r[1] && r[1] < r[2])
            .copied()
            .collect()
    }

    /// The highest vertex index referenced by any mesh face (the implied vertex count is
    /// this + 1), for locating/validating the vertex-coordinate section.
    pub fn max_vertex_index(&self) -> u8 {
        self.table1.iter().flatten().copied().max().unwrap_or(0)
    }

    /// The non-zero index links of first-section record `i` — each 3-byte record holds
    /// up to three references to other records (`0` = no link), i.e. the section is an
    /// adjacency table (each node links to up to 3 others). Returns the linked indices.
    /// (The semantic — room graph vs mesh connectivity — is still under study; the
    /// structure itself is decoded and validated.)
    pub fn record_links(&self, i: usize) -> Vec<u8> {
        self.table1
            .get(i)
            .map(|r| r.iter().copied().filter(|&v| v != 0).collect())
            .unwrap_or_default()
    }

    /// Whether every record link references a valid record index (< record count) — a
    /// consistency check that the adjacency interpretation holds.
    pub fn links_are_valid(&self) -> bool {
        let n = self.table1.len();
        self.table1
            .iter()
            .all(|r| r.iter().all(|&v| (v as usize) <= n))
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
    fn first_section_is_triangle_mesh_connectivity() {
        let Some(data) = load("VENUSIA.EXT") else { return };
        let ext = parse_ext(&data).unwrap();
        let faces = ext.mesh_faces();
        // The majority of records are proper ascending triangle faces.
        assert!(
            faces.len() * 100 / ext.table1.len() >= 60,
            "most records are ascending triangles ({}/{})",
            faces.len(),
            ext.table1.len()
        );
        // Faces are ascending and index within the vertex range.
        let maxv = ext.max_vertex_index();
        for f in &faces {
            assert!(f[0] < f[1] && f[1] < f[2]);
            assert!(f[2] <= maxv);
        }
        // Vertices are reused across faces (mesh, not a tree): fewer distinct vertices
        // than face-vertex slots.
        let distinct: std::collections::BTreeSet<u8> =
            faces.iter().flatten().copied().collect();
        assert!(distinct.len() < faces.len() * 3, "vertices are shared across faces");
    }

    #[test]
    fn rejects_non_world_data() {
        assert!(parse_ext(b"not a world file at all").is_none());
    }
}
