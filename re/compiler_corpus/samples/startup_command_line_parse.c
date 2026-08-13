/* Codegen probe for BLOODPRG 0x0006F1. */

typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef struct command_tail {
    u8 length;
    char text[127];
} command_tail;

extern char argument_token[128];
extern void NEAR startup_option_apply(char *token);

void NEAR startup_command_line_parse_probe(
        const command_tail FAR *command_line)
{
    const char FAR *source;
    char *destination;
    u16 remaining;
    char value;

    remaining = command_line->length;
    source = command_line->text;
    while (remaining != 0u) {
        destination = argument_token;
        do {
            value = *source++;
            if (value == ' ') {
                break;
            }
            *destination++ = value;
            --remaining;
        } while (remaining != 0u);

        *destination = '\0';
        startup_option_apply(argument_token);
        if (remaining != 0u) {
            --remaining;
        }
    }
}
