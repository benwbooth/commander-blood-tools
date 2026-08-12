#include "../include/bloodprg_ship3d.h"

int CB_NEAR ship_3d_object_table_bit_test_full(cb_u16 object_offset,
        const volatile cb_u8 CB_NEAR *bitset_base)
{
    const volatile bloodprg_vm_directory_entry CB_FAR *entry;
    cb_u16 object_index;
    cb_u16 byte_offset;
    cb_u16 shifted;

    entry = vm_record_directory;
    object_index = 0;
    while (entry->object_offset != object_offset) {
        ++entry;
        ++object_index;
    }

    byte_offset = (cb_u16)((cb_u16)vm_field_offset(
        SHIP_3D_SOURCE_BITSET_SELECTOR, SHIP_3D_SOURCE_BITSET_KIND) +
        (object_index >> 3));
    shifted = (cb_u16)bitset_base[byte_offset];
    shifted <<= (cb_u16)((object_index & 7u) + 1u);
    return (shifted & 0x0100u) != 0;
}
