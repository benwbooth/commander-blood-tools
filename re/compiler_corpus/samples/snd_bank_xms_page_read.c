/*
 * Codegen probe for BLOODPRG 0x00BD4E.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef signed int i16;
typedef unsigned int u16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef struct xms_move_request {
    u32 length;
    u16 source_handle;
    u32 source_offset;
    u16 destination_handle;
    volatile u8 FAR *destination;
} xms_move_request;

extern volatile i16 snd_bank_xms_handle;
extern volatile xms_move_request shared_xms_move_request;
void NEAR cb_xms_move_probe(volatile xms_move_request *request);

#if defined(__WATCOMC__)
#pragma aux snd_bank_xms_page_read_probe parm [ax] [es di] modify exact []
#endif

void NEAR snd_bank_xms_page_read_probe(u16 page,
        volatile u8 FAR *destination)
{
    shared_xms_move_request.length = 0x4000u;
    shared_xms_move_request.source_handle = (u16)snd_bank_xms_handle;
    shared_xms_move_request.source_offset = (u32)page << 14;
    shared_xms_move_request.destination_handle = 0;
    shared_xms_move_request.destination = destination;
    cb_xms_move_probe(&shared_xms_move_request);
}
