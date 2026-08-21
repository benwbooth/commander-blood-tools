#include "../include/bloodprg_vm.h"

#define VM_TEXT_PRESERVE_ACTIVE 0x0001u
#define VM_TEXT_EXTRA_CONTROL_WORD 0x0004u
#define VM_TEXT_ARM_SKIP 0x0008u
#define VM_TEXT_ARM_LOOP 0x0010u
#define VM_TEXT_ACTIVE 0x8000u
#define VM_TEXT_ALREADY_SHOWN 0x8000u
#define VM_TEXT_PRESENTATION_FIELD_INDEX 0x0131u
#define VM_TEXT_PRESENTATION_RECORD 0x00C4u
#define VM_TEXT_LINE_LIMIT 35u

cb_u8 CB_NEAR *CB_NEAR vm_op_a6_text(cb_u8 CB_NEAR *script_bytes)
{
    volatile cb_u8 CB_FAR *line_record;
    cb_u8 CB_NEAR *selector_bytes;
    const char CB_FAR *dictionary_word;
    volatile char CB_GAME_DATA *output;
    cb_u16 control;
    cb_u16 dictionary_offset;
    cb_u8 line_length;
    cb_u8 next_length;
    cb_i16 field_offset;

    line_record = vm_record_base_gs;
    line_record += *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);

    selector_bytes = script_bytes;
    vm_text_selector_bytes_gs = selector_bytes;
    ++script_bytes;

    control = *(const cb_u16 CB_NEAR *)script_bytes;
    script_bytes += sizeof(cb_u16);

    if ((control & VM_TEXT_ARM_SKIP) != 0) {
        vm_skip_count_gs = (cb_u8)(((control >> 12) & 7u) + 1u);
    }
    if ((control & VM_TEXT_ARM_LOOP) != 0) {
        vm_resume_state_gs = 1;
        vm_resume_value_gs = 0;
        vm_text_loop_target_gs = *(const cb_u16 CB_NEAR *)script_bytes;
        script_bytes += sizeof(cb_u16);
    }

    if ((control & VM_TEXT_ACTIVE) == 0 ||
            (vm_text_display_active_gs | vm_presentation_defer_a_gs) != 0 ||
            (*(volatile cb_u16 CB_FAR *)(line_record + 2) &
                VM_TEXT_ALREADY_SHOWN) != 0) {
        goto consume_to_end;
    }

    field_offset = (cb_i8)vm_field_offset_table_gs[
        VM_TEXT_PRESENTATION_FIELD_INDEX];
    if (*(volatile cb_u16 CB_FAR *)(line_record + field_offset) !=
            VM_TEXT_PRESENTATION_RECORD ||
            !vm_condition_5(control, line_record, script_bytes)) {
        goto consume_to_end;
    }

    vm_text_selector_gs = (cb_i8)*selector_bytes;
    if ((control & VM_TEXT_PRESERVE_ACTIVE) == 0) {
        selector_bytes[2] &= (cb_u8)~(VM_TEXT_ACTIVE >> 8);
    }
    if ((control & VM_TEXT_EXTRA_CONTROL_WORD) != 0) {
        script_bytes += sizeof(cb_u16);
    }

    if ((vm_text_word_list_mode_gs & 1u) != 0) {
        vm_text_voice_trigger_gs = 1;
        vm_text_mode_0cfa_gs = 0;
        vm_presentation_defer_a_gs = 0;
        vm_text_word_list_mode_gs = 0;
        *(volatile cb_u16 CB_FAR *)(line_record + 2) |=
            VM_TEXT_ALREADY_SHOWN;

        output = vm_text_buffer_gs;
        line_length = 0;
        for (;;) {
            dictionary_offset = *(const cb_u16 CB_NEAR *)script_bytes;
            if (dictionary_offset == 0 || dictionary_offset == 0xFFFFu) {
                break;
            }
            script_bytes += sizeof(cb_u16);

            dictionary_word = vm_dic_words_gs + dictionary_offset;
            while (*dictionary_word != '\0') {
                *output++ = *dictionary_word++;
                ++line_length;
            }

            dictionary_word = vm_dic_words_gs +
                *(const cb_u16 CB_NEAR *)script_bytes;
            next_length = (cb_u8)strlen_b(dictionary_word);
            if (*dictionary_word == ',' || *dictionary_word == '.' ||
                    *dictionary_word == '?' || *dictionary_word == '!' ||
                    *dictionary_word == ':') {
                continue;
            }

            *output++ = ' ';
            ++line_length;
            if ((cb_u8)(next_length + line_length) >= VM_TEXT_LINE_LIMIT) {
                line_length = 0;
                *output++ = '\r';
            }
        }

        *output++ = '\r';
        *output = '\0';
        vm_text_display_active_gs = 1;
        vm_text_reveal_cursor_gs = 0;
        vm_yield_flag_gs += 2;
        vm_presentation_hold_ready_gs = 0;
        vm_presentation_request_flags_gs |= 1;
    } else {
        scan_zero_word((const cb_i16 CB_NEAR *)script_bytes);
        *(volatile cb_u16 CB_FAR *)(line_record + 2) |=
            VM_TEXT_ALREADY_SHOWN;
        vm_text_display_active_gs = 0;
        vm_text_mode_0cf9_gs = 1;
        vm_presentation_request_flags_gs |= 1;
        vm_yield_flag_gs += 2;
        vm_presentation_defer_a_gs = 1;
        vm_presentation_hold_ready_gs = 0;
        vm_text_menu_pending_gs = 1;
        vm_text_menu_end_gs = (cb_u16)script_bytes;
        vm_text_menu_words_gs = (const cb_u16 CB_FAR *)script_bytes;
    }

consume_to_end:
    while (*(const cb_u16 CB_NEAR *)script_bytes != 0) {
        script_bytes += sizeof(cb_u16);
    }
    return script_bytes + sizeof(cb_u16);
}
