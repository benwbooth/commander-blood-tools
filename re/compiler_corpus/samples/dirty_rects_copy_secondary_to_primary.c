/* Codegen probe for BLOODPRG 0x00509D. */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

typedef struct dirty_rect_probe {
    u16 left;
    u16 right;
    u16 top;
    u16 bottom;
} dirty_rect_probe;

extern volatile u8 dirty_copy_flags_probe;
extern volatile u8 FAR *primary_buffer_probe;
extern volatile u8 FAR *secondary_buffer_probe;

#if defined(__WATCOMC__)
#pragma aux dirty_rects_copy_secondary_to_primary_probe \
        parm [es di] modify exact []
#endif

void FAR dirty_rects_copy_secondary_to_primary_probe(
        const volatile dirty_rect_probe FAR *rectangles)
{
    const volatile u8 FAR *source;
    volatile u8 FAR *destination;
    u16 offset;
    u16 width;
    u16 rows;
    u16 columns;

    if ((dirty_copy_flags_probe & 1u) == 0u) {
        return;
    }

    while ((i16)rectangles->left >= 0) {
        width = (u16)(rectangles->right - rectangles->left);
        rows = (u16)(rectangles->bottom - rectangles->top);
        offset = (u16)(rectangles->top * 320u + rectangles->left);
        source = secondary_buffer_probe + offset;
        destination = primary_buffer_probe + offset;

        do {
            columns = width;
            while (columns != 0u) {
                *destination++ = *source++;
                --columns;
            }
            source += (u16)(320u - width);
            destination += (u16)(320u - width);
            --rows;
        } while (rows != 0u);

        ++rectangles;
    }
}
