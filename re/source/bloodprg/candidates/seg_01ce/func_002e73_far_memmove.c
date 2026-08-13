#include <dos.h>

#include "../include/bloodprg_ems.h"

#define FAR_MEMMOVE_CHUNK 0xfa00UL

void CB_FAR far_memmove(volatile cb_u8 CB_FAR *destination,
        const volatile cb_u8 CB_FAR *source, cb_u32 byte_count)
{
    volatile cb_u32 CB_FAR *destination_words;
    const volatile cb_u32 CB_FAR *source_words;
    cb_u32 source_linear;
    cb_u32 destination_linear;
    cb_u32 source_end;
    cb_u32 destination_end;
    cb_u32 remaining_after_chunk;
    cb_u16 chunk_bytes;
    cb_u16 word_count;
    cb_u16 tail_bytes;

    source_linear = ((cb_u32)FP_SEG(source) << 4) + FP_OFF(source);
    destination_linear =
            ((cb_u32)FP_SEG(destination) << 4) + FP_OFF(destination);

    if ((cb_i32)source_linear > (cb_i32)destination_linear ||
            (cb_i32)(source_linear + byte_count) <
                    (cb_i32)destination_linear) {
        do {
            source = (const volatile cb_u8 CB_FAR *)MK_FP(
                    (cb_u16)(source_linear >> 4),
                    (cb_u16)source_linear & 0x000fu);
            destination = (volatile cb_u8 CB_FAR *)MK_FP(
                    (cb_u16)(destination_linear >> 4),
                    (cb_u16)destination_linear & 0x000fu);

            remaining_after_chunk = byte_count - FAR_MEMMOVE_CHUNK;
            if ((cb_i32)remaining_after_chunk < 0) {
                chunk_bytes = (cb_u16)byte_count;
            } else {
                chunk_bytes = (cb_u16)FAR_MEMMOVE_CHUNK;
            }

            word_count = chunk_bytes >> 2;
            source_words = (const volatile cb_u32 CB_FAR *)source;
            destination_words = (volatile cb_u32 CB_FAR *)destination;
            while (word_count != 0) {
                *destination_words++ = *source_words++;
                --word_count;
            }

            tail_bytes = chunk_bytes & 3u;
            source = (const volatile cb_u8 CB_FAR *)source_words;
            destination = (volatile cb_u8 CB_FAR *)destination_words;
            while (tail_bytes != 0) {
                *destination++ = *source++;
                --tail_bytes;
            }

            if ((cb_i32)remaining_after_chunk < 0) {
                break;
            }
            source_linear += FAR_MEMMOVE_CHUNK;
            destination_linear += FAR_MEMMOVE_CHUNK;
            byte_count = remaining_after_chunk;
        } while (1);
    } else {
        source_end = source_linear + byte_count;
        destination_end = destination_linear + byte_count;

        do {
            source_end -= FAR_MEMMOVE_CHUNK;
            destination_end -= FAR_MEMMOVE_CHUNK;
            source = (const volatile cb_u8 CB_FAR *)MK_FP(
                    (cb_u16)(source_end >> 4),
                    (cb_u16)FAR_MEMMOVE_CHUNK);
            destination = (volatile cb_u8 CB_FAR *)MK_FP(
                    (cb_u16)(destination_end >> 4),
                    (cb_u16)FAR_MEMMOVE_CHUNK);

            remaining_after_chunk = byte_count - FAR_MEMMOVE_CHUNK;
            if ((cb_i32)remaining_after_chunk < 0) {
                chunk_bytes = (cb_u16)byte_count;
            } else {
                chunk_bytes = (cb_u16)FAR_MEMMOVE_CHUNK;
            }

            word_count = chunk_bytes >> 2;
            source_words = (const volatile cb_u32 CB_FAR *)source;
            destination_words = (volatile cb_u32 CB_FAR *)destination;
            while (word_count != 0) {
                *destination_words = *source_words;
                --destination_words;
                --source_words;
                --word_count;
            }

            tail_bytes = chunk_bytes & 3u;
            source = (const volatile cb_u8 CB_FAR *)source_words;
            destination = (volatile cb_u8 CB_FAR *)destination_words;
            while (tail_bytes != 0) {
                *destination = *source;
                --destination;
                --source;
                --tail_bytes;
            }

            if ((cb_i32)remaining_after_chunk < 0) {
                break;
            }
            byte_count = remaining_after_chunk;
        } while (1);
    }
}
