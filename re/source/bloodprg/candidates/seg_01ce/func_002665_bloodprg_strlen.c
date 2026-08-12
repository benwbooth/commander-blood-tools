#include "../include/bloodprg_common.h"

cb_u16 CB_FAR bloodprg_strlen(const volatile char CB_FAR *text)
{
    cb_u16 length;

    length = 0;
    while (length != 0xffffu) {
        if (*text == '\0') {
            return length;
        }
        ++text;
        ++length;
    }

    return 0xfffeu;
}
