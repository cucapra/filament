module Neg #(
  parameter WIDTH = 32
) (
  input wire logic signed [WIDTH-1:0] in,
  output wire logic signed [WIDTH-1:0] out
);
  assign out = -in;
endmodule