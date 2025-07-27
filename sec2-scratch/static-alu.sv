// ALU module - uses floating-point Add and Mul modules with op signal
module ALU #(
    parameter WIDTH = 32,
    parameter ADD_S = 1,
    parameter MUL_S = 1
) (
    input wire clk,
    input wire reset,
    input wire op,              // 0 for add, 1 for multiply
    input wire [WIDTH-1:0] a,   // IEEE 754 single precision
    input wire [WIDTH-1:0] b,   // IEEE 754 single precision
    output reg [WIDTH-1:0] result // IEEE 754 single precision result
);

    localparam MAX = ADD_S > MUL_S ? ADD_S : MUL_S;
    localparam ADD_B = MAX - ADD_S;
    localparam MUL_B = MAX - MUL_S;

    // Intermediate signals
    wire [WIDTH-1:0] add_result;
    wire [WIDTH-1:0] mul_result;

    // Operation selection
    typedef enum logic {
        OP_ADD = 1'b0,
        OP_MUL = 1'b1
    } alu_op_t;

    // Instantiate Add module with configurable pipeline stages
    Add #(.WIDTH(WIDTH), .PIPELINE_STAGES(ADD_S)) adder (
        .clk(clk),
        .a(a),
        .b(b),
        .sum(add_result)
    );

    // Instantiate Mul module with configurable pipeline stages
    Mul #(.WIDTH(WIDTH), .PIPELINE_STAGES(MUL_S)) multiplier (
        .clk(clk),
        .a(a),
        .b(b),
        .product(mul_result)
    );

    wire [WIDTH-1:0] add_balanced;
    wire [WIDTH-1:0] mul_balanced;
    wire op_balanced;

    Shift_register #(.WIDTH(WIDTH), .STAGES(ADD_B)) shift_add (
        .clk(clk),
        .data_in(add_result),
        .data_out(add_balanced)
    );

    Shift_register #(.WIDTH(WIDTH), .STAGES(MUL_B)) shift_mul (
        .clk(clk),
        .data_in(mul_result),
        .data_out(mul_balanced)
    );

    Shift_register #(.WIDTH(1), .STAGES(MAX)) shift_op (
        .clk(clk),
        .data_in(op),
        .data_out(op_balanced)
    );

    // Combinational logic for operation selection
    always_comb begin
        case (op_balanced)
            OP_ADD: result = add_balanced;     // Floating-point Addition
            OP_MUL: result = mul_balanced;     // Floating-point Multiplication
            default: result = 32'h00000000; // Floating-point +0.0
        endcase
    end
endmodule
