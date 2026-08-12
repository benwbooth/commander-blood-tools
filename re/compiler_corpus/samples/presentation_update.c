/*
 * Codegen probe for BLOODPRG 0x009F53.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern volatile u8 presentation_gate;
extern volatile u8 ship_active_flags_low;
extern volatile u8 bridge_redraw_pending;
extern volatile u16 active_line;
extern volatile u8 presentation_request_flags;

void NEAR presentation_queue_finish_probe(void);

void FAR presentation_update_probe(void)
{
#if defined(__TURBOC__) || defined(__BORLANDC__)
    asm push ax;
    asm push bx;
    asm push cx;
#endif

    if ((presentation_gate & 1u) != 0) {
        presentation_queue_finish_probe();
        if ((ship_active_flags_low & 8u) != 0) {
            bridge_redraw_pending = 1;
        }
        active_line = 0xffffu;
        presentation_gate = 0;
        presentation_request_flags &= (u8)~2u;
    }

#if defined(__TURBOC__) || defined(__BORLANDC__)
    asm pop cx;
    asm pop bx;
    asm pop ax;
#endif
}
