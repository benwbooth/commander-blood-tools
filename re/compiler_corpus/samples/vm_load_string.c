/*
 * Codegen probe for BLOODPRG 0x0067C8.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u8 string_buffer[];
extern volatile u8 finale_requested;
extern volatile u8 request_flags;
extern volatile u16 ship_flags;
extern volatile u8 scene_gate;
extern volatile u16 active_line;
extern volatile u8 presentation_gate;
extern volatile u16 actor_record;
extern volatile u8 dialog_gate;

#if defined(__WATCOMC__)
#pragma aux vm_load_string_probe parm [si] value [si] modify exact [ax bp si]
#endif

const u8 NEAR *NEAR vm_load_string_probe(const u8 NEAR *script_bytes)
{
    volatile u8 *dst;
    u8 ch;

    dst = string_buffer;
    do {
        ch = *script_bytes++;
        *dst++ = ch;
    } while (ch != 0);
    ++script_bytes;

    if (string_buffer[0] == 'f' && string_buffer[1] == 'i' &&
            string_buffer[2] == 'n' && string_buffer[3] == '.') {
        finale_requested = 1;
    }
    if ((request_flags & 2u) == 0 &&
            ((ship_flags & 1u) != 0 || (scene_gate & 1u) != 0)) {
        active_line = 7;
        request_flags |= 2u;
        presentation_gate = 0;
        actor_record = 0xffffu;
        dialog_gate = 0;
    }

    return script_bytes;
}
