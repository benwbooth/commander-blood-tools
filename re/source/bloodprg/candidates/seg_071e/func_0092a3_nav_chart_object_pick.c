#include "../include/bloodprg_nav.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#endif

#define NAV_CHART_KIND_SHIP 0x0010u
#define NAV_CHART_KIND_BLACK_HOLE 0x0100u
#define NAV_CHART_MARKER_BIAS 2u
#define NAV_CHART_DEFAULT_WIDTH 0x0cu
#define NAV_CHART_DEFAULT_HEIGHT 0x0bu
#define NAV_CHART_BLACK_HOLE_WIDTH 0x13u
#define NAV_CHART_BLACK_HOLE_HEIGHT 0x0cu
#define NAV_CHART_SHIP_WIDTH 0x15u
#define NAV_CHART_SHIP_HEIGHT 0x0au

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NAV_CHART_OBJECT_AT(type, base, offset) \
    ((const volatile type CB_FAR *)MK_FP(FP_SEG(base), (offset)))
#else
#define NAV_CHART_OBJECT_AT(type, base, offset) \
    ((const volatile type CB_FAR *)((base) + (offset)))
#endif

cb_u16 CB_NEAR nav_chart_object_pick(
        const volatile cb_u8 CB_FAR *record_base)
{
    const volatile bloodprg_nav_chart_object CB_FAR *object;
    const volatile bloodprg_nav_chart_arche CB_FAR *arche;
    const volatile bloodprg_nav_chart_point CB_FAR *marker;
    const volatile cb_u16 CB_NEAR *list;
    cb_u16 object_offset;
    cb_u16 pointer_x;
    cb_u16 pointer_y;
    cb_u16 left;
    cb_u16 top;
    cb_u16 remaining;

    remaining = nav_chart_object_count;
    if (remaining == 0u) {
        return 0u;
    }

    pointer_x = (cb_u16)mouse_x;
    pointer_y = (cb_u16)mouse_y;
    list = vm_nav_chart_object_offsets;
    object_offset = 0u;

    do {
        object_offset = *list++;
        object = NAV_CHART_OBJECT_AT(
                bloodprg_nav_chart_object, record_base, object_offset);
        marker = &object->marker[0];
        nav_chart_pick_width = NAV_CHART_DEFAULT_WIDTH;
        nav_chart_pick_height = NAV_CHART_DEFAULT_HEIGHT;

        if ((object->kind & NAV_CHART_KIND_BLACK_HOLE) != 0u) {
            nav_chart_pick_width = NAV_CHART_BLACK_HOLE_WIDTH;
            nav_chart_pick_height = NAV_CHART_BLACK_HOLE_HEIGHT;
            arche = NAV_CHART_OBJECT_AT(
                    bloodprg_nav_chart_arche,
                    record_base,
                    vm_arche_record_offset);
            if (object->endpoint_context != arche->endpoint_context) {
                marker = &object->marker[1];
            } else if ((object->kind & NAV_CHART_KIND_SHIP) != 0u) {
                nav_chart_pick_width = NAV_CHART_SHIP_WIDTH;
                nav_chart_pick_height = NAV_CHART_SHIP_HEIGHT;
            }
        } else if ((object->kind & NAV_CHART_KIND_SHIP) != 0u) {
            nav_chart_pick_width = NAV_CHART_SHIP_WIDTH;
            nav_chart_pick_height = NAV_CHART_SHIP_HEIGHT;
        }

        left = (cb_u16)(marker->x - NAV_CHART_MARKER_BIAS);
        if (pointer_x >= left
                && pointer_x <= (cb_u16)(left + nav_chart_pick_width)) {
            top = (cb_u16)(marker->y - NAV_CHART_MARKER_BIAS);
            if (pointer_y >= top
                    && pointer_y <= (cb_u16)(top + nav_chart_pick_height)) {
                return object_offset;
            }
        }
    } while (--remaining != 0u);

    return 0u;
}
