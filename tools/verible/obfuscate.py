#!/usr/bin/env python3
import sys
import tempfile
import subprocess
from importlib.machinery import SourceFileLoader
from os import path

get_interface = (
    SourceFileLoader("interface", path.join(path.dirname(__file__), "interface.py"))
    .load_module()
    .get_interface
)


def obfuscate(file, mapping):
    obfuscation_map = tempfile.NamedTemporaryFile("w")
    obfuscation_map.write(
        "\n".join(f"{key} {value}" for (key, value) in mapping.items())
    )
    obfuscation_map.flush()

    verible = subprocess.run(
        ["verible-verilog-obfuscate", "--load_map", obfuscation_map.name],
        stdout=subprocess.PIPE,
        stdin=open(file, "r"),
    )

    obfuscation_map.close()
    return verible.stdout.decode("utf-8")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <filename> <module name>...")
        sys.exit(1)

    fname = sys.argv[1]
    to_preserve = sys.argv[2:]
    interface = get_interface(fname)

    ports = sum([interface[fname][mod] for mod in to_preserve], [])

    print(
        obfuscate(
            fname,
            dict((preserved, preserved) for preserved in set(ports + to_preserve)),
        )
    )
