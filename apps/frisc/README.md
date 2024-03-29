# `frisc`: A RISC-V Processor in Filament

`frisc` is an experimental, statically-scheduled [RISC-V processor][riscv] written in Filament that implements the base instruction set.
We're implementing simple 3-stage pipeline and after validating it, will work on more complex pipelines and instruction extensions.

The driver code is written in Calyx and invokes modules generated by Filament.

[riscv]: https://riscv.org/wp-content/uploads/2017/05/riscv-spec-v2.2.pdf
