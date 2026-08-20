# Natural C Candidate Coverage

Current measured coverage:

| module | indexed routines | natural-C candidates | missing |
| --- | ---: | ---: | ---: |
| `bloodprg` | 321 | 321 | 0 |
| `xdb_amer` | 55 | 55 | 0 |
| `xdb_croolis` | 47 | 47 | 0 |
| `xdb_manu3` | 12 | 12 | 0 |
| `xdb_scrut` | 55 | 55 | 0 |
| total | 490 | 490 | 0 |

Overall candidate coverage is 490 of 490 standardized indexed routines, or
100.00 percent. No legacy overlapping XDB callback dump remains unindexed.

Generate the live report, including the missing routine list, with:

```sh
python3 re/tools/source_candidates.py --coverage
```

The JSON report is derived from `re/assembly/routine_index.tsv` plus every
`re/source/**/candidates/manifest.tsv`. A routine is counted as covered only
when the candidate manifest entry resolves to the same module and routine
offset as the assembly inventory.
