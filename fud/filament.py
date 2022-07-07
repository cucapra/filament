from pathlib import Path

from fud.stages import Stage, SourceType
from fud.utils import shell


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


__STAGES__ = [FilamentStage]
