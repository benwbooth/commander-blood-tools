#include <dos.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_resource.h"

#define BLOODPRG_PBM_FORM_ID 0x204d4250UL
#define BLOODPRG_PBM_CMAP_ID 0x50414d43UL
#define BLOODPRG_PBM_BODY_ID 0x59444f42UL
#define BLOODPRG_PBM_PALETTE_BYTES 768u
#define BLOODPRG_PBM_LIMITED_PALETTE_BYTES 576u
#define BLOODPRG_PBM_SCREEN_BYTES 64000UL

cb_i16 CB_FAR pbm_image_load_and_decode(volatile char CB_FAR *path,
        volatile cb_u8 CB_FAR *file_buffer_end)
{
    volatile bloodprg_dos_dta CB_FAR *dta;
    volatile cb_u8 CB_FAR *cursor;
    volatile cb_u8 CB_FAR *candidate;
    volatile cb_u8 CB_FAR *palette_source;
    volatile cb_u8 CB_FAR *output;
    cb_u32 output_remaining;
    cb_u16 file_handle;
    cb_u16 file_size;
    cb_u16 remaining;
    cb_u16 palette_bytes;
    cb_u16 count;
    cb_u8 value;
    cb_u8 last_value;
    cb_i8 control;
    int embedded;

    file_handle = resource_source_select(path);
    file_size = (cb_u16)resource_archive_remaining;
    embedded = (resource_path_is_embedded & 1u) != 0;
    if (!embedded) {
        dta = cb_dos_get_dta();
        (void)cb_dos_find_first(path);
        file_size = (cb_u16)dta->file_size;
        if (!cb_dos_open_read_only(path, &file_handle)) {
            return -1;
        }
    }

    cursor = (volatile cb_u8 CB_FAR *)MK_FP(
            FP_SEG(file_buffer_end),
            (cb_u16)(FP_OFF(file_buffer_end) - file_size));
    (void)cb_dos_read(file_handle, cursor, file_size);
    if (!embedded) {
        cb_dos_close(file_handle);
    }

    remaining = file_size;
    for (;;) {
        do {
            if (remaining == 0) {
                return -1;
            }
            value = *cursor++;
            --remaining;
        } while (value != (cb_u8)'P');
        if (remaining == 0) {
            return -1;
        }
        candidate = cursor - 1;
        if (*(volatile cb_u32 CB_FAR *)candidate
                == BLOODPRG_PBM_FORM_ID) {
            break;
        }
    }

    for (;;) {
        do {
            if (remaining == 0) {
                return -1;
            }
            value = *cursor++;
            --remaining;
        } while (value != (cb_u8)'C');
        if (remaining == 0) {
            return -1;
        }
        candidate = cursor - 1;
        if (*(volatile cb_u32 CB_FAR *)candidate
                == BLOODPRG_PBM_CMAP_ID) {
            break;
        }
    }

    cursor += 7;
    palette_source = cursor;
    if ((pbm_palette_refresh & 1u) != 0) {
        pbm_palette_dirty = 1;
        palette_bytes = BLOODPRG_PBM_PALETTE_BYTES;
        if ((pbm_ship_palette_limit | pbm_scene_palette_limit) != 0) {
            palette_bytes = BLOODPRG_PBM_LIMITED_PALETTE_BYTES;
        }
        for (count = 0; count < palette_bytes; ++count) {
            pbm_live_palette[count] = (cb_u8)(*palette_source++ >> 2);
        }
    }

    cursor += BLOODPRG_PBM_PALETTE_BYTES;
    for (;;) {
        do {
            if (remaining == 0) {
                return -1;
            }
            value = *cursor++;
            --remaining;
        } while (value != (cb_u8)'B');
        if (remaining == 0) {
            return -1;
        }
        candidate = cursor - 1;
        if (*(volatile cb_u32 CB_FAR *)candidate
                == BLOODPRG_PBM_BODY_ID) {
            break;
        }
    }

    cursor += 7;
    output = graphics_back_buffer;
    output_remaining = BLOODPRG_PBM_SCREEN_BYTES;
    last_value = 0;

    if ((pbm_transparent_zero & 1u) != 0) {
        while ((cb_u16)output_remaining != 0) {
            control = (cb_i8)*cursor++;
            if (control < 0) {
                count = (cb_u16)(-(cb_i16)control + 1);
                output_remaining -= (cb_u32)count;
                last_value = *cursor++;
                if (last_value == 0) {
                    output += count;
                } else {
                    while (count-- != 0) {
                        *output++ = last_value;
                    }
                }
            } else {
                count = (cb_u16)control + 1u;
                output_remaining -= (cb_u32)count;
                while (count-- != 0) {
                    last_value = *cursor++;
                    if (last_value != 0) {
                        *output = last_value;
                    }
                    ++output;
                }
            }
        }
    } else {
        while ((cb_u16)output_remaining != 0) {
            control = (cb_i8)*cursor++;
            if (control < 0) {
                count = (cb_u16)(-(cb_i16)control + 1);
                output_remaining -= (cb_u32)count;
                last_value = *cursor++;
                while (count-- != 0) {
                    *output++ = last_value;
                }
            } else {
                count = (cb_u16)control + 1u;
                output_remaining -= (cb_u32)count;
                while (count-- != 0) {
                    last_value = *cursor++;
                    *output++ = last_value;
                }
            }
        }
    }

    return (cb_i16)last_value;
}
