#include "../include/bloodprg_entity.h"

void CB_FAR dirty_rects_copy_secondary_to_primary(
        const volatile bloodprg_dirty_rect CB_FAR *rectangles)
{
    const volatile cb_u8 CB_FAR *source;
    volatile cb_u8 CB_FAR *destination;
    cb_u16 offset;
    cb_u16 width;
    cb_u16 rows;
    cb_u16 columns;

    if ((bloodprg_dirty_copy_flags & 1u) == 0u) {
        return;
    }

    while ((cb_i16)rectangles->left >= 0) {
        width = (cb_u16)(rectangles->right - rectangles->left);
        rows = (cb_u16)(rectangles->bottom - rectangles->top);
        offset = (cb_u16)(rectangles->top * 320u + rectangles->left);
        /* The original runtime supplies both buffers at segment offset zero. */
        source = bloodprg_secondary_buffer + offset;
        destination = bloodprg_display_buffer + offset;

        do {
            columns = width;
            while (columns != 0u) {
                *destination++ = *source++;
                --columns;
            }
            source += (cb_u16)(320u - width);
            destination += (cb_u16)(320u - width);
            --rows;
        } while (rows != 0u);

        ++rectangles;
    }
}
