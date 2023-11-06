# Push fft through synthesis
## Using flopoco (at 06e7a80)
```sh
cargo run apps/fft/standalone.fil > out.sv
fud e -vv --from synth-verilog out.sv --to resource-estimate
```
Returns:
```json
{
  "lut": 5273,
  "dsp": 4,
  "registers": 1362,
  "muxes": 278,
  "clb_registers": 4824,
  "carry8": 63,
  "f7_muxes": 0,
  "f8_muxes": 0,
  "f9_muxes": 0,
  "clb": 10160,
  "meet_timing": 1,
  "worst_slack": 1.299,
  "period": 7.0,
  "frequency": 142.857,
  "uram": -1,
  "cell_lut1": 22,
  "cell_lut2": 380,
  "cell_lut3": 1785,
  "cell_lut4": 602,
  "cell_lut5": 1536,
  "cell_lut6": 1889,
  "cell_fdre": 4824
}
```