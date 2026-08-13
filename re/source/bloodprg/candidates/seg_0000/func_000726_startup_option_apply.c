#include "../include/bloodprg_startup.h"

enum {
    STARTUP_OPTION_COPY_DIRECTORY = 0x01,
    STARTUP_OPTION_AUDIO_CONFIGURATION = 0x02
};

typedef struct bloodprg_startup_option {
    char prefix[3];
    cb_u8 actions;
    cb_u8 driver_id;
} bloodprg_startup_option;

/* SS:0x023A in the original data segment. */
static const bloodprg_startup_option startup_options[] = {
    {{'S', '1', '6'}, STARTUP_OPTION_AUDIO_CONFIGURATION, 0x2au},
    {{'M', 'I', 'D'}, 0u,                                  0x01u},
    {{'S', 'D', 'B'}, STARTUP_OPTION_AUDIO_CONFIGURATION, 0x2au},
    {{'S', 'B', 'P'}, STARTUP_OPTION_AUDIO_CONFIGURATION, 0x2au},
    {{'G', 'R', 'V'}, STARTUP_OPTION_AUDIO_CONFIGURATION, 0x01u},
    {{'W', 'R', 'I'}, STARTUP_OPTION_COPY_DIRECTORY,      0x00u},
    {{'\0', '\0', '\0'}, 0u,                              0x00u}
};

void CB_NEAR startup_option_apply(char *token)
{
    const bloodprg_startup_option *option;

    option = startup_options;
    while (option->prefix[0] != '\0') {
        if (option->prefix[0] == token[0] &&
            option->prefix[1] == token[1] &&
            option->prefix[2] == token[2]) {
            char *suffix;

            suffix = token + 3;
            if ((option->actions & STARTUP_OPTION_COPY_DIRECTORY) != 0u) {
                char *destination;

                destination = startup_write_directory;
                while (*suffix != '\0') {
                    *destination++ = *suffix++;
                }

                /* The shipped WRI argument ends in a separator to remove. */
                if (destination == startup_write_directory) {
                    startup_original_drive = 0u;
                } else {
                    destination[-1] = '\0';
                }
            } else if ((option->actions &
                        STARTUP_OPTION_AUDIO_CONFIGURATION) != 0u) {
                cb_u8 final_digit;
                cb_u16 value;

                final_digit = (cb_u8)suffix[3];
                suffix[3] = '\0';
                value = (cb_u16)ascii_digit_parse(suffix);
                value = (cb_u16)(value << 4);
                value = (cb_u16)(
                        value | (cb_u8)(final_digit - (cb_u8)'0'));
                startup_audio_configuration = value;
                startup_audio_driver_id = option->driver_id;
            }
            return;
        }
        ++option;
    }
}
