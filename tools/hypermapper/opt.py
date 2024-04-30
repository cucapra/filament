import json
import pandas
import shutil
from argparse import ArgumentParser
from hypermapper import optimizer
import hypermapper
import math
import subprocess
from os import path
import logging as log
from tempfile import TemporaryDirectory
import os
from utils import dl_to_ld
from multiprocessing import Pool
import matplotlib.pyplot as plt


# Interface
def gen_interface(tmpdir: TemporaryDirectory, filamentfile: str, gen_config: str):
    out = path.join(tmpdir.name, "interface.json")

    # run the fft file
    with open(out, "w") as f:
        subprocess.run(
            [
                "/home/filament/target/debug/filament",
                filamentfile,
                "--bindings",
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
    tmpdir: TemporaryDirectory,
    filamentfile: str,
    main_params: list[int],
    gen_params: dict[str, dict[str, str]],
):
    # Create the globals configuration
    conf_file = open(path.join(tmpdir.name, "conf.toml"), "w")

    # Add main parameters
    conf_file.write(f"params.main = {main_params}\n")

    # Generate a file that looks like
    # [globals.<key>]
    # <subkey> = <value>
    # ...
    for k, v in gen_params.items():
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
            f" --bindings {conf_file.name}",
            "--from",
            "filament",
            "--to",
            "icarus-verilog",
            filamentfile,
            "-o",
            path.join(tmpdir.name, "fft.sv"),
            "--quiet",
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
            "--quiet",
        ]
    )

    # Read the resource estimate
    with open(path.join(tmpdir.name, "resources.json")) as f:
        resources = json.load(f)

    tmpdir.cleanup()
    # Loop through resources and set -1 values to a very large number
    # This is to make failing designs bad
    # for k, v in resources.items():
    #     if v == -1 or resources["meet_timing"] == 0:
    #         resources[k] = 1e12
    print(resources)
    return resources


def compile_and_synth(
    filamentfile: str,
    clock_period: int,
    main_params: list[int],
    gen_params: dict[str, dict[str, str]],
):
    tmpdir = TemporaryDirectory()
    latency = compile(tmpdir, filamentfile, main_params, gen_params)
    resources = synth(path.join(tmpdir.name, "fft.sv"), clock_period)
    return {**latency, **resources}


def compile_flopoco_fft(
    iterative: int, num_butterflies: int, target_frequency: int, clock_period: int
):
    print(
        f"Synthesizing {'Iterative' if iterative > 0 else 'Streaming'} with {num_butterflies} butterflies, target frequency {target_frequency} and clock period {clock_period}"
    )
    synth_results = compile_and_synth(
        path.join(path.dirname(__file__), "flopocofft.fil"),
        clock_period,
        [num_butterflies, iterative],
        {"globals.flopoco": {"conf": f"frequency={target_frequency} target=Virtex6"}},
    )
    # We care about the time interval between operations
    synth_results["time_ii"] = synth_results["ii"] * synth_results["period"]
    return synth_results


def compile_and_synth_parallel(args):
    args = list(
        zip(
            args["iterative"],
            [2**x for x in args["num_butterflies_log2"]],
            args["target_frequency"],
            args["clock_period"],
        )
    )
    print(args)
    with Pool(10) as p:
        ret = p.starmap(compile_flopoco_fft, args)
    ret = dl_to_ld(ret)
    print(ret)
    return ret


if __name__ == "__main__":
    root = os.path.dirname(__file__)
    tmpdir = TemporaryDirectory()

    parser = ArgumentParser()
    parser.add_argument("--graphs-only", action="store_true")

    args = parser.parse_args()

    scenario = {
        "application_name": "flopocofft",
        "optimization_objectives": ["time_ii", "lut", "registers"],
        "optimization_iterations": 10,
        "evaluations_per_optimization_iteration": 10,
        "input_parameters": {
            "iterative": {"parameter_type": "integer", "values": [0, 1]},
            "num_butterflies_log2": {"parameter_type": "integer", "values": [1, 3]},
            "target_frequency": {"parameter_type": "integer", "values": [50, 950]},
            "clock_period": {"parameter_type": "integer", "values": [1, 20]},
        },
        "feasible_output": {
            "name": "meet_timing",
            "true_value": 1,
            "false_value": 0,
            "enable_feasible_predictor": True,
        },
    }

    if os.path.exists("flopocofft_output_samples.csv"):
        scenario = {
            **scenario,
            "resume_optimization": True,
            "resume_optimization_data": "flopocofft_output_samples.csv",
        }

    with open(path.join(tmpdir.name, "scenario.json"), "w") as f:
        json.dump(scenario, f)

    if not args.graphs_only:
        optimizer.optimize(
            path.join(tmpdir.name, "scenario.json"), compile_and_synth_parallel
        )

    # Now we are generating graphs
    # Handle a bug in hypermapper where "true_value" and "false_value" for feasability must be strings now
    scenario["feasible_output"]["true_value"] = str(
        scenario["feasible_output"]["true_value"]
    )
    scenario["feasible_output"]["false_value"] = str(
        scenario["feasible_output"]["false_value"]
    )

    with open(path.join(tmpdir.name, "scenario.json"), "w") as f:
        json.dump(scenario, f)

    # Generate the graphs
    # Make results directory
    resdir = path.join(root, "results")
    os.makedirs(resdir, exist_ok=True)

    # Copy csv file to results directory
    shutil.copyfile(
        "flopocofft_output_samples.csv",
        path.join(resdir, "flopocofft_output_samples.csv"),
    )

    # Also copy to tmpdir for plotting
    shutil.copyfile(
        "flopocofft_output_samples.csv",
        path.join(tmpdir.name, "flopocofft_output_samples.csv"),
    )

    hypermapper.plot_optimization_results.plot_regret(
        path.join(tmpdir.name, "scenario.json"), [tmpdir.name], out_dir=resdir
    )

    hypermapper.compute_pareto.compute(
        path.join(tmpdir.name, "scenario.json"),
        path.join(resdir, "flopocofft_output_samples.csv"),
        path.join(resdir, "pareto.csv"),
    )

    hypermapper.plot_pareto.plot(
        path.join(tmpdir.name, "scenario.json"),
        [
            (
                path.join(resdir, "pareto.csv"),
                path.join(resdir, "flopocofft_output_samples.csv"),
            )
        ],
        path.join(resdir, "pareto.pdf"),
    )

    df = pandas.read_csv(path.join(resdir, "pareto.csv"))

    print(df)

    df["frequency"] = 1000 / df["clock_period"]

    for objective in scenario["optimization_objectives"]:
        # Plot a scatter plot of all the points
        fig = plt.figure()
        ax = fig.add_subplot()

        ax.scatter(df["frequency"], df[objective])

        ax.set_xlabel("Frequency")
        ax.set_ylabel(objective)

        plt.savefig(path.join(resdir, f"{objective}_scatter.pdf"))

    tmpdir.cleanup()
