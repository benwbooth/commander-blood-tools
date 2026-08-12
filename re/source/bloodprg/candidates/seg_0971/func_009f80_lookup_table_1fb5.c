#include "../include/bloodprg_common.h"

typedef struct presentation_line_record {
    cb_u16 flags;
} presentation_line_record;

typedef struct presentation_line_index_entry {
    presentation_line_record *record;
    cb_u16 asset_name_offset;
} presentation_line_index_entry;

extern volatile presentation_line_index_entry presentation_line_index[]; /* DS:0x1FB5 */

#if defined(__WATCOMC__)
#pragma aux lookup_table_1fb5 parm [ax] value [bx] modify [bx]
#endif

presentation_line_record *CB_NEAR lookup_table_1fb5(cb_u16 index)
{
    return presentation_line_index[index].record;
}
