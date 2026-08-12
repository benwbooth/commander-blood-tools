/*
 * Codegen probe for BLOODPRG 0x006339.
 * This is not recovered game source.
 */
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

extern const signed char FAR field_offset_table[];
extern volatile u16 history_ring_index;
extern volatile u16 FAR *history_words;
extern volatile u8 text_word_list_mode;
extern volatile u8 yield_flag;
extern volatile u16 presentation_word_buffer[];

extern u16 FAR prng_next(u16 modulus);
extern const u8 NEAR *NEAR token_special(u16 terminator,
        const u8 NEAR *script_bytes);

#if defined(__WATCOMC__)
#pragma aux token_special parm [ax] [si] value [si] modify exact [si]
#pragma aux condition_5_probe parm [cx] [es di] [si] value [ax] modify exact [ax bx dx]
#endif

int NEAR condition_5_probe(u16 flags,
        const volatile u8 FAR *record,
        const u8 NEAR *script_bytes)
{
    const u8 NEAR *cursor;
    const u16 NEAR *candidate;
    const u16 NEAR *words;
    volatile u16 NEAR *out;
    u8 control;
    u8 detail;
    u8 required;
    u8 count;
    u8 ring_offset;
    u8 i;
    u16 operand;
    u16 record_word;
    u16 history_word;
    u16 offset;

    cursor = script_bytes;
    control = (u8)flags;
    detail = (u8)(flags >> 8);

    if ((control & 0x02u) != 0 && prng_next(5) != 0) {
        return 0;
    }

    if ((control & 0x04u) != 0) {
        offset = (u8)field_offset_table[
            ((((u16)detail >> 1) & 7u) + 1u) * 16u + 1u];
        record_word = *(const volatile u16 FAR *)(record + offset);
        operand = *(const u16 NEAR *)cursor;
        cursor += 2;

        if ((control & 0x80u) != 0) {
            if ((i16)operand >= (i16)record_word) {
                return 0;
            }
        } else if ((detail & 1u) != 0) {
            if (record_word != operand) {
                return 0;
            }
        } else if ((i16)record_word <= (i16)operand) {
            return 0;
        }
    }

    if ((control & 0x40u) != 0) {
        cursor = token_special(0xffffu, cursor);
        required = (u8)(detail & 7u);
        if (required == 0) {
            words = (const u16 NEAR *)cursor;
            count = 0;
            while (words[count] != 0) {
                ++count;
            }
            if (count != 0) {
                ring_offset = (u8)((history_ring_index - 2u) & 0x0fu);
                while (count != 0) {
                    history_word = history_words[ring_offset >> 1];
                    candidate = words;
                    while (*candidate != history_word && *candidate != 0) {
                        ++candidate;
                    }
                    if (*candidate == 0) {
                        return 0;
                    }
                    ring_offset = (u8)((ring_offset - 2u) & 0x0fu);
                    --count;
                }
            }
        } else {
            for (;;) {
                operand = *(const u16 NEAR *)cursor;
                cursor += 2;
                if (operand == 0 || operand == 0xffffu) {
                    return 0;
                }

                for (i = 0; i != 8u; ++i) {
                    if (operand == history_words[i]) {
                        --required;
                        if (required == 0) {
                            break;
                        }
                    }
                }
                if (required == 0) {
                    break;
                }
            }
        }
    }

    if ((control & 0x20u) != 0) {
        text_word_list_mode = 1;
    }

    if ((control & 0x10u) != 0) {
        out = presentation_word_buffer;
        cursor = token_special(0xffffu, cursor);
        yield_flag = 1;
        do {
            *out = *(const u16 NEAR *)cursor;
            cursor += 2;
            ++out;
        } while (out[-1] != 0);
    }

    return 1;
}
