from pathlib import Path

from fud.stages import Stage, SourceType
from fud.utils import shell


class Cocotb(Stage):
    def __init__(self):
        super().__init__(
            src_state='icarus-verilog',
            target_state='cocotb',
            input_type=SourceType.Path,
            output_type=SourceType.Terminal,
            description="Compile a filament program to calyx",
        )
