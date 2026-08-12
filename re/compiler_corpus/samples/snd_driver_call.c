/*
 * Codegen probe for BLOODPRG 0x00BB9D.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

typedef void (FAR *snd_driver_callback_type)(u16 command);

extern snd_driver_callback_type snd_driver_callback;
extern volatile u8 snd_driver_pending_flag;

void FAR snd_driver_call_probe(void)
{
    snd_driver_callback(0u);
    snd_driver_pending_flag = 0;
}
