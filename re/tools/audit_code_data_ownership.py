#!/usr/bin/env python3
"""Verify file-local BLOODPRG objects that are owned by the code segment."""
from __future__ import annotations

import argparse
import re
from pathlib import Path


INSTRUCTION_ROW = re.compile(
    r"^[0-9A-Fa-f]{4,8}\s+(?:[0-9A-Fa-f]{2}\s+)+\s*(?P<text>.*?)\s*$"
)
LABEL_ROW = re.compile(
    r"^[0-9A-Fa-f]{4,8}\s+(?P<label>[A-Za-z_$?][\w$?@]*):\s*$"
)
SEGMENT_ROW = re.compile(r"^Segment:\s+(?P<name>\S+)\s+")

CODE_DATA_OWNERS = {
    "_decimal_append_scratch": "func_0024b2_decimal_append_i16.lst",
    "_decimal_digit_place_values": "func_002612_ascii_digit_parse.lst",
    "_blood_prng_seed_word": "func_002de2_blood_prng_next.lst",
    "_blood_prng_mix_low": "func_002de2_blood_prng_next.lst",
    "_blood_prng_mix_high": "func_002de2_blood_prng_next.lst",
    "_blood_prng_counter": "func_002de2_blood_prng_next.lst",
    "_ems_device_signature": "func_00099f_extended_memory_backends_init.lst",
}

MINIMUM_DIRECT_CS_REFERENCES = {
    "_decimal_digit_place_values": 1,
    "_blood_prng_seed_word": 2,
    "_blood_prng_mix_low": 2,
    "_blood_prng_mix_high": 2,
    "_blood_prng_counter": 2,
    "_ems_device_signature": 1,
}

DECIMAL_CURSOR_ROUTINES = {
    "func_0024b2_decimal_append_i16.lst": "decimal_append_i16_",
    "func_0024eb_decimal_append_i32.lst": "decimal_append_i32_",
}


def listing_lines(listing_dir: Path) -> dict[str, list[str]]:
    return {
        path.name: path.read_text(encoding="ascii", errors="replace").splitlines()
        for path in listing_dir.glob("*.lst")
    }


def audit(listing_dir: Path) -> list[str]:
    listings = listing_lines(listing_dir)
    errors: list[str] = []

    for symbol, owner_name in CODE_DATA_OWNERS.items():
        lines = listings.get(owner_name)
        if lines is None:
            errors.append(f"missing owner listing for {symbol}: {owner_name}")
            continue
        section = ""
        owner_section = None
        for line in lines:
            segment_match = SEGMENT_ROW.match(line)
            if segment_match:
                section = segment_match["name"]
                continue
            label_match = LABEL_ROW.match(line)
            if label_match and label_match["label"].lower() == symbol:
                owner_section = section
                break
        if owner_section is None:
            errors.append(f"{owner_name}: definition not found for {symbol}")
        elif not owner_section.endswith("_TEXT"):
            errors.append(
                f"{owner_name}: {symbol} is in {owner_section}, expected CODE"
            )

    direct_counts = {symbol: 0 for symbol in MINIMUM_DIRECT_CS_REFERENCES}
    for name, lines in listings.items():
        for line in lines:
            match = INSTRUCTION_ROW.match(line)
            if not match:
                continue
            text = match["text"].lower()
            for symbol in direct_counts:
                if symbol not in text:
                    continue
                if re.search(rf"\b(?:offset|seg)\s+{re.escape(symbol)}\b", text):
                    continue
                direct_counts[symbol] += 1
                if f"cs:{symbol}" not in text:
                    errors.append(f"{name}: non-CS access to {symbol}: {text}")

    for symbol, minimum in MINIMUM_DIRECT_CS_REFERENCES.items():
        if direct_counts[symbol] < minimum:
            errors.append(
                f"{symbol}: found {direct_counts[symbol]} direct CS references, "
                f"expected at least {minimum}"
            )

    for name, routine in DECIMAL_CURSOR_ROUTINES.items():
        lines = listings.get(name)
        if lines is None:
            errors.append(f"missing decimal cursor listing: {name}")
            continue
        instructions = [
            match["text"].lower()
            for line in lines
            if (match := INSTRUCTION_ROW.match(line))
        ]
        if not any(f"seg {routine}" in text for text in instructions):
            errors.append(f"{name}: scratch cursor has no CODE segment load")
        for text in instructions:
            if re.search(r"\bptr\s+(?!es:)[^,]*\[(?:bx|si)\]", text):
                errors.append(f"{name}: scratch cursor uses caller DS: {text}")

    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--listing-dir", type=Path, required=True)
    args = parser.parse_args()
    errors = audit(args.listing_dir)
    if errors:
        raise SystemExit("\n".join(errors))
    print(
        "code-data ownership: "
        f"{len(CODE_DATA_OWNERS)} definitions and "
        f"{sum(MINIMUM_DIRECT_CS_REFERENCES.values())} direct CS references verified"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
