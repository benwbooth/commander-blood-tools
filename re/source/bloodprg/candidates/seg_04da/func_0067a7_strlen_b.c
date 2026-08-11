#include "../include/bloodprg_common.h"

cb_u16 CB_NEAR strlen_b(const char CB_FAR *s)
{
    const char CB_FAR *p;

    p = s;
    while (*p != '\0') {
        ++p;
    }

    return (cb_u16)(p - s);
}
