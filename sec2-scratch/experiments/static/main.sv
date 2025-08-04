// Static Latency-Sensitive ALU
module Static_ALU #(
    parameter ADDER_STAGES = 2,    // Number of pipeline stages for adder
    parameter MULT_STAGES = 3      // Number of pipeline stages for multiplier
) (
    input clk,
    input reset,
    input [31:0] operand_a,
    input [31:0] operand_b,
    input [1:0] operation,         // 2'b00: Add, 2'b01: Subtract, 2'b10: Multiply, 2'b11: Reserved
    output reg [31:0] result,
    output reg exception,
    output reg overflow,
    output reg underflow,
    output reg valid           // Result is valid after pipeline latency
);

// Operation encoding  
localparam OP_ADD = 2'b00;
localparam OP_SUB = 2'b01;
localparam OP_MUL = 2'b10;

// Determine maximum pipeline depth
localparam MAX_STAGES = (ADDER_STAGES > MULT_STAGES) ? ADDER_STAGES : MULT_STAGES;

// Adder signals
wire [31:0] add_result;
wire [31:0] sub_operand_b;

// Multiplier signals  
wire [31:0] mult_result;
wire mult_exception, mult_overflow, mult_underflow;

// Pipeline registers for operation and result selection
reg [1:0] operation_pipe [0:MAX_STAGES];
reg [31:0] result_pipe [0:MAX_STAGES-1];
reg exception_pipe [0:MAX_STAGES-1];
reg overflow_pipe [0:MAX_STAGES-1];
reg underflow_pipe [0:MAX_STAGES-1];

// Valid signal pipeline - tracks when results are available
reg [MAX_STAGES:0] valid_pipe;

integer i;

// Subtraction: negate operand_b for addition
assign sub_operand_b = {~operand_b[31], operand_b[30:0]};

// Instantiate adder wrapper
FP_Adder_Wrapper #(.STAGES(ADDER_STAGES)) adder (
    .clk(clk),
    .reset(reset),
    .a(operand_a),
    .b((operation == OP_SUB) ? sub_operand_b : operand_b),
    .result(add_result)
);

// Instantiate multiplier wrapper
FP_Mult_Wrapper #(.STAGES(MULT_STAGES)) multiplier (
    .clk(clk),
    .reset(reset),
    .a(operand_a),
    .b(operand_b),
    .result(mult_result),
    .exception(mult_exception),
    .overflow(mult_overflow),
    .underflow(mult_underflow)
);

// Pipeline management
always @(posedge clk) begin
    if (reset) begin
        // Reset pipeline registers
        for (i = 0; i <= MAX_STAGES; i++) begin
            operation_pipe[i] <= 2'b00;
        end
        for (i = 0; i < MAX_STAGES; i++) begin
            result_pipe[i] <= 32'h0;
            exception_pipe[i] <= 1'b0;
            overflow_pipe[i] <= 1'b0;
            underflow_pipe[i] <= 1'b0;
        end
        valid_pipe <= {(MAX_STAGES+1){1'b0}};
        
        result <= 32'h0;
        exception <= 1'b0;
        overflow <= 1'b0;
        underflow <= 1'b0;
        valid <= 1'b0;
    end else begin
        // Shift operation through pipeline
        operation_pipe[0] <= operation;
        for (i = 1; i <= MAX_STAGES; i++) begin
            operation_pipe[i] <= operation_pipe[i-1];
        end
        
        // Shift valid signal through pipeline
        valid_pipe[0] <= 1'b1;  // Input is always valid
        for (i = 1; i <= MAX_STAGES; i++) begin
            valid_pipe[i] <= valid_pipe[i-1];
        end
        
        // Select results based on operation at appropriate pipeline stage
        case (operation_pipe[ADDER_STAGES])
            OP_ADD, OP_SUB: begin
                result <= add_result;
                exception <= 1'b0;  // Adder doesn't generate exceptions in this implementation
                overflow <= 1'b0;
                underflow <= 1'b0;
                valid <= valid_pipe[ADDER_STAGES];
            end
            default: begin  // Multiplication and default case
                result <= mult_result;
                exception <= mult_exception;
                overflow <= mult_overflow;
                underflow <= mult_underflow;
                valid <= valid_pipe[MULT_STAGES];
            end
        endcase
    end
end

// Pipeline variant modules are compiled separately

endmodule