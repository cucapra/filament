
//// =========== Fixed Point Library ============

module FXPMult #(
  parameter W = 32,
  parameter D = 16
) (
  input  wire logic [W-1:0] left,
  input  wire logic [W-1:0] right,
  output wire logic [W-1:0] out
);
  wire [W+D-1:0] t_out, l_ext, r_ext;
  assign l_ext = {{D{left[W-1]}}, left};
  assign r_ext = {{D{right[W-1]}}, right};
  assign t_out = l_ext * r_ext;
  assign out   = t_out[W+D-1:D];
endmodule
