// Parameterized wrapper for floating point multiplier
module FP_Mult_Wrapper #(
    parameter STAGES = 1  // 1, 2, 3, or 4 stages
) (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output [31:0] result,
    output exception,
    output overflow,
    output underflow
);

generate
    if (STAGES == 1) begin : gen_1stage
        FP_Mult_1Stage multiplier (
            .clk(clk),
            .reset(reset),
            .a(a),
            .b(b),
            .result(result),
            .exception(exception),
            .overflow(overflow),
            .underflow(underflow)
        );
    end else if (STAGES == 2) begin : gen_2stage
        FP_Mult_2Stage multiplier (
            .clk(clk),
            .reset(reset),
            .a(a),
            .b(b),
            .result(result),
            .exception(exception),
            .overflow(overflow),
            .underflow(underflow)
        );
    end else if (STAGES == 3) begin : gen_3stage
        FP_Mult_3Stage multiplier (
            .clk(clk),
            .reset(reset),
            .a(a),
            .b(b),
            .result(result),
            .exception(exception),
            .overflow(overflow),
            .underflow(underflow)
        );
    end else if (STAGES == 4) begin : gen_4stage
        FP_Mult_4Stage multiplier (
            .clk(clk),
            .reset(reset),
            .a(a),
            .b(b),
            .result(result),
            .exception(exception),
            .overflow(overflow),
            .underflow(underflow)
        );
    end else begin : gen_error
        // Unsupported stage count - generate compile error
        initial begin
            $error("Unsupported STAGES parameter: %d. Must be 1, 2, 3, or 4.", STAGES);
        end
    end
endgenerate

endmodule
