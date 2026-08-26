//! Checked overlap-safe moves within flat byte storage.

use std::{fmt, ops::Range};

/// Invalid source or destination range supplied to an in-place byte move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteMoveError {
    /// The source range is reversed or extends beyond the owned byte storage.
    SourceOutsideStorage {
        /// First requested source byte.
        start: usize,
        /// Exclusive requested source end.
        end: usize,
        /// Number of available bytes.
        storage_len: usize,
    },
    /// The destination range overflows or extends beyond the owned byte storage.
    DestinationOutsideStorage {
        /// First requested destination byte.
        start: usize,
        /// Number of bytes requested.
        byte_count: usize,
        /// Number of available bytes.
        storage_len: usize,
    },
}

impl fmt::Display for ByteMoveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ByteMoveError {}

/// Move an overlap-safe byte range within one owned flat allocation.
///
/// This is the flat-memory replacement for `far_memmove` at BLOODPRG routine
/// offset `0x002E73`. Rust's slice move retains the routine's observable copy
/// semantics while making its pointer normalization, copy-direction flag, and
/// 64,000-byte transfer chunks unnecessary. Invalid ranges are rejected before
/// storage is mutated instead of wrapping through an unrelated region.
pub fn move_bytes_in_place(
    storage: &mut [u8],
    source: Range<usize>,
    destination_start: usize,
) -> Result<(), ByteMoveError> {
    let byte_count =
        source
            .end
            .checked_sub(source.start)
            .ok_or(ByteMoveError::SourceOutsideStorage {
                start: source.start,
                end: source.end,
                storage_len: storage.len(),
            })?;
    if source.end > storage.len() {
        return Err(ByteMoveError::SourceOutsideStorage {
            start: source.start,
            end: source.end,
            storage_len: storage.len(),
        });
    }
    if destination_start
        .checked_add(byte_count)
        .is_none_or(|end| end > storage.len())
    {
        return Err(ByteMoveError::DestinationOutsideStorage {
            start: destination_start,
            byte_count,
            storage_len: storage.len(),
        });
    }

    storage.copy_within(source, destination_start);
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 10;
    const TEST_STORAGE_LEN: usize = 1_048_576;

    #[derive(Deserialize)]
    struct MoveOracle {
        name: String,
        source_linear: usize,
        destination_linear: usize,
        byte_count: u64,
    }

    #[test]
    fn flat_moves_account_for_every_original_overlap_and_chunk_vector() {
        let vectors: Vec<MoveOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_2e73_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let mut storage: Vec<u8> = (0..TEST_STORAGE_LEN)
                .map(|index| (index * 37 + case_index * 29 + 11) as u8)
                .collect();
            let before = storage.clone();
            let byte_count = usize::try_from(vector.byte_count).unwrap();
            let source_end = vector.source_linear.checked_add(byte_count);
            let destination_end = vector.destination_linear.checked_add(byte_count);
            let valid = source_end.is_some_and(|end| end <= storage.len())
                && destination_end.is_some_and(|end| end <= storage.len());

            if valid {
                let source = vector.source_linear..source_end.unwrap();
                let mut expected = storage.clone();
                expected.copy_within(source.clone(), vector.destination_linear);
                move_bytes_in_place(&mut storage, source, vector.destination_linear)
                    .unwrap_or_else(|error| panic!("{}: {error}", vector.name));
                assert_eq!(storage, expected, "{}", vector.name);
            } else {
                let error = move_bytes_in_place(
                    &mut storage,
                    vector.source_linear..source_end.unwrap_or(usize::MAX),
                    vector.destination_linear,
                )
                .unwrap_err();
                assert!(
                    matches!(
                        error,
                        ByteMoveError::SourceOutsideStorage { .. }
                            | ByteMoveError::DestinationOutsideStorage { .. }
                    ),
                    "{}: {error}",
                    vector.name
                );
                assert_eq!(storage, before, "{}", vector.name);
            }
        }
    }

    #[test]
    fn reversed_source_is_rejected_without_mutation() {
        let mut storage = [1, 2, 3, 4];
        let before = storage;
        let reversed_source = Range { start: 3, end: 1 };
        assert_eq!(
            move_bytes_in_place(&mut storage, reversed_source, 0),
            Err(ByteMoveError::SourceOutsideStorage {
                start: 3,
                end: 1,
                storage_len: 4,
            })
        );
        assert_eq!(storage, before);
    }
}
