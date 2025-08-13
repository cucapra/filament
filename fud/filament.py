import os
import re
from pathlib import Path
import shutil
import logging as log
import json

from fud import errors
from fud.stages import Stage, SourceType, Source
from fud.utils import shell, TmpDir,unwrap_or
from enum import Enum


class CocotbOutput(Enum):
    DAT = 0
    VCD = 1
    DIR = 2


class CocotbExecBase(Stage):
    """
    Execute a Filament generated verilog program through the cocotb testbench
    """

    name = "cocotb"

    def __init__(self, target_state, out, description):
        self.out = out
        super().__init__(
            src_state="filament",
            target_state=target_state,
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description=description,
        )

    @staticmethod
    def pre_install():
        try:
            import cocotb
            import find_libpython

            if find_libpython.find_libpython() is None:
                raise errors.FudRegisterError(
                    "cocotb",
                    (
                        "Failed to find libpython. "
                        "Ensure that your python installation has access to the libpython shared library"
                    ),
                )
        except ImportError:
            raise errors.FudRegisterError(
                "cocotb",
                (
                    "Failed to import find_libpython. "
                    "Ensure that `cocotb' is installed: pip install cocotb"
                ),
            )

        try:
            import pytest
        except ImportError:
            raise errors.FudRegisterError(
                "cocotb",
                (
                    "Failed to import pytest. "
                    "Ensure that `pytest' is installed: pip install pytest"
                ),
            )

    @staticmethod
    def defaults():
        root = Path(__file__).parent.parent.resolve()
        exec = (root / "target" / "debug" / "filament").resolve()
        # Location of the cocotb harness
        self_loc = Path(__file__).parent.resolve()
        common = (self_loc / "harness").resolve()
        return {
            "common": str(common),
            "exec": str(exec),
            "library": str(root),
        }

    def save_file(self, builder, stream, dir, filename):
        """
        Save a file to the given location
        """

        @builder.step(f"Save {{file}} to {{dir}}/{filename}")
        def save(
            stream: SourceType.String, dir: SourceType.Directory
        ) -> SourceType.Path:
            save_loc = Path(dir.name) / filename
            with open(save_loc, "w") as out:
                out.write(stream)
            return Source.path(save_loc)

        return save(stream, dir)

    def _define_steps(self, input, builder, config) -> Source:

        def transform_data(data_path, dir):
            """
            Transform data in data_path from having binary/hex encoding into decimal.
            Creates a new file inside dir, which should be a temp directory.
            """
            data = str(data_path)
            file_orig = open(data)
            file_new = open(dir.name + Path(data).stem + ".json", "w")
            data_dict = json.load(file_orig)

            # iterate through data
            for key in data_dict:
                # for each item in the json object, check if it needs to be transformed
                for i in range(len(data_dict[key])):
                    val = data_dict[key][i]
                    # if it is a string, check if it is in binary or hex
                    if isinstance(val,str):
                        if val.startswith('0b'): # binary format
                            binary_data = val[2:]
                            try:
                                conv = int(binary_data,2)
                            except ValueError:
                                raise errors.InvalidNumericType("\"" + str(val) + "\"" + " in " + data)
                        elif val.startswith('0x'): # hex format
                            binary_data = val[2:]
                            try:
                                conv = int(binary_data,16)
                            except ValueError:
                                raise errors.InvalidNumericType("\"" + str(val) + "\"" + " in " + data)
                        else: # none of the above -> unsupported
                            raise errors.InvalidNumericType("\"" + str(val) + "\"" + " in " + data)
                    else: # already in decimal
                        conv = val
                    # update info in json
                    data_dict[key][i] = conv
            json_obj = json.dumps(data_dict,indent=4)
            file_new.write(json_obj)
            return Path(file_new.name).resolve()

        @builder.step()
        def get_data() -> SourceType.Path:
            """Get data for execution"""
            name = ["stages", self.name, "data"]
            data = config.get(name)
            if data is None:
                raise errors.MissingDynamicConfiguration(".".join(name))

            # return absolute path to the file
            return Path(data).resolve()

        @builder.step()
        def copy_file(
            path: SourceType.Path, dir: SourceType.Directory, file: SourceType.String
        ):
            """
            Copy the file to the given directory
            """
            shutil.copy(path, Path(dir.name) / file)

        @builder.step()
        def mktmp() -> SourceType.Directory:
            """Make a temporary directory"""
            return TmpDir()

        @builder.step()
        def data_gen(file: SourceType.Path, dir: SourceType.Directory) -> SourceType.Stream:
            """
            Generate data file in dir with binary/hex converted to decimal
            """
            data_transformed = transform_data(file, dir)
            cmd = " ".join(
                [
                    "cat ",
                    "{path}"
                ]
            )
            return shell(cmd.format(path = data_transformed))

        @builder.step()
        def interface_gen(file: SourceType.Path) -> SourceType.Stream:
            """
            Generate the interface file for the Filament program
            """
            cmd = " ".join(
                [
                    config["stages", FilamentStage.name, "exec"],
                    "--library",
                    config["stages", FilamentStage.name, "library"],
                    "--dump-interface",
                    # We should only run this after the module has been type
                    # checked.
                    "--unsafe-skip-discharge",
                    config.get(["stages", "filament", "flags"]) or "",
                    "{path}",
                ]
            )
            return shell(cmd.format(path=file))

        @builder.step()
        def copy_harness(dir: SourceType.String):
            """
            Copy the cocotb harness to the temporary directory
            """

            files = ["Makefile", "sim.py", "utils.py"]
            common = config["stages", self.name, "common"]
            for file in files:
                src = Path(common) / file
                shutil.copy(src, Path(dir) / file)

        @builder.step()
        def add_vcd_gen(code: SourceType.String) -> SourceType.String:
            """
            Replace
            `// COMPONENT END: main`
            with:
            `ifdef COCOTB_SIM
            initial begin
                $dumpfile ("out.vcd");
                $dumpvars (0, main);
                #1;
            end
            `endif
            // COMPONENT END: main
            """
            return code.replace(
                "// COMPONENT END: main",
                """
            `ifdef COCOTB_SIM
            initial begin
                $dumpfile ("out.vcd");
                $dumpvars (0, main);
                #1;
            end
            `endif
            // COMPONENT END: main
            """,
            )

        @builder.step()
        def run(
            dir: SourceType.String, interface: SourceType.Path, data: SourceType.Path
        ) -> SourceType.Stream:
            """
            Run the simulation
            """
            # Switch to the tmpdir
            os.chdir(Path(dir))
            # Randomize value
            randomize = config.get(["stages", self.name, "randomize"])
            # Number of reset cycles
            reset_cycles = config.get(["stages", self.name, "reset_cycles"])
            # Execute the make command
            cmd = " ".join(
                [
                    "COMPILE_ARGS='-DICARUS -gstrict-ca-eval'",
                    "make",
                    "-B",
                    # XXX(rachit): we shouldn't need this .data here
                    f"INTERFACE={interface.data}",
                    f"DATA_FILE={data.data}",
                    f"RANDOMIZE={int(randomize)}" if randomize is not None else "",
                    f"RESET_CYCLES={reset_cycles}" if reset_cycles is not None else "",
                ]
            )
            return shell(cmd)

        @builder.step()
        def read_vcd(dir: SourceType.String) -> SourceType.String:
            """
            Read the vcd file
            """
            with open(Path(dir) / "out.vcd", "r") as f:
                return f.read()

        # Schedule the program
        data = get_data()
        dir = mktmp()
        copy_harness(dir)
        mb_verilog = config.get(["stages", self.name, "verilog"])

        # If cocotb.verilog is defined, use that instead of the generating verilog
        if mb_verilog is not None:
            copy_file(Source.path(mb_verilog), dir, Source("out.sv", SourceType.String))
        else:
            # Compile the Filament program to icarus verilog
            path = config.registry.make_path("filament", "icarus-verilog")
            builder.ctx.append("to_verilog")
            verilog_stream = builder.also_do_path(input, path, config)
            builder.ctx.pop()
            # If this is a VCD, we need to add the VCD generation code
            if self.out == CocotbOutput.VCD:
                verilog_stream = add_vcd_gen(verilog_stream)
            # Save the verilog stream into the temporary directory
            self.save_file(builder, verilog_stream, dir, "out.sv")

        # Generate the interface file
        interface_stream = interface_gen(input)
        interface_path = self.save_file(
            builder, interface_stream, dir, "interface.json"
        )

        # Generate modified data file
        data_stream = data_gen(data, dir)
        data_path = self.save_file(builder,data_stream,dir,"data_1.json")

        # Run the program
        out = run(dir, interface_path, data_path)

        if self.out == CocotbOutput.VCD:
            return read_vcd(dir)
        else:
            return out


class CocotbVCD(CocotbExecBase):
    """
    Execute a Filament generated verilog program through the cocotb testbench
    """

    def __init__(self):
        super().__init__(
            target_state="cocotb-vcd",
            out=CocotbOutput.VCD,
            description="Run a Filament program through the cocotb testbench and generate a VCD",
        )


class CocotbOut(CocotbExecBase):
    """
    Execute a Filament generated verilog program through the cocotb testbench
    """

    def __init__(self):
        super().__init__(
            target_state="cocotb-out-raw",
            out=CocotbOutput.DAT,
            description="Run a Filament program through the cocotb testbench and generate output",
        )


class CleanupCocotb(Stage):
    """
    Execute a Filament generated verilog program through the cocotb testbench
    """

    name = "cocotb-cleanup"

    def __init__(self):
        super().__init__(
            src_state="cocotb-out-raw",
            target_state="cocotb-out",
            input_type=SourceType.Stream,
            output_type=SourceType.Stream,
            description="Cleanup the output produced by cocotb",
        )

    @staticmethod
    def pre_install():
        pass

    @staticmethod
    def defaults():
        return {}

    def _define_steps(self, input, builder, config):
        @builder.step()
        def get_results(output: SourceType.Stream) -> SourceType.String:
            """
            Grep the results from cocotb's output
            """
            # Find line that starts with "Outputs:" and return it
            for line in output.readlines():
                # Check if the line contains "error" or "Error"
                if re.search(b"error", line, re.IGNORECASE) is not None:
                    log.error(line.decode("utf-8").strip())
                if re.search(b"warn", line, re.IGNORECASE) is not None:
                    log.warn(line.decode("utf-8").strip())

                if line.startswith(b"Outputs:"):
                    # Remove Output: from the front of the line
                    return line.split(b" ", 1)[1].decode("UTF-8")

            raise ValueError("No results were found")

        return get_results(input)


class FilamentStage(Stage):
    """
    Compile a filament program to calyx
    """

    name = "filament"

    def __init__(self):
        super().__init__(
            src_state="filament",
            target_state="icarus-verilog",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description="Compile a filament program to verilog",
        )

    @staticmethod
    def pre_install():
        pass

    @staticmethod
    def defaults():
        # Path to the filament respository
        root = Path(__file__).parent.parent.resolve()
        exec = (root / "target" / "debug" / "filament").resolve()

        return {"exec": str(exec), "library": str(root), "file_extensions": [".fil"]}

    def _define_steps(self, input_data, builder, config):
        flags = unwrap_or(config.get(["stages", self.name, "flags"]), "")
        cmd = " ".join(
            [
                config["stages", self.name, "exec"],
                "--library",
                config["stages", self.name, "library"],
                flags,
                "{path}",
            ]
        )

        @builder.step(description=cmd)
        def to_verilog(input_path: SourceType.Path) -> SourceType.Stream:
            return shell(cmd.format(path=input_path))

        return to_verilog(input_data)


__STAGES__ = [FilamentStage, CocotbOut, CocotbVCD, CleanupCocotb]
