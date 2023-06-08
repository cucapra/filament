module onehotmux #(
  parameter WIDTH = 32
) (
  input wire logic [7:0] sel,
  input wire logic [WIDTH-1:0] in0,
  input wire logic [WIDTH-1:0] in1,
  input wire logic [WIDTH-1:0] in2,
  input wire logic [WIDTH-1:0] in3,
  input wire logic [WIDTH-1:0] in4,
  input wire logic [WIDTH-1:0] in5,
  input wire logic [WIDTH-1:0] in6,
  input wire logic [WIDTH-1:0] in7,
  output reg [WIDTH-1:0] out
);
  always @(*) begin
    case(sel)
      8'b00000001: out = in0;
      8'b00000010: out = in1;
      8'b00000100: out = in2;
      8'b00001000: out = in3;
      8'b00010000: out = in4;
      8'b00100000: out = in5;
      8'b01000000: out = in6;
      8'b10000000: out = in7;
      default: out = in0;
    endcase
  end
endmodule

module rightshifter #(
  parameter WIDTH = 32
) (
  input wire logic signed [WIDTH-1:0] in,
  input wire logic [4:0] shamt,
  input wire logic isArith, // 0 for logical, 1 for arithmetic
  output logic [WIDTH-1:0] out
);
  assign out = isArith ? in >>> shamt : in >> shamt;
endmodule

module Ternary #(
  parameter WIDTH = 32
) (
  input wire logic guard,
  input wire logic [WIDTH-1:0] opTrue,
  input wire logic [WIDTH-1:0] opFalse,
  output wire logic [WIDTH-1:0] out
);
  assign out = guard ? opTrue : opFalse;
endmodule;