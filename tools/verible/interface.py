#!/usr/bin/env python3
import sys
import json
import subprocess


def get_by_tag(node, tag):
    if node is None:
        return []
    if node["tag"] == tag:
        return [node]
    if "children" not in node:
        return []

    acc = []
    for childnode in node["children"]:
        acc += get_by_tag(childnode, tag)
    return acc


def get_interface(filename):
    verible = subprocess.run(
        ["verible-verilog-syntax", filename, "--export_json", "--printtree"],
        stdout=subprocess.PIPE,
    )
    syntaxtree = json.loads(verible.stdout)
    interface = {}

    for fname, body in syntaxtree.items():
        # Get the syntax tree
        body = body["tree"]
        module_headers = get_by_tag(body, "kModuleHeader")

        identifiers = {}
        for header in module_headers:
            tags = [id["text"] for id in get_by_tag(header, "SymbolIdentifier")]
            identifiers[tags[0]] = tags[1:]
        interface[fname] = identifiers

    return interface


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <filename>")
        sys.exit(1)
    print(json.dumps(get_interface(sys.argv[1]), indent=2))
