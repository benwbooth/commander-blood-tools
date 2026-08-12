#include "../include/bloodprg_common.h"

typedef struct lookup_table_1fb5_entry {
    cb_u16 value;
    cb_u16 unknown_02;
} lookup_table_1fb5_entry;

extern const volatile lookup_table_1fb5_entry lookup_table_1fb5_records[]; /* DS:0x1FB5 */

cb_u16 CB_NEAR lookup_table_1fb5(cb_u16 index)
{
    return lookup_table_1fb5_records[index].value;
}
