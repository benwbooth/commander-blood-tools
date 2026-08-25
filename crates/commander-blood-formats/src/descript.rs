//! Typed structures for the DESCRIPT scene and dialogue database.

/// Semantic kind byte stored immediately before each DESCRIPT record length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DescriptRecordKind {
    /// Planet or local-place presentation record.
    Location = 1,
    /// Character conversation record.
    Character = 2,
    /// Standalone video sequence record.
    Sequence = 4,
    /// Inventory or world-object presentation record.
    Object = 15,
}

impl DescriptRecordKind {
    /// Decode one shipped record-kind byte.
    pub const fn decode(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Location),
            2 => Some(Self::Character),
            4 => Some(Self::Sequence),
            15 => Some(Self::Object),
            _ => None,
        }
    }

    /// Return the exact serialized kind byte.
    pub const fn encode(self) -> u8 {
        self as u8
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    const DIRECTORY_COUNT_SIZE: usize = 2;
    const DIRECTORY_ENTRY_SIZE: usize = 18;
    const DIRECTORY_NAME_SIZE: usize = 16;
    const EXPECTED_RECORD_COUNTS: [(DescriptRecordKind, usize); 4] = [
        (DescriptRecordKind::Location, 64),
        (DescriptRecordKind::Character, 35),
        (DescriptRecordKind::Sequence, 11),
        (DescriptRecordKind::Object, 35),
    ];

    fn original_asset() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood/DESCRIPT.DES")
    }

    #[test]
    fn every_shipped_directory_offset_has_a_known_preceding_kind() {
        let data = std::fs::read(original_asset()).unwrap();
        let count = usize::from(u16::from_le_bytes(data[..2].try_into().unwrap()));
        let mut counts = EXPECTED_RECORD_COUNTS.map(|(kind, _count)| (kind, 0));

        for index in 0..count {
            let entry = DIRECTORY_COUNT_SIZE + index * DIRECTORY_ENTRY_SIZE;
            let offset = usize::from(u16::from_le_bytes(
                data[entry + DIRECTORY_NAME_SIZE..entry + DIRECTORY_ENTRY_SIZE]
                    .try_into()
                    .unwrap(),
            ));
            let kind = DescriptRecordKind::decode(data[offset - 1]).unwrap();
            counts
                .iter_mut()
                .find(|(candidate, _count)| *candidate == kind)
                .unwrap()
                .1 += 1;
        }

        assert_eq!(counts, EXPECTED_RECORD_COUNTS);
    }
}
