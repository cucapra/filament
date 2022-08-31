# type: ignore

import os
import json
import cocotb
from cocotb.triggers import Timer, FallingEdge, RisingEdge
from cocotb.clock import Clock
from cocotb.binary import BinaryValue


def validate_data(data):
    """
    Validate the data format provided to the file.
    """
    total_txns = len(list(data.values())[0])
    assert all([len(vs) == total_txns for vs in data.values()]
               ), "Invalid: Mismatched sizes for inputs"
    return total_txns


def construct_transaction_fsm(interface):
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

    assert len(interface["interfaces"]
               ) == 1, "Unsupported: multiple interfaces"

    # Construct a model of what needs to be done for one transaction
    async def run(mod, data):
        # Dictionary to store the outputs
        # Maps signal_name -> txn_id -> listof values
        outputs = {sig["name"]: {} for sig in interface["outputs"]}

        async def txn(idx):
            """
            Run a complete transaction
            """
            # Add new dict for this transaction to the outputs
            for sig in outputs.keys():
                outputs[sig][idx] = []

            ev = interface["interfaces"][0]

            # Fully execute the module by triggering it till the number of
            # times prescribed by the delay
            for st in range(0, ev["delay"] + 1):
                # Start the transaction by setting the interface signal to 1
                if st == 0:
                    trg = 1
                else:
                    trg = 0
                mod._id(ev["name"], extended=False).value = trg

                # Set input values
                for inp in interface["inputs"]:
                    assert inp["event"] == ev["event"], "input uses different event"
                    if st >= inp["start"] and st < inp["end"]:
                        v = data[inp["name"]][idx]
                    else:
                        v = BinaryValue('x')
                    mod._id(inp["name"], extended=False).value = v

                # Wait for the falling edge so that combinational computations
                # propagate.
                # NOTE(rachit): Not sure if this will actually work for combinational
                # outputs.
                await FallingEdge(mod.clk)

                # For each output, record the value if we expect it to be valid
                for out in interface["outputs"]:
                    assert out["event"] == ev["event"], "output uses different event"
                    name = out["name"]
                    if st >= out["start"] and st < out["end"]:
                        v = mod._id(name, extended=False).value
                        outputs[name][idx].append(v.integer)

                # Wait for end of cycle
                await RisingEdge(mod.clk)

        # New transaction should only trigger at the start of a cycle
        await RisingEdge(mod.clk)

        for idx in range(0, validate_data(data)):
            await txn(idx)

        return outputs

    return run


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


async def setup_design(mod, interface):
    """
    Connects a clock to the given module and runs the reset signal for 5 cycles
    """

    await cocotb.start(Clock(mod.clk, 10, units='step').start())

    # Set all interface values to 0
    for event in interface["interfaces"]:
        mod._id(event["name"], extended=False).value = 0

    # Reset phase
    await RisingEdge(mod.clk)
    mod.reset.setimmediatevalue(1)
    await Timer(20, units='step')  # wait a bit
    mod.reset.setimmediatevalue(0)
    await FallingEdge(mod.clk)  # wait for falling edge/"negedge"


async def run_design(mod):
    interface_file = os.environ.get('INTERFACE')
    assert interface_file, "Provide the location to interface file by setting the environment variable INTERFACE"
    data_file = os.environ.get('DATA_FILE')
    assert data_file, "Provide the location to data file by setting the environment variable DATA_FILE"

    with open(interface_file) as f:
        interface = json.load(f)

    with open(data_file) as f:
        data = json.load(f)

    # Setup the design and run it
    await setup_design(mod, interface)
    runner = construct_transaction_fsm(interface)
    outputs = await runner(mod, data)
    out = json.dumps(outputs)
    print("Outputs:", out)
