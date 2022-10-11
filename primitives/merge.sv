module Merge #(
    parameter WIDTH = 32
) (
    input clk,
    input reset,
    input logic go_G,
    input logic go_S,
    input wire logic [WIDTH-1:0] in0,
    input wire logic [WIDTH-1:0] in1,
    output wire logic [WIDTH-1:0] out
);

reg state;
always_ff @(posedge clk) begin
    if (reset) begin
        // Start at G state because we cannot have an 'x value here
        state <= 1'd0;
    end else if (go_G) begin
        state <= 1'd0;
    end else if (go_S) begin
        state <= 1'd1;
    end else begin
        state <= state;
    end
end

logic cmp_G = state == 1'd0;
logic cmp_S = state == 1'd1;
logic forward_in0 = go_G || cmp_G;
logic forward_in1 = go_S || cmp_S;

// No need to register the inputs because the user is required to hold them.
assign out = forward_in0 ? in0 :
             forward_in1 ? in1 :
             'x;
endmodule