#include "../include/bloodprg_startup.h"

void CB_NEAR startup_command_line_parse(
        const bloodprg_command_tail CB_FAR *command_tail)
{
    const char CB_FAR *source;
    char *destination;
    cb_u16 remaining;
    char value;

    remaining = command_tail->length;
    source = command_tail->text;
    while (remaining != 0u) {
        destination = startup_argument_token;
        do {
            value = *source++;
            if (value == ' ') {
                break;
            }
            *destination++ = value;
            --remaining;
        } while (remaining != 0u);

        *destination = '\0';
        startup_option_apply(startup_argument_token);
        if (remaining != 0u) {
            --remaining;
        }
    }
}
