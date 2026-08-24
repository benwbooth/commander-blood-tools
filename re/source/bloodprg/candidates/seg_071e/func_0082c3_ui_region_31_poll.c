#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_input.h"

#define BLOODPRG_UI_REGION_POLL_COUNT 32
#define BLOODPRG_UI_REGION_SLOT 31

cb_i16 CB_FAR ui_region_31_poll(void)
{
    cb_i16 attempts_remaining;
    volatile bloodprg_entity_record *record;

    attempts_remaining = BLOODPRG_UI_REGION_POLL_COUNT - 1;
    record = &bloodprg_entity_table[BLOODPRG_UI_REGION_SLOT];

    do {
        if ((record->flags & BLOODPRG_ENTITY_STATE0_FLAG) != 0u &&
                region_record_hittest(
                    (const volatile bloodprg_rect_i16 CB_NEAR *)
                    &record->draw_x)) {
            return attempts_remaining;
        }
        --record;
        --attempts_remaining;
    } while (attempts_remaining >= 0);

    return -1;
}
