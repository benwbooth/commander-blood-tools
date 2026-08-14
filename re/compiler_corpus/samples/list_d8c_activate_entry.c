/* Codegen probe for BLOODPRG 0x00A552. */
#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;

#define FAR far
#define NEAR near
#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA FAR
#endif

typedef struct decode_result {
    const volatile u8 FAR *source;
    volatile u8 FAR *destination;
} decode_result;

#pragma pack(1)
typedef struct link_record {
    const volatile u16 FAR *target;
    u16 expected_key;
} link_record;
#pragma pack()

extern volatile u16 list_d8c_sound_offset;
extern volatile u16 list_d8c_palette_offset;
extern volatile u16 list_d8c_buffer_end_offset;
extern volatile u16 list_d8c_default_entry_segment;
extern volatile u16 list_d8c_active_offset;
extern volatile u16 list_d8c_active_segment;
extern volatile u16 list_d8c_active_layout;
extern volatile u16 list_d8c_active_row_mode;
extern volatile u8 resource_decode_rectangular;
extern volatile u8 resource_skip_back_buffer_present;
extern volatile u8 resource_draw_via_back_buffer;
extern volatile u16 GAME_DATA resource_decode_mode;

int NEAR flag_test_b17_probe(void);
void NEAR queue_d8c_consume_probe(void);
decode_result NEAR resource_payload_decode_dispatch_probe(
        const volatile u8 FAR *source,
        volatile u8 FAR *destination,
        u16 alternate_destination_segment);

void NEAR list_d8c_activate_entry_probe(
        u16 entry_extent,
        volatile u16 FAR *entry,
        u16 storage_segment)
{
    const volatile link_record FAR *link;
    volatile u16 FAR *cursor;
    volatile u16 FAR *destination;
    u16 source_segment;
    u16 source_offset;
    u16 record_offset;
    u16 record_extent;
    u16 selected_storage_segment;
    u16 layout;
    u16 row_mode;
    u16 end_offset;

    resource_decode_mode = 0u;
    resource_decode_rectangular = 0u;
    list_d8c_sound_offset = 0xFFFFu;
    list_d8c_palette_offset = 0xFFFFu;

    source_segment = FP_SEG(entry);
    source_offset = FP_OFF(entry);
    end_offset = (u16)(source_offset + entry_extent);
    if (end_offset < source_offset ||
            end_offset > list_d8c_buffer_end_offset) {
        source_offset = 0u;
    }
    cursor = (volatile u16 FAR *)MK_FP(source_segment, source_offset);

    record_offset = FP_OFF(cursor);
    layout = *cursor++;
    if (layout == 0x6473u) {
        if (flag_test_b17_probe()) {
            list_d8c_sound_offset = FP_OFF(cursor);
        }
        record_extent = *cursor;
        cursor = (volatile u16 FAR *)MK_FP(
                source_segment, (u16)(record_offset + record_extent));
        record_offset = FP_OFF(cursor);
        layout = *cursor++;
    }

    while (layout == 0x6C70u) {
        record_extent = *cursor++;
        list_d8c_palette_offset = FP_OFF(cursor);
        cursor = (volatile u16 FAR *)MK_FP(
                source_segment, (u16)(record_offset + record_extent));
        record_offset = FP_OFF(cursor);
        layout = *cursor++;
    }

    if (layout == 0x6D6Du) {
        link = (const volatile link_record FAR *)cursor;
        cursor = (volatile u16 FAR *)link->target;
        if (*cursor++ != link->expected_key) {
            queue_d8c_consume_probe();
            return;
        }
        layout = *cursor++;
        source_segment = FP_SEG(cursor);
    }

    selected_storage_segment = storage_segment;
    if ((layout & 0x0400u) != 0u) {
        selected_storage_segment = list_d8c_default_entry_segment;
    }
    destination = (volatile u16 FAR *)MK_FP(selected_storage_segment, 0u);
    list_d8c_active_segment = selected_storage_segment;
    list_d8c_active_offset = 0u;
    list_d8c_active_layout = layout;
    *destination++ = layout;

    row_mode = *cursor++;
    list_d8c_active_row_mode = row_mode;
    *destination++ = row_mode;
    if ((u8)row_mode == 0u) {
        return;
    }
    if ((layout & 0x0200u) == 0u) {
        list_d8c_active_offset = (u16)(FP_OFF(cursor) - 4u);
        list_d8c_active_segment = FP_SEG(cursor);
        return;
    }
    if ((resource_skip_back_buffer_present & 1u) == 0u &&
            (resource_draw_via_back_buffer & 1u) == 0u &&
            (u8)(row_mode >> 8) == 0xFFu) {
        resource_decode_rectangular = 1u;
        list_d8c_active_offset = FP_OFF(cursor);
        list_d8c_active_segment = FP_SEG(cursor);
        return;
    }
    (void)resource_payload_decode_dispatch_probe(
            (const volatile u8 FAR *)cursor,
            (volatile u8 FAR *)destination,
            storage_segment);
}
