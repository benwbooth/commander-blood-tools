/*
 * Codegen probe for one-byte XDB near-return methods.
 * This is not recovered game source.
 */
#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

void NEAR xdb_near_noop_probe(void)
{
}
