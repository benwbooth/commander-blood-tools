#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"

#define CONFIRM_DIALOG_ACTIVE 0x02u
#define CONFIRM_DIALOG_UI_ACTIVE 0x04u
#define CONFIRM_DIALOG_BACKGROUND_COLOR 0xe2u
#define CONFIRM_DIALOG_FOREGROUND_COLOR 0xe8u
#define CONFIRM_DIALOG_X 90u
#define CONFIRM_DIALOG_Y 80u
#define CONFIRM_DIALOG_WIDTH 140u
#define CONFIRM_DIALOG_HEIGHT 40u

void CB_NEAR confirm_dialog_step(void)
{
    if ((ship_3d_nav_choice_sound_gate & CONFIRM_DIALOG_ACTIVE) == 0u) {
        return;
    }

    confirm_dialog_state = 1u;
    vm_ui_flags |= CONFIRM_DIALOG_UI_ACTIVE;
    framebuffer_rect_fill(
            CONFIRM_DIALOG_BACKGROUND_COLOR,
            CONFIRM_DIALOG_X,
            CONFIRM_DIALOG_Y,
            CONFIRM_DIALOG_WIDTH,
            CONFIRM_DIALOG_HEIGHT);
    composite_draw_a(
            CONFIRM_DIALOG_FOREGROUND_COLOR,
            CONFIRM_DIALOG_X,
            CONFIRM_DIALOG_Y,
            CONFIRM_DIALOG_WIDTH,
            CONFIRM_DIALOG_HEIGHT);
    square_caps_text_draw_display(
            confirm_dialog_question,
            100u,
            88u,
            CONFIRM_DIALOG_FOREGROUND_COLOR);
    square_caps_text_draw_display(
            confirm_dialog_yes,
            120u,
            105u,
            CONFIRM_DIALOG_FOREGROUND_COLOR);
    square_caps_text_draw_display(
            confirm_dialog_no,
            180u,
            105u,
            CONFIRM_DIALOG_FOREGROUND_COLOR);

    if (region_record_hittest(&confirm_dialog_yes_region)) {
        --ship_3d_nav_choice_sound_gate;
    } else if (region_record_hittest(&confirm_dialog_no_region)) {
        ship_3d_nav_choice_sound_gate = 0u;
        vm_ui_state.word &= (cb_u16)~CONFIRM_DIALOG_UI_ACTIVE;
        confirm_dialog_state = 11u;
        mouse_primary_pressed = 0u;
        mouse_press_pending = 0u;
    }
}
