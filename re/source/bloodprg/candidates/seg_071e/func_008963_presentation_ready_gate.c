#include <dos.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define PRESENTATION_WORD_CHOICE_UI_FLAG 0x04u
#define PRESENTATION_WORD_CHOICE_PHASE_MASK 0x07u
#define PRESENTATION_WORD_CHOICE_CENTER_X 225u
#define PRESENTATION_WORD_CHOICE_STEPS 4u

#if defined(__TURBOC__) || defined(__BORLANDC__)
#define PRESENTATION_WORD_CHOICE_TURBO 1
#pragma warn -rch
#else
#define PRESENTATION_WORD_CHOICE_TURBO 0
#endif

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define PRESENTATION_WORD_CHOICE_SEGMENT(pointer) \
    FP_SEG((const void CB_FAR *)(pointer))
#else
#define PRESENTATION_WORD_CHOICE_SEGMENT(pointer) 0u
#endif

#if defined(__WATCOMC__)
static cb_i16 CB_NEAR presentation_word_choice_widget(
        const cb_u16 CB_NEAR *items,
        cb_u16 string_segment);
#pragma aux presentation_word_choice_widget = \
        "mov es,dx" \
        "call far ptr list_widget_layout_unified" \
        parm [si] [dx] value [ax] modify exact [ax es]
#elif !PRESENTATION_WORD_CHOICE_TURBO
static cb_i16 CB_NEAR presentation_word_choice_widget(
        const cb_u16 CB_NEAR *items,
        cb_u16 string_segment)
{
    (void)string_segment;
    return list_widget_layout_unified(items);
}
#endif

void CB_FAR presentation_ready_gate(void)
{
    const volatile cb_u16 CB_NEAR *items;
    cb_i16 selection;
    cb_u16 string_segment;
    int transition_complete;
#if PRESENTATION_WORD_CHOICE_TURBO
    const bloodprg_rect_i16 CB_NEAR *transition_source;
    const bloodprg_rect_i16 CB_NEAR *transition_target;
    void (CB_FAR *transition_step)(
            const bloodprg_rect_i16 CB_NEAR *,
            const bloodprg_rect_i16 CB_NEAR *);

    /* Preserve ordinary extern records for the TASM-owned ABI calls below. */
    if (0) {
        (void)list_widget_layout_unified(
                (const cb_u16 *)vm_presentation_word_buffer);
        framebuffer_rect_interpolate_and_remap_step(
                (const bloodprg_rect_i16 CB_NEAR *)
                    presentation_choice_current_rect,
                (const bloodprg_rect_i16 CB_NEAR *)
                    &presentation_word_choice_target_rect);
    }
#endif

#if PRESENTATION_WORD_CHOICE_TURBO
    asm push ax;
    asm push es;
#elif defined(__WATCOMC__)
    _asm push ax;
    _asm push es;
#endif

    if ((vm_presentation_active & 1u) == 0u
            || (vm_word_choice_active & 1u) == 0u
            || (vm_presentation_request_flags & 2u) != 0u) {
        goto restore_registers;
    }

    items = vm_presentation_word_buffer;
    if (*items == 0u) {
        goto restore_registers;
    }
    string_segment = PRESENTATION_WORD_CHOICE_SEGMENT(vm_dic_words);

    if ((vm_presentation_word_choice_phase
            & PRESENTATION_WORD_CHOICE_PHASE_MASK) == 0u) {
        vm_ui_flags |= PRESENTATION_WORD_CHOICE_UI_FLAG;
        ++vm_presentation_word_choice_phase;
        ship_3d_target_layout_preserve_widths = 0u;
        ship_3d_target_layout_center_x = PRESENTATION_WORD_CHOICE_CENTER_X;
        ship_3d_target_layout_extra_entry = 0u;
        framebuffer_transition_current_step = 0u;
        framebuffer_transition_total_steps = PRESENTATION_WORD_CHOICE_STEPS;
        presentation_list_editing = 1u;
#if PRESENTATION_WORD_CHOICE_TURBO
        asm mov si, items;
        asm mov es, string_segment;
        asm call far ptr _list_widget_layout_unified;
#else
        (void)presentation_word_choice_widget(
                (const cb_u16 CB_NEAR *)items, string_segment);
#endif
        presentation_list_editing = 0u;
        presentation_word_choice_target_rect.x =
                presentation_choice_current_rect[0];
        presentation_word_choice_target_rect.width =
                presentation_choice_current_rect[2];
    }

    if ((vm_presentation_word_choice_phase & 2u) == 0u) {
        transition_complete = framebuffer_transition_current_step
                == framebuffer_transition_total_steps;
#if PRESENTATION_WORD_CHOICE_TURBO
        transition_source = (const bloodprg_rect_i16 CB_NEAR *)
                presentation_choice_current_rect;
        transition_target = (const bloodprg_rect_i16 CB_NEAR *)
                &presentation_word_choice_target_rect;
        transition_step = framebuffer_rect_interpolate_and_remap_step;
        asm push si;
        asm mov si, transition_source;
        asm mov di, transition_target;
        asm call dword ptr transition_step;
        asm pop si;
#else
        framebuffer_rect_interpolate_and_remap_step(
                (const bloodprg_rect_i16 CB_NEAR *)
                    presentation_choice_current_rect,
                (const bloodprg_rect_i16 CB_NEAR *)
                    &presentation_word_choice_target_rect);
#endif
        if (!transition_complete) {
            goto restore_registers;
        }
        ++vm_presentation_word_choice_phase;
    }

    if ((vm_presentation_word_choice_phase & 1u) == 0u) {
#if PRESENTATION_WORD_CHOICE_TURBO
        asm mov si, items;
        asm mov es, string_segment;
        asm call far ptr _list_widget_layout_unified;
        asm mov selection, ax;
#else
        selection = presentation_word_choice_widget(
                (const cb_u16 CB_NEAR *)items, string_segment);
#endif
        if (selection < 0) {
            goto restore_registers;
        }
        vm_presentation_selected_word = items[(cb_u16)selection];
        framebuffer_transition_current_step = 0u;
        ++vm_presentation_word_choice_phase;
        goto restore_registers;
    }

    transition_complete = framebuffer_transition_current_step
            == framebuffer_transition_total_steps;
#if PRESENTATION_WORD_CHOICE_TURBO
    transition_source = (const bloodprg_rect_i16 CB_NEAR *)
            &presentation_word_choice_target_rect;
    transition_target = (const bloodprg_rect_i16 CB_NEAR *)
            presentation_choice_current_rect;
    transition_step = framebuffer_rect_interpolate_and_remap_step;
    asm mov si, transition_source;
    asm mov di, transition_target;
    asm call dword ptr transition_step;
#else
    framebuffer_rect_interpolate_and_remap_step(
            (const bloodprg_rect_i16 CB_NEAR *)
                &presentation_word_choice_target_rect,
            (const bloodprg_rect_i16 CB_NEAR *)
                presentation_choice_current_rect);
#endif
    if (!transition_complete) {
        goto restore_registers;
    }

    vm_block_match_value = vm_presentation_selected_word;
    vm_word_choice_active = 0u;
    vm_presentation_defer_a = 0u;
    vm_text_display_active = 0u;
    vm_dialogue_hold_complete = 0u;
    vm_presentation_word_choice_phase = 0u;
    vm_presentation_word_buffer[0] = 0u;
    vm_presentation_request_flags &= (cb_u8)~1u;

restore_registers:
#if PRESENTATION_WORD_CHOICE_TURBO
    asm pop es;
    asm pop ax;
#elif defined(__WATCOMC__)
    _asm pop es;
    _asm pop ax;
#endif
}

#if PRESENTATION_WORD_CHOICE_TURBO
#pragma warn .rch
#endif

#undef PRESENTATION_WORD_CHOICE_SEGMENT
#undef PRESENTATION_WORD_CHOICE_TURBO
