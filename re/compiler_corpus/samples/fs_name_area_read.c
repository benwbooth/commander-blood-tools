/*
 * Codegen probe for BLOODPRG 0x007788.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef signed char i8;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#if defined(__WATCOMC__)
#define FS_DATA __based(__segname("FS_DATA"))
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define FS_DATA FAR
#define GAME_DATA FAR
#endif

extern char FS_DATA resource_name_area[];
extern volatile u8 GAME_DATA name_area_dirty;

#if defined(__WATCOMC__)
#pragma aux fs_name_area_read_probe parm [si] value [si] modify exact [ax si di]
#endif

const u8 NEAR *NEAR fs_name_area_read_probe(const u8 NEAR *script_bytes)
{
    char FS_DATA *dst;
    u8 ch;

    dst = resource_name_area;
    for (;;) {
        ch = *script_bytes++;
        if ((i8)ch < 0 || ch < 0x20u) {
            --script_bytes;
            break;
        }
        *dst++ = (char)ch;
    }
    *dst = '\0';
    name_area_dirty = 1;
    return script_bytes;
}
