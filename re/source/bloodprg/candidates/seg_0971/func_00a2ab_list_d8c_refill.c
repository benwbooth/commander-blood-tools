#include <dos.h>

#include "../include/bloodprg_list.h"
#include "../include/bloodprg_resource.h"

#define LIST_D8C_ROLLOVER_ENABLED 0x01u
#define LIST_D8C_SYNTHESIZE_LINKS 0x04u
#define LIST_D8C_CACHED_RANGE_VALID 0x08u
#define LIST_D8C_WINDOW_MASK 0x07FFu
#define LIST_D8C_WINDOW_LOOKAHEAD 0x0800u
#define LIST_D8C_ROLLOVER_RESERVATION 0x1000u
#define LIST_D8C_LINK_MARKER 0x6D6Du
#define LIST_D8C_LINK_EXTENT 10u
#define LIST_D8C_LINK_BODY_BYTES 8u
#define LIST_D8C_LINK_COUNT 4u

#pragma pack(1)
typedef struct list_d8c_cached_range {
    cb_u32 source_offset;
    cb_u32 source_remaining;
} list_d8c_cached_range;

typedef struct list_d8c_link_body {
    cb_u16 marker;
    const volatile cb_u16 CB_FAR *target;
    cb_u16 expected_key;
} list_d8c_link_body;
#pragma pack()

typedef char list_d8c_cached_range_size_must_be_8[
        sizeof(list_d8c_cached_range) == 8 ? 1 : -1];
typedef char list_d8c_link_body_size_must_be_8[
        sizeof(list_d8c_link_body) == 8 ? 1 : -1];

cb_u16 CB_NEAR list_d8c_refill(cb_u16 link_target_offset)
{
    const volatile bloodprg_resource_descriptor CB_NEAR *descriptor;
    const volatile list_d8c_cached_range CB_NEAR *cached_range;
    volatile list_d8c_link_body CB_FAR *link;
    volatile cb_u16 CB_FAR *entry_header;
    const volatile cb_u16 CB_FAR *link_target;
    cb_u16 entry_extent;
    cb_u16 cursor_offset;
    cb_u16 chunk;
    cb_u16 window_bytes;
    cb_u16 previous_wrap_count;
    cb_u16 entry_offset;
    cb_u16 buffer_segment;
    cb_u16 link_index;
    cb_u16 target_extent;
    cb_u8 check_source_first;

    check_source_first = 0u;
    for (;;) {
        if (!check_source_first) {
            chunk = list_d8c_iteration_count;
            if (chunk != 0u) {
                if ((cb_i8)(cb_u8)resource_flags >= 0) {
                    window_bytes = (cb_u16)(-(cb_u16)resource_source_offset);
                    window_bytes &= LIST_D8C_WINDOW_MASK;
                    window_bytes += LIST_D8C_WINDOW_LOOKAHEAD;
                    if (window_bytes < chunk) {
                        chunk = window_bytes;
                    }
                }
                if (!queue_d8c_has_room(chunk)) {
                    return link_target_offset;
                }
                list_d8c_iteration_count -= chunk;
                (void)ems_paged_read(chunk);
                return link_target_offset;
            }
        }
        check_source_first = 0u;

        if (list_d8c_wrap_count != list_d8c_secondary_wrap_limit &&
                resource_source_remaining != 0UL) {
            if (!list_d8c_read(&entry_extent, &cursor_offset)) {
                return link_target_offset;
            }
            queue_d8c_wrap(entry_extent, cursor_offset);
            continue;
        }

        if (((cb_u8)resource_flags & LIST_D8C_ROLLOVER_ENABLED) == 0u) {
            presentation_queue_finish();
            return link_target_offset;
        }
        if (!queue_d8c_has_room(LIST_D8C_ROLLOVER_RESERVATION)) {
            return link_target_offset;
        }

        previous_wrap_count = list_d8c_wrap_count;
        list_d8c_wrap_bounds_reset();
        list_d8c_read_wrap_limit = previous_wrap_count;
        list_d8c_rollover_state = 0u;

        if (resource_active_id != resource_requested_id) {
            descriptor = lookup_table_1fb5(resource_active_id);
            cached_range = (const volatile list_d8c_cached_range CB_NEAR *)
                    ((const volatile cb_u8 CB_NEAR *)descriptor -
                    sizeof(list_d8c_cached_range));
            if ((descriptor->flags & LIST_D8C_CACHED_RANGE_VALID) == 0u ||
                    (cb_u16)(cached_range->source_offset >> 16) == 0u) {
                resource_requested_id = resource_active_id;
                /* The binary's fallback calls 0x009FA2 with an invalid frame. */
                return link_target_offset;
            }
            resource_requested_id = resource_active_id;
            resource_range_start = cached_range->source_offset;
            resource_range_remaining = cached_range->source_remaining;
            resource_flags = (resource_flags & 0xFF00u) | descriptor->flags;
        }

        resource_source_remaining = resource_range_remaining;
        resource_source_offset = resource_range_start;

        if (((cb_u8)resource_flags & LIST_D8C_SYNTHESIZE_LINKS) != 0u) {
            for (link_index = 0u;
                    link_index < LIST_D8C_LINK_COUNT;
                    ++link_index) {
                entry_offset = list_d8c_head_offset;
                buffer_segment = list_d8c_head_segment;
                entry_header = (volatile cb_u16 CB_FAR *)MK_FP(
                        buffer_segment, entry_offset);
                queue_d8c_enqueue(2u);
                *entry_header = LIST_D8C_LINK_EXTENT;
                queue_d8c_wrap(
                        LIST_D8C_LINK_EXTENT,
                        (cb_u16)(entry_offset + 2u));

                link_target = (const volatile cb_u16 CB_FAR *)MK_FP(
                        buffer_segment, link_target_offset);
                target_extent = *link_target;
                link = (volatile list_d8c_link_body CB_FAR *)MK_FP(
                        buffer_segment, list_d8c_head_offset);
                link->marker = LIST_D8C_LINK_MARKER;
                link->target = link_target;
                link->expected_key = target_extent;
                link_target_offset += target_extent;
                queue_d8c_enqueue(LIST_D8C_LINK_BODY_BYTES);
            }
            list_d8c_rollover_state = 0x80u;
        }
        check_source_first = 1u;
    }
}
