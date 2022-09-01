import os
from pathlib import Path
import shutil

from fud import errors
from fud.stages import Stage, SourceType, Source
from fud.utils import shell, TmpDir


class CocotbExec(Stage):
    """
    Execute a Filament generated verilog program through the cocotb testbench
    """

    name = "cocotb"

    def __init__(self):
        super().__init__(
            src_state="filament",
            target_state="cocotb-out",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description="Execute a Filament generated verilog program through the cocotb testbench",
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
        def save(stream: SourceType.String, dir: SourceType.Directory) -> SourceType.Path:
            save_loc = Path(dir.name) / filename
            with open(save_loc, 'w') as out:
                out.write(stream)
            return Source.path(save_loc)
        return save(stream, dir)

    def _define_steps(self, input, builder, config) -> Source:

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
        def copy_file(path: SourceType.Path, dir: SourceType.Directory, file: SourceType.String):
            """
            Copy the file to the given directory
            """
            shutil.copy(path, Path(dir.name) / file)

        @builder.step()
        def mktmp() -> SourceType.Directory:
            """Make a temporary directory"""
            return TmpDir()

        @builder.step()
        def interface_gen(file: SourceType.Path) -> SourceType.Stream:
            """
            Generate the interface file for the Filament program
            """
            cmd = " ".join([
                config["stages", self.name, "exec"],
                "--library",
                config["stages", self.name, "library"],
                "--calyx-primitives",
                config["global", "futil_directory"],
                "--dump-interface",
                "{path}"
            ])
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
        def run(dir: SourceType.String, interface: SourceType.Path, data: SourceType.Path) -> SourceType.Stream:
            """
            Run the simulation
            """
            # Switch to the tmpdir
            os.chdir(Path(dir))
            # Execute the make command
            cmd = " ".join([
                "make", "-B",
                # XXX(rachit): we shouldn't need this .data here
                f"INTERFACE={interface.data}",
                f"DATA_FILE={data}"
            ])
            return shell(cmd)

        @builder.step()
        def get_results(output: SourceType.Stream) -> SourceType.String:
            """
            Grep the results from cocotb's output
            """
            # Find line that starts with "Outputs:" and return it
            for line in output.readlines():
                if line.startswith(b"Outputs:"):
                    # Remove Output: from the front of the line
                    return line.split(b" ", 1)[1].decode('UTF-8')

            return "No results were found"

        # Schedule the program
        data = get_data()
        dir = mktmp()
        copy_harness(dir)
        mb_verilog = config.get(["stages", self.name, "verilog"])

        # If cocotb.verilog is defined, use that instead of the generating verilog
        if mb_verilog is not None:
            copy_file(Source.path(mb_verilog), dir,
                      Source("out.sv", SourceType.String))
        else:
            # Compile the Filament program to icarus verilog
            path = config.registry.make_path("filament", "icarus-verilog")
            builder.ctx.append("to_verilog")
            verilog_stream = builder.also_do_path(input, path, config)
            builder.ctx.pop()
            # Save the verilog stream into the temporary directory
            self.save_file(builder, verilog_stream, dir, "out.sv")

        # Generate the interface file
        interface_stream = interface_gen(input)
        interface_path = self.save_file(
            builder, interface_stream, dir, "interface.json")

        # Run the program
        out = run(dir, interface_path, data)
        return get_results(out)


class FilamentStage(Stage):
    """
    Compile a filament program to calyx
    """

    name = "filament"

    def __init__(self):
        super().__init__(
            src_state="filament",
            target_state="futil",
            input_type=SourceType.Path,
            output_type=SourceType.Stream,
            description="Compile a filament program to calyx",
        )

    @staticmethod
    def defaults():
        # Path to the filament respository
        root = Path(__file__).parent.parent.resolve()
        exec = (root / "target" / "debug" / "filament").resolve()

        return {
            "exec": str(exec),
            "library": str(root),
            "file_extensions": [".fil"]
        }

    def _define_steps(self, input_data, builder, config):

        cmd = " ".join([
            config["stages", self.name, "exec"],
            "--library",
            config["stages", self.name, "library"],
            "--calyx-primitives",
            config["global", "futil_directory"],
            "{path}"
        ])

        @builder.step(description=cmd)
        def to_calyx(input_path: SourceType.Path) -> SourceType.Stream:
            return shell(
                cmd.format(path=input_path)
            )

        return to_calyx(input_data)


__STAGES__ = [FilamentStage, CocotbExec]
