#!/usr/bin/env python3
"""Lifted blocks where two predicates MERGE and a further test then runs.

This is the exact structural shape that produced audit-fixes #575, and the reason
that bug survived a differential sweep against the original instructions.

`0x92F4` is entered from two places: `0x92CB`'s "not a black hole" branch and
`0x92E9`'s "black hole, and it is the arche's". It then tests the SHIP bit and, on
success, overwrites a value the black-hole path had already written. A port that
reads the routine as `if black_hole {..} else if ship {..}` is wrong only for an
object that arrives at the merge via the SECOND predecessor AND passes the test --
one specific PATH, not one block and not one edge.

Block coverage does not see it (every block ran). Edge coverage does not see it
either (`0x92D3 -> 0x92F4` ran, and `0x92F4 -> 0x92FC` ran -- just never in the same
execution). Only the length-3 path is uncovered, so a sweep whose inputs never build
that combination passes while the port is wrong.

This finds the candidate shapes statically: a block with 2+ predecessors that ends in
a conditional. Each is a place where "which predecessor did we come from" changes
what the following test means, and therefore a place where a kind-ladder-shaped port
can be wrong. It reports how many distinct predecessor x successor combinations the
shape admits -- the number of paths a sweep must build to distinguish implementations.

NOT a list of bugs. Most merges are benign (a shared tail, a loop head). It is a list
of places where passing a differential test proves less than it appears to.

Usage:
    python3 tools/check_merge_then_branch.py [--all] [--func func_92a3]
"""
import re
import sys
from collections import defaultdict

AUTO = "src/recomp/auto.rs"
SWEPT = "src/recomp/mod.rs"

FN = re.compile(r"^pub fn (func_[0-9a-f]+)\(m: &mut Machine\)", re.M)
ARM = re.compile(r"^\s{12}(0x[0-9a-f]+) => \{", re.M)
GOTO = re.compile(r"__blk = (0x[0-9a-f]+);")


def parse(text):
    """func -> {block: [successors]} in source order."""
    out = {}
    starts = [(m.group(1), m.start()) for m in FN.finditer(text)]
    for i, (name, pos) in enumerate(starts):
        end = starts[i + 1][1] if i + 1 < len(starts) else len(text)
        body = text[pos:end]
        arms = [(m.group(1), m.start()) for m in ARM.finditer(body)]
        blocks = {}
        for j, (blk, bpos) in enumerate(arms):
            bend = arms[j + 1][1] if j + 1 < len(arms) else len(body)
            # dedupe successors but keep order: a block ending in a conditional has
            # two DISTINCT targets, one ending in a jump has one.
            seen, succ = set(), []
            for g in GOTO.finditer(body[bpos:bend]):
                t = g.group(1)
                if t not in seen:
                    seen.add(t)
                    succ.append(t)
            blocks[blk] = succ
        out[name] = blocks
    return out


def main():
    text = open(AUTO, encoding="utf-8", errors="replace").read()
    graphs = parse(text)
    # SWEPT means "called by a differential TEST", not "present in the registry".
    # The first version matched `super::auto::func_x` anywhere in mod.rs, which
    # includes the dispatch table at mod.rs:777 -- so every lifted function came back
    # SWEPT and the report was meaningless. Scan only inside `#[test] fn` bodies.
    mod_text = open(SWEPT, encoding="utf-8").read()
    swept = set()
    # ...and only NATIVE-vs-lift tests. The `*_batch_matches_oracle` pair runs 75
    # lifts against recorded register state; there is no hand-written port function
    # in those, so nothing can disagree in the #575 way. Counting them made 119
    # shapes look relevant when 14 are.
    for m in re.finditer(r"#\[test\]\s*\n\s*fn (native_\w+)\(\) \{", mod_text):
        depth, i = 0, m.end() - 1
        while i < len(mod_text):
            if mod_text[i] == "{":
                depth += 1
            elif mod_text[i] == "}":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        swept |= set(re.findall(r"super::auto::(func_[0-9a-f]+)", mod_text[m.start():i]))

    only = None
    if "--func" in sys.argv:
        only = sys.argv[sys.argv.index("--func") + 1]

    total, reported = 0, 0
    for name in sorted(graphs):
        if only and name != only:
            continue
        blocks = graphs[name]
        preds = defaultdict(list)
        for blk, succ in blocks.items():
            for s in succ:
                preds[s].append(blk)
        shapes = [
            (blk, preds[blk], blocks[blk])
            for blk in sorted(blocks)
            if len(preds[blk]) >= 2 and len(blocks[blk]) >= 2
        ]
        if not shapes:
            continue
        total += len(shapes)
        # A swept function is where this MATTERS: an untested lift has no differential
        # claim to overstate in the first place.
        if not (name in swept or "--all" in sys.argv):
            continue
        reported += len(shapes)
        print(f"\n{name}{'  [SWEPT]' if name in swept else ''}")
        for blk, ps, ss in shapes:
            paths = len(ps) * len(ss)
            print(f"  {blk}  <- {', '.join(ps)}   then -> {', '.join(ss)}"
                  f"   ({paths} paths)")

    print(f"\n{total} merge-then-branch shape(s) across {len(graphs)} lifted function(s); "
          f"{reported} shown")
    print("Each is a place where block AND edge coverage can be complete while a")
    print("length-3 path is not -- which is how #575 passed its own sweep.")


if __name__ == "__main__":
    main()
