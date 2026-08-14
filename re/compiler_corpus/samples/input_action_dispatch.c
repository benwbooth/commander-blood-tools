/* Codegen probe for BLOODPRG 0x00210E. */

typedef unsigned char u8;
typedef signed char i8;
typedef unsigned int u16;

#define FAR far
#define NEAR near
#define CODE_DATA __based(__segname("_CODE"))

typedef void (NEAR *input_action_handler_probe)(u8 raw_low_byte);

extern volatile u8 input_dispatch_state_probe;
extern const i8 CODE_DATA input_translation_probe[256];
extern input_action_handler_probe CODE_DATA input_handlers_probe[];

u16 FAR kbd_read_int16_probe(void);

#pragma aux kbd_read_int16_probe value [ax] modify exact [ax]

void FAR input_action_dispatch_probe(void)
{
    u16 key;
    u8 raw_low_byte;
    u8 translated_code;
    i8 action_index;

    input_dispatch_state_probe = 0u;
    key = kbd_read_int16_probe();
    if (key == 0u) {
        return;
    }

    raw_low_byte = (u8)key;
    translated_code = raw_low_byte;
    if (translated_code == 0u) {
        translated_code = (u8)((key >> 8) | 0x80u);
    }

    action_index = input_translation_probe[translated_code];
    if (action_index >= 0) {
        input_handlers_probe[(u8)action_index](raw_low_byte);
    }
}
