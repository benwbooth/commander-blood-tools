/* Codegen probe for BLOODPRG 0x000726. */

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA FAR
#endif

enum {
    COPY_DIRECTORY = 0x01,
    AUDIO_CONFIGURATION = 0x02
};

typedef struct startup_option {
    char prefix[3];
    u8 actions;
    u8 driver_id;
} startup_option;

static const startup_option startup_options[] = {
    {{'S', '1', '6'}, AUDIO_CONFIGURATION, 0x2au},
    {{'M', 'I', 'D'}, 0u,                  0x01u},
    {{'S', 'D', 'B'}, AUDIO_CONFIGURATION, 0x2au},
    {{'S', 'B', 'P'}, AUDIO_CONFIGURATION, 0x2au},
    {{'G', 'R', 'V'}, AUDIO_CONFIGURATION, 0x01u},
    {{'W', 'R', 'I'}, COPY_DIRECTORY,      0x00u},
    {{'\0', '\0', '\0'}, 0u,              0x00u}
};

extern u8 current_drive;
extern char write_directory[32];
extern volatile u8 GAME_DATA audio_driver_id;
extern volatile u16 GAME_DATA audio_configuration;
extern i16 FAR ascii_digit_parse(const char *text);

#if defined(__WATCOMC__)
#pragma aux ascii_digit_parse parm [si] value [ax] modify exact [ax]
#endif

void NEAR startup_option_apply_probe(char *token)
{
    const startup_option *option;

    option = startup_options;
    while (option->prefix[0] != '\0') {
        if (option->prefix[0] == token[0] &&
            option->prefix[1] == token[1] &&
            option->prefix[2] == token[2]) {
            char *suffix;

            suffix = token + 3;
            if ((option->actions & COPY_DIRECTORY) != 0u) {
                char *destination;

                destination = write_directory;
                while (*suffix != '\0') {
                    *destination++ = *suffix++;
                }
                if (destination == write_directory) {
                    current_drive = 0u;
                } else {
                    destination[-1] = '\0';
                }
            } else if ((option->actions & AUDIO_CONFIGURATION) != 0u) {
                u8 final_digit;
                u16 value;

                final_digit = (u8)suffix[3];
                suffix[3] = '\0';
                value = (u16)ascii_digit_parse(suffix);
                value = (u16)(value << 4);
                value = (u16)(value | (u8)(final_digit - (u8)'0'));
                audio_configuration = value;
                audio_driver_id = option->driver_id;
            }
            return;
        }
        ++option;
    }
}
