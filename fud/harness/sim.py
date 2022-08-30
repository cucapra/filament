# type: ignore
import cocotb
from utils import run_design


@cocotb.test()
async def test(dut):
    """Try accessing the design."""
    await run_design(dut)
