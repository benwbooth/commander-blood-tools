/*
 * Codegen probe for BLOODPRG 0x002BFD.
 * This is not recovered game source.
 */
#include <dos.h>

typedef unsigned char u8;
typedef signed char i8;
typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#define PBM_FORM_ID 0x204d4250UL
#define PBM_CMAP_ID 0x50414d43UL
#define PBM_BODY_ID 0x59444f42UL
#define PBM_PALETTE_BYTES 768u
#define PBM_LIMITED_PALETTE_BYTES 576u
#define PBM_SCREEN_BYTES 64000UL

typedef struct dos_dta_probe {
    u8 reserved_00[0x1a];
    u32 file_size;
} dos_dta_probe;

extern volatile u32 archive_remaining_probe;
extern volatile u8 path_is_embedded_probe;
extern volatile u8 ship_palette_limit_probe;
extern volatile u8 scene_palette_limit_probe;
extern volatile u8 palette_refresh_probe;
extern volatile u8 palette_dirty_probe;
extern volatile u8 transparent_zero_probe;
extern volatile u8 palette_probe[768];
extern volatile u8 FAR *back_buffer_probe;

u16 FAR resource_source_select_probe(volatile char FAR *path);
volatile dos_dta_probe FAR *NEAR dos_get_dta_probe(void);
int NEAR dos_find_first_probe(const volatile char FAR *path);
int NEAR dos_open_read_only_probe(const volatile char FAR *path, u16 *handle);
u16 NEAR dos_read_probe(
        u16 handle, volatile u8 FAR *destination, u16 byte_count);
void NEAR dos_close_probe(u16 handle);

i16 FAR pbm_image_load_and_decode_probe(volatile char FAR *path,
        volatile u8 FAR *file_buffer_end)
{
    volatile dos_dta_probe FAR *dta;
    volatile u8 FAR *cursor;
    volatile u8 FAR *candidate;
    volatile u8 FAR *palette_source;
    volatile u8 FAR *output;
    u32 output_remaining;
    u16 file_handle;
    u16 file_size;
    u16 remaining;
    u16 palette_bytes;
    u16 count;
    u8 value;
    u8 last_value;
    i8 control;
    int embedded;

    file_handle = resource_source_select_probe(path);
    file_size = (u16)archive_remaining_probe;
    embedded = (path_is_embedded_probe & 1u) != 0;
    if (!embedded) {
        dta = dos_get_dta_probe();
        (void)dos_find_first_probe(path);
        file_size = (u16)dta->file_size;
        if (!dos_open_read_only_probe(path, &file_handle)) {
            return -1;
        }
    }

    cursor = (volatile u8 FAR *)MK_FP(
            FP_SEG(file_buffer_end),
            (u16)(FP_OFF(file_buffer_end) - file_size));
    (void)dos_read_probe(file_handle, cursor, file_size);
    if (!embedded) {
        dos_close_probe(file_handle);
    }

    remaining = file_size;
    for (;;) {
        do {
            if (remaining == 0) return -1;
            value = *cursor++;
            --remaining;
        } while (value != (u8)'P');
        if (remaining == 0) return -1;
        candidate = cursor - 1;
        if (*(volatile u32 FAR *)candidate == PBM_FORM_ID) break;
    }
    for (;;) {
        do {
            if (remaining == 0) return -1;
            value = *cursor++;
            --remaining;
        } while (value != (u8)'C');
        if (remaining == 0) return -1;
        candidate = cursor - 1;
        if (*(volatile u32 FAR *)candidate == PBM_CMAP_ID) break;
    }

    cursor += 7;
    palette_source = cursor;
    if ((palette_refresh_probe & 1u) != 0) {
        palette_dirty_probe = 1;
        palette_bytes = PBM_PALETTE_BYTES;
        if ((ship_palette_limit_probe | scene_palette_limit_probe) != 0) {
            palette_bytes = PBM_LIMITED_PALETTE_BYTES;
        }
        for (count = 0; count < palette_bytes; ++count) {
            palette_probe[count] = (u8)(*palette_source++ >> 2);
        }
    }

    cursor += PBM_PALETTE_BYTES;
    for (;;) {
        do {
            if (remaining == 0) return -1;
            value = *cursor++;
            --remaining;
        } while (value != (u8)'B');
        if (remaining == 0) return -1;
        candidate = cursor - 1;
        if (*(volatile u32 FAR *)candidate == PBM_BODY_ID) break;
    }

    cursor += 7;
    output = back_buffer_probe;
    output_remaining = PBM_SCREEN_BYTES;
    last_value = 0;
    if ((transparent_zero_probe & 1u) != 0) {
        while ((u16)output_remaining != 0) {
            control = (i8)*cursor++;
            if (control < 0) {
                count = (u16)(-(i16)control + 1);
                output_remaining -= (u32)count;
                last_value = *cursor++;
                if (last_value == 0) {
                    output += count;
                } else {
                    while (count-- != 0) *output++ = last_value;
                }
            } else {
                count = (u16)control + 1u;
                output_remaining -= (u32)count;
                while (count-- != 0) {
                    last_value = *cursor++;
                    if (last_value != 0) *output = last_value;
                    ++output;
                }
            }
        }
    } else {
        while ((u16)output_remaining != 0) {
            control = (i8)*cursor++;
            if (control < 0) {
                count = (u16)(-(i16)control + 1);
                output_remaining -= (u32)count;
                last_value = *cursor++;
                while (count-- != 0) *output++ = last_value;
            } else {
                count = (u16)control + 1u;
                output_remaining -= (u32)count;
                while (count-- != 0) {
                    last_value = *cursor++;
                    *output++ = last_value;
                }
            }
        }
    }
    return (i16)last_value;
}
