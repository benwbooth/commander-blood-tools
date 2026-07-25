//! The game's resource/level directory — the `resource_name_table` at `FS:0x0c04`
//! (file 0xCDF4, label `resource_name_table`): 16-byte filename records **indexed by
//! resource ID**. The game loads any resource (sprites, the script1-5 bytecode sets,
//! and the `.ext` planet/cyberspace worlds) by its resource ID through this table +
//! the decoded resource loader (`vm_resource_profile_select` 0x53A0). The `index` field
//! here is the true resource ID. Worlds (`.ext`) are IDs 22..36 + sub-levels; they are
//! the navigable destinations and match the `fd/1<name>*.lbm` location art.

/// A directory entry: the base filename (no dir/extension) and its kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelEntry {
    /// The resource ID (`FS:0x0c04` table index) the loader uses.
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

/// The decoded resource directory (`FS:0x0c04`), `index` = resource ID = table
/// position. IDs 0..21 are engine sprites/drivers/the script1 set/buffers, 22..36 the
/// primary `.ext` worlds, 37+ the script2 set + more worlds/sub-levels.
/// The resource directory's file offset (`FS:0x0c04`; `FS_SEGMENT` `0x0bbf`
/// gives base `0x600 + 0xBBF*16` = `0xC1F0`, so the table starts at `0xCDF4`).
///
/// Cited, not just derived: `resource_name_table_access` (`0x3FC7`) does
/// `mov dx,0xc04` @`0x3FD4` with `ds = fs`.
pub const LEVEL_DIRECTORY_FILE_OFFSET: usize = 0xCDF4;
/// The RESOURCE DESCRIPTOR table's stride: `shl bx,3` @`0x51A5` turns a resource
/// ID into a descriptor offset, so records are 8 bytes and the table is based at
/// `FS:0x0000` (no base is added to `bx`).
///
/// The name table sits at `FS:0x0C04` in the same segment, which is consistent:
/// 95 descriptors occupy `0x2F8` bytes, comfortably below `0xC04`.
pub const RESOURCE_DESCRIPTOR_STRIDE: u16 = 8;
/// Descriptor `+0`: the SEGMENT the resource was loaded at (`mov ax,[bx]` /
/// `mov ds,ax` @`0x51B7`).
pub const RESOURCE_DESCRIPTOR_SEGMENT: u16 = 0;
/// Descriptor `+2`: flags. `test word [bx+2],3` @`0x51AC` asks "already
/// resident?"; the loader then sets bit 1 with `or word [bx+2],2` @`0x51B3` and
/// returns 1 without re-reading the file.
pub const RESOURCE_DESCRIPTOR_FLAGS: u16 = 2;
/// The residency mask tested at `0x51AC`.
pub const RESOURCE_FLAG_RESIDENT: u16 = 3;
/// The bit the loader sets on a cache hit (`0x51B3`).
pub const RESOURCE_FLAG_IN_USE: u16 = 2;

/// A resource ID's descriptor offset within the `FS` segment (`0x51A5`).
pub fn resource_descriptor_offset(id: u16) -> u16 {
    id.wrapping_mul(RESOURCE_DESCRIPTOR_STRIDE)
}

/// The resource loader's READ CHUNK SIZE: `mov cx,0x7d00` @`0x4041`, the count
/// for the `int 21h`/`AH=3Fh` read at `0x4049`, repeated until the size taken
/// from the FindFirst record (`es:[bx+0x1a]` @`0x3FF4`) is exhausted
/// (`sub ebp,eax` @`0x404D`).
///
/// COINCIDENCE, NOT A RELATIONSHIP: the audio streamer reads its source from
/// `DS:0x7D06` (`mov si,0x7d06` @`0xBB00`), which is numerically this value plus
/// the 6-byte header. One is a byte COUNT, the other a DS OFFSET, and nothing
/// decoded so far connects them. Recorded here so the equality is visible without
/// being asserted — matching numbers have twice been the start of a wrong
/// inference in this project (audit-fixes #114, #194).
pub const RESOURCE_READ_CHUNK: u16 = 0x7D00;

/// One directory slot: a 16-byte NUL-padded filename, the same record shape as
/// the world-art table's name field ([`WORLD_ART_RECORD`]).
///
/// The stride is the resolver's own arithmetic: `shl ax,4` @`0x3FD9` then
/// `add dx,ax` @`0x3FDC` turns a resource ID into its filename address. So 16 is
/// read off an instruction rather than inferred from the data's alignment.
pub const LEVEL_DIRECTORY_SLOT: usize = 16;

/// Read the resource directory OUT OF THE IMAGE rather than trusting the
/// transcription below.
///
/// [`LEVEL_DIRECTORY`] is a hand-copied prefix of this table, which makes it a
/// content-bearing literal — the defect class `CLAUDE.md` names first. It stays
/// for now because callers index it by resource ID, but
/// `level_directory_literal_matches_the_image` holds it to the bytes, and the
/// image has MORE entries than the literal does.
///
/// Slots are read until one is not a NUL-padded printable name, which is how the
/// table ends in the image.
pub fn parse_level_directory(image: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = LEVEL_DIRECTORY_FILE_OFFSET;
    while at + LEVEL_DIRECTORY_SLOT <= image.len() {
        let slot = &image[at..at + LEVEL_DIRECTORY_SLOT];
        let Some(end) = slot.iter().position(|&b| b == 0) else {
            break;
        };
        if end == 0
            || !slot[..end].iter().all(|b| b.is_ascii_graphic())
            || !slot[end..].iter().all(|&b| b == 0)
        {
            break;
        }
        out.push(String::from_utf8_lossy(&slot[..end]).into_owned());
        at += LEVEL_DIRECTORY_SLOT;
    }
    out
}

/// The runtime directory, parsed from the image once at startup.
static RUNTIME_DIRECTORY: std::sync::OnceLock<Vec<LevelEntry>> = std::sync::OnceLock::new();

/// The port's stem convention: `.spr` and `.ext` are stripped, every other
/// extension is kept (`nosound.drv`, `script1.cod` stay whole).
///
/// This is the PORT'S normalisation, not the game's — the table stores full
/// filenames. It exists because callers key worlds and sprites by bare stem, and
/// it is reproduced here exactly so the derived directory equals the transcribed
/// one over their common range (`derived_directory_reproduces_the_literal`).
fn level_stem(name: &str) -> &str {
    for ext in [".spr", ".ext"] {
        if let Some(cut) = name.strip_suffix(ext) {
            return cut;
        }
    }
    name
}

/// Install the resource directory parsed from `BLOODPRG.EXE`.
///
/// Call once with the image at startup. Until then — and in tests that never
/// load it — [`directory`] falls back to the transcribed [`LEVEL_DIRECTORY`],
/// which is a 53-entry PREFIX of the real 95.
///
/// The names are leaked deliberately: the directory lives for the process, and
/// leaking keeps `LevelEntry::stem` a `&'static str` so no caller signature
/// changes. A one-time allocation of 95 short strings is the whole cost.
pub fn init_level_directory(image: &[u8]) -> usize {
    let entries: Vec<LevelEntry> = parse_level_directory(image)
        .into_iter()
        .enumerate()
        .take(u8::MAX as usize)
        .map(|(index, name)| {
            let kind = if name.ends_with(".ext") {
                LevelKind::World
            } else if name.ends_with(".spr") {
                LevelKind::Sprite
            } else if name.starts_with("script") {
                LevelKind::Script
            } else {
                LevelKind::Resource
            };
            let stem: &'static str = Box::leak(level_stem(&name).to_owned().into_boxed_str());
            LevelEntry { index: index as u8, stem, kind }
        })
        .collect();
    let len = entries.len();
    let _ = RUNTIME_DIRECTORY.set(entries);
    len
}

/// The directory in force: the image-parsed one when [`init_level_directory`]
/// has run, else the transcribed prefix.
pub fn directory() -> &'static [LevelEntry] {
    RUNTIME_DIRECTORY
        .get()
        .map(Vec::as_slice)
        .unwrap_or(LEVEL_DIRECTORY)
}

/// One directory entry read from the IMAGE, with the kind inferred from the
/// filename's extension.
///
/// The kind inference is the PORT'S classification, not a field the table
/// carries — the game's table is filenames only. It is stated here so the
/// distinction stays visible: the NAMES are game data, the KINDS are ours.
pub fn level_entry_from_image(image: &[u8], index: u16) -> Option<(String, LevelKind)> {
    let names = parse_level_directory(image);
    let name = names.get(index as usize)?.clone();
    let kind = if name.ends_with(".ext") {
        LevelKind::World
    } else if name.ends_with(".spr") {
        LevelKind::Sprite
    } else if name.starts_with("script") {
        LevelKind::Script
    } else {
        LevelKind::Resource
    };
    Some((name, kind))
}

pub const LEVEL_DIRECTORY: &[LevelEntry] = &[
    LevelEntry { index: 0, stem: "fupcom", kind: LevelKind::Sprite },
    LevelEntry { index: 1, stem: "nosound.drv", kind: LevelKind::Resource },
    LevelEntry { index: 2, stem: "script1.cod", kind: LevelKind::Script },
    LevelEntry { index: 3, stem: "script1.bas", kind: LevelKind::Script },
    LevelEntry { index: 4, stem: "script1.var", kind: LevelKind::Script },
    LevelEntry { index: 5, stem: "script1.dic", kind: LevelKind::Script },
    LevelEntry { index: 6, stem: "script1.deb", kind: LevelKind::Script },
    LevelEntry { index: 7, stem: "radio", kind: LevelKind::Sprite },
    LevelEntry { index: 8, stem: "buffer", kind: LevelKind::Resource },
    LevelEntry { index: 9, stem: "buffer", kind: LevelKind::Resource },
    LevelEntry { index: 10, stem: "buffer", kind: LevelKind::Resource },
    LevelEntry { index: 11, stem: "buffer", kind: LevelKind::Resource },
    LevelEntry { index: 12, stem: "buffer", kind: LevelKind::Resource },
    LevelEntry { index: 13, stem: "bappel", kind: LevelKind::Sprite },
    LevelEntry { index: 14, stem: "bappel", kind: LevelKind::Sprite },
    LevelEntry { index: 15, stem: "btv", kind: LevelKind::Sprite },
    LevelEntry { index: 16, stem: "borxx", kind: LevelKind::Sprite },
    LevelEntry { index: 17, stem: "bcarte", kind: LevelKind::Sprite },
    LevelEntry { index: 18, stem: "bhyper", kind: LevelKind::Sprite },
    LevelEntry { index: 19, stem: "bpol", kind: LevelKind::Sprite },
    LevelEntry { index: 20, stem: "aphyper", kind: LevelKind::Sprite },
    LevelEntry { index: 21, stem: "appol", kind: LevelKind::Sprite },
    LevelEntry { index: 22, stem: "black", kind: LevelKind::World },
    LevelEntry { index: 23, stem: "kult", kind: LevelKind::World },
    LevelEntry { index: 24, stem: "rondo", kind: LevelKind::World },
    LevelEntry { index: 25, stem: "venusia", kind: LevelKind::World },
    LevelEntry { index: 26, stem: "erazor", kind: LevelKind::World },
    LevelEntry { index: 27, stem: "mastacho", kind: LevelKind::World },
    LevelEntry { index: 28, stem: "magnus", kind: LevelKind::World },
    LevelEntry { index: 29, stem: "ekatomb", kind: LevelKind::World },
    LevelEntry { index: 30, stem: "crazy", kind: LevelKind::World },
    LevelEntry { index: 31, stem: "eden", kind: LevelKind::World },
    LevelEntry { index: 32, stem: "kortex", kind: LevelKind::World },
    LevelEntry { index: 33, stem: "vista", kind: LevelKind::World },
    LevelEntry { index: 34, stem: "moskito", kind: LevelKind::World },
    LevelEntry { index: 35, stem: "pterra", kind: LevelKind::World },
    LevelEntry { index: 36, stem: "cyber", kind: LevelKind::World },
    LevelEntry { index: 37, stem: "script2.cod", kind: LevelKind::Script },
    LevelEntry { index: 38, stem: "script2.bas", kind: LevelKind::Script },
    LevelEntry { index: 39, stem: "script2.var", kind: LevelKind::Script },
    LevelEntry { index: 40, stem: "script2.dic", kind: LevelKind::Script },
    LevelEntry { index: 41, stem: "script2.deb", kind: LevelKind::Script },
    LevelEntry { index: 42, stem: "dnsdb.drv", kind: LevelKind::Resource },
    LevelEntry { index: 43, stem: "corpo", kind: LevelKind::World },
    LevelEntry { index: 44, stem: "carte", kind: LevelKind::Sprite },
    LevelEntry { index: 45, stem: "bigark", kind: LevelKind::World },
    LevelEntry { index: 46, stem: "cyber2", kind: LevelKind::World },
    LevelEntry { index: 47, stem: "cyber3", kind: LevelKind::World },
    LevelEntry { index: 48, stem: "eden2", kind: LevelKind::World },
    LevelEntry { index: 49, stem: "eden3", kind: LevelKind::World },
    LevelEntry { index: 50, stem: "ekatomb2", kind: LevelKind::World },
    LevelEntry { index: 51, stem: "ekatomb3", kind: LevelKind::World },
    LevelEntry { index: 52, stem: "erazor2", kind: LevelKind::World },
];

/// The primary navigable planet worlds — the distinct destinations shown on the nav
/// map (the top-level `.ext` worlds, excluding cyberspace levels and `2`/`3` sub-levels
/// which are entered from their parent world).
pub fn primary_worlds() -> impl Iterator<Item = &'static LevelEntry> {
    directory().iter().filter(|e| {
        e.kind == LevelKind::World
            && !e.stem.starts_with("cyber")
            && !e.stem.ends_with('2')
            && !e.stem.ends_with('3')
    })
}

/// Look up a directory entry by its resource ID.
pub fn entry(index: u8) -> Option<&'static LevelEntry> {
    directory().get(index as usize)
}

/// The resource ID the game loads a world by (its `FS:0x0c04` table index), given the
/// world stem (`"venusia"` → 25). This is the handle passed to the resource loader
/// (`vm_resource_profile_select` / `resource_handle_resolve`). Returns `None` for
/// unknown stems.
pub fn world_resource_id(stem: &str) -> Option<u8> {
    LEVEL_DIRECTORY
        .iter()
        .find(|e| e.kind == LevelKind::World && e.stem == stem)
        .map(|e| e.index)
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

/// The world's abbreviation used in its `fd/` art names, without the leading floor
/// digit — so it matches every floor of the world. Derived from
/// [`world_location_art_prefix`] by dropping a single leading digit (`1venus` →
/// `venus`, `kortex` → `kortex`).
pub fn world_location_abbrev(world: &str) -> Option<&'static str> {
    let prefix = world_location_art_prefix(world)?;
    Some(match prefix.strip_prefix(|c: char| c.is_ascii_digit()) {
        Some(rest) if !rest.is_empty() => rest,
        _ => prefix,
    })
}

/// Whether an `fd/` filename belongs to `world` on any floor: it starts with the world's
/// abbreviation, optionally preceded by a single floor digit (`1magnu…`, `2magnu…`).
pub fn art_belongs_to_world(filename: &str, world: &str) -> bool {
    let Some(abbrev) = world_location_abbrev(world) else {
        return false;
    };
    let f = filename;
    f.starts_with(abbrev)
        || (f.as_bytes().first().map(|b| b.is_ascii_digit()).unwrap_or(false)
            && f[1..].starts_with(abbrev))
}

/// The floor number of an `fd/` art filename (the leading digit, or 1 if none).
pub fn art_floor(filename: &str) -> u32 {
    filename
        .chars()
        .next()
        .and_then(|c| c.to_digit(10))
        .unwrap_or(1)
}

/// Parse a world's `fd/` room-art filename into `(room, view)`: after the world's
/// [`world_location_art_prefix`], the trailing letter is the view-angle (the direction
/// the player faces — `b`/`d`/`f`/`g`) and the leading part is the room id. E.g.
/// `1venus2f` → room `"2"`, view `'f'`; `kortex1b` → room `"1"`, view `'b'`. Returns
/// `None` if `filename` doesn't match the prefix.
pub fn parse_room_view(filename: &str, prefix: &str) -> Option<(String, char)> {
    let name = filename.strip_suffix(".lbm").unwrap_or(filename);
    let rest = name.strip_prefix(prefix)?;
    let view = rest.chars().last()?;
    if !view.is_ascii_alphabetic() {
        return None;
    }
    let room = rest[..rest.len() - view.len_utf8()].to_string();
    Some((room, view))
}

/// One row of the WORLD-ARTWORK table at `DS:0x2BC7` (file `0xFFE7`): a 22-byte
/// record whose `+0x00` is a 16-byte NUL-terminated display name and whose
/// `+0x10` is a RESOURCE ID into the same filename table [`LEVEL_DIRECTORY`]
/// mirrors (file `0xCDF4`). `+0x12` is a group word, `31` for every entry;
/// `+0x14` is zero throughout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldArtEntry {
    pub name: &'static str,
    pub resource_id: u16,
}

/// Byte size of one `DS:0x2BC7` record — 22, and the LAYOUT says why: a 16-byte
/// NUL-padded name followed by three `u16` fields (id, group, extra).
///
/// Not an immediate: the stride is the record's shape. `Kortex` sits at `+0`,
/// `Kukaracha` at `+0x16`, `Ekatomb` at `+0x2C`, so a wrong stride would land
/// mid-name immediately — which is what
/// `world_art_records_are_name_plus_three_words` checks against the image.
pub const WORLD_ART_RECORD: usize = 0x16;
/// File offset of the table (`DS:0x2BC7`, DS base `0xD420`).
pub const WORLD_ART_TABLE_FILE_OFFSET: usize = 0xFFE7;

/// The game's own object-name -> artwork lookup, read by the info panel's first
/// zoom frame: it walks these records comparing each name against the selected
/// object's inline name (`0x9098..0x90B6`, the string compare at `0x1CE:0x2C4`),
/// then loads `[si+0x10] | 0x8000` as a resource (`0x90B8..0x90C3`).
///
/// The table is why the display names cannot be guessed from the asset names:
/// `Oddland` is `trou.ext`, `Bonus` is `forest.ext`, `Troma` is `glacia.ext`,
/// and `Trashlando` shares `kortex.ext` with `Kortex`. Pinned to the image
/// byte-for-byte by `world_art_directory_matches_the_ds2bc7_table`, which also
/// checks every id resolves to a real filename record.
pub const WORLD_ART_DIRECTORY: [WorldArtEntry; 42] = [
    WorldArtEntry { name: "Kortex",      resource_id: 32 },
    WorldArtEntry { name: "Kukaracha",   resource_id: 75 },
    WorldArtEntry { name: "Ekatomb",     resource_id: 29 },
    WorldArtEntry { name: "Shark",       resource_id: 92 },
    WorldArtEntry { name: "Cyberock",    resource_id: 36 },
    WorldArtEntry { name: "Mastachok",   resource_id: 27 },
    WorldArtEntry { name: "Crazystone",  resource_id: 30 },
    WorldArtEntry { name: "Rondo",       resource_id: 24 },
    WorldArtEntry { name: "Venusia",     resource_id: 25 },
    WorldArtEntry { name: "Vista",       resource_id: 33 },
    WorldArtEntry { name: "Eden",        resource_id: 31 },
    WorldArtEntry { name: "Qx20",        resource_id: 64 },
    WorldArtEntry { name: "Corpo",       resource_id: 43 },
    WorldArtEntry { name: "Pterra",      resource_id: 35 },
    WorldArtEntry { name: "Erazor",      resource_id: 26 },
    WorldArtEntry { name: "Magnus",      resource_id: 28 },
    WorldArtEntry { name: "Ondoya",      resource_id: 94 },
    WorldArtEntry { name: "Tumul",       resource_id: 74 },
    WorldArtEntry { name: "Malus",       resource_id: 59 },
    WorldArtEntry { name: "Bonus",       resource_id: 54 },
    WorldArtEntry { name: "Kult",        resource_id: 23 },
    WorldArtEntry { name: "Troma",       resource_id: 55 },
    WorldArtEntry { name: "Attrox",      resource_id: 56 },
    WorldArtEntry { name: "Trashlando",  resource_id: 32 },
    WorldArtEntry { name: "Moskito",     resource_id: 34 },
    WorldArtEntry { name: "Oddland",     resource_id: 72 },
    WorldArtEntry { name: "Ekato",       resource_id: 50 },
    WorldArtEntry { name: "Erazo",       resource_id: 52 },
    WorldArtEntry { name: "Masta",       resource_id: 61 },
    WorldArtEntry { name: "Ron",         resource_id: 65 },
    WorldArtEntry { name: "Venusia2",    resource_id: 69 },
    WorldArtEntry { name: "Vistar",      resource_id: 70 },
    WorldArtEntry { name: "Edena",       resource_id: 48 },
    WorldArtEntry { name: "Golgos",      resource_id: 62 },
    WorldArtEntry { name: "Lovia",       resource_id: 63 },
    WorldArtEntry { name: "Sat",         resource_id: 67 },
    WorldArtEntry { name: "Tempest",     resource_id: 68 },
    WorldArtEntry { name: "Vulcan",      resource_id: 71 },
    WorldArtEntry { name: "Magnu",       resource_id: 58 },
    WorldArtEntry { name: "Kraner",      resource_id: 73 },
    WorldArtEntry { name: "Cyborg",      resource_id: 60 },
    WorldArtEntry { name: "Bigbang",     resource_id: 91 },
];

/// The artwork resource id for an object's inline NAME, matched the way the
/// engine matches it (`0x1CE:0x2C4` is case-insensitive, like the built-in
/// object-name scan at `0x5486`).
pub fn world_art_resource_id(name: &str) -> Option<u16> {
    WORLD_ART_DIRECTORY
        .iter()
        .find(|e| e.name.eq_ignore_ascii_case(name))
        .map(|e| e.resource_id)
}

/// Parse the table straight out of a `BLOODPRG.EXE` image, so a checkout with
/// the binary never has to trust the transcription above. Stops at the first
/// record whose name byte is zero (`cmp byte [si],0 / je` @`0x90A7`).
pub fn parse_world_art_table(exe: &[u8]) -> Vec<(String, u16)> {
    let mut out = Vec::new();
    let mut index = 0;
    loop {
        let base = WORLD_ART_TABLE_FILE_OFFSET + index * WORLD_ART_RECORD;
        let Some(rec) = exe.get(base..base + WORLD_ART_RECORD) else {
            break;
        };
        if rec[0] == 0 {
            break;
        }
        let name = rec[..16].split(|&b| b == 0).next().unwrap_or_default();
        let id = u16::from_le_bytes([rec[0x10], rec[0x11]]);
        out.push((String::from_utf8_lossy(name).into_owned(), id));
        index += 1;
    }
    out
}

#[cfg(test)]
mod tests {

    /// The derived directory must reproduce the transcribed one over their common
    /// range — same stems, same kinds, same indices. If it does, the literal adds
    /// nothing and the parse can replace it; if it does not, one of them is wrong.
    #[test]
    fn derived_directory_reproduces_the_literal() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        let count = init_level_directory(&exe);
        assert_eq!(count, 95, "the whole table is installed");

        let live = directory();
        assert_eq!(live.len(), 95, "accessors now see all 95 slots");
        for want in LEVEL_DIRECTORY {
            let got = &live[want.index as usize];
            assert_eq!(got.stem, want.stem, "slot {} stem", want.index);
            assert_eq!(got.kind, want.kind, "slot {} kind", want.index);
            assert_eq!(got.index, want.index);
        }

        // And the worlds the literal never had are now reachable by ID.
        assert_eq!(entry(54).map(|e| e.stem), Some("forest"));
        assert_eq!(entry(76).map(|e| e.stem), Some("script3.cod"));
        assert_eq!(entry(94).map(|e| e.kind), Some(LevelKind::World));

        // The stem rule is the port's, and it is applied only to .spr/.ext.
        assert_eq!(level_stem("fupcom.spr"), "fupcom");
        assert_eq!(level_stem("black.ext"), "black");
        assert_eq!(level_stem("nosound.drv"), "nosound.drv");
        assert_eq!(level_stem("script1.cod"), "script1.cod");
    }

    /// `0x51A5`/`0x51AC`/`0x51B3`: the descriptor stride and the residency test,
    /// checked as instruction BYTES so the constants cannot drift.
    #[test]
    fn resource_descriptor_layout_matches_the_loader() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        // c1 e3 03 = shl bx,3 -- the stride is 1 << 3.
        assert_eq!(&exe[0x51A5..0x51A8], &[0xC1, 0xE3, 0x03]);
        assert_eq!(1u16 << exe[0x51A7], RESOURCE_DESCRIPTOR_STRIDE);
        // f7 47 02 03 00 = test word [bx+2],3 -- offset AND mask in one check.
        assert_eq!(&exe[0x51AC..0x51B1], &[0xF7, 0x47, 0x02, 0x03, 0x00]);
        assert_eq!(exe[0x51AE] as u16, RESOURCE_DESCRIPTOR_FLAGS);
        assert_eq!(exe[0x51AF] as u16, RESOURCE_FLAG_RESIDENT);
        // 83 4f 02 02 = or word [bx+2],2
        assert_eq!(&exe[0x51B3..0x51B7], &[0x83, 0x4F, 0x02, 0x02]);
        assert_eq!(exe[0x51B6] as u16, RESOURCE_FLAG_IN_USE);

        // The two tables coexist: 95 descriptors end well before the names.
        let names = parse_level_directory(&exe);
        assert_eq!(names.len(), 95);
        assert!(
            resource_descriptor_offset(names.len() as u16) < 0xC04,
            "descriptors must not reach the name table at FS:0x0C04"
        );
        // The drivers are addressable by ID through both tables.
        assert_eq!(names[1], "nosound.drv");
        assert_eq!(resource_descriptor_offset(1), 8);
    }

    /// `0x4041 mov cx,0x7d00` — the read chunk size, checked as BYTES in the
    /// image so a doc edit cannot drift from the instruction.
    #[test]
    fn resource_read_chunk_matches_the_loader_instruction() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        // b9 imm16 = mov cx,imm16
        assert_eq!(exe[0x4041], 0xB9, "0x4041 is not a mov cx,imm16");
        let imm = u16::from_le_bytes([exe[0x4042], exe[0x4043]]);
        assert_eq!(imm, RESOURCE_READ_CHUNK);
        // And the read that consumes it really is AH=3Fh.
        assert_eq!(&exe[0x4046..0x404B], &[0xB8, 0x00, 0x3F, 0xCD, 0x21]);
    }

    /// The image carries resources the transcribed literal never had — including
    /// the whole script3/4/5 sets the frontend already loads by name.
    #[test]
    fn level_directory_image_has_entries_the_literal_lacks() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        let names = parse_level_directory(&exe);
        assert_eq!(names.len(), 95, "the shipped directory has 95 slots");
        assert!(
            names.len() > LEVEL_DIRECTORY.len(),
            "the literal is a PREFIX, not the table"
        );

        // The literal stops at 54; these are real slots past it.
        assert_eq!(
            level_entry_from_image(&exe, 76),
            Some(("script3.cod".to_string(), LevelKind::Script))
        );
        assert_eq!(
            level_entry_from_image(&exe, 86),
            Some(("script5.cod".to_string(), LevelKind::Script))
        );
        assert_eq!(
            level_entry_from_image(&exe, 94),
            Some(("ondoya.ext".to_string(), LevelKind::World))
        );
        assert_eq!(level_entry_from_image(&exe, 95), None, "past the table");

        // Every slot the literal DOES cover keeps agreeing (belt and braces with
        // level_directory_literal_matches_the_image).
        assert_eq!(names[1], "nosound.drv");
        // The literal holds indices 0..=52, so 53 is the first omitted slot.
        assert_eq!(LEVEL_DIRECTORY.len(), 53, "the literal's size, checked not assumed");
        assert_eq!(names[53], "erazor3.ext", "the first slot the literal omits");
        assert_eq!(names.len() - LEVEL_DIRECTORY.len(), 42, "entries never transcribed");
    }

    /// The transcribed [`LEVEL_DIRECTORY`] against the bytes it was copied from.
    /// A literal that restates game data is only as good as the copy.
    #[test]
    fn level_directory_literal_matches_the_image() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        let names = parse_level_directory(&exe);
        assert!(names.len() >= LEVEL_DIRECTORY.len(), "image has at least the literal");

        for entry in LEVEL_DIRECTORY {
            let actual = &names[entry.index as usize];
            // The literal stores some names stem-only ("fupcom" for "fupcom.spr").
            let agrees = actual == entry.stem
                || actual.split('.').next() == Some(entry.stem)
                || actual.starts_with(entry.stem);
            assert!(
                agrees,
                "slot {} is {actual:?} in the image but {:?} in the literal",
                entry.index, entry.stem
            );
        }
    }

    /// The world-art record stride is `16-byte name + 3 words` = 22. Checked by
    /// walking the table at the claimed stride and requiring every record to start
    /// with a printable, NUL-terminated name — a wrong stride lands mid-name and
    /// fails on the first or second record.
    #[test]
    fn world_art_records_are_name_plus_three_words() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        assert_eq!(WORLD_ART_RECORD, 16 + 3 * 2, "name field plus three u16s");

        let base = WORLD_ART_TABLE_FILE_OFFSET;
        for index in 0..8 {
            let rec = &exe[base + index * WORLD_ART_RECORD..][..WORLD_ART_RECORD];
            let name_len = rec[..16].iter().position(|&b| b == 0).expect("NUL-padded");
            assert!(name_len > 0, "record {index} has a name");
            assert!(
                rec[..name_len].iter().all(|b| b.is_ascii_graphic()),
                "record {index} name is printable: {:?}",
                &rec[..name_len]
            );
            // The padding really is padding, not the next field bleeding in.
            assert!(rec[name_len..16].iter().all(|&b| b == 0), "record {index} padding");
        }
    }
    use super::*;

    /// Every `LEVEL_DIRECTORY` entry must match the game's resource name table compiled into
    /// BLOODPRG.EXE (file 0xCDF4 = FS:0x0c04, 16-byte filename records indexed by resource id),
    /// byte-for-byte at the record's stem. Skips if the exe isn't in this checkout.
    #[test]
    fn level_directory_matches_bloodprg_resource_table() {
        let exe = match std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        {
            Ok(b) => b,
            Err(_) => return,
        };
        const BASE: usize = 0xCDF4;
        let mut checked = 0;
        for e in LEVEL_DIRECTORY {
            let rec = &exe[BASE + e.index as usize * 16..BASE + e.index as usize * 16 + 16];
            let exe_name = rec.split(|&b| b == 0).next().unwrap();
            let exe_str = std::str::from_utf8(exe_name).unwrap().to_ascii_lowercase();
            let stem = e.stem.to_ascii_lowercase();
            // sprite/world stems drop the extension; compare the leading filename stem.
            let exe_stem = exe_str.split('.').next().unwrap();
            let eng_stem = stem.split('.').next().unwrap();
            assert_eq!(
                exe_stem, eng_stem,
                "id {}: engine {:?} vs exe {:?}",
                e.index, e.stem, exe_str
            );
            checked += 1;
        }
        assert_eq!(checked, LEVEL_DIRECTORY.len());
        assert_eq!(checked, 53, "expected 53 resource entries");
        // The TABLE is larger than the port's directory and its extent is fixed by
        // a layout identity: 0xCDF4 + 95*16 = 0xD3E4, exactly the script-profile
        // table. So there are 95 records (ids 0..94), which is also the highest id
        // the world-art table uses (94 = ondoya.ext). The port's 53 entries are the
        // subset it needs, not the whole table -- worth pinning so the two numbers
        // are never confused.
        const NAME_TABLE: usize = 0xCDF4;
        const NAME_COUNT: usize = 95;
        assert_eq!(NAME_TABLE + NAME_COUNT * 16, 0x0D3E4);
        let highest_art = WORLD_ART_DIRECTORY
            .iter()
            .map(|e| e.resource_id)
            .max()
            .unwrap();
        assert_eq!(highest_art as usize, NAME_COUNT - 1);
        assert!(
            LEVEL_DIRECTORY.iter().all(|e| (e.index as usize) < NAME_COUNT),
            "every ported entry indexes inside the table"
        );
    }

    #[test]
    fn world_art_directory_matches_the_ds2bc7_table() {
        let exe = match std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        {
            Ok(b) => b,
            Err(_) => return,
        };
        let parsed = parse_world_art_table(&exe);
        assert_eq!(
            parsed.len(),
            WORLD_ART_DIRECTORY.len(),
            "the table's terminator (first zero name byte) bounds it at 42 records"
        );
        for (entry, (name, id)) in WORLD_ART_DIRECTORY.iter().zip(parsed.iter()) {
            assert_eq!((entry.name, entry.resource_id), (name.as_str(), *id));
        }
        // LAYOUT IDENTITY: 42 records of 0x16 bytes from DS:0x2BC7 end at
        // DS:0x2F63, two bytes before the nav camera origin at DS:0x2F65.
        assert_eq!(
            0x2BC7 + WORLD_ART_DIRECTORY.len() * WORLD_ART_RECORD,
            0x2F63
        );
        // Every artwork id must resolve to a real filename record (file 0xCDF4).
        for entry in WORLD_ART_DIRECTORY {
            let off = 0xCDF4 + entry.resource_id as usize * 16;
            let name = exe[off..off + 16].split(|&b| b == 0).next().unwrap();
            assert!(
                !name.is_empty(),
                "{} -> id {} has no filename record",
                entry.name,
                entry.resource_id
            );
        }
        // The names the display uses are NOT the asset names — the reason this
        // table has to be read rather than inferred.
        assert_eq!(world_art_resource_id("Oddland"), Some(72)); // trou.ext
        assert_eq!(world_art_resource_id("Bonus"), Some(54)); // forest.ext
        assert_eq!(world_art_resource_id("troma"), Some(55)); // glacia.ext, case-insensitive
        assert_eq!(world_art_resource_id("nowhere"), None);
    }

    #[test]
    fn world_resource_ids_match_the_fs0c04_table() {
        // The resource IDs the game loads worlds by (verified vs the FS:0x0c04 table).
        assert_eq!(world_resource_id("black"), Some(22));
        assert_eq!(world_resource_id("venusia"), Some(25));
        assert_eq!(world_resource_id("magnus"), Some(28));
        assert_eq!(world_resource_id("cyber"), Some(36));
        assert_eq!(world_resource_id("corpo"), Some(43));
        // Sprites/scripts aren't "worlds".
        assert_eq!(world_resource_id("bcarte"), None);
        assert_eq!(world_resource_id("nope"), None);
        // Round-trips: entry(id).stem == the queried world.
        for w in ["venusia", "magnus", "cyber"] {
            let id = world_resource_id(w).unwrap();
            assert_eq!(entry(id).unwrap().stem, w);
        }
    }

    #[test]
    fn art_matches_a_world_across_all_floors() {
        assert_eq!(world_location_abbrev("magnus"), Some("magnu"));
        assert_eq!(world_location_abbrev("kortex"), Some("kortex"));
        // Floor 1 and floor 2 art both belong to magnus.
        assert!(art_belongs_to_world("1magnu1f.lbm", "magnus"));
        assert!(art_belongs_to_world("2magnu1b.lbm", "magnus"));
        assert_eq!(art_floor("1magnu1f.lbm"), 1);
        assert_eq!(art_floor("2magnu1b.lbm"), 2);
        // A different world's art doesn't match.
        assert!(!art_belongs_to_world("1venus1f.lbm", "magnus"));
    }

    #[test]
    fn parses_room_and_view_from_art_filenames() {
        assert_eq!(
            parse_room_view("1venus2f.lbm", "1venus"),
            Some(("2".to_string(), 'f'))
        );
        assert_eq!(
            parse_room_view("1ekato1b.lbm", "1ekato"),
            Some(("1".to_string(), 'b'))
        );
        assert_eq!(
            parse_room_view("kortex1g.lbm", "kortex"),
            Some(("1".to_string(), 'g'))
        );
        // Wrong prefix -> None.
        assert_eq!(parse_room_view("1magnu1f.lbm", "1venus"), None);
    }

    #[test]
    fn directory_indices_are_dense_and_ordered() {
        for (i, e) in LEVEL_DIRECTORY.iter().enumerate() {
            assert_eq!(e.index as usize, i, "entry {i} index matches position");
        }
        // Resource IDs match the FS:0x0c04 table: script1 set at 2..6, worlds at 22+.
        assert_eq!(entry(2).unwrap().stem, "script1.cod");
        assert_eq!(entry(22).unwrap().stem, "black");
        assert_eq!(entry(25).unwrap().stem, "venusia");
        // cyberspace is resource ID 36, its extra levels 46/47.
        assert_eq!(entry(36).unwrap().stem, "cyber");
        assert_eq!(entry(46).unwrap().stem, "cyber2");
        assert_eq!(entry(47).unwrap().stem, "cyber3");
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
        // THE COUNT DEPENDS ON WHICH DIRECTORY IS IN FORCE, so this asserts against
        // the directory rather than a remembered number. The transcribed prefix
        // yields 16 top-level worlds; the image's full 95-slot table yields 32,
        // because 16 more `.ext` worlds were never transcribed (audit-fixes #203).
        //
        // Asserting a bare 16 made this test ORDER-DEPENDENT once
        // `init_level_directory` existed: any earlier test installing the real
        // table changed the answer. Keying off `directory().len()` removes the
        // dependence instead of hiding it behind a bigger constant.
        let expected = if directory().len() > LEVEL_DIRECTORY.len() { 32 } else { 16 };
        assert_eq!(names.len(), expected, "directory has {} slots", directory().len());

        // Either way the filter's shape holds: every survivor is a World whose
        // stem is neither cyberspace nor a numbered sub-level.
        for stem in &names {
            assert!(!stem.starts_with("cyber"));
            assert!(!stem.ends_with('2') && !stem.ends_with('3'));
        }
    }
}
