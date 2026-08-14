/* Codegen probe for BLOODPRG 0x00A1F3. */
typedef unsigned char u8;
typedef unsigned int u16;

#define NEAR near

extern volatile u16 resource_flags_probe;
extern volatile u8 list_rollover_state_probe;

void NEAR list_d8c_refill_probe(u16 link_target_offset);

void NEAR list_d8c_refill_with_rollover_latch_probe(
        u16 link_target_offset)
{
    list_rollover_state_probe = (u8)resource_flags_probe & 0x80u;
    list_d8c_refill_probe(link_target_offset);
    list_rollover_state_probe = 0u;
}
