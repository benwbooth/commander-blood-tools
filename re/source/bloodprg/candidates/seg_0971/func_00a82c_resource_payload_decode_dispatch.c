#include <dos.h>

#include "../include/bloodprg_list.h"

#define RESOURCE_DECODE_SIGNATURE_BYTES 6u
#define RESOURCE_DECODE_DESTINATION_OFFSET_MASK 0xFDFFu
#define RESOURCE_DECODE_CHECKSUM_AB 0xABu
#define RESOURCE_DECODE_CHECKSUM_AD 0xADu

bloodprg_resource_decode_result CB_NEAR resource_payload_decode_dispatch(
        const volatile cb_u8 CB_FAR *source,
        volatile cb_u8 CB_FAR *destination,
        cb_u16 alternate_destination_segment)
{
    bloodprg_resource_decode_result result;
    cb_u8 checksum;
    cb_u16 index;

    destination = (volatile cb_u8 CB_FAR *)MK_FP(
            FP_SEG(destination),
            FP_OFF(destination) & RESOURCE_DECODE_DESTINATION_OFFSET_MASK);
    checksum = 0u;
    for (index = 0u; index < RESOURCE_DECODE_SIGNATURE_BYTES; ++index) {
        checksum = (cb_u8)(checksum + source[index]);
    }

    if (checksum == RESOURCE_DECODE_CHECKSUM_AD) {
        resource_decode_mode = 3u;
        destination = (volatile cb_u8 CB_FAR *)MK_FP(
                alternate_destination_segment, FP_OFF(destination));
        resource_payload_decode_ad(source, destination);
        source = (const volatile cb_u8 CB_FAR *)MK_FP(
                FP_SEG(destination), 0u);
    } else if (checksum == RESOURCE_DECODE_CHECKSUM_AB) {
        resource_payload_decode_ab(source, destination);
        source = (const volatile cb_u8 CB_FAR *)MK_FP(
                FP_SEG(source), FP_OFF(destination));
    }

    result.source = source;
    result.destination = destination;
    return result;
}
