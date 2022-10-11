module Modulo #(
    parameter WIDTH = 32
) (
    input wire [WIDTH-1:0] left,
    input wire [WIDTH-1:0] right,
    output wire [WIDTH-1:0] out
);
    assign out = left % right;
endmodule

module Div #(
    parameter WIDTH = 32
) (
    input wire [WIDTH-1:0] left,
    input wire [WIDTH-1:0] right,
    output wire [WIDTH-1:0] out
);
    assign out = left / right;
endmodule