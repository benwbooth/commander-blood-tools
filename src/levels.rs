//! The game's level/world-file directory — decoded from the 16-byte filename-record
//! table in segment `0x0ca3` (see `re/REVERSE.md`). Level loading is table-driven off
//! this directory indexed by world number; the `.ext` planet worlds are the navigable
//! destinations (they match the `fd/1<name>*.lbm` location art).

/// A directory entry: the base filename (no dir/extension) and its kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelEntry {
    /// Directory index (the world number the loader uses).
    pub index: u8,
    /// The base file stem, e.g. `"venusia"` for `venusia.ext`.
    pub stem: &'static str,
    /// What the entry is.
    pub kind: LevelKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevelKind {
    /// A navigable planet world (`*.ext`, including cyberspace + sub-levels).
    World,
    /// A bridge/HUD sprite bank (`*.spr`).
    Sprite,
    /// A `script2.*` bytecode/data file.
    Script,
    /// Other resource (`dnsdb.drv`, etc.).
    Resource,
}

/// The decoded directory, in table order (index = position in the segment-0x0ca3 table).
pub const LEVEL_DIRECTORY: &[LevelEntry] = &[
    LevelEntry { index: 0, stem: "bcarte", kind: LevelKind::Sprite },
    LevelEntry { index: 1, stem: "bhyper", kind: LevelKind::Sprite },
    LevelEntry { index: 2, stem: "bpol", kind: LevelKind::Sprite },
    LevelEntry { index: 3, stem: "aphyper", kind: LevelKind::Sprite },
    LevelEntry { index: 4, stem: "appol", kind: LevelKind::Sprite },
    LevelEntry { index: 5, stem: "black", kind: LevelKind::World },
    LevelEntry { index: 6, stem: "kult", kind: LevelKind::World },
    LevelEntry { index: 7, stem: "rondo", kind: LevelKind::World },
    LevelEntry { index: 8, stem: "venusia", kind: LevelKind::World },
    LevelEntry { index: 9, stem: "erazor", kind: LevelKind::World },
    LevelEntry { index: 10, stem: "mastacho", kind: LevelKind::World },
    LevelEntry { index: 11, stem: "magnus", kind: LevelKind::World },
    LevelEntry { index: 12, stem: "ekatomb", kind: LevelKind::World },
    LevelEntry { index: 13, stem: "crazy", kind: LevelKind::World },
    LevelEntry { index: 14, stem: "eden", kind: LevelKind::World },
    LevelEntry { index: 15, stem: "kortex", kind: LevelKind::World },
    LevelEntry { index: 16, stem: "vista", kind: LevelKind::World },
    LevelEntry { index: 17, stem: "moskito", kind: LevelKind::World },
    LevelEntry { index: 18, stem: "pterra", kind: LevelKind::World },
    LevelEntry { index: 19, stem: "cyber", kind: LevelKind::World },
    LevelEntry { index: 20, stem: "script2.cod", kind: LevelKind::Script },
    LevelEntry { index: 21, stem: "script2.bas", kind: LevelKind::Script },
    LevelEntry { index: 22, stem: "script2.var", kind: LevelKind::Script },
    LevelEntry { index: 23, stem: "script2.dic", kind: LevelKind::Script },
    LevelEntry { index: 24, stem: "script2.deb", kind: LevelKind::Script },
    LevelEntry { index: 25, stem: "dnsdb.drv", kind: LevelKind::Resource },
    LevelEntry { index: 26, stem: "corpo", kind: LevelKind::World },
    LevelEntry { index: 27, stem: "carte", kind: LevelKind::Sprite },
    LevelEntry { index: 28, stem: "bigark", kind: LevelKind::World },
    LevelEntry { index: 29, stem: "cyber2", kind: LevelKind::World },
    LevelEntry { index: 30, stem: "cyber3", kind: LevelKind::World },
    LevelEntry { index: 31, stem: "eden2", kind: LevelKind::World },
    LevelEntry { index: 32, stem: "eden3", kind: LevelKind::World },
    LevelEntry { index: 33, stem: "ekatomb2", kind: LevelKind::World },
    LevelEntry { index: 34, stem: "ekatomb3", kind: LevelKind::World },
    LevelEntry { index: 35, stem: "erazor2", kind: LevelKind::World },
];

/// The primary navigable planet worlds — the distinct destinations shown on the nav
/// map (the top-level `.ext` worlds, excluding cyberspace levels and `2`/`3` sub-levels
/// which are entered from their parent world).
pub fn primary_worlds() -> impl Iterator<Item = &'static LevelEntry> {
    LEVEL_DIRECTORY.iter().filter(|e| {
        e.kind == LevelKind::World
            && !e.stem.starts_with("cyber")
            && !e.stem.ends_with('2')
            && !e.stem.ends_with('3')
    })
}

/// Look up a directory entry by its world index.
pub fn entry(index: u8) -> Option<&'static LevelEntry> {
    LEVEL_DIRECTORY.get(index as usize)
}

/// The 8-byte header every `.ext` world file begins with — verified identical across the
/// planet worlds (venusia/eden/magnus/black/kortex/pterra/…) AND the cyberspace levels
/// (cyber/cyber2/cyber3). So cyberspace is a world in the same format as the planets, not
/// a special minigame data blob; decoding the world format decodes all of them.
pub const EXT_WORLD_MAGIC: [u8; 8] = [0x02, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x81];

/// Whether `data` is a `.ext` world file (begins with [`EXT_WORLD_MAGIC`]).
pub fn is_ext_world(data: &[u8]) -> bool {
    data.len() >= EXT_WORLD_MAGIC.len() && data[..EXT_WORLD_MAGIC.len()] == EXT_WORLD_MAGIC
}

/// The `fd/` location-art filename prefix for a world's rooms. Each world has multiple
/// room backgrounds under `fd/` (floors `1`/`2`/`3`, view-angle suffixes `b`/`d`/`f`/`g`);
/// the naming is per-world (some `1<5char>` like `1venus`, some `<name>1<suffix>` like
/// `kortex1`), so this is an explicit verified map rather than a computed rule. Returns
/// the prefix that a world's location LBMs start with, or `None` if the world's art isn't
/// under this naming (e.g. sprite/script entries, or worlds shown via HNM not LBM).
pub fn world_location_art_prefix(stem: &str) -> Option<&'static str> {
    Some(match stem {
        "venusia" => "1venus",
        "eden" => "1eeden",
        "ekatomb" => "1ekato",
        "erazor" => "1erazo",
        "kult" => "1kkult",
        "magnus" => "1magnu",
        "mastacho" => "1masta",
        "rondo" => "1rondo",
        "vista" => "1vista",
        "cyber" => "1cyber",
        "kortex" => "kortex",
        "pterra" => "pterra",
        "crazy" => "crazys",
        "moskito" => "moskit",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_indices_are_dense_and_ordered() {
        for (i, e) in LEVEL_DIRECTORY.iter().enumerate() {
            assert_eq!(e.index as usize, i, "entry {i} index matches position");
        }
        // cyberspace is entry 19, its extra levels 29/30.
        assert_eq!(entry(19).unwrap().stem, "cyber");
        assert_eq!(entry(29).unwrap().stem, "cyber2");
        assert_eq!(entry(30).unwrap().stem, "cyber3");
    }

    #[test]
    fn world_location_art_prefixes_resolve_to_real_fd_files() {
        // Each mapped world must have at least one matching fd/ location LBM.
        let dir = ["output/_tmp_dat/fd", "../output/_tmp_dat/fd"]
            .iter().map(std::path::Path::new).find(|p| p.exists());
        let Some(dir) = dir else { return };
        let files: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_lowercase())
            .collect();
        for world in ["venusia", "eden", "ekatomb", "kult", "magnus", "kortex", "pterra"] {
            let prefix = world_location_art_prefix(world).unwrap();
            assert!(
                files.iter().any(|f| f.starts_with(prefix) && f.ends_with(".lbm")),
                "world {world} -> prefix {prefix} has an fd/ LBM"
            );
        }
        // A non-mapped entry returns None.
        assert!(world_location_art_prefix("script2.cod").is_none());
    }

    #[test]
    fn all_worlds_share_the_ext_magic_incl_cyberspace() {
        // Every world file — planets and cyberspace alike — begins with EXT_WORLD_MAGIC.
        // Confirms the shared format. Skips if assets aren't present.
        let dir = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter().map(std::path::Path::new).find(|p| p.exists());
        let Some(dir) = dir else { return };
        let mut checked = 0;
        for name in ["VENUSIA", "EDEN", "MAGNUS", "BLACK", "KORTEX", "PTERRA", "CYBER", "CYBER2"] {
            let p = dir.join(format!("{name}.EXT"));
            let Ok(data) = std::fs::read(&p) else { continue };
            assert!(is_ext_world(&data), "{name}.EXT begins with the shared world magic");
            checked += 1;
        }
        // A non-world byte string is rejected.
        assert!(!is_ext_world(b"not a world file"));
        let _ = checked;
    }

    #[test]
    fn primary_worlds_are_the_named_planets() {
        let names: Vec<_> = primary_worlds().map(|e| e.stem).collect();
        assert!(names.contains(&"venusia"));
        assert!(names.contains(&"magnus"));
        assert!(names.contains(&"ekatomb"));
        assert!(names.contains(&"eden"));
        // Excludes cyberspace + numbered sub-levels.
        assert!(!names.contains(&"cyber"));
        assert!(!names.contains(&"eden2"));
        assert!(!names.contains(&"ekatomb3"));
        // The full planet set is 16 distinct top-level worlds (entries 5-18 + corpo +
        // bigark; cyber and the numbered sub-levels are entered from a parent world).
        assert_eq!(names.len(), 16);
    }
}
