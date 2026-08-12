#ifndef XDB_ALIEN_H
#define XDB_ALIEN_H

#include "xdb_common.h"

#define XDB_ALIEN_CURSOR_BIAS 0x005eu
#define XDB_ALIEN_FIELD_DELTA 0x000fu

typedef struct xdb_alien_biased_state {
    xdb_u8 field_000[0x52];
    xdb_i16 field_052;
} xdb_alien_biased_state;

typedef struct xdb_alien_state {
    xdb_u8 field_000[0x0b0];
    xdb_i16 field_0b0;
} xdb_alien_state;

typedef struct xdb_alien_method_context {
    xdb_u8 field_00[0x16];
    volatile xdb_alien_state *state;
} xdb_alien_method_context;

extern volatile xdb_i16 xdb_alien_method_delta; /* CS:0x0099 */
extern volatile xdb_u8 *xdb_amer_slot11_cursor; /* AMER CS:0x1BC2 */
extern volatile xdb_u8 *xdb_croolis_slot11_cursor; /* CROOLIS CS:0x1B2E */
extern volatile xdb_u8 *xdb_scrut_slot11_cursor; /* SCRUT CS:0x1BE3 */

#endif
