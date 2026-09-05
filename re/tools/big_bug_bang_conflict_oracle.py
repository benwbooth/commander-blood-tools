#!/usr/bin/env python3
"""Record Big Bug Bang D4 conflict results, executing every original called helper.

Reuse the settlement oracle's synthetic object graph and real-code runner.
This is an offline reference tool, not a dependency of the Rust game.
"""

import argparse
import hashlib
import json
from pathlib import Path
import random
import runpy
from types import SimpleNamespace

reference = SimpleNamespace(**runpy.run_path(str(Path(__file__).with_name("big_bug_bang_settlement_oracle.py"))))
OPPONENT = 72
AGGRESSION = 50
ACTORS = [reference.SOURCE, reference.CLONE_A, reference.CLONE_B, reference.HONK_ACTOR,
          reference.OTHER_GROUP, reference.NESTED]


def fixture(engaged=False):
    state, directory = reference.fixture()
    for actor in ACTORS:
        reference.word(state, actor + OPPONENT, 0)
        reference.word(state, actor + AGGRESSION, 400)
    for location in [reference.HOME, reference.NEAR, reference.TIE, reference.ARK_LOCATION, reference.TRASH]:
        reference.word(state, location + 24, 0)
    reference.word(state, reference.NEAR + 24, reference.OTHER_GROUP)
    reference.word(state, reference.NEAR + reference.FLAGS, 5)
    reference.word(state, reference.OTHER_GROUP + reference.FLAGS, 5)
    reference.word(state, reference.OTHER_GROUP + reference.QUANTITY, 500)
    reference.word(state, reference.OTHER_GROUP + reference.ACTOR_LOCATION, reference.NEAR)
    if engaged:
        for actor, opponent in [(reference.SOURCE, reference.OTHER_GROUP), (reference.OTHER_GROUP, reference.SOURCE)]:
            reference.word(state, actor + OPPONENT, opponent)
            reference.word(state, actor + reference.FLAGS, 13)
    return state, directory


def vectors(executable):
    for query in [0, 1]:
        for engaged in [False, True]:
            for rate in [0, 20, 1000, 32768, 65535]:
                state, directory = fixture(engaged)
                yield reference.run(executable, f"q{query}_engaged{int(engaged)}_rate{rate}", state, directory, query=query, attack_rate=rate)
        for index, (offset, value) in enumerate([
            (reference.SOURCE + reference.QUANTITY, 99),
            (reference.SOURCE + reference.QUANTITY, 100),
            (reference.SOURCE + AGGRESSION, 199),
            (reference.SOURCE + AGGRESSION, 200),
            (reference.SOURCE + reference.RELIEF, 799),
            (reference.SOURCE + reference.RELIEF, 800),
            (reference.SOURCE + reference.RELIEF, 65535),
            (reference.OTHER_GROUP + reference.QUANTITY, 50),
            (reference.OTHER_GROUP + reference.FLAGS, 1),
            (reference.OTHER_GROUP + reference.GROUP, 1),
            (reference.OTHER_GROUP + OPPONENT, reference.CLONE_A),
            (reference.NEAR + reference.FLAGS, 0),
        ]):
            state, directory = fixture()
            reference.word(state, offset, value)
            yield reference.run(executable, f"q{query}_gate{index}", state, directory, query=query, attack_rate=20)
        for source_quantity in [0, 10, 11, 200, 32767, 65535]:
            state, directory = fixture(True)
            reference.word(state, reference.SOURCE + reference.QUANTITY, source_quantity)
            reference.word(state, reference.OTHER_GROUP + reference.QUANTITY, 12)
            yield reference.run(executable, f"q{query}_disengage{source_quantity}", state, directory, query=query, attack_rate=1000)
        for flags in [1, 3]:
            state, directory = fixture(True)
            reference.word(state, reference.SOURCE + reference.QUANTITY, 10)
            reference.word(state, reference.CLONE_A + reference.FLAGS, flags)
            reference.word(state, reference.CLONE_A + OPPONENT, reference.OTHER_GROUP)
            yield reference.run(executable, f"q{query}_replacement_flags{flags}", state, directory, query=query, attack_rate=20)
        state, directory = fixture(True)
        reference.word(state, reference.SOURCE + reference.QUANTITY, 10)
        reference.word(state, reference.HOME + reference.FLAGS, 7)
        # Native 0x724E tests flag bit 1, not Actor kind, and reads byte 72
        # relative to that object. Here it aliases the following location's
        # parent word. Preserve this evidence instead of guessing an actor filter.
        reference.word(state, reference.HOME + OPPONENT, reference.OTHER_GROUP)
        yield reference.run(executable, f"q{query}_nonactor_replacement_alias", state, directory, query=query, attack_rate=20)
        for countdown in [1, 65535]:
            state, directory = fixture()
            yield reference.run(executable, f"q{query}_clock{countdown}", state, directory, query=query, countdown=countdown, override=1, attack_rate=321)
        for override in [0, 1]:
            state, directory = fixture(True)
            reference.word(state, reference.SOURCE + reference.QUANTITY, 10)
            reference.word(state, reference.OTHER_GROUP + reference.RELIEF, 0)
            yield reference.run(executable, f"q{query}_retreat_range{override}", state, directory, query=query, override=override, attack_rate=20)
        for name, group, rate, quantity, defender in [
            ("both_sides", 3, 20, 500, 500),
            ("fault_after_first_actor", 3, 20000, 100, 6000),
            ("empty_group", 0, 20, 500, 500),
        ]:
            state, directory = fixture(True)
            reference.word(state, reference.SOURCE + reference.QUANTITY, quantity)
            reference.word(state, reference.OTHER_GROUP + reference.QUANTITY, defender)
            yield reference.run(executable, f"q{query}_{name}", state, directory, query=query, group=group, attack_rate=rate)
    randomizer = random.Random(20260908)
    for index in range(48):
        state, directory = fixture(True)
        for actor in [reference.SOURCE, reference.OTHER_GROUP]:
            for field in [reference.QUANTITY, AGGRESSION, reference.BALANCE, reference.RELIEF]:
                reference.word(state, actor + field, randomizer.randrange(65536))
        yield reference.run(executable, f"random{index}", state, directory,
                            query=index % 2, attack_rate=randomizer.randrange(65536))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    executable = args.executable.read_bytes()
    if hashlib.sha256(executable).hexdigest() != reference.EXECUTABLE_SHA256:
        raise SystemExit("unsupported BLOOD2PG.EXE build; refusing fixed-offset oracle")
    results = list(vectors(executable))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(item, separators=(",", ":")) + "\n" for item in results))
    calls = set().union(*(set(item["native_handlers_called"]) for item in results))
    print(f"wrote {len(results)} native conflict cases; {sum(item['divide_error'] for item in results)} divide errors; {sum(item['branch_taken'] for item in results)} failed guards; entries {sorted(calls)}")


if __name__ == "__main__":
    main()
