#include "../include/bloodprg_vm.h"

int CB_FAR string_compare(const volatile char CB_NEAR *left,
        const volatile char CB_FAR *right)
{
    cb_u8 ch;

    for (;;) {
        ch = (cb_u8)*left;
        if (ch != (cb_u8)*right) {
            return 0;
        }
        if (ch == 0) {
            return 1;
        }
        ++left;
        ++right;
    }
}
