module Const #(
  parameter WIDTH = 32,
  parameter VALUE = 0
) (
  output wire logic [WIDTH-1:0] out
);
  assign out = VALUE;
endmodule

//// =========== Computation ============

module Add #(
  parameter IN_WIDTH = 32,
  parameter OUT_WIDTH = 32
) (
  input wire logic [IN_WIDTH-1:0] left,
  input wire logic [IN_WIDTH-1:0] right,
  output wire logic [OUT_WIDTH-1:0] out
);
  assign out = left + right;
endmodule

module Sub #(
  parameter IN_WIDTH = 32,
  parameter OUT_WIDTH = 32
) (
  input wire logic [IN_WIDTH-1:0] left,
  input wire logic [IN_WIDTH-1:0] right,
  output wire logic [OUT_WIDTH-1:0] out
);
  assign out = left - right;
endmodule

module MultComb #(
  parameter IN_WIDTH = 32,
  parameter OUT_WIDTH = 32
) (
  input wire logic [IN_WIDTH-1:0] left,
  input wire logic [IN_WIDTH-1:0] right,
  output wire logic [OUT_WIDTH-1:0] out
);
  assign out = left * right;
endmodule

//// =========== Binary Logical Operations ============

module And #(
  parameter WIDTH = 32
) (
  input wire logic [WIDTH-1:0] left,
  input wire logic [WIDTH-1:0] right,
  output wire logic [WIDTH-1:0] out
);
  assign out = left & right;
endmodule

module Or #(
  parameter WIDTH = 32
) (
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic [WIDTH-1:0] out
);
  assign out = left | right;
endmodule

module Xor #(
  parameter WIDTH = 32
) (
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic [WIDTH-1:0] out
);
  assign out = left ^ right;
endmodule

module Not #(
  parameter WIDTH = 32
) (
  input wire logic  [WIDTH-1:0] in,
  output wire logic [WIDTH-1:0] out
);
  assign out = ~in;
endmodule

//// =========== Comparions ============

module Gt #(
  parameter WIDTH = 32
) (
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic out
);
  assign out = left > right;
endmodule

module Eq #(
  parameter WIDTH = 32
) (
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic out
);
  assign out = left == right;
endmodule

module Lt #(
  parameter WIDTH = 32
) (
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic out
);
  assign out = left < right;
endmodule

module Gte #(
  parameter WIDTH = 32
) (
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic out
);
  assign out = left >= right;
endmodule

module Lte #(
  parameter WIDTH = 32
) (
  input wire logic  [WIDTH-1:0] left,
  input wire logic  [WIDTH-1:0] right,
  output wire logic out
);
  assign out = left <= right;
endmodule

//// =========== Extension ============

module ZeroExtend #(
  parameter IN_WIDTH = 32,
  parameter OUT_WIDTH = 32
) (
  input wire logic [IN_WIDTH-1:0] in,
  output wire logic [OUT_WIDTH-1:0] out
);
  parameter EXTEND = OUT_WIDTH - IN_WIDTH;
  assign out = {{EXTEND{1'b0}}, in};
endmodule

module Extend #(
  parameter IN_WIDTH = 32,
  parameter OUT_WIDTH = 32
) (
  input wire logic [IN_WIDTH-1:0] in,
  output wire logic [OUT_WIDTH-1:0] out
);
  assign out = {OUT_WIDTH{in}};
endmodule

module Concat #(
  parameter LEFT = 32,
  parameter RIGHT = 32,
  parameter OUT = 64
) (
  input wire logic [LEFT-1:0] left,
  input wire logic [RIGHT-1:0] right,
  output wire logic [OUT-1:0] out
);
  assign out = {left, right};
endmodule

//// =========== Select bits ============
module Select #(
  parameter WIDTH = 32,
  parameter POS = 0
) (
  input wire logic [WIDTH-1:0] in,
  output wire logic out
);
  assign out = in[POS];
endmodule

module Slice #(
  parameter IN_WIDTH = 32,
  parameter MSB = 31,
  parameter LSB = 0,
  parameter OUT_WIDTH = 32
) (
  input wire logic [IN_WIDTH-1:0] in,
  output wire logic [OUT_WIDTH-1:0] out
);
  assign out = in[MSB:LSB];
endmodule

/// =========== Reduction Operations ============
module ReduceAnd #(
  parameter WIDTH = 32
) (
  input wire logic [WIDTH-1:0] in,
  output wire logic out
);
  assign out = &in;
endmodule

module ReduceOr #(
  parameter WIDTH = 32
) (
  input wire logic [WIDTH-1:0] in,
  output wire logic out
);
  assign out = |in;
endmodule

/// ========== Shift Operations ============
module ShiftLeft #(
  parameter WIDTH = 32
) (
  input wire logic [WIDTH-1:0] in,
  input wire logic [WIDTH-1:0] shift,
  output wire logic [WIDTH-1:0] out
);
  assign out = in << shift;
endmodule

module ShiftRight #(
  parameter WIDTH = 32
) (
  input wire logic [WIDTH-1:0] in,
  input wire logic [WIDTH-1:0] shift,
  output wire logic [WIDTH-1:0] out
);
  assign out = in >> shift;
endmodule

module ArithShiftRight #(
  parameter WIDTH = 32
) (
  input wire logic signed [WIDTH-1:0] in,
  input wire logic [WIDTH-1:0] shift,
  output wire logic [WIDTH-1:0] out
); 
  assign out = in >>> shift;
endmodule

module Mux #(
  parameter WIDTH = 32
) (
  input wire logic sel,
  input wire logic [WIDTH-1:0] in0,
  input wire logic [WIDTH-1:0] in1,
  output logic [WIDTH-1:0] out
);
  assign out = sel ? in0 : in1;
endmodule

