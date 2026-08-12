#include "../include/bloodprg_resource.h"

bloodprg_resource_descriptor *CB_NEAR lookup_table_1fb5(cb_u16 index)
{
    return resource_index[index].descriptor;
}
