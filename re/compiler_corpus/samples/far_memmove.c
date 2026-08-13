/*
 * Codegen probe for BLOODPRG 0x002E73.
 * This is not recovered game source.
 */
#include <dos.h>

typedef unsigned char u8;
typedef signed long i32;
typedef unsigned int u16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

#define COPY_CHUNK 0xfa00UL

void FAR far_memmove_probe(volatile u8 FAR *destination,
        const volatile u8 FAR *source, u32 byte_count);

void FAR far_memmove_probe(volatile u8 FAR *destination,
        const volatile u8 FAR *source, u32 byte_count)
{
    volatile u32 FAR *destination_words;
    const volatile u32 FAR *source_words;
    u32 source_linear;
    u32 destination_linear;
    u32 source_end;
    u32 destination_end;
    u32 remaining_after_chunk;
    u16 chunk_bytes;
    u16 word_count;
    u16 tail_bytes;

    source_linear = ((u32)FP_SEG(source) << 4) + FP_OFF(source);
    destination_linear = ((u32)FP_SEG(destination) << 4) +
            FP_OFF(destination);

    if ((i32)source_linear > (i32)destination_linear ||
            (i32)(source_linear + byte_count) < (i32)destination_linear) {
        do {
            source = (const volatile u8 FAR *)MK_FP(
                    (u16)(source_linear >> 4),
                    (u16)source_linear & 0x000fu);
            destination = (volatile u8 FAR *)MK_FP(
                    (u16)(destination_linear >> 4),
                    (u16)destination_linear & 0x000fu);
            remaining_after_chunk = byte_count - COPY_CHUNK;
            chunk_bytes = (i32)remaining_after_chunk < 0 ?
                    (u16)byte_count : (u16)COPY_CHUNK;

            word_count = chunk_bytes >> 2;
            source_words = (const volatile u32 FAR *)source;
            destination_words = (volatile u32 FAR *)destination;
            while (word_count != 0) {
                *destination_words++ = *source_words++;
                --word_count;
            }
            tail_bytes = chunk_bytes & 3u;
            source = (const volatile u8 FAR *)source_words;
            destination = (volatile u8 FAR *)destination_words;
            while (tail_bytes != 0) {
                *destination++ = *source++;
                --tail_bytes;
            }

            if ((i32)remaining_after_chunk < 0) {
                break;
            }
            source_linear += COPY_CHUNK;
            destination_linear += COPY_CHUNK;
            byte_count = remaining_after_chunk;
        } while (1);
    } else {
        source_end = source_linear + byte_count;
        destination_end = destination_linear + byte_count;
        do {
            source_end -= COPY_CHUNK;
            destination_end -= COPY_CHUNK;
            source = (const volatile u8 FAR *)MK_FP(
                    (u16)(source_end >> 4), (u16)COPY_CHUNK);
            destination = (volatile u8 FAR *)MK_FP(
                    (u16)(destination_end >> 4), (u16)COPY_CHUNK);
            remaining_after_chunk = byte_count - COPY_CHUNK;
            chunk_bytes = (i32)remaining_after_chunk < 0 ?
                    (u16)byte_count : (u16)COPY_CHUNK;

            word_count = chunk_bytes >> 2;
            source_words = (const volatile u32 FAR *)source;
            destination_words = (volatile u32 FAR *)destination;
            while (word_count != 0) {
                *destination_words = *source_words;
                --destination_words;
                --source_words;
                --word_count;
            }
            tail_bytes = chunk_bytes & 3u;
            source = (const volatile u8 FAR *)source_words;
            destination = (volatile u8 FAR *)destination_words;
            while (tail_bytes != 0) {
                *destination = *source;
                --destination;
                --source;
                --tail_bytes;
            }
            if ((i32)remaining_after_chunk < 0) {
                break;
            }
            byte_count = remaining_after_chunk;
        } while (1);
    }
}
