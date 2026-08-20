# Natural C Candidate Coverage

Current measured coverage:

| module | indexed routines | natural-C candidates | missing |
| --- | ---: | ---: | ---: |
| `bloodprg` | 321 | 321 | 0 |
| `xdb_amer` | 45 | 45 | 0 |
| `xdb_croolis` | 37 | 37 | 0 |
| `xdb_manu3` | 12 | 12 | 0 |
| `xdb_scrut` | 37 | 37 | 0 |
| total | 452 | 452 | 0 |

Overall candidate coverage is 452 of 452 standardized indexed routines, or
100.00 percent. Four additional legacy XDB candidate dumps are deliberately
reported as unindexed: their overlapping callback-state ranges still need a
control-flow ownership audit before they can be split into routine assembly
owners.

Generate the live report, including the missing routine list, with:

```sh
python3 re/tools/source_candidates.py --coverage
```

The JSON report is derived from `re/assembly/routine_index.tsv` plus every
`re/source/**/candidates/manifest.tsv`. A routine is counted as covered only
when the candidate manifest entry resolves to the same module and routine
offset as the assembly inventory.
