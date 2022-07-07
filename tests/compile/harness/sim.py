# type: ignore
import cocotb
import sys


@cocotb.test()
async def test(dut):
    """Try accessing the design."""

    # Add path to the test harness folder to enable imports
    sys.path.insert(1, "../")
    from harness.utils import run_design

    await run_design(dut)
