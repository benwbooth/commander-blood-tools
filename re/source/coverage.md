# Natural C Candidate Coverage

Current measured coverage:

| module | indexed routines | natural-C candidates | missing |
| --- | ---: | ---: | ---: |
| `bloodprg` | 338 | 338 | 0 |
| `xdb_amer` | 55 | 55 | 0 |
| `xdb_croolis` | 54 | 54 | 0 |
| `xdb_manu3` | 12 | 12 | 0 |
| `xdb_scrut` | 62 | 62 | 0 |
| total | 521 | 521 | 0 |

Overall candidate coverage is 521 of 521 standardized indexed routines, or
100.00 percent. No legacy overlapping XDB callback dump remains unindexed.

Generate the live report, including the missing routine list, with:

```sh
python3 re/tools/source_candidates.py --coverage
```

The JSON report is derived from `re/assembly/routine_index.tsv` plus every
`re/source/**/candidates/manifest.tsv`. A routine is counted as covered only
when the candidate manifest entry resolves to the same module and routine
offset as the assembly inventory.
