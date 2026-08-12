#include "../include/bloodprg_vm.h"

cb_u16 CB_NEAR strlen_b(const char CB_FAR *s)
{
    cb_u16 length;

    length = 0;
    while (length != 0xffffu) {
        if (*s == '\0') {
            return length;
        }
        ++s;
        ++length;
    }

    return 0xfffeu;
}
