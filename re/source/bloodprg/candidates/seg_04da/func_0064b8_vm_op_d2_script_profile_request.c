#include "../include/bloodprg_vm.h"

const cb_i8 CB_NEAR *CB_NEAR vm_op_d2_script_profile_request(
    const cb_i8 CB_NEAR *script_bytes)
{
    cb_i16 request;

    request = (int)*script_bytes++ - 1;
    vm_script_profile_request = request;
    return script_bytes;
}
