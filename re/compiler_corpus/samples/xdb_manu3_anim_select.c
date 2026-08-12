/* Codegen probe for the MANU3 animation-table selector. */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u16 tween_phase;
extern volatile u16 tween_script_offset;
extern volatile u16 sequence_table_offset;
extern volatile u16 active_slot_offsets[];
extern void NEAR xdb_manu3_tween_constructor_probe(
        volatile u16 NEAR *active_slot_cursor);

void NEAR xdb_manu3_anim_select_probe(u16 selector);

#if defined(__WATCOMC__)
#pragma aux xdb_manu3_tween_constructor_probe \
        parm [bx] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_anim_select_probe \
        parm [bx] modify exact [ax bx cx dx si di bp]
#endif

void NEAR xdb_manu3_anim_select_probe(u16 selector)
{
    u16 table_offset = sequence_table_offset;
    volatile u16 NEAR *relative_offsets =
            (volatile u16 NEAR *)table_offset;

    selector &= 0x001fu;
    tween_phase = 0;
    tween_script_offset = (u16)(
            table_offset + relative_offsets[selector]);
    xdb_manu3_tween_constructor_probe(active_slot_offsets);
}
