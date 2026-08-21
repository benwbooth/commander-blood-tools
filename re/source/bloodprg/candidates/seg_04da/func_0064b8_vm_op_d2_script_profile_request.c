#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_d2_script_profile_request(
    bloodprg_vm_image_ptr script_bytes)
{
    cb_i16 request;

    request = (cb_i16)(cb_i8)*script_bytes++ - 1;
    vm_script_profile_request = request;
    return script_bytes;
}
