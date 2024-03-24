import json
from hypermapper import optimizer
import math
import subprocess
from os import path
import logging as log
from tempfile import TemporaryDirectory
import os


# Interface
def gen_interface(tmpdir: TemporaryDirectory, filamentfile: str, gen_config: str):
    out = path.join(tmpdir.name, "interface.json")

    # run the fft file
    with open(out, "w") as f:
        subprocess.run(
            [
                "/home/filament/target/debug/filament",
                filamentfile,
                "--gen-config",
                gen_config,
                "--preserve-names",
                "--dump-interface",
                "--library",
                "/home/filament",
            ],
            stdout=f,
        )

    # Change the current working directory back because fud messes with it
    os.chdir(path.dirname(__file__))

    with open(out) as f:
        ret = json.load(f)
    ret = ret["interfaces"][0]
    return {"latency": ret["states"], "ii": ret["delay"]}


# Generates verilog
def compile(
    tmpdir: TemporaryDirectory, filamentfile: str, params: dict[str, dict[str, str]]
):
    # Create the globals configuration
    conf_file = open(path.join(tmpdir.name, "conf.toml"), "w")

    # Generate a file that looks like
    # [globals.<key>]
    # <subkey> = <value>
    # ...
    for k, v in params.items():
        conf_file.write(f"[globals.{k}]\n")
        for subkey, value in v.items():
            conf_file.write(f'{subkey} = "{value}"\n')

    conf_file.flush()

    latency = gen_interface(tmpdir, filamentfile, conf_file.name)

    subprocess.run(
        [
            "fud",
            "e",
            "-s",
            "filament.flags",
            f" --gen-config {conf_file.name}",
            "--from",
            "filament",
            "--to",
            "icarus-verilog",
            filamentfile,
            "-o",
            path.join(tmpdir.name, "fft.sv"),
        ]
    )

    return latency


# Synthesize a design and get the resource estimate
def synth(verilog_file, clock_period=7):
    tmpdir = TemporaryDirectory()
    log.info(f"Synthesizing {verilog_file} to {tmpdir.name} with period {clock_period}")
    # Write xdc file
    constraint_xdc = open(path.join(tmpdir.name, "constraints.xdc"), "w")
    constraint_xdc.write(
        f"""
create_clock -period {clock_period:.2f} -name clk [get_ports clk]
"""
    )
    constraint_xdc.flush()

    # run the fft file through fud to get a synthesis estimate
    # Load the local synth.tcl file
    subprocess.run(
        [
            "fud",
            "e",
            "-s",
            "synth-verilog.tcl",
            path.join(path.dirname(__file__), "synth.tcl"),
            "-s",
            "synth-verilog.constraints",
            constraint_xdc.name,
            "--from",
            "synth-verilog",
            "--to",
            "resource-estimate",
            verilog_file,
            "-o",
            path.join(tmpdir.name, "resources.json"),
        ]
    )

    # Read the resource estimate
    with open(path.join(tmpdir.name, "resources.json")) as f:
        resources = json.load(f)

    print(resources)
    tmpdir.cleanup()
    # Loop through resources and set -1 values to infinity
    # This is to make failing designs bad
    for k, v in resources.items():
        if v == -1:
            resources[k] = float("inf")
    return resources


def compile_and_synth(filamentfile: str, params: dict[str, dict[str, str]]):
    tmpdir = TemporaryDirectory()
    latency = compile(tmpdir, filamentfile, params)
    resources = synth(path.join(tmpdir.name, "fft.sv"))
    return {**latency, **resources}


if __name__ == "__main__":
    root = os.path.dirname(__file__)
    tmpdir = TemporaryDirectory()
    print(
        compile_and_synth(
            path.join(root, "flopocofft.fil"),
            {"flopoco": {"conf": "frequency=800 target=Virtex6"}},
        )
    )
