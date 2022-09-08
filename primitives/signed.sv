module NegConst #(
  parameter WIDTH = 32,
  parameter VALUE = 0
) (
  input logic _go,
  output logic signed [WIDTH-1:0] out
);
  assign out = -VALUE;
endmodule

module SAdd #(
  parameter WIDTH = 32
) (
  input logic _go,
  input logic signed [WIDTH-1:0] left,
  input logic signed [WIDTH-1:0] right,
  output logic signed [WIDTH-1:0] out
);
  assign out = left + right;
endmodule