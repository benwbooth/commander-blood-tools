#include <dos.h>

#include "../include/bloodprg_list.h"

#define LIST_D8C_SOUND_MARKER 0x6473u
#define LIST_D8C_PALETTE_MARKER 0x6C70u
#define LIST_D8C_LINK_MARKER 0x6D6Du
#define LIST_D8C_FLAG_COMPRESSED 0x0200u
#define LIST_D8C_FLAG_NO_COORDINATES 0x0400u
#define LIST_D8C_TRANSPARENT_MODE 0xFFu

#pragma pack(1)
typedef struct list_d8c_link_record {
    const volatile cb_u16 CB_FAR *target;
    cb_u16 expected_key;
} list_d8c_link_record;
#pragma pack()

typedef char list_d8c_link_record_size_must_be_6[
        sizeof(list_d8c_link_record) == 6 ? 1 : -1];

void CB_NEAR list_d8c_activate_entry(
        cb_u16 entry_extent,
        volatile cb_u16 CB_FAR *entry,
        cb_u16 storage_segment)
{
    const volatile list_d8c_link_record CB_FAR *link;
    volatile cb_u16 CB_FAR *cursor;
    volatile cb_u16 CB_FAR *destination;
    cb_u16 source_segment;
    cb_u16 source_offset;
    cb_u16 record_offset;
    cb_u16 record_extent;
    cb_u16 selected_storage_segment;
    cb_u16 layout;
    cb_u16 row_mode;
    cb_u16 end_offset;

    resource_decode_mode = 0u;
    resource_decode_rectangular = 0u;
    list_d8c_sound_offset = 0xFFFFu;
    list_d8c_palette_offset = 0xFFFFu;

    source_segment = FP_SEG(entry);
    source_offset = FP_OFF(entry);
    end_offset = (cb_u16)(source_offset + entry_extent);
    if (end_offset < source_offset ||
            end_offset > list_d8c_buffer_end_offset) {
        source_offset = 0u;
    }
    cursor = (volatile cb_u16 CB_FAR *)MK_FP(
            source_segment, source_offset);

    record_offset = FP_OFF(cursor);
    layout = *cursor++;
    if (layout == LIST_D8C_SOUND_MARKER) {
        if (flag_test_b17()) {
            list_d8c_sound_offset = FP_OFF(cursor);
        }
        record_extent = *cursor;
        cursor = (volatile cb_u16 CB_FAR *)MK_FP(
                source_segment,
                (cb_u16)(record_offset + record_extent));
        record_offset = FP_OFF(cursor);
        layout = *cursor++;
    }

    while (layout == LIST_D8C_PALETTE_MARKER) {
        record_extent = *cursor++;
        list_d8c_palette_offset = FP_OFF(cursor);
        cursor = (volatile cb_u16 CB_FAR *)MK_FP(
                source_segment,
                (cb_u16)(record_offset + record_extent));
        record_offset = FP_OFF(cursor);
        layout = *cursor++;
    }

    if (layout == LIST_D8C_LINK_MARKER) {
        link = (const volatile list_d8c_link_record CB_FAR *)cursor;
        cursor = (volatile cb_u16 CB_FAR *)link->target;
        if (*cursor++ != link->expected_key) {
            queue_d8c_consume();
            return;
        }
        layout = *cursor++;
        source_segment = FP_SEG(cursor);
    }

    selected_storage_segment = storage_segment;
    if ((layout & LIST_D8C_FLAG_NO_COORDINATES) != 0u) {
        selected_storage_segment = list_d8c_default_entry_segment;
    }
    destination = (volatile cb_u16 CB_FAR *)MK_FP(
            selected_storage_segment, 0u);
    list_d8c_active_segment = selected_storage_segment;
    list_d8c_active_offset = 0u;
    list_d8c_active_layout = layout;
    *destination++ = layout;

    row_mode = *cursor++;
    list_d8c_active_row_mode = row_mode;
    *destination++ = row_mode;
    if ((cb_u8)row_mode == 0u) {
        return;
    }

    if ((layout & LIST_D8C_FLAG_COMPRESSED) == 0u) {
        list_d8c_active_offset = (cb_u16)(FP_OFF(cursor) - 4u);
        list_d8c_active_segment = FP_SEG(cursor);
        return;
    }

    if ((resource_skip_back_buffer_present & 1u) == 0u &&
            (resource_draw_via_back_buffer & 1u) == 0u &&
            (cb_u8)(row_mode >> 8) == LIST_D8C_TRANSPARENT_MODE) {
        resource_decode_rectangular = 1u;
        list_d8c_active_offset = FP_OFF(cursor);
        list_d8c_active_segment = FP_SEG(cursor);
        return;
    }

    (void)resource_payload_decode_dispatch(
            (const volatile cb_u8 CB_FAR *)cursor,
            (volatile cb_u8 CB_FAR *)destination,
            storage_segment);
}
