
//// =========== Fixed Point Library ============

module FXPMult #(
  parameter W = 32,
  parameter D = 16
) (
  input  wire logic [W-1:0] left,
  input  wire logic [W-1:0] right,
  output wire logic [W-1:0] out
);
  wire [W+D-1:0] t_out;
  assign t_out = left * right;
  assign out   = t_out[W+D-1:D];
endmodule
