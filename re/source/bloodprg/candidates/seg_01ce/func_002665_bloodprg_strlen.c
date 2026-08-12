#include "../include/bloodprg_common.h"

cb_u16 CB_FAR bloodprg_strlen(const volatile char CB_FAR *text)
{
    const volatile char CB_FAR *end;

    end = text;
    while (*end != '\0') {
        ++end;
    }

    return (cb_u16)(end - text);
}
