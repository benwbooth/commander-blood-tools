/* Codegen probe for BLOODPRG 0x00A867. */
typedef unsigned char u8;
typedef signed char i8;
typedef unsigned int u16;
typedef signed int i16;

#define FAR far
#define NEAR near
#define GAME_DATA __based(__segname("GAME_DATA"))

#define READ_BYTE(value) \
    do { \
        (value) = *source++; \
    } while (0)

#define READ_WORD(value) \
    do { \
        (value) = *(const volatile u16 FAR *)source; \
        source += 2; \
    } while (0)

#define READ_BIT(value) \
    do { \
        (value) = (u16)(control_bits & 1u); \
        control_bits >>= 1; \
        if (control_bits == 0u) { \
            READ_WORD(control_word); \
            (value) = (u16)(control_word & 1u); \
            control_bits = (u16)((control_word >> 1) | 0x8000u); \
        } \
    } while (0)

extern volatile u16 GAME_DATA decode_mode;

void NEAR resource_payload_decode_ab_probe(
        const volatile u8 FAR *source,
        volatile u8 FAR *destination)
{
    const volatile u8 FAR *copy_source;
    u16 control_bits;
    u16 control_word;
    u16 bit;
    u16 length;
    i16 displacement;
    u8 value;

    decode_mode = 1u;
    source += 6;
    control_bits = 0u;

    for (;;) {
        READ_BIT(bit);
        if (bit != 0u) {
            READ_BYTE(value);
            *destination++ = value;
            continue;
        }

        READ_BIT(bit);
        if (bit == 0u) {
            length = 0u;
            READ_BIT(bit);
            length = (u16)((length << 1) | bit);
            READ_BIT(bit);
            length = (u16)((length << 1) | bit);
            READ_BYTE(value);
            displacement = (i16)(i8)value;
        } else {
            READ_WORD(control_word);
            length = (u16)(control_word & 7u);
            displacement = (i16)((control_word >> 3) | 0xE000u);
            if (length == 0u) {
                READ_BYTE(value);
                length = value;
                if (length == 0u) {
                    break;
                }
            }
        }

        length = (u16)(length + 2u);
        copy_source = destination + displacement;
        do {
            *destination++ = *copy_source++;
        } while (--length != 0u);
    }
}
