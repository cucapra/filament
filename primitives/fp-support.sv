// Returns input[30:23]
module Slice30_23
(
    input logic _go,
    input logic [31:0] in,
    output logic [7:0] out
);
    assign out = in[30:23];
endmodule

// Returns in[23:1]
module Slice23_1
(
    input logic _go,
    input logic [23:0] in,
    output logic [22:0] out
);
    assign out = in[23:1];
endmodule

// Returns input[20:0]
module Slice22_0
(
    input logic _go,
    input logic [31:0] in,
    output logic [22:0] out
);
    assign out = in[22:0];
endmodule

module IsZero #(
    parameter WIDTH = 32
)
(
    input logic _go,
    input logic [WIDTH-1:0] in,
    output logic out
);
    assign out = in == '0;
endmodule

module IsOne (
    input wire logic _go,
    input wire logic in,
    output wire logic out
);
    assign out = in == '1;
endmodule

// Computes out = {1'b1,in[22:1]};
module NormInp
(
    input wire logic _go,
    input wire logic [22:0] in,
    output wire logic [22:0] out
);
    assign out = {1'b1,in[22:1]};
endmodule

module ShiftRight23
(
    input wire logic _go,
    input wire logic [22:0] in,
    input wire logic [7:0] amount,
    output wire logic [22:0] out
);
    assign out = in >> amount;
endmodule

module ShiftLeft24
(
    input wire logic _go,
    input wire logic [23:0] in,
    input wire logic [3:0] amount,
    output wire logic [23:0] out
);
    assign out = in << amount;
endmodule

module MergeRes(
    input wire logic _go,
    input wire logic sign,
    input wire logic [7:0] exp,
    input wire logic [22:0] mant,
    output wire logic [31:0] out
);
    assign out = { sign, exp, mant };
endmodule