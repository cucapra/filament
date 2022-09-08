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
    input logic _go,
    input logic in,
    output logic out
);
    assign out = in == '1;
endmodule

// Computes out = {1'b1,in[22:1]};
module NormInp
(
    input logic _go,
    input logic [22:0] in,
    output logic [22:0] out
);
    assign out = {1'b1,in[22:1]};
endmodule

module ShiftRight23
(
    input logic _go,
    input logic [22:0] in,
    input logic [7:0] amount,
    output logic [22:0] out
);
    assign out = in >> amount;
endmodule

module ShiftLeft24
(
    input logic _go,
    input logic [24:0] in,
    input logic [3:0] amount,
    output logic [24:0] out
);
    assign out = in << amount;
endmodule

module MergeRes(
    input logic _go,
    input logic sign,
    input logic [7:0] exp,
    input logic [22:0] mant,
    output logic [31:0] out
);
    assign out = { sign, exp, mant };
endmodule