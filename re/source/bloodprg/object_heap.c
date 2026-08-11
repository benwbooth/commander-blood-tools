#include "bloodprg/object_heap.h"

#define CB_LOOKUP_ENTRY_SIZE 0x14u
#define CB_LOOKUP_OBJECT_OFF 0x10u
#define CB_LOOKUP_STATE_OFF 0x12u

static cb_u16 cb_read16_far(const cb_u8 CB_FAR *p)
{
    return (cb_u16)(p[0] | ((cb_u16)p[1] << 8));
}
/*
 * BLOODPRG 0x00149B.
 *
 * Assembly source:
 * re/assembly/bloodprg/seg_008b/func_00149b_object_heap_access.asm
 *
 * The loop processes the current 20-byte lookup entry, advances to the next
 * entry, then tests the next entry's state word for continuation.
 */
void CB_NEAR cb_bloodprg_00149b_object_heap_access(
    cb_u8 CB_FAR *object_heap,
    const cb_u8 CB_FAR *lookup_table)
{
    const cb_u8 CB_FAR *entry;
    cb_u16 object_off;
    cb_u16 object_flags;

    entry = lookup_table;
    for (;;) {
        object_off = cb_read16_far(entry + CB_LOOKUP_OBJECT_OFF);
        object_flags = cb_read16_far(object_heap + object_off);

        if ((object_flags & 0x0118u) != 0 &&
            (object_heap[(cb_u16)(object_off + 2u)] & 2u) != 0) {
            object_heap[(cb_u16)(object_off + 0x14u)] =
                (cb_u8)(object_heap[(cb_u16)(object_off + 0x14u)] + 1u);
        }

        entry += CB_LOOKUP_ENTRY_SIZE;
        if (cb_read16_far(entry + CB_LOOKUP_STATE_OFF) != 1u) {
            break;
        }
    }
}

/*
 * BLOODPRG 0x00604E.
 *
 * Assembly source:
 * re/assembly/bloodprg/seg_04da/func_00604e_active_object_list_build.asm
 *
 * Builds a 0xffff-terminated list of active object offsets from the same
 * 20-byte lookup table. The original stops at the first lookup entry whose
 * state word is not 1.
 */
void CB_NEAR cb_bloodprg_00604e_active_object_list_build(
    const cb_u8 CB_FAR *lookup_table,
    const cb_u8 CB_FAR *object_heap,
    cb_u16 CB_FAR *out_object_offsets)
{
    const cb_u8 CB_FAR *entry;
    cb_u16 object_off;

    entry = lookup_table;
    for (;;) {
        if (cb_read16_far(entry + CB_LOOKUP_STATE_OFF) != 1u) {
            break;
        }

        object_off = cb_read16_far(entry + CB_LOOKUP_OBJECT_OFF);
        if ((object_heap[(cb_u16)(object_off + 2u)] & 2u) != 0) {
            *out_object_offsets = object_off;
            ++out_object_offsets;
        }

        entry += CB_LOOKUP_ENTRY_SIZE;
    }

    *out_object_offsets = 0xffffu;
}
