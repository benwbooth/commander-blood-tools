#!/usr/bin/env python3
"""UNVERIFIED functions whose BODY cannot carry a decoded rule.

327 of the ledger's open rows are undocumented functions, and a large share are
one-line accessors -- `self.field`, `self.0.len()`, a delegation to another
function. Those genuinely have nothing to cite: the rule lives in whatever they
read, and a doc comment repeating the field name would add words, not evidence.

The rest are real work. Separating them by hand invites settling a routine that
looked short, so this classifies mechanically and CONSERVATIVELY -- a function is
TRIVIAL only when its body is at most `--max-lines` lines and contains none of:

  * arithmetic or bit operators (`+ - * / % << >> & | ^`), which is where a
    decoded rule hides (the 16-bit subtract of audit-fixes #586 was ONE line);
  * a comparison other than a plain `==`/`!=` on an argument;
  * indexing, casts (`as`), or a literal above 1 -- the shapes that carry a
    magic number;
  * a loop or match.

Everything else is UNKNOWN and stays on the queue. The point is a floor on what
can be dismissed, not a ceiling on what must be read.

Usage:
    python3 tools/classify_trivial_fns.py [--max-lines N] [--show-unknown] [--file F]
"""
import csv
import re
import sys

LEDGER = "docs/function-audit.tsv"

# Anything here means the body might encode a rule.
RULE_SHAPES = [
    re.compile(r"[+\-*/%^]|<<|>>|(?<![&|])&(?!&)|(?<![|&])\|(?!\|)"),
    re.compile(r"\bas\b\s+\w+"),
    re.compile(r"\b(for|while|loop|match)\b"),
    # METHODS THAT ARE ARITHMETIC. `value.rem_euclid(modulus)` has no operator
    # character but is a modular wrap -- a decoded rule in one line, and the first
    # run of this tool waved it through (`bridge::wrap`). Any method whose job is
    # numeric counts as a rule shape.
    re.compile(
        r"\.(rem_euclid|wrapping_\w+|saturating_\w+|checked_\w+|overflowing_\w+"
        r"|abs|signum|pow|min|max|clamp|count_ones|trailing_zeros|leading_zeros"
        r"|rotate_\w+|to_le_bytes|from_le_bytes|swap_bytes)\s*\("
    ),
    # A NAMED DECODED CONSTANT. `selector != TEXT_SELECTOR_NONE && selector !=
    # TEXT_SELECTOR_SILENT` is one line with no arithmetic, and it is entirely a
    # decoded rule -- which of the selector values ask for voice. The second run of
    # this tool waved it through (`vm::text_selector_requests_voice`), and
    # TEXT_SELECTOR_SILENT is itself an open row (#512), so settling its only
    # consumer would have buried the question.
    re.compile(r"\b[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)+\b"),
    re.compile(r"\[[^\]]"),           # indexing / slicing
    re.compile(r"[<>]=?"),            # ordering comparisons
    re.compile(r"\b0x[0-9A-Fa-f]+\b"),
    re.compile(r"(?<![\w.])[2-9]\d*(?![\w.])"),  # a literal above 1
]


def body_of(src, start_idx):
    """Lines of the function body, by brace depth from its signature.

    The signature line's own tail counts. `fn rd_u8(r: &mut dyn Read) -> u8 { let mut
    b = [0; 1]; ...; b[0] }` is a whole function on ONE line, and the first version
    started collecting only from the NEXT line -- so it returned an empty body, no
    rule shape matched, and the function was declared trivial BECAUSE IT HAD NOT BEEN
    READ. Empty now means unknown, and the tail is included.
    """
    depth = 0
    out = []
    started = False
    for k in range(start_idx, min(len(src), start_idx + 60)):
        line = src[k]
        depth += line.count("{") - line.count("}")
        if "{" in line and not started:
            started = True
            tail = line.split("{", 1)[1]
            if tail.strip():
                out.append(tail)
        elif started:
            out.append(line)
        if started and depth <= 0:
            break
    if out and out[-1].strip() == "}":
        out.pop()
    return out


def main():
    max_lines = 3
    if "--max-lines" in sys.argv:
        max_lines = int(sys.argv[sys.argv.index("--max-lines") + 1])
    only = None
    if "--file" in sys.argv:
        only = sys.argv[sys.argv.index("--file") + 1]

    rows = [
        r
        for r in csv.DictReader(open(LEDGER, newline=""), delimiter="\t")
        if r["status"] == "UNVERIFIED" and r["kind"] == "fn"
    ]
    cache = {}
    trivial, unknown = [], []
    for r in rows:
        path = r["file"]
        if only and path != only:
            continue
        if path not in cache:
            try:
                cache[path] = open(path, encoding="utf-8", errors="replace").read().splitlines()
            except OSError:
                cache[path] = []
        src = cache[path]
        idx = int(r["line"]) - 1
        if idx >= len(src):
            continue
        # skip anything with a doc comment: those are a different queue
        if idx > 0 and src[idx - 1].lstrip().startswith("///"):
            unknown.append((path, r["line"], r["item"], "has a doc"))
            continue
        body = body_of(src, idx)
        text = "\n".join(l.split("//")[0] for l in body)
        if not text.strip():
            # NOT trivial -- unread. See body_of.
            unknown.append((path, r["line"], r["item"], "body not extracted"))
            continue
        if len(body) > max_lines:
            unknown.append((path, r["line"], r["item"], f"{len(body)} lines"))
            continue
        hit = next((p.pattern for p in RULE_SHAPES if p.search(text)), None)
        if hit:
            unknown.append((path, r["line"], r["item"], "rule shape"))
        else:
            trivial.append((path, r["line"], r["item"], " ".join(text.split())[:70]))

    print(f"{len(rows)} UNVERIFIED fn row(s); {len(trivial)} cannot carry a rule "
          f"(<= {max_lines} lines, no arithmetic/index/cast/loop/literal), "
          f"{len(unknown)} need reading\n")
    for path, line, item, body in trivial:
        print(f"  {path}:{line} {item}")
        print(f"      {body}")
    if "--show-unknown" in sys.argv:
        print("\nNEEDS READING:")
        for path, line, item, why in unknown:
            print(f"  {path}:{line} {item}  ({why})")


if __name__ == "__main__":
    main()
