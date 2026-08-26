//! Typed checksum dispatch for compressed presentation payloads.

use std::error::Error;
use std::fmt;

use super::{
    AbDecodeOutcome, PresentationAdError, PresentationAdOutcome, PresentationDecodeError,
    decode_presentation_ab, decode_presentation_ad,
};

const SIGNATURE_BYTE_COUNT: usize = 6;
const AB_SIGNATURE: u8 = 0xAB;
const AD_SIGNATURE: u8 = 0xAD;

/// Compressed presentation format selected by the six-byte header sum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationPayloadKind {
    /// The payload does not use either recognized compressed format.
    Unrecognized {
        /// Wrapping sum of the six signature bytes.
        checksum: u8,
    },
    /// LSB-first AB LZ payload.
    Ab,
    /// Pair-staged, MSB-first AD run payload.
    Ad,
}

/// Owned result of decoding one recognized presentation payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationPayload {
    /// The original dispatcher leaves unrecognized payloads untouched.
    Unrecognized {
        /// Wrapping sum that did not select AB or AD.
        checksum: u8,
    },
    /// Complete AB decoder result.
    Ab(AbDecodeOutcome),
    /// Complete AD decoder result.
    Ad(PresentationAdOutcome),
}

/// Invalid presentation signature or selected compressed payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationDispatchError {
    /// Fewer than six signature bytes were available.
    SignatureTruncated {
        /// Available source bytes.
        available: usize,
    },
    /// The selected AB decoder rejected the payload.
    Ab(PresentationDecodeError),
    /// The selected AD decoder rejected the payload.
    Ad(PresentationAdError),
}

impl fmt::Display for PresentationDispatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid dispatched presentation payload: {self:?}"
        )
    }
}

impl Error for PresentationDispatchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Ab(source) => Some(source),
            Self::Ad(source) => Some(source),
            Self::SignatureTruncated { .. } => None,
        }
    }
}

/// Classify a presentation payload by its wrapping six-byte signature sum.
///
/// This is the decision boundary in `resource_payload_decode_dispatch` at
/// BLOODPRG offset `0x00A82C`. The original inherited string direction and
/// destination-offset mask are eliminated memory-layout concerns; owned source
/// bytes are always inspected in their logical order.
pub fn presentation_payload_kind(
    source: &[u8],
) -> Result<PresentationPayloadKind, PresentationDispatchError> {
    let signature = source.get(..SIGNATURE_BYTE_COUNT).ok_or(
        PresentationDispatchError::SignatureTruncated {
            available: source.len(),
        },
    )?;
    let checksum = signature.iter().copied().fold(u8::MIN, u8::wrapping_add);
    Ok(match checksum {
        AB_SIGNATURE => PresentationPayloadKind::Ab,
        AD_SIGNATURE => PresentationPayloadKind::Ad,
        checksum => PresentationPayloadKind::Unrecognized { checksum },
    })
}

/// Select and decode one compressed presentation payload into owned bytes.
///
/// Recognized AB and AD streams are delegated to their oracle-verified flat
/// decoders. Unrecognized signatures retain the original no-dispatch outcome.
pub fn decode_presentation_payload(
    source: &[u8],
) -> Result<PresentationPayload, PresentationDispatchError> {
    match presentation_payload_kind(source)? {
        PresentationPayloadKind::Unrecognized { checksum } => {
            Ok(PresentationPayload::Unrecognized { checksum })
        }
        PresentationPayloadKind::Ab => decode_presentation_ab(source)
            .map(PresentationPayload::Ab)
            .map_err(PresentationDispatchError::Ab),
        PresentationPayloadKind::Ad => decode_presentation_ad(source)
            .map(PresentationPayload::Ad)
            .map_err(PresentationDispatchError::Ad),
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const DISPATCH_VECTOR_COUNT: usize = 8;
    const AB_EMPTY_STREAM: &[u8] = &[
        0x31, 0x42, 0x53, 0x64, 0x75, 0x0C, 0x02, 0x00, 0x00, 0x00, 0x00,
    ];
    const AD_EMPTY_STREAM: &[u8] = &[0x00, 0x00, 0x00, 0x00, 0x04, 0xA9, 0x31, 0x00, 0xF0];

    #[derive(Deserialize)]
    struct DispatchOracle {
        name: String,
        header_bytes_in_read_order: [u8; SIGNATURE_BYTE_COUNT],
        checksum: u8,
        direction: String,
        path: String,
    }

    #[test]
    fn signature_selection_matches_every_original_vector() {
        let vectors: Vec<DispatchOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a82c_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), DISPATCH_VECTOR_COUNT);

        for vector in vectors {
            let kind = presentation_payload_kind(&vector.header_bytes_in_read_order).unwrap();
            let expected = match vector.path.as_str() {
                "ab" => PresentationPayloadKind::Ab,
                "ad" => PresentationPayloadKind::Ad,
                "none" => PresentationPayloadKind::Unrecognized {
                    checksum: vector.checksum,
                },
                path => panic!("unexpected oracle dispatch path {path}"),
            };
            assert_eq!(kind, expected, "{}", vector.name);
            assert_eq!(
                vector
                    .header_bytes_in_read_order
                    .into_iter()
                    .fold(u8::MIN, u8::wrapping_add),
                vector.checksum,
                "{}",
                vector.name
            );
            assert!(
                matches!(vector.direction.as_str(), "forward" | "backward"),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn recognized_empty_streams_reach_their_typed_decoders() {
        let ab = decode_presentation_payload(AB_EMPTY_STREAM).unwrap();
        let ad = decode_presentation_payload(AD_EMPTY_STREAM).unwrap();
        assert!(matches!(
            ab,
            PresentationPayload::Ab(AbDecodeOutcome { ref bytes, .. }) if bytes.is_empty()
        ));
        assert!(matches!(
            ad,
            PresentationPayload::Ad(PresentationAdOutcome { ref bytes, .. }) if bytes.is_empty()
        ));
    }

    #[test]
    fn short_signatures_do_not_enter_a_decoder() {
        assert_eq!(
            decode_presentation_payload(&[u8::MIN; SIGNATURE_BYTE_COUNT - 1]),
            Err(PresentationDispatchError::SignatureTruncated {
                available: SIGNATURE_BYTE_COUNT - 1,
            })
        );
    }
}
