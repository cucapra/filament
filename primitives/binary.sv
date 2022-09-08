`default_nettype none

module Const #(
  parameter WIDTH = 32,
  parameter VALUE = 0
) (
  input logic _go,
  output [WIDTH-1:0] out
);
  assign out = VALUE;
endmodule

module Add #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [WIDTH-1:0] left,
  input wire logic [WIDTH-1:0] right,
  output wire logic [WIDTH-1:0] out
);
  assign out = left + right;
endmodule

module Sub #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [WIDTH-1:0] left,
  input wire logic [WIDTH-1:0] right,
  output wire logic [WIDTH-1:0] out
);
  assign out = left - right;
endmodule

module MultComb #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [WIDTH-1:0] left,
  input wire logic [WIDTH-1:0] right,
  output wire logic [WIDTH-1:0] out
);
  assign out = left * right;
endmodule

module And #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [WIDTH-1:0] left,
  input wire logic [WIDTH-1:0] right,
  output wire logic [WIDTH-1:0] out
);
  assign out = left & right;
endmodule

module Or #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic [WIDTH-1:0] out
);
  assign out = left | right;
endmodule

module Not #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic  [WIDTH-1:0] in,
  output wire logic [WIDTH-1:0] out
);
  assign out = ~in;
endmodule

module Gt #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic out
);
  assign out = left > right;
endmodule

module Eq #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic out
);
  assign out = left == right;
endmodule

module Lt #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic out
);
  assign out = left < right;
endmodule

module Gte #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic out
);
  assign out = left >= right;
endmodule

module Lte #(
  parameter WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic out
);
  assign out = left <= right;
endmodule

module ZeroExtend #(
  parameter IN_WIDTH = 32,
  parameter OUT_WIDTH = 32
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [IN_WIDTH-1:0] in,
  output wire logic [OUT_WIDTH-1:0] out
);
  parameter EXTEND = OUT_WIDTH - IN_WIDTH;
  assign out = {{EXTEND{1'b0}}, in};
endmodule

module Select #(
  parameter WIDTH = 32,
  parameter POS = 0
) (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [WIDTH-1:0] in,
  output wire logic out
);
  assign out = in[POS];
endmodule
