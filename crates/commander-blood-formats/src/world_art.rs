//! World-artwork layout records embedded in `BLOODPRG.EXE`.

const WORLD_ARTWORK_TABLE_FILE_OFFSET: usize = 0xFFE7;
const WORLD_ARTWORK_ENTRY_COUNT: usize = 42;
const WORLD_ARTWORK_ENTRY_SIZE: usize = 22;
const WORLD_ARTWORK_NAME_CAPACITY: usize = 16;
const RESOURCE_ID_OFFSET: usize = 16;
const ENTITY_ID_OFFSET: usize = 18;
const ACTIVE_OFFSET: usize = 20;
const WORD_SIZE: usize = 2;

/// One decoded row of the native world-artwork layout table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldArtworkLayout {
    name: Box<[u8]>,
    /// Resource-table identity loaded when this row is selected.
    pub resource_id: u16,
    /// Render entity configured for the loaded artwork.
    pub entity_id: u16,
    /// Mutable selection flag cleared and assigned by the ship-view runtime.
    pub active: bool,
}

impl WorldArtworkLayout {
    /// Construct one bounded layout row for runtime-owned tables.
    pub fn new(
        name: impl Into<Box<[u8]>>,
        resource_id: u16,
        entity_id: u16,
        active: bool,
    ) -> Option<Self> {
        let name = name.into();
        if name.is_empty() || name.len() > WORLD_ARTWORK_NAME_CAPACITY || name.contains(&u8::MIN) {
            return None;
        }
        Some(Self {
            name,
            resource_id,
            entity_id,
            active,
        })
    }

    /// Return the exact display-name bytes used for object matching.
    pub fn name(&self) -> &[u8] {
        &self.name
    }
}

/// Decode the complete terminated world-artwork layout from the executable.
pub fn decode_bloodprg_world_artwork_layout(
    executable: &[u8],
) -> Option<Box<[WorldArtworkLayout]>> {
    decode_world_artwork_layout(executable, WORLD_ARTWORK_TABLE_FILE_OFFSET)
}

/// Decode the sequel table traversed in 22-byte steps at file 0x801B.
pub fn decode_blood2pg_world_artwork_layout(
    executable: &[u8],
) -> Option<Box<[WorldArtworkLayout]>> {
    decode_world_artwork_layout(executable, 0x12787)
}

fn decode_world_artwork_layout(
    executable: &[u8],
    table_file_offset: usize,
) -> Option<Box<[WorldArtworkLayout]>> {
    let table_size = WORLD_ARTWORK_ENTRY_COUNT.checked_mul(WORLD_ARTWORK_ENTRY_SIZE)?;
    let table_end = table_file_offset.checked_add(table_size)?;
    let table = executable.get(table_file_offset..table_end)?;
    let terminator = executable.get(table_end..table_end + WORD_SIZE)?;
    if terminator != [u8::MIN; WORD_SIZE] {
        return None;
    }

    let mut layouts = Vec::with_capacity(WORLD_ARTWORK_ENTRY_COUNT);
    for record in table.chunks_exact(WORLD_ARTWORK_ENTRY_SIZE) {
        let name_field = &record[..WORLD_ARTWORK_NAME_CAPACITY];
        let name_length = name_field
            .iter()
            .position(|byte| *byte == u8::MIN)
            .unwrap_or(WORLD_ARTWORK_NAME_CAPACITY);
        if name_length == usize::MIN {
            return None;
        }
        layouts.push(WorldArtworkLayout::new(
            Box::<[u8]>::from(&name_field[..name_length]),
            read_word(record, RESOURCE_ID_OFFSET),
            read_word(record, ENTITY_ID_OFFSET),
            record[ACTIVE_OFFSET] != u8::MIN,
        )?);
    }
    Some(layouts.into_boxed_slice())
}

fn read_word(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        data[offset..offset + WORD_SIZE]
            .try_into()
            .expect("fixed world-artwork record field"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHIPPED_ENTITY_ID: u16 = 31;
    const PTERRA_RESOURCE_ID: u16 = 35;

    #[test]
    fn shipped_world_artwork_table_has_all_named_rows() {
        let executable = include_bytes!("../../../re/bin/BLOODPRG.EXE");
        let layouts = decode_bloodprg_world_artwork_layout(executable).unwrap();

        assert_eq!(layouts.len(), WORLD_ARTWORK_ENTRY_COUNT);
        assert!(layouts.iter().all(|layout| !layout.active));
        assert!(
            layouts
                .iter()
                .all(|layout| layout.entity_id == SHIPPED_ENTITY_ID)
        );
        let pterra = layouts
            .iter()
            .find(|layout| layout.name() == b"Pterra")
            .unwrap();
        assert_eq!(pterra.resource_id, PTERRA_RESOURCE_ID);
    }

    #[test]
    fn world_artwork_decoder_requires_the_table_terminator() {
        let mut executable = include_bytes!("../../../re/bin/BLOODPRG.EXE").to_vec();
        let terminator =
            WORLD_ARTWORK_TABLE_FILE_OFFSET + WORLD_ARTWORK_ENTRY_COUNT * WORLD_ARTWORK_ENTRY_SIZE;
        executable[terminator] = u8::MAX;
        assert!(decode_bloodprg_world_artwork_layout(&executable).is_none());
    }
}
