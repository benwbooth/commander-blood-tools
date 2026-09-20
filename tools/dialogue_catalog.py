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


def summarize(graph):
    cod = graph["cod"]
    bas = graph["bas"]
    sites = cod["text_sites"] + (bas["text_sites"] if bas else [])
    return dict(profiles=1, text_sites=len(sites), cod_text_sites=len(cod["text_sites"]),
                bas_text_sites=len(bas["text_sites"]) if bas else 0,
                inline_choice_sites=sum(bool(site["choice_operands"]) for site in sites),
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
    previous = None
    for site in graph["cod"]["text_sites"]:
        if site["procedure"] != previous:
            previous = site["procedure"]
            lines += [f"### {previous}", ""]
        lines += [f"#### COD {site['offset']:04X} / {site['record_name'] or 'unnamed record'}", "",
                  fence(site["text"]), ""]
        if site["choice_operands"]:
            lines += ["Choice operands: " + ", ".join(json.dumps(operand_label(word), ensure_ascii=False)
                                                      for word in site["choice_operands"]), ""]
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
                label.append(f"{item['offset']:04X}: " + site["text"].replace("\n", " "))
                if site["choice_operands"]:
                    label.append("CHOICES: " + " | ".join(map(operand_label, site["choice_operands"])))
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
    provenance = dict(analyzer_sha256=digest(args.analyzer), exporter_sha256=digest(__file__),
                      helper_sha256=digest(ROOT / "tools/video_anthology.py"),
                      sources={f"{game}/{path.name}": digest(path) for game, path in sources})
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
                       scope="Static authored graphs, not feasible-playthrough or video coverage. Source language retained.")
        lines = ["# Static Dialogue Catalog", "", summary["scope"], "",
                 "Source language is retained. The default CB sources are English; default BBB sources are French. "
                 "State-dependent numbers and inventory choices remain symbolic.", "",
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
    parser.add_argument("--timeout", type=int, default=120, help="per-profile static analysis timeout")
    parser.add_argument("--out", type=Path, required=True)
    export(parser.parse_args())


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        sys.exit(str(error))
