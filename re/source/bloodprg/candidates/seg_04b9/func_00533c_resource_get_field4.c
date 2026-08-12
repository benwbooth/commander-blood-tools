#include "../include/bloodprg_resource.h"

cb_u32 CB_FAR resource_get_field4(cb_u16 handle)
{
    return fs_resource_handle_table[handle].field_04;
}
