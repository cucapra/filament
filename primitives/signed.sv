module NegConst #(
  parameter WIDTH = 32,
  parameter VALUE = 0
) (
  input wire logic _go,
  output wire logic signed [WIDTH-1:0] out
);
  assign out = -VALUE;
endmodule

module SAdd #(
  parameter WIDTH = 32
) (
  input wire logic _go,
  input wire logic signed [WIDTH-1:0] left,
  input wire logic signed [WIDTH-1:0] right,
  output wire logic signed [WIDTH-1:0] out
);
  assign out = left + right;
endmodule