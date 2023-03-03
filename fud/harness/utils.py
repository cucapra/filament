# type: ignore

import os
import json
import random
import cocotb
from cocotb import triggers
from cocotb.binary import BinaryValue
from cocotb.triggers import FallingEdge, RisingEdge, ClockCycles, First, ReadOnly
from cocotb.clock import Clock

RESET_CYCLES = 3
MAX_CYCLES = 10000


def representable(n, width):
    """
    Returns true iff `n` can be represented as a `width` bit number.
    """
    return n >= 0 and n < (1 << width)


def validate_data(data):
    """
    Validate the data format provided to the file.
    """
    total_txns = len(list(data.values())[0])
    assert all(
        [len(vs) == total_txns for vs in data.values()]
    ), "Invalid: Mismatched sizes for inputs"
    return total_txns


def construct_transaction_fsm(interface, randomize):
    """
    Generates a coroutine that models the provided interface.
    The coroutine deals with the inputs in the form of a "transaction".
    For each defined input in the `data` array, `data[<key>][i]` is part
    of the `i` transaction sent to the module.

    The coroutine executes a transaction fully before starting another one.
    It is guaranteed to wait as long as the delay provided by the module
    requires.

    The interface defines how the inputs are sent to the module and how long
    they are held up.
    Similarly, the coroutine expects the outputs to be available during the
    specified cycles and match the value in the data array.
    """

    assert len(interface["interfaces"]) > 0, "No interfaces defined"
    assert len(interface["interfaces"]) == 1, "Unsupported: multiple interfaces"

    # Track which inputs have been written to
    inp_write_idx = [0 for _ in interface["inputs"]]

    async def finalize_writes(mod):
        """
        Check that all inputs have been written to once and inputs
        without writes get assigned 'x
        """
        while True:
            await FallingEdge(mod.clk)
            # mod._log.warning(f"Finalizing writes: {inp_write_idx}")
            for idx, inp in enumerate(interface["inputs"]):
                if inp_write_idx[idx] == 0:
                    # mod._log.warning(f"{inp['name']} <= 'x")
                    # No writes to this input, assign 'x
                    mod._id(inp["name"], extended=False).value = BinaryValue(
                        "x", inp["width"]
                    )

                else:
                    assert (
                        inp_write_idx[idx] == 1
                    ), f"Invalid: {inp_write_idx[idx]} writes to input `{inp['name']}'"

                # Reset the write index
                inp_write_idx[idx] = 0

    # Construct a model of what needs to be done for one transaction
    async def run(mod, data):
        # Dictionary to store the outputs
        # Maps signal_name -> txn_id -> listof values
        outputs = {sig["name"]: {} for sig in interface["outputs"]}

        async def txn(idx, event):
            """
            Run a complete transaction
            """
            # Add new dict for this transaction to the outputs
            for sig in outputs.keys():
                outputs[sig][idx] = []

            # Fully execute the module by triggering it till the number of
            # times prescribed by the delay
            for st in range(0, event["states"]):
                # Start the transaction by setting the interface signal to 1
                if st == 0:
                    trg = 1
                else:
                    trg = 0
                # XXX(rachit): This might cause problems if another transaction
                # attempts to write 1 while this writes 0
                # If the interface is not @phantom, provide it
                if not event["phantom"]:
                    mod._id(event["name"], extended=False).value = trg

                # Set input values
                for inp_idx, inp in enumerate(interface["inputs"]):
                    assert inp["event"] == event["event"], "input uses different event"
                    if st >= inp["start"] and st < inp["end"]:
                        # mod._log.warning(f"{inp['name']} <= {data[inp['name']][idx]}")
                        v = data[inp["name"]][idx]
                        width = inp["width"]
                        assert representable(
                            v, width
                        ), f"Invalid: Value {v} not representable in {width} bits"
                        mod._id(inp["name"], extended=False).value = v
                        # Track which inputs have been written to
                        inp_write_idx[inp_idx] += 1

                # Wait for the falling edge so that combinational computations
                # propagate.
                await FallingEdge(mod.clk)

                # For each output, record the value if we expect it to be valid
                for out in interface["outputs"]:
                    assert out["event"] == event["event"], "output uses different event"
                    name = out["name"]
                    if st >= out["start"] and st < out["end"]:
                        v = mod._id(name, extended=False).value
                        try:
                            out = v.integer
                        except ValueError:
                            out = v.binstr
                        outputs[name][idx].append(out)

                # Wait for end of cycle
                await RisingEdge(mod.clk)

        # New transaction should only trigger at the start of a cycle
        await RisingEdge(mod.clk)

        # List of all transactions
        tasks = []
        for idx in range(0, validate_data(data)):
            event = interface["interfaces"][0]
            # Start this transaction
            task = cocotb.start_soon(txn(idx, event))
            tasks.append(task.join())
            # Wait for the specified delay
            delay = (random.randint(0, int(randomize)) if randomize else 0) + event[
                "delay"
            ]
            await ClockCycles(mod.clk, delay)

        # Wait for all transactions to complete
        await triggers.Combine(*tasks)

        return outputs

    return (run, finalize_writes)


async def log_signal(mod, signal_name, times):
    """
    Log a signal every cycle for `times` number of cycles.
    """
    count = times
    while count != 0:
        count = count - 1
        await RisingEdge(mod.clk)
        val = mod._id(signal_name, extended=False).value
        mod._log.info("%s = %s", signal_name, val)


def counter(mod, max_cycles):
    """
    Counts the number of cycles the clock is running.
    """
    count = {"count": 0}

    async def inner():
        while count["count"] < max_cycles:
            await RisingEdge(mod.clk)
            count["count"] += 1
            # mod._log.warn(f"Cycle: {count['count']}")
        return {"error": f"Max cycles {max_cycles} reached"}

    return inner, count


async def setup_design(mod, interface, reset_cycles, max_cycles):
    """
    Connects a clock to the given module and runs the reset signal for 5 cycles
    """

    await cocotb.start(Clock(mod.clk, 10, units="step").start())

    # Set all interface values to 0
    for event in interface["interfaces"]:
        if not event["phantom"]:
            mod._id(event["name"], extended=False).value = 0

    # Initialize a cycle counter
    (proc, count) = counter(mod, max_cycles)
    counter_task = cocotb.start_soon(proc())

    # Reset phase
    mod.reset.value = 1
    await ClockCycles(mod.clk, reset_cycles)  # wait a bit
    mod.reset.value = 0

    return (counter_task, count)


async def run_design(mod):
    interface_file = os.environ.get("INTERFACE")
    assert interface_file, (
        "Provide the location to interface file by "
        "setting the environment variable INTERFACE"
    )
    data_file = os.environ.get("DATA_FILE")
    assert data_file, (
        "Provide the location to data file by"
        " setting the environment variable DATA_FILE"
    )
    # Randomize the pipeline delay with these many cycles
    randomize = os.environ.get("RANDOMIZE")
    # Get the number of reset cycles
    reset_cycles = int(os.environ.get("RESET_CYCLES") or RESET_CYCLES)
    # Maximum number of cycles
    max_cycles = int(os.environ.get("MAX_CYCLES") or MAX_CYCLES)

    with open(interface_file) as f:
        interface = json.load(f)

    with open(data_file) as f:
        data = json.load(f)

    # Setup the design and run it
    (counter_task, count) = await setup_design(mod, interface, reset_cycles, max_cycles)
    (runner, finalizer) = construct_transaction_fsm(interface, randomize)
    cocotb.start_soon(finalizer(mod))
    main = cocotb.start_soon(runner(mod, data))
    outputs = await First(counter_task, main)

    if "error" in outputs:
        print("Outputs:", outputs)
    else:
        # Kill the cycle counter and add the cycle count to outputs
        counter_task.kill()
        # also substract the 1 cycle it takes to propagate the go signal
        outputs["cycles"] = count["count"] - (reset_cycles + 1)

        out = json.dumps(outputs)
        print("Outputs:", out)
