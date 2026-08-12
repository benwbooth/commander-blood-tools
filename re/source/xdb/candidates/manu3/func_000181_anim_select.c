#include "../include/xdb_manu3.h"

void XDB_NEAR xdb_manu3_anim_select(xdb_u16 selector)
{
    xdb_u16 table_offset = xdb_manu3_sequence_table_offset;
    volatile xdb_u16 XDB_NEAR *relative_offsets =
            (volatile xdb_u16 XDB_NEAR *)table_offset;

    selector &= 0x001fu;
    xdb_manu3_tween_phase = 0;
    xdb_manu3_tween_script_offset = (xdb_u16)(
            table_offset + relative_offsets[selector]);
    xdb_manu3_tween_constructor(xdb_manu3_active_slot_offsets);
}
