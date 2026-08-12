typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

#define MOUSE_BUTTON_PRIMARY 0x01u
#define MOUSE_BUTTON_SECONDARY 0x02u

extern volatile u16 mouse_button_state;
extern volatile u16 mouse_previous_button_state;
extern volatile u8 mouse_primary_pressed;
extern volatile u8 mouse_secondary_pressed;
extern volatile u8 mouse_press_pending;

u16 NEAR mouse_button_edges_update_probe(void);

#if defined(__WATCOMC__)
#pragma aux mouse_button_edges_update_probe value [ax] modify exact [ax]
#endif

u16 NEAR mouse_button_edges_update_probe(void)
{
    u8 buttons;
    u16 current_word;

    buttons = (u8)mouse_button_state;
    if ((buttons & MOUSE_BUTTON_PRIMARY) != 0) {
        if ((buttons &= (u8)mouse_previous_button_state) == 0) {
            mouse_primary_pressed = 1;
            mouse_press_pending = 1;
        }
    }

    if ((buttons & MOUSE_BUTTON_SECONDARY) != 0) {
        if ((buttons &= (u8)mouse_previous_button_state) == 0) {
            mouse_secondary_pressed = 1;
            mouse_press_pending = 1;
        }
    }

    current_word = mouse_button_state;
    mouse_previous_button_state = current_word;
    return current_word;
}
