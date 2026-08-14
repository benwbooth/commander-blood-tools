/* Codegen probe for BLOODPRG 0x00A2AB. */
#include <dos.h>

typedef signed char i8;
typedef unsigned char u8;
typedef unsigned int u16;
typedef unsigned long u32;

#define FAR far
#define NEAR near

#define ROLLOVER_ENABLED 0x01u
#define SYNTHESIZE_LINKS 0x04u
#define CACHED_RANGE_VALID 0x08u
#define WINDOW_MASK 0x07FFu
#define WINDOW_LOOKAHEAD 0x0800u
#define ROLLOVER_RESERVATION 0x1000u
#define LINK_MARKER 0x6D6Du
#define LINK_EXTENT 10u
#define LINK_BODY_BYTES 8u
#define LINK_COUNT 4u

#pragma pack(1)
typedef struct cached_range_probe {
    u32 source_offset;
    u32 source_remaining;
} cached_range_probe;

typedef struct resource_descriptor_probe {
    u8 flags;
    u8 variant;
    char filename[1];
} resource_descriptor_probe;

typedef struct link_body_probe {
    u16 marker;
    const volatile u16 FAR *target;
    u16 expected_key;
} link_body_probe;
#pragma pack()

extern volatile u16 resource_requested_id_probe;
extern volatile u16 resource_active_id_probe;
extern volatile u16 resource_flags_probe;
extern volatile u32 resource_range_start_probe;
extern volatile u32 resource_range_remaining_probe;
extern volatile u32 resource_source_offset_probe;
extern volatile u32 resource_source_remaining_probe;
extern volatile u16 list_iteration_count_probe;
extern volatile u16 list_wrap_count_probe;
extern volatile u16 list_read_wrap_limit_probe;
extern volatile u16 list_secondary_wrap_limit_probe;
extern volatile u16 list_head_offset_probe;
extern volatile u16 list_head_segment_probe;
extern volatile u8 list_rollover_state_probe;

resource_descriptor_probe NEAR *NEAR descriptor_lookup_probe(u16 resource_id);
int NEAR queue_has_room_probe(u16 byte_count);
int NEAR list_read_probe(u16 *entry_extent, u16 *cursor_offset);
int NEAR paged_read_probe(u16 byte_count);
void NEAR queue_wrap_probe(u16 byte_count, u16 cursor_offset);
void NEAR queue_enqueue_probe(u16 byte_count);
void NEAR queue_finish_probe(void);
void NEAR wrap_bounds_reset_probe(void);

#if defined(__WATCOMC__)
#pragma aux descriptor_lookup_probe parm [ax] value [bx] modify [bx]
#endif

void NEAR list_d8c_refill_probe(u16 link_target_offset)
{
    const volatile resource_descriptor_probe NEAR *descriptor;
    const volatile cached_range_probe NEAR *cached_range;
    volatile link_body_probe FAR *link;
    volatile u16 FAR *entry_header;
    const volatile u16 FAR *link_target;
    u16 entry_extent;
    u16 cursor_offset;
    u16 chunk;
    u16 window_bytes;
    u16 previous_wrap_count;
    u16 entry_offset;
    u16 buffer_segment;
    u16 link_index;
    u16 target_extent;
    u8 check_source_first;

    check_source_first = 0u;
    for (;;) {
        if (!check_source_first) {
            chunk = list_iteration_count_probe;
            if (chunk != 0u) {
                if ((i8)(u8)resource_flags_probe >= 0) {
                    window_bytes = (u16)(-(u16)resource_source_offset_probe);
                    window_bytes &= WINDOW_MASK;
                    window_bytes += WINDOW_LOOKAHEAD;
                    if (window_bytes < chunk) {
                        chunk = window_bytes;
                    }
                }
                if (!queue_has_room_probe(chunk)) {
                    return;
                }
                list_iteration_count_probe -= chunk;
                (void)paged_read_probe(chunk);
                return;
            }
        }
        check_source_first = 0u;

        if (list_wrap_count_probe != list_secondary_wrap_limit_probe &&
                resource_source_remaining_probe != 0UL) {
            if (!list_read_probe(&entry_extent, &cursor_offset)) {
                return;
            }
            queue_wrap_probe(entry_extent, cursor_offset);
            continue;
        }

        if (((u8)resource_flags_probe & ROLLOVER_ENABLED) == 0u) {
            queue_finish_probe();
            return;
        }
        if (!queue_has_room_probe(ROLLOVER_RESERVATION)) {
            return;
        }

        previous_wrap_count = list_wrap_count_probe;
        wrap_bounds_reset_probe();
        list_read_wrap_limit_probe = previous_wrap_count;
        list_rollover_state_probe = 0u;

        if (resource_active_id_probe != resource_requested_id_probe) {
            descriptor = descriptor_lookup_probe(resource_active_id_probe);
            cached_range = (const volatile cached_range_probe NEAR *)
                    ((const volatile u8 NEAR *)descriptor -
                    sizeof(cached_range_probe));
            if ((descriptor->flags & CACHED_RANGE_VALID) == 0u ||
                    (u16)(cached_range->source_offset >> 16) == 0u) {
                resource_requested_id_probe = resource_active_id_probe;
                return;
            }
            resource_requested_id_probe = resource_active_id_probe;
            resource_range_start_probe = cached_range->source_offset;
            resource_range_remaining_probe = cached_range->source_remaining;
            resource_flags_probe = (resource_flags_probe & 0xFF00u) |
                    descriptor->flags;
        }

        resource_source_remaining_probe = resource_range_remaining_probe;
        resource_source_offset_probe = resource_range_start_probe;

        if (((u8)resource_flags_probe & SYNTHESIZE_LINKS) != 0u) {
            for (link_index = 0u; link_index < LINK_COUNT; ++link_index) {
                entry_offset = list_head_offset_probe;
                buffer_segment = list_head_segment_probe;
                entry_header = (volatile u16 FAR *)MK_FP(
                        buffer_segment, entry_offset);
                queue_enqueue_probe(2u);
                *entry_header = LINK_EXTENT;
                queue_wrap_probe(LINK_EXTENT, (u16)(entry_offset + 2u));

                link_target = (const volatile u16 FAR *)MK_FP(
                        buffer_segment, link_target_offset);
                target_extent = *link_target;
                link = (volatile link_body_probe FAR *)MK_FP(
                        buffer_segment, list_head_offset_probe);
                link->marker = LINK_MARKER;
                link->target = link_target;
                link->expected_key = target_extent;
                link_target_offset += target_extent;
                queue_enqueue_probe(LINK_BODY_BYTES);
            }
            list_rollover_state_probe = 0x80u;
        }
        check_source_first = 1u;
    }
}
