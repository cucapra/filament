// Complete Dynamic ALU Design with Input Registers for Hold Timing Fix
// Top-level module required for synthesis

module main (
    input clk,
    input reset,
    
    // Input interface
    input [31:0] operand_a,
    input [31:0] operand_b,
    input [1:0] operation,
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

    // INPUT REGISTERS - Architectural fix for hold violations
    // These add one cycle of latency but provide sufficient data path delay
    reg [31:0] operand_a_reg, operand_b_reg;
    reg [1:0] operation_reg;
    reg valid_in_reg;
    reg ready_in_reg;
    reg reset_reg;
    
    // Ready signal needs special handling for pipelined inputs
    // We're ready if the ALU is ready OR if we have pipeline space
    wire alu_ready_out;
    assign ready_out = alu_ready_out; // Simplified for now - could add FIFO logic
    
    // Input register stage with synchronous reset
    always @(posedge clk) begin
        if (reset) begin
            operand_a_reg <= 32'h0;
            operand_b_reg <= 32'h0;
            operation_reg <= 2'b00;
            valid_in_reg <= 1'b0;
            ready_in_reg <= 1'b0;
            reset_reg <= 1'b1;  // Keep reset asserted for one cycle
        end else begin
            operand_a_reg <= operand_a;
            operand_b_reg <= operand_b;
            operation_reg <= operation;
            valid_in_reg <= valid_in;
            ready_in_reg <= ready_in;
            reset_reg <= 1'b0;   // Release reset after one cycle
        end
    end

    // Instantiate Dynamic ALU with registered inputs
    Dynamic_ALU #(
        .ADDER_STAGES(2),
        .MULT_STAGES(3)
    ) alu_inst (
        .clk(clk),
        .reset(reset_reg),        // Use registered reset
        .operand_a(operand_a_reg), // Use registered inputs
        .operand_b(operand_b_reg),
        .operation(operation_reg),
        .valid_in(valid_in_reg),
        .ready_out(alu_ready_out),
        .result(result),          // Outputs connect directly
        .exception(exception),
        .overflow(overflow),
        .underflow(underflow),
        .valid_out(valid_out),
        .ready_in(ready_in_reg)   // Use registered ready_in
    );

endmodule

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

endmodule

// Latency-Insensitive wrapper for floating point adder with ready-valid interface
module FP_Adder_LI_Wrapper #(
    parameter STAGES = 1  // 1, 2, 3, or 4 stages
) (
    input clk,
    input reset,
    
    // Input interface
    input [31:0] a,
    input [31:0] b,
    input valid_in,
    output ready_out,
    
    // Output interface  
    output [31:0] result,
    output valid_out,
    input ready_in
);

// Internal FIFO to buffer inputs and track pipeline
localparam FIFO_SIZE = (STAGES < 2) ? 4 : (2*STAGES);
localparam ADDR_WIDTH = (FIFO_SIZE <= 4) ? 2 : 3;  // 2 bits for size ≤4, 3 bits for size ≤8
reg [63:0] input_fifo [0:FIFO_SIZE-1];   // Store {a, b}
reg [STAGES:0] valid_shift;              // Track valid data through pipeline
reg [ADDR_WIDTH-1:0] fifo_head, fifo_tail;
reg [ADDR_WIDTH:0] fifo_count;           // One extra bit to count up to FIFO_SIZE

// Core adder signals
reg [31:0] core_a, core_b;
wire [31:0] core_result;

// Output valid persistence
reg output_valid_reg;
wire transaction_complete = output_valid_reg && ready_in;

// Stall logic with proper backpressure
wire output_stalled = output_valid_reg && !ready_in;
wire pipeline_advance = !output_stalled;
wire input_ready = (fifo_count < FIFO_SIZE) && !output_stalled && !reset;

assign ready_out = input_ready;
assign valid_out = output_valid_reg;
assign result = core_result;

integer i;

// Instantiate the core adder
FP_Adder_Wrapper #(.STAGES(STAGES)) core_adder (
    .clk(clk),
    .reset(reset),
    .a(core_a),
    .b(core_b),
    .result(core_result)
);

always @(posedge clk) begin
    if (reset) begin
        fifo_head <= 0;
        fifo_tail <= 0;
        fifo_count <= 0;
        valid_shift <= 0;
        output_valid_reg <= 0;
        core_a <= 0;
        core_b <= 0;
        
        // Clear FIFO
        for (i = 0; i < FIFO_SIZE; i++) begin
            input_fifo[i] <= 64'h0;
        end
    end else begin
        // Input side: enqueue when valid input and we're ready
        if (valid_in && ready_out) begin
            input_fifo[fifo_tail] <= {a, b};
            /* verilator lint_off WIDTHEXPAND */
            fifo_tail <= ADDR_WIDTH'((fifo_tail + 1'b1) % FIFO_SIZE);
            /* verilator lint_on WIDTHEXPAND */
            fifo_count <= fifo_count + 1'b1;
        end
        
        // Output valid management - latch valid until transaction completes
        if (valid_shift[STAGES] && !output_valid_reg) begin
            output_valid_reg <= 1'b1;  // Latch valid when data reaches output
        end else if (transaction_complete) begin
            output_valid_reg <= 1'b0;  // Clear after handshake
        end
        
        // Pipeline advance logic - only when not stalled
        if (pipeline_advance) begin
            // Shift valid bits through pipeline
            for (i = STAGES; i > 0; i--) begin
                valid_shift[i] <= valid_shift[i-1];
            end
            
            // Feed new data to adder if FIFO has data
            if (fifo_count > 0) begin
                {core_a, core_b} <= input_fifo[fifo_head];
                /* verilator lint_off WIDTHEXPAND */
                fifo_head <= ADDR_WIDTH'((fifo_head + 1'b1) % FIFO_SIZE);
                /* verilator lint_on WIDTHEXPAND */
                fifo_count <= fifo_count - 1'b1;
                valid_shift[0] <= 1'b1;
            end else begin
                valid_shift[0] <= 1'b0;
            end
        end
    end
end

endmodule

// Parameterized wrapper for floating point adder
module FP_Adder_Wrapper #(
    parameter STAGES = 1  // 1, 2, 3, or 4 stages
) (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output [31:0] result
);

generate
    if (STAGES == 1) begin : gen_1stage
        FP_Adder_1Stage adder (
            .clk(clk),
            .reset(reset),
            .a(a),
            .b(b),
            .result(result)
        );
    end else if (STAGES == 2) begin : gen_2stage
        FP_Adder_2Stage adder (
            .clk(clk),
            .reset(reset),
            .a(a),
            .b(b),
            .result(result)
        );
    end else if (STAGES == 3) begin : gen_3stage
        FP_Adder_3Stage adder (
            .clk(clk),
            .reset(reset),
            .a(a),
            .b(b),
            .result(result)
        );
    end else if (STAGES == 4) begin : gen_4stage
        FP_Adder_4Stage adder (
            .clk(clk),
            .reset(reset),
            .a(a),
            .b(b),
            .result(result)
        );
    end else begin : gen_error
        // Unsupported stage count - generate compile error
        initial begin
            $error("Unsupported STAGES parameter: %d. Must be 1, 2, 3, or 4.", STAGES);
        end
    end
endgenerate

endmodule

// 1-stage pipelined floating point adder
module FP_Adder_1Stage (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output reg [31:0] result
);

reg [31:0] a_reg, b_reg;

// Pipeline stage 1: Store inputs and compute result combinationally
always @(posedge clk) begin
    if (reset) begin
        a_reg <= 0;
        b_reg <= 0;
        result <= 0;
    end else begin
        a_reg <= a;
        b_reg <= b;
        result <= compute_sum(a_reg, b_reg);
    end
end

// Combinational function to compute floating point addition
function [31:0] compute_sum;
    input [31:0] num1, num2;
    reg [7:0] e1, e2, larger_exp, final_exp;
    reg [22:0] m1, m2, small_mantissa, large_mantissa, final_mantissa;
    reg [23:0] add_mantissa, norm_mantissa;
    reg s1, s2, final_sign;
    reg [4:0] shift_amt;
    /* verilator lint_off WIDTHEXPAND */
    reg signed [8:0] exp_adjust;
    /* verilator lint_on WIDTHEXPAND */
    begin
        // Extract fields
        s1 = num1[31]; s2 = num2[31];
        e1 = num1[30:23]; e2 = num2[30:23];
        m1 = num1[22:0]; m2 = num2[22:0];
        
        // Determine larger exponent and align mantissas
        if (e1 > e2) begin
            larger_exp = e1;
            large_mantissa = (e1 != 0) ? {1'b1, m1[22:1]} : m1;
            small_mantissa = (e2 != 0) ? {1'b1, m2[22:1]} : m2;
            small_mantissa = small_mantissa >> (e1 - e2);
        end else begin
            larger_exp = e2;
            large_mantissa = (e2 != 0) ? {1'b1, m2[22:1]} : m2;
            small_mantissa = (e1 != 0) ? {1'b1, m1[22:1]} : m1;
            small_mantissa = small_mantissa >> (e2 - e1);
        end
        
        // Add or subtract mantissas
        if (s1 == s2) begin
            add_mantissa = large_mantissa + small_mantissa;
        end else begin
            add_mantissa = large_mantissa - small_mantissa;
        end
        
        // Normalize result
        if (add_mantissa[23]) begin
            norm_mantissa = add_mantissa >> 1;
            exp_adjust = 1;
        end else if (add_mantissa[22]) begin
            norm_mantissa = add_mantissa;
            exp_adjust = 0;
        end else begin
            // Find first 1 bit and shift left
            shift_amt = 0;
            if (add_mantissa != 0) begin
                while (!add_mantissa[22 - shift_amt] && shift_amt < 23) 
                    shift_amt = shift_amt + 1;
                norm_mantissa = add_mantissa << shift_amt;
                /* verilator lint_off WIDTHEXPAND */
                exp_adjust = 9'(-9'({4'b0, shift_amt}));
                /* verilator lint_on WIDTHEXPAND */
            end else begin
                norm_mantissa = 0;
                /* verilator lint_off WIDTHEXPAND */
                exp_adjust = 9'(-9'({1'b0, larger_exp}));
                /* verilator lint_on WIDTHEXPAND */
            end
        end
        
        final_exp = larger_exp + exp_adjust[7:0];
        final_mantissa = norm_mantissa[22:0];
        
        // Determine sign
        if (s1 == s2) begin
            final_sign = s1;
        end else if (e1 > e2) begin
            final_sign = s1;
        end else if (e2 > e1) begin
            final_sign = s2;
        end else if (m1 > m2) begin
            final_sign = s1;
        end else begin
            final_sign = s2;
        end
        
        compute_sum = {final_sign, final_exp, final_mantissa};
    end
endfunction

endmodule

// 2-stage pipelined floating point adder
module FP_Adder_2Stage (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output reg [31:0] result
);

// Stage 1 registers
reg [7:0] e1_s1, e2_s1, larger_exp_s1;
reg [22:0] m1_s1, m2_s1, small_mantissa_s1, large_mantissa_s1;
reg s1_s1, s2_s1;
reg [4:0] shift_amt_s1;

// Stage 2 registers  
reg [23:0] add_mantissa_s2;
reg [7:0] larger_exp_s2;
reg s1_s2, s2_s2;
reg [7:0] e1_s2, e2_s2;
reg [22:0] m1_s2, m2_s2;

always @(posedge clk) begin
    if (reset) begin
        // Reset stage 1
        e1_s1 <= 0; e2_s1 <= 0; larger_exp_s1 <= 0;
        m1_s1 <= 0; m2_s1 <= 0; small_mantissa_s1 <= 0; large_mantissa_s1 <= 0;
        s1_s1 <= 0; s2_s1 <= 0; shift_amt_s1 <= 0;
        
        // Reset stage 2
        add_mantissa_s2 <= 0; larger_exp_s2 <= 0;
        s1_s2 <= 0; s2_s2 <= 0; e1_s2 <= 0; e2_s2 <= 0; m1_s2 <= 0; m2_s2 <= 0;
        result <= 0;
    end else begin
        // Stage 1: Input processing and mantissa alignment
        s1_s1 <= a[31]; s2_s1 <= b[31];
        e1_s1 <= a[30:23]; e2_s1 <= b[30:23];
        m1_s1 <= a[22:0]; m2_s1 <= b[22:0];
        
        if (a[30:23] > b[30:23]) begin
            larger_exp_s1 <= a[30:23];
            large_mantissa_s1 <= (a[30:23] != 0) ? {1'b1, a[22:1]} : a[22:0];
            small_mantissa_s1 <= ((b[30:23] != 0) ? {1'b1, b[22:1]} : b[22:0]) >> (a[30:23] - b[30:23]);
        end else begin
            larger_exp_s1 <= b[30:23];
            large_mantissa_s1 <= (b[30:23] != 0) ? {1'b1, b[22:1]} : b[22:0];
            small_mantissa_s1 <= ((a[30:23] != 0) ? {1'b1, a[22:1]} : a[22:0]) >> (b[30:23] - a[30:23]);
        end
        
        // Stage 2: Addition and result formation
        s1_s2 <= s1_s1; s2_s2 <= s2_s1;
        e1_s2 <= e1_s1; e2_s2 <= e2_s1; m1_s2 <= m1_s1; m2_s2 <= m2_s1;
        larger_exp_s2 <= larger_exp_s1;
        
        if (s1_s1 == s2_s1) begin
            add_mantissa_s2 <= large_mantissa_s1 + small_mantissa_s1;
        end else begin
            add_mantissa_s2 <= large_mantissa_s1 - small_mantissa_s1;
        end
        
        // Normalize and form result
        result <= normalize_result(add_mantissa_s2, larger_exp_s2, s1_s2, s2_s2, e1_s2, e2_s2, m1_s2, m2_s2);
    end
end

function [31:0] normalize_result;
    input [23:0] add_mant;
    input [7:0] larger_exp;
    input s1, s2;
    input [7:0] e1, e2;
    input [22:0] m1, m2;
    
    reg [7:0] final_exp;
    reg [22:0] final_mantissa;
    reg final_sign;
    reg [23:0] norm_mantissa;
    /* verilator lint_off WIDTHEXPAND */
    integer exp_adjust;
    /* verilator lint_on WIDTHEXPAND */
    reg [4:0] shift_amt;
    
    begin
        // Normalize
        if (add_mant[23]) begin
            norm_mantissa = add_mant >> 1;
            exp_adjust = 1;
        end else if (add_mant[22]) begin
            norm_mantissa = add_mant;
            exp_adjust = 0;
        end else begin
            shift_amt = 0;
            if (add_mant != 0) begin
                while (!add_mant[22 - shift_amt] && shift_amt < 23) 
                    shift_amt = shift_amt + 1;
                norm_mantissa = add_mant << shift_amt;
                /* verilator lint_off WIDTHEXPAND */
                exp_adjust = 9'(-9'({4'b0, shift_amt}));
                /* verilator lint_on WIDTHEXPAND */
            end else begin
                norm_mantissa = 0;
                /* verilator lint_off WIDTHEXPAND */
                exp_adjust = 9'(-9'({1'b0, larger_exp}));
                /* verilator lint_on WIDTHEXPAND */
            end
        end
        
        final_exp = 8'(larger_exp + exp_adjust);
        final_mantissa = norm_mantissa[22:0];
        
        // Determine sign
        if (s1 == s2) begin
            final_sign = s1;
        end else if (e1 > e2) begin
            final_sign = s1;
        end else if (e2 > e1) begin
            final_sign = s2;
        end else if (m1 > m2) begin
            final_sign = s1;
        end else begin
            final_sign = s2;
        end
        
        normalize_result = {final_sign, final_exp, final_mantissa};
    end
endfunction

endmodule

// 3-stage pipelined floating point adder - stub
module FP_Adder_3Stage (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output reg [31:0] result
);
    // Simplified stub - in real implementation would have 3 pipeline stages
    always @(posedge clk) begin
        if (reset) begin
            result <= 0;
        end else begin
            result <= a + b; // Simplified
        end
    end
endmodule

// 4-stage pipelined floating point adder - stub
module FP_Adder_4Stage (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output reg [31:0] result
);
    // Simplified stub - in real implementation would have 4 pipeline stages
    always @(posedge clk) begin
        if (reset) begin
            result <= 0;
        end else begin
            result <= a + b; // Simplified
        end
    end
endmodule

// Latency-Insensitive wrapper for floating point multiplier with ready-valid interface
module FP_Mult_LI_Wrapper #(
    parameter STAGES = 1  // 1, 2, 3, or 4 stages
) (
    input clk,
    input reset,
    
    // Input interface
    input [31:0] a,
    input [31:0] b,
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

// Internal FIFO to buffer inputs and track pipeline
localparam FIFO_SIZE = (STAGES < 2) ? 4 : (2*STAGES);
localparam ADDR_WIDTH = (FIFO_SIZE <= 4) ? 2 : 3;  // 2 bits for size ≤4, 3 bits for size ≤8
reg [63:0] input_fifo [0:FIFO_SIZE-1];   // Store {a, b}
reg [STAGES:0] valid_shift;              // Track valid data through pipeline
reg [ADDR_WIDTH-1:0] fifo_head, fifo_tail;
reg [ADDR_WIDTH:0] fifo_count;           // One extra bit to count up to FIFO_SIZE

// Core multiplier signals
reg [31:0] core_a, core_b;
wire [31:0] core_result;
wire core_exception, core_overflow, core_underflow;

// Output registers for flags
reg out_exception, out_overflow, out_underflow;

// Output valid persistence
reg output_valid_reg;
wire transaction_complete = output_valid_reg && ready_in;

// Stall logic with proper backpressure
wire output_stalled = output_valid_reg && !ready_in;
wire pipeline_advance = !output_stalled;
wire input_ready = (fifo_count < FIFO_SIZE) && !output_stalled && !reset;

assign ready_out = input_ready;
assign valid_out = output_valid_reg;
assign result = core_result;
assign exception = out_exception;
assign overflow = out_overflow;
assign underflow = out_underflow;

integer i;

// Instantiate the core multiplier
FP_Mult_Wrapper #(.STAGES(STAGES)) core_multiplier (
    .clk(clk),
    .reset(reset),
    .a(core_a),
    .b(core_b),
    .result(core_result),
    .exception(core_exception),
    .overflow(core_overflow),
    .underflow(core_underflow)
);

always @(posedge clk) begin
    if (reset) begin
        fifo_head <= 0;
        fifo_tail <= 0;
        fifo_count <= 0;
        valid_shift <= 0;
        output_valid_reg <= 0;
        core_a <= 0;
        core_b <= 0;
        out_exception <= 0;
        out_overflow <= 0;
        out_underflow <= 0;
        
        // Clear FIFO
        for (i = 0; i < FIFO_SIZE; i++) begin
            input_fifo[i] <= 64'h0;
        end
    end else begin
        // Input side: enqueue when valid input and we're ready
        if (valid_in && ready_out) begin
            input_fifo[fifo_tail] <= {a, b};
            /* verilator lint_off WIDTHEXPAND */
            fifo_tail <= ADDR_WIDTH'((fifo_tail + 1'b1) % FIFO_SIZE);
            /* verilator lint_on WIDTHEXPAND */
            fifo_count <= fifo_count + 1'b1;
        end
        
        // Output valid management - latch valid until transaction completes
        if (valid_shift[STAGES] && !output_valid_reg) begin
            output_valid_reg <= 1'b1;  // Latch valid when data reaches output
            // Capture exception flags when output becomes valid
            out_exception <= core_exception;
            out_overflow <= core_overflow;
            out_underflow <= core_underflow;
        end else if (transaction_complete) begin
            output_valid_reg <= 1'b0;  // Clear after handshake
        end
        
        // Pipeline advance logic - only when not stalled
        if (pipeline_advance) begin
            // Shift valid bits through pipeline
            for (i = STAGES; i > 0; i--) begin
                valid_shift[i] <= valid_shift[i-1];
            end
            
            // Feed new data to multiplier if FIFO has data
            if (fifo_count > 0) begin
                {core_a, core_b} <= input_fifo[fifo_head];
                /* verilator lint_off WIDTHEXPAND */
                fifo_head <= ADDR_WIDTH'((fifo_head + 1'b1) % FIFO_SIZE);
                /* verilator lint_on WIDTHEXPAND */
                fifo_count <= fifo_count - 1'b1;
                valid_shift[0] <= 1'b1;
            end else begin
                valid_shift[0] <= 1'b0;
            end
        end
    end
end

endmodule

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

// 1-stage pipelined floating point multiplier
module FP_Mult_1Stage (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output reg [31:0] result,
    output reg exception,
    output reg overflow,
    output reg underflow
);

reg [31:0] a_reg, b_reg;

always @(posedge clk) begin
    if (reset) begin
        a_reg <= 0;
        b_reg <= 0;
        result <= 0;
        exception <= 0;
        overflow <= 0;
        underflow <= 0;
    end else begin
        a_reg <= a;
        b_reg <= b;
        
        // Compute multiplication result
        {result, exception, overflow, underflow} <= compute_mult(a_reg, b_reg);
    end
end

function [34:0] compute_mult;  // {result[31:0], exception, overflow, underflow}
    input [31:0] num_a, num_b;
    
    reg sign, round, normalised, zero;
    reg [8:0] exponent, sum_exponent, exp_norm;
    reg [22:0] product_mantissa;
    reg [23:0] op_a, op_b;
    reg [47:0] product, product_normalised;
    reg exc, ovf, unf;
    reg [31:0] res;
    
    begin
        sign = num_a[31] ^ num_b[31];
        exc = (&num_a[30:23]) | (&num_b[30:23]);
        
        // Extract operands with hidden bit
        op_a = (|num_a[30:23]) ? {1'b1, num_a[22:0]} : {1'b0, num_a[22:0]};
        op_b = (|num_b[30:23]) ? {1'b1, num_b[22:0]} : {1'b0, num_b[22:0]};
        
        product = op_a * op_b;
        normalised = product[47] ? 1'b1 : 1'b0;
        product_normalised = normalised ? product : product << 1;
        
        round = |product_normalised[22:0];
        product_mantissa = product_normalised[46:24] + (product_normalised[23] & round);
        
        sum_exponent = num_a[30:23] + num_b[30:23];
        exp_norm = sum_exponent - 8'd127;
        exponent = exp_norm + normalised;
        
        ovf = (exponent[8] & !exc & !exponent[7]);
        unf = (exponent[8] & !exc & exponent[7]);
        
        res = exc ? {sign, 31'd0} :
              ovf ? {sign, 8'hFF, 23'd0} :
              unf ? {sign, 31'd0} :
              {sign, exponent[7:0], product_mantissa};
              
        compute_mult = {res, exc, ovf, unf};
    end
endfunction

endmodule

// 2-stage pipelined floating point multiplier - stub
module FP_Mult_2Stage (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output reg [31:0] result,
    output reg exception,
    output reg overflow,
    output reg underflow
);
    // Simplified stub - in real implementation would have 2 pipeline stages
    always @(posedge clk) begin
        if (reset) begin
            result <= 0;
            exception <= 0;
            overflow <= 0;
            underflow <= 0;
        end else begin
            result <= a; // Simplified
            exception <= 0;
            overflow <= 0;
            underflow <= 0;
        end
    end
endmodule

// 3-stage pipelined floating point multiplier
module FP_Mult_3Stage (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output reg [31:0] result,
    output reg exception,
    output reg overflow,
    output reg underflow
);

// Stage 1 registers
reg sign_s1, exc_s1;
reg [23:0] op_a_s1, op_b_s1;
reg [8:0] sum_exp_s1;

// Stage 2 registers
reg sign_s2, exc_s2;
reg [47:0] product_s2;
reg [8:0] sum_exp_s2;

// Stage 3 registers
reg sign_s3, exc_s3;
reg [47:0] product_norm_s3;
reg [8:0] exponent_s3;

always @(posedge clk) begin
    if (reset) begin
        // Reset all stages
        sign_s1 <= 0; exc_s1 <= 0; op_a_s1 <= 0; op_b_s1 <= 0; sum_exp_s1 <= 0;
        sign_s2 <= 0; exc_s2 <= 0; product_s2 <= 0; sum_exp_s2 <= 0;
        sign_s3 <= 0; exc_s3 <= 0; product_norm_s3 <= 0; exponent_s3 <= 0;
        result <= 0; exception <= 0; overflow <= 0; underflow <= 0;
    end else begin
        // Stage 1: Input processing
        sign_s1 <= a[31] ^ b[31];
        exc_s1 <= (&a[30:23]) | (&b[30:23]);
        op_a_s1 <= (|a[30:23]) ? {1'b1, a[22:0]} : {1'b0, a[22:0]};
        op_b_s1 <= (|b[30:23]) ? {1'b1, b[22:0]} : {1'b0, b[22:0]};
        sum_exp_s1 <= a[30:23] + b[30:23];
        
        // Stage 2: Multiplication
        sign_s2 <= sign_s1;
        exc_s2 <= exc_s1;
        product_s2 <= op_a_s1 * op_b_s1;
        sum_exp_s2 <= sum_exp_s1;
        
        // Stage 3: Normalization
        sign_s3 <= sign_s2;
        exc_s3 <= exc_s2;
        
        if (product_s2[47]) begin
            product_norm_s3 <= product_s2;
            exponent_s3 <= sum_exp_s2 - 8'd127 + 1;
        end else begin
            product_norm_s3 <= product_s2 << 1;
            exponent_s3 <= sum_exp_s2 - 8'd127;
        end
        
        // Form final result
        {result, exception, overflow, underflow} <= form_final_result(sign_s3, exc_s3, product_norm_s3, exponent_s3);
    end
end

function [34:0] form_final_result;  // {result[31:0], exception, overflow, underflow}
    input sign, exc;
    input [47:0] product_norm;
    input [8:0] exponent;
    
    reg round;
    reg [22:0] product_mantissa;
    reg ovf, unf;
    reg [31:0] res;
    
    begin
        round = |product_norm[22:0];
        product_mantissa = product_norm[46:24] + {22'b0, (product_norm[23] & round)};
        
        ovf = (exponent[8] & !exc & !exponent[7]);
        unf = (exponent[8] & !exc & exponent[7]);
        
        res = exc ? {sign, 31'd0} :
              ovf ? {sign, 8'hFF, 23'd0} :
              unf ? {sign, 31'd0} :
              {sign, exponent[7:0], product_mantissa};
              
        form_final_result = {res, exc, ovf, unf};
    end
endfunction

endmodule

// 4-stage pipelined floating point multiplier - stub
module FP_Mult_4Stage (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output reg [31:0] result,
    output reg exception,
    output reg overflow,
    output reg underflow
);
    // Simplified stub - in real implementation would have 4 pipeline stages
    always @(posedge clk) begin
        if (reset) begin
            result <= 0;
            exception <= 0;
            overflow <= 0;
            underflow <= 0;
        end else begin
            result <= a; // Simplified
            exception <= 0;
            overflow <= 0;
            underflow <= 0;
        end
    end
endmodule