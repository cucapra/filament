// Latency-Insensitive ALU with ready-valid interface
module Dynamic_ALU #(
    parameter ADDER_STAGES = 2,    // Number of pipeline stages for adder
    parameter MULT_STAGES = 3      // Number of pipeline stages for multiplier
) (
    input clk,
    input reset,
    
    // Input interface
    input [31:0] operand_a,
    input [31:0] operand_b,
    input [1:0] operation,         // 2'b00: Add, 2'b01: Subtract, 2'b10: Multiply, 2'b11: Reserved
    input valid_in,
    output ready_out,
    
    // Output interface
    output [31:0] result,
    output exception,
    output overflow,
    output underflow,
    output valid_out,
    input ready_in
);

// Operation encoding  
localparam OP_ADD = 2'b00;
localparam OP_SUB = 2'b01;
localparam OP_MUL = 2'b10;

// Input arbitration and demux
wire add_valid_in, mult_valid_in;
wire add_ready_out, mult_ready_out;
wire [31:0] sub_operand_b;

// Adder/subtractor signals
wire [31:0] add_result;
wire add_valid_out;

// Multiplier signals
wire [31:0] mult_result;
wire mult_exception, mult_overflow, mult_underflow;
wire mult_valid_out;

// Output arbitration
reg [1:0] output_select;

// Ready signals to functional units
wire add_ready_in, mult_ready_in;

// Subtraction: negate operand_b for addition
assign sub_operand_b = {~operand_b[31], operand_b[30:0]};

// Input demultiplexing based on operation
assign add_valid_in = valid_in && (operation == OP_ADD || operation == OP_SUB);
assign mult_valid_in = valid_in && (operation == OP_MUL);

// Ready signal - we're ready if the target functional unit is ready
assign ready_out = (operation == OP_ADD || operation == OP_SUB) ? add_ready_out : 
                   (operation == OP_MUL) ? mult_ready_out : add_ready_out;

// Instantiate adder with LI wrapper
FP_Adder_LI_Wrapper #(.STAGES(ADDER_STAGES)) adder (
    .clk(clk),
    .reset(reset),
    .a(operand_a),
    .b((operation == OP_SUB) ? sub_operand_b : operand_b),
    .valid_in(add_valid_in),
    .ready_out(add_ready_out),
    .result(add_result),
    .valid_out(add_valid_out),
    .ready_in(add_ready_in)
);

// Instantiate multiplier with LI wrapper
FP_Mult_LI_Wrapper #(.STAGES(MULT_STAGES)) multiplier (
    .clk(clk),
    .reset(reset),
    .a(operand_a),
    .b(operand_b),
    .valid_in(mult_valid_in),
    .ready_out(mult_ready_out),
    .result(mult_result),
    .exception(mult_exception),
    .overflow(mult_overflow),
    .underflow(mult_underflow),
    .valid_out(mult_valid_out),
    .ready_in(mult_ready_in)
);

// Output arbitration - simple priority scheme
// Adder has priority when both are ready
assign output_select = add_valid_out ? 2'b00 : 2'b10;

// Ready signal routing - only the selected unit gets the ready
assign add_ready_in = add_valid_out ? ready_in : 1'b0;
assign mult_ready_in = (!add_valid_out && mult_valid_out) ? ready_in : 1'b0;

// Output assignment
assign valid_out = add_valid_out || mult_valid_out;
assign result = add_valid_out ? add_result : mult_result;
assign exception = add_valid_out ? 1'b0 : mult_exception;
assign overflow = add_valid_out ? 1'b0 : mult_overflow;
assign underflow = add_valid_out ? 1'b0 : mult_underflow;

// Required modules are compiled separately

endmodule