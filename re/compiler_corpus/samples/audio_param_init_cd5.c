/*
 * Codegen probe for BLOODPRG 0x00B7B0.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

typedef void (FAR *driver_init_callback)(u16 configuration);
typedef void (FAR *driver_command_callback)(u16 command);
typedef void (FAR *clip_callback)(int clip_index);

typedef union driver_entry {
    struct {
        u16 offset;
        u16 segment;
    } address;
    driver_init_callback initialize;
    driver_command_callback command;
} driver_entry;

extern volatile driver_entry driver_entries[9];
extern clip_callback play_clip_callback;
extern volatile u16 audio_configuration;

void FAR play_clip_probe(int clip_index);

#if defined(__WATCOMC__)
#pragma aux audio_param_init_cd5_probe parm [ax] modify exact []
#pragma aux play_clip_probe parm [ax] modify exact []
#endif

void FAR audio_param_init_cd5_probe(u16 driver_segment)
{
    u16 index;

    for (index = 0; index < 9u; ++index) {
        driver_entries[index].address.segment = driver_segment;
    }

    play_clip_callback = play_clip_probe;
    driver_entries[0].initialize(audio_configuration);
}
