//! Runtime state produced while applying typed DESCRIPT records.

use commander_blood_formats::descript::DescriptRecordKind;

/// Boundary detected after the current DESCRIPT command stream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DescriptRecordBoundary {
    next_record_kind: Option<DescriptRecordKind>,
}

impl DescriptRecordBoundary {
    /// Return whether execution must stop before the next record.
    pub const fn should_stop(self) -> bool {
        self.next_record_kind.is_some()
    }

    /// Return the kind byte that begins the following directory record.
    pub const fn next_record_kind(self) -> Option<DescriptRecordKind> {
        self.next_record_kind
    }

    fn stop_before(&mut self, kind: DescriptRecordKind) {
        self.next_record_kind = Some(kind);
    }
}

/// Stop the current stream before a following Location record.
///
/// This translates `byte_parser_op_01_mark_b16` at BLOODPRG file offset
/// `0x007542`. The native Boolean marker becomes an explicit record kind.
pub fn stop_before_location_record(boundary: &mut DescriptRecordBoundary) {
    boundary.stop_before(DescriptRecordKind::Location);
}

/// Stop the current stream before a following Character record.
///
/// This translates `byte_parser_op_02_mark_b16` at BLOODPRG file offset
/// `0x007549`.
pub fn stop_before_character_record(boundary: &mut DescriptRecordBoundary) {
    boundary.stop_before(DescriptRecordKind::Character);
}

/// Stop the current stream before a following Object record.
///
/// This translates `byte_parser_op_0f_mark_b16` at BLOODPRG file offset
/// `0x007550`.
pub fn stop_before_object_record(boundary: &mut DescriptRecordBoundary) {
    boundary.stop_before(DescriptRecordKind::Object);
}

/// Stop the current stream before a following Sequence record.
///
/// This translates `byte_parser_op_04_mark_b16` at BLOODPRG file offset
/// `0x007557`.
pub fn stop_before_sequence_record(boundary: &mut DescriptRecordBoundary) {
    boundary.stop_before(DescriptRecordKind::Sequence);
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 2;

    #[derive(Deserialize)]
    struct StopOracle {
        name: String,
        flag_after: u8,
    }

    fn assert_stop_handler(
        input: &str,
        expected_kind: DescriptRecordKind,
        handler: fn(&mut DescriptRecordBoundary),
    ) {
        let vectors: Vec<StopOracle> = serde_json::from_str(input).unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut boundary = DescriptRecordBoundary::default();
            match vector.name.as_str() {
                "already_set" => handler(&mut boundary),
                "overwrite_marker" if expected_kind == DescriptRecordKind::Location => {
                    stop_before_sequence_record(&mut boundary);
                }
                "overwrite_marker" => stop_before_location_record(&mut boundary),
                name => panic!("unknown DESCRIPT stop oracle {name}"),
            }

            handler(&mut boundary);
            assert_eq!(vector.flag_after, 1, "{}", vector.name);
            assert!(boundary.should_stop(), "{}", vector.name);
            assert_eq!(
                boundary.next_record_kind(),
                Some(expected_kind),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn location_boundary_matches_original_marker_writes() {
        assert_stop_handler(
            include_str!("../../../../../re/tools/oracle_vectors/func_7542_natural.json"),
            DescriptRecordKind::Location,
            stop_before_location_record,
        );
    }

    #[test]
    fn character_boundary_matches_original_marker_writes() {
        assert_stop_handler(
            include_str!("../../../../../re/tools/oracle_vectors/func_7549_natural.json"),
            DescriptRecordKind::Character,
            stop_before_character_record,
        );
    }

    #[test]
    fn object_boundary_matches_original_marker_writes() {
        assert_stop_handler(
            include_str!("../../../../../re/tools/oracle_vectors/func_7550_natural.json"),
            DescriptRecordKind::Object,
            stop_before_object_record,
        );
    }

    #[test]
    fn sequence_boundary_matches_original_marker_writes() {
        assert_stop_handler(
            include_str!("../../../../../re/tools/oracle_vectors/func_7557_natural.json"),
            DescriptRecordKind::Sequence,
            stop_before_sequence_record,
        );
    }
}
