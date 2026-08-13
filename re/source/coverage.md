# Natural C Candidate Coverage

Current measured coverage:

| module | indexed routines | natural-C candidates | missing |
| --- | ---: | ---: | ---: |
| `bloodprg` | 318 | 163 | 155 |
| `xdb_amer` | 27 | 27 | 0 |
| `xdb_croolis` | 25 | 25 | 0 |
| `xdb_manu3` | 12 | 12 | 0 |
| `xdb_scrut` | 25 | 25 | 0 |
| total | 407 | 252 | 155 |

Overall candidate coverage is 252 of 407 indexed routines, or 61.92 percent.
All current candidates point at indexed routines.

Generate the live report, including the missing routine list, with:

```sh
python3 re/tools/source_candidates.py --coverage
```

The JSON report is derived from `re/assembly/routine_index.tsv` plus every
`re/source/**/candidates/manifest.tsv`. A routine is counted as covered only
when the candidate manifest entry resolves to the same module and routine
offset as the assembly inventory.
