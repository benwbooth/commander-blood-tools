#include "../include/bloodprg_vm.h"

static cb_u16 vm_condition_word(const cb_u8 *bytes)
{
    return *(const cb_u16 *)bytes;
}

static int vm_condition_word_in_list(cb_u16 word, const cb_u16 *list)
{
    while (*list != 0) {
        if (*list == word) {
            return 1;
        }
        ++list;
    }
    return 0;
}

int CB_NEAR vm_condition_5(cb_u16 flags,
        const volatile cb_u8 CB_FAR *record,
        const cb_u8 *script_bytes)
{
    const cb_u8 *cursor;
    const cb_u16 *words;
    volatile cb_u16 *out;
    cb_u8 control;
    cb_u8 detail;
    cb_u8 required;
    cb_u8 count;
    cb_u8 ring_offset;
    cb_u8 i;
    cb_u16 operand;
    cb_u16 record_word;
    cb_u16 history_word;
    cb_u16 field_offset;

    cursor = script_bytes;
    control = (cb_u8)flags;
    detail = (cb_u8)(flags >> 8);

    if ((control & 0x02u) != 0 && blood_prng_next(5) != 0) {
        return 0;
    }

    if ((control & 0x04u) != 0) {
        field_offset = (cb_u8)vm_field_offset_table[
            ((((cb_u16)detail >> 1) & 7u) + 1u) * 16u + 1u];
        record_word = *(const volatile cb_u16 CB_FAR *)(record + field_offset);
        operand = vm_condition_word(cursor);
        cursor += 2;

        if ((control & 0x80u) != 0) {
            if ((cb_i16)operand >= (cb_i16)record_word) {
                return 0;
            }
        } else if ((detail & 1u) != 0) {
            if (record_word != operand) {
                return 0;
            }
        } else if ((cb_i16)record_word <= (cb_i16)operand) {
            return 0;
        }
    }

    if ((control & 0x40u) != 0) {
        vm_token_special(&cursor, 0xffffu);
        required = (cb_u8)(detail & 7u);
        if (required == 0) {
            words = (const cb_u16 *)cursor;
            count = 0;
            while (words[count] != 0) {
                ++count;
            }
            if (count != 0) {
                ring_offset = (cb_u8)((vm_blood_history_ring_index - 2u) & 0x0fu);
                while (count != 0) {
                    history_word = vm_blood_history_words[ring_offset >> 1];
                    if (!vm_condition_word_in_list(history_word, words)) {
                        return 0;
                    }
                    ring_offset = (cb_u8)((ring_offset - 2u) & 0x0fu);
                    --count;
                }
            }
        } else {
            for (;;) {
                operand = vm_condition_word(cursor);
                cursor += 2;
                if (operand == 0 || operand == 0xffffu) {
                    return 0;
                }

                for (i = 0; i != 8u; ++i) {
                    if (operand == vm_blood_history_words[i]) {
                        --required;
                        break;
                    }
                }
                if (required == 0) {
                    break;
                }
            }
        }
    }

    if ((control & 0x20u) != 0) {
        vm_text_word_list_mode = 1;
    }

    if ((control & 0x10u) != 0) {
        out = vm_presentation_word_buffer;
        vm_token_special(&cursor, 0xffffu);
        vm_yield_flag = 1;
        do {
            *out = vm_condition_word(cursor);
            cursor += 2;
            ++out;
        } while (out[-1] != 0);
    }

    return 1;
}
