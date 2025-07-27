// ALU module - uses floating-point Add and Mul modules with op signal
module ALU #(
    parameter WIDTH = 32,
    parameter PIPELINE_STAGES = 1  // Number of pipeline stages for Add/Mul modules
) (
    input wire clk,
    input wire reset,
    input wire op,              // 0 for add, 1 for multiply
    input wire [WIDTH-1:0] a,   // IEEE 754 single precision
    input wire [WIDTH-1:0] b,   // IEEE 754 single precision
    output reg [WIDTH-1:0] result // IEEE 754 single precision result
);
    // Intermediate signals
    wire [WIDTH-1:0] add_result;
    wire [WIDTH-1:0] mul_result;

    // Operation selection
    typedef enum logic {
        OP_ADD = 1'b0,
        OP_MUL = 1'b1
    } alu_op_t;

    // Instantiate Add module with configurable pipeline stages
    Add #(.WIDTH(WIDTH), .PIPELINE_STAGES(PIPELINE_STAGES)) adder (
        .clk(clk),
        .a(a),
        .b(b),
        .sum(add_result)
    );

    // Instantiate Mul module with configurable pipeline stages
    Mul #(.WIDTH(WIDTH), .PIPELINE_STAGES(PIPELINE_STAGES)) multiplier (
        .clk(clk),
        .a(a),
        .b(b),
        .product(mul_result)
    );

    // Combinational logic for operation selection
    always_comb begin
        case (op)
            OP_ADD: result = add_result;     // Floating-point Addition
            OP_MUL: result = mul_result;     // Floating-point Multiplication
            default: result = 32'h00000000; // Floating-point +0.0
        endcase
    end
endmodule
