#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Export static CB/BBB dialogue graphs from readable scripts; never run the game."""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile

from video_anthology import digest, save_json

ROOT = Path(__file__).resolve().parents[1]


def apply_english(graph, translation):
    """Bind display text only; original operands, instructions and edges stay intact."""
    if (graph["game"] != "bbb" or translation.get("format") != "bbb-cod-display-translation-v1"
            or translation.get("language") != "en" or translation.get("profile") != graph["profile"]):
        raise ValueError("wrong English catalog format, language, game, or profile")
    for key in ("cod_sha256", "dic_sha256"):
        if not graph.get("resources", {}).get(key) or translation.get(key) != graph["resources"][key]:
            raise ValueError(f"English catalog {key} does not match compiled source")
    sites = graph["cod"]["text_sites"]
    expected = {f"bbb.{graph['profile'].lower()}.cod.{site['offset']:08x}" for site in sites}
    messages = translation.get("messages")
    if not isinstance(messages, dict) or set(messages) != expected:
        raise ValueError("English catalog must cover exactly the authored text sites")
    displays = []
    for site in sites:
        key = f"bbb.{graph['profile'].lower()}.cod.{site['offset']:08x}"
        sections = messages[key]
        if (not isinstance(sections, list) or not sections
                or any(not isinstance(section, str) or not section.isascii()
                       or any(ord(char) < 32 or ord(char) == 127 for char in section) for section in sections)):
            raise ValueError(f"{key}: English sections must be printable ASCII strings")
        source_sections = site["sections"]
        if len(sections) != len(source_sections):
            raise ValueError(f"{key}: English section count mismatch")
        expected_numbers = [word["offset"] for word in source_sections[0] if word["kind"] == "state_number"]
        actual_numbers = []
        for word in sections[0].split():
            marker = re.fullmatch(r"<state:([0-9]+)>", word)
            if marker:
                actual_numbers.append(int(marker[1]))
            elif "<" in word or ">" in word:
                raise ValueError(f"{key}: malformed English dynamic marker")
        if actual_numbers != expected_numbers:
            raise ValueError(f"{key}: English prose changes ordered state-number operands")
        if bool(sections[0].strip()) != bool(source_sections[0]):
            raise ValueError(f"{key}: English prose changes empty/nonempty text")
        choices = []
        for index, (source, translated) in enumerate(zip(source_sections[1:], sections[1:])):
            if index:
                choices.append(dict(kind="separator"))
            if source == [dict(kind="inventory_choices")]:
                if len(sections) != 2 or translated != "<inventory_choices>":
                    raise ValueError(f"{key}: English inventory section must preserve its sole generator")
                choices.append(dict(kind="inventory_choices"))
            else:
                labels = translated.split()
                if len(labels) != len(source) or any(word["kind"] != "dictionary" for word in source):
                    raise ValueError(f"{key}: English choice count or kind mismatch")
                choices.extend(dict(word, text=label) for word, label in zip(source, labels))
        displays.append(dict(language="en", catalog_id=key, text=sections[0],
                             choice_operands=choices, sections=sections))
    for site, display in zip(sites, displays):
        site["display"] = display
    graph["localization"] = dict(language="en", format=translation["format"],
                                 translated_text_sites=len(displays), binding="exact compiled COD/DIC SHA-256")


def display_text(site):
    return site.get("display", site)["text"]


def display_choices(site):
    return site.get("display", site)["choice_operands"]


def summarize(graph):
    cod = graph["cod"]
    bas = graph["bas"]
    sites = cod["text_sites"] + (bas["text_sites"] if bas else [])
    return dict(profiles=1, text_sites=len(sites), cod_text_sites=len(cod["text_sites"]),
                bas_text_sites=len(bas["text_sites"]) if bas else 0,
                inline_choice_sites=sum(bool(site["choice_operands"]) for site in sites),
                english_overlay_sites=sum(site.get("display", {}).get("language") == "en" for site in sites),
                dynamic_text_sites=sum(site["dynamic"] for site in sites),
                procedures=cod["control_flow"]["procedure_count"],
                unresolved_guard_branches=len(cod["control_flow"]["unresolved_guard_branches"]),
                bas_selectors=len(bas["control_flow"]["nodes"]) if bas else 0,
                bas_menu_rows=len(bas["choice_edges"]) if bas else 0)


def operand_label(operand):
    return operand.get("text", "{" + operand["kind"] + "}")


def fence(text):
    # Original dialogue may itself contain Markdown syntax.
    marker = "`" * max(3, max((len(piece) for piece in re.findall(r"`+", text)), default=0) + 1)
    return marker + "text\n" + text + "\n" + marker


def profile_markdown(graph):
    summary = summarize(graph)
    lines = [f"# {graph['game'].upper()} {graph['profile']}", "",
             "Static authored dialogue, not an executed playthrough. Numeric values and inventory remain symbolic.", "",
             "[Exact source](source.blood) | [Full graph](graph.json) | [COD graph](cod.dot)", "",
             f"{summary['text_sites']} text sites, {summary['procedures']} procedures, "
             f"{summary['bas_selectors']} BAS selectors.", "",
             "## COD Dialogue", "",
             "Procedure order is source order, not a proposed playthrough. Block/edge conditions are in graph.json and cod.dot.", ""]
    if graph.get("localization"):
        lines += ["English display text from [the bound catalog](translation.json). "
                  "Original text, dictionary IDs, and conditions remain in graph.json and source.blood.", ""]
    previous = None
    for site in graph["cod"]["text_sites"]:
        if site["procedure"] != previous:
            previous = site["procedure"]
            lines += [f"### {previous}", ""]
        lines += [f"#### COD {site['offset']:04X} / {site['record_name'] or 'unnamed record'}", "",
                  fence(display_text(site)), ""]
        if site["choice_operands"]:
            lines += ["Choice operands: " + ", ".join(json.dumps(operand_label(word), ensure_ascii=False)
                                                      for word in display_choices(site)), ""]
        controls = {key: site[key] for key in ("resume_offset", "skip_next_if_not_shown", "control_word")
                    if site[key] is not None}
        if site["recent_choice_count"]:
            controls["recent_choice_count"] = site["recent_choice_count"]
        if controls:
            lines += ["Controls: `" + json.dumps(controls) + "`", ""]
    if graph["bas"]:
        bas = graph["bas"]
        lines += ["## BAS Selector Trees", "", "[Selector choice graph](bas.dot)", "",
                  "Choice links use the first matching selector in the same object's list. "
                  "No local match is not proof of an unreachable choice. Inline choice operands remain in graph.json.", ""]
        sites = defaultdict(list)
        edges = defaultdict(list)
        for site in bas["text_sites"]:
            sites[site["selector_node"]].append(site)
        for edge in bas["choice_edges"]:
            edges[edge["from_node"]].append(edge)
        for node in bas["control_flow"]["nodes"]:
            owner = bas["control_flow"]["lists"][node["list_index"]]["entrypoint"]["object_name"]
            lines += [f"### {owner}: {node['selector_name']} (BAS {node['offset']:04X})", ""]
            for site in sites[node["offset"]]:
                lines += [f"BAS {site['offset']:04X}:", "", fence(site["text"]), ""]
            for edge in edges[node["offset"]]:
                target = f"BAS {edge['to_node']:04X}" if edge["to_node"] is not None else "no local selector match"
                lines += [f"- {json.dumps(edge['choice']['text'], ensure_ascii=False)} -> {target}"]
            lines.append("")
    return "\n".join(lines)


def cod_dot(graph):
    cod = graph["cod"]
    instructions = defaultdict(list)
    texts = {site["offset"]: site for site in cod["text_sites"]}
    for item in cod["instructions"]:
        instructions[item["block"]].append(item)
    lines = ["digraph COD {", '  graph [rankdir="TB"];', '  node [shape="box"];']
    for block in cod["control_flow"]["blocks"]:
        label = [f"{block['procedure']} / {block['start']:04X}"]
        for item in instructions[block["start"]]:
            site = texts.get(item["offset"])
            if site:
                label.append(f"{item['offset']:04X}: " + display_text(site).replace("\n", " "))
                if site["choice_operands"]:
                    label.append("CHOICES: " + " | ".join(map(operand_label, display_choices(site))))
            else:
                label.append(f"{item['offset']:04X}: {item['description']}")
        lines.append(f"  b{block['start']} [label={json.dumps(chr(10).join(label), ensure_ascii=False)}];")
    for edge in cod["control_flow"]["edges"]:
        style = ', style="dashed"' if edge["kind"] == "frame_resume" else ""
        label = f"{edge['kind']} @{edge['from_instruction']:04X}"
        lines.append(f"  b{edge['from_block']} -> b{edge['to_block']} [label={json.dumps(label)}{style}];")
    return "\n".join(lines + ["}", ""])


def bas_dot(graph):
    bas = graph["bas"]
    lines = ["digraph BAS {", '  node [shape="box"];']
    for node in bas["control_flow"]["nodes"]:
        owner = bas["control_flow"]["lists"][node["list_index"]]["entrypoint"]["object_name"]
        label = f"{owner} / {node['selector_name']} / {node['offset']:04X}"
        lines.append(f"  n{node['offset']} [label={json.dumps(label, ensure_ascii=False)}];")
    for index, edge in enumerate(bas["choice_edges"]):
        target = f"n{edge['to_node']}" if edge["to_node"] is not None else f"unresolved{index}"
        if edge["to_node"] is None:
            lines.append(f'  {target} [label="no local selector match", shape="ellipse"];')
        lines.append(f"  n{edge['from_node']} -> {target} [label={json.dumps(edge['choice']['text'], ensure_ascii=False)}];")
    return "\n".join(lines + ["}", ""])


def export(args):
    roots = {"cb": args.cb_source, "bbb": args.bbb_source}
    sources = [(game, root / f"script{index}.blood") for game, root in roots.items()
               for index in range(1, {"cb": 5, "bbb": 17}[game] + 1)]
    catalogs = {f"script{index}": args.bbb_english / f"script{index}.json" for index in range(1, 18)} if args.bbb_language == "en" else {}
    provenance = dict(analyzer_sha256=digest(args.analyzer), exporter_sha256=digest(__file__),
                      helper_sha256=digest(ROOT / "tools/video_anthology.py"),
                      sources={f"{game}/{path.name}": digest(path) for game, path in sources},
                      bbb_language=args.bbb_language,
                      english_catalogs={name: digest(path) for name, path in catalogs.items()})
    fingerprint = hashlib.sha256(json.dumps(provenance, sort_keys=True).encode()).hexdigest()
    if args.out.exists():
        raise ValueError(f"output already exists; use a new directory: {args.out}")
    args.out.parent.mkdir(parents=True, exist_ok=True)
    # Publish only after all 22 profiles pass decoding and artifact generation.
    with tempfile.TemporaryDirectory(prefix="dialogue-catalog-", dir=args.out.parent) as temporary:
        output = Path(temporary) / "catalog"
        output.mkdir()
        profiles = []
        totals = {game: Counter() for game in roots}
        for game, source in sources:
            result = subprocess.run([str(args.analyzer.resolve()), str(source.resolve())],
                                    check=True, capture_output=True, text=True, timeout=args.timeout)
            graph = json.loads(result.stdout)
            if graph["schema"] != 1 or graph["game"] != game or graph["profile"].lower() != source.stem:
                raise ValueError(f"unexpected analyzer result for {source}")
            relative = Path(game) / source.stem
            directory = output / relative
            directory.mkdir(parents=True)
            if game == "bbb" and catalogs:
                catalog_bytes = catalogs[source.stem].read_bytes()
                sha = hashlib.sha256(catalog_bytes).hexdigest()
                if sha != provenance["english_catalogs"][source.stem]:
                    raise ValueError(f"English catalog changed during analysis: {source.stem}")
                apply_english(graph, json.loads(catalog_bytes))
                graph["localization"]["catalog_sha256"] = sha
                (directory / "translation.json").write_bytes(catalog_bytes)
            save_json(directory / "graph.json", graph)
            (directory / "source.blood").write_bytes(source.read_bytes())
            if digest(directory / "source.blood") != provenance["sources"][f"{game}/{source.name}"]:
                raise ValueError(f"source changed during analysis: {source}")
            (directory / "dialogue.md").write_text(profile_markdown(graph))
            (directory / "cod.dot").write_text(cod_dot(graph))
            if graph["bas"]:
                (directory / "bas.dot").write_text(bas_dot(graph))
            counts = summarize(graph)
            totals[game].update(counts)
            profiles.append(dict(game=game, profile=graph["profile"], directory=str(relative), counts=counts))
            print(f"{game} {graph['profile']}: {counts['text_sites']} text sites", flush=True)
        summary = dict(schema=1, fingerprint=fingerprint, provenance=provenance, totals=totals, profiles=profiles,
                       scope="Static authored graphs, not feasible-playthrough or video coverage. Original source and identities retained.")
        language_note = ("BBB dialogue and choice labels use the hash-bound English display catalogs. "
                         "Conditions and source operands retain their original identities. " if catalogs else
                         "BBB uses the original source language (--bbb-language source). ")
        lines = ["# Static Dialogue Catalog", "", summary["scope"], "",
                 language_note + "State-dependent numbers and inventory choices remain symbolic.", "",
                 "| Game | Profiles | Text Sites | Inline Choice Sites | BAS Selectors | BAS Menu Rows |",
                 "| --- | ---: | ---: | ---: | ---: | ---: |"]
        for game, counts in totals.items():
            lines.append(f"| {game.upper()} | {counts['profiles']} | {counts['text_sites']} | "
                         f"{counts['inline_choice_sites']} | {counts['bas_selectors']} | {counts['bas_menu_rows']} |")
        lines += ["", "## Profiles", ""]
        for profile in profiles:
            lines.append(f"- [{profile['game'].upper()} {profile['profile']}]({profile['directory']}/dialogue.md)")
        (output / "index.md").write_text("\n".join(lines) + "\n")
        summary["artifacts"] = {str(path.relative_to(output)): digest(path)
                                for path in sorted(output.rglob("*")) if path.is_file()}
        save_json(output / "catalog.json", summary)
        if digest(args.analyzer) != provenance["analyzer_sha256"]:
            raise ValueError("analyzer changed during export")
        output.rename(args.out)
    print(f"Static catalog: {args.out / 'index.md'}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--analyzer", type=Path, default=ROOT / "target/release/examples/dialogue_catalog")
    parser.add_argument("--cb-source", type=Path, default=ROOT / "re/vm/profiles")
    parser.add_argument("--bbb-source", type=Path, default=ROOT / "re/vm/big-bug-bang-profiles")
    parser.add_argument("--bbb-language", choices=("en", "source"), default="en")
    parser.add_argument("--bbb-english", type=Path, default=ROOT / "localization/big-bug-bang/en")
    parser.add_argument("--timeout", type=int, default=120, help="per-profile static analysis timeout")
    parser.add_argument("--out", type=Path, required=True)
    export(parser.parse_args())


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        sys.exit(str(error))
