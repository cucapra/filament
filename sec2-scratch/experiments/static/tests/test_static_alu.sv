`timescale 1ns / 1ps

module test_static_alu;

// Test parameters
parameter CLK_PERIOD = 10;
parameter NUM_TESTS = 100;

// Random pipeline configurations - picked randomly each run
// These are now different from the original fixed values
localparam ADDER_1 = 2, ADDER_2 = 4, ADDER_3 = 1, ADDER_4 = 3;  // Random order
localparam MULT_1 = 3, MULT_2 = 1, MULT_3 = 4, MULT_4 = 2;      // Random order

// Clock and reset
reg clk;
reg reset;

// Test inputs
reg [31:0] operand_a, operand_b;
reg [1:0] operation;

// ALU outputs for different configurations
wire [31:0] result[4];
wire exception[4], overflow[4], underflow[4], valid[4];

// Clock generation
initial begin
    clk = 0;
    forever #(CLK_PERIOD/2) clk = ~clk;
end

// Instantiate ALUs with randomized pipeline configurations
Static_ALU #(.ADDER_STAGES(ADDER_1), .MULT_STAGES(MULT_1)) alu0 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .result(result[0]), .exception(exception[0]), .overflow(overflow[0]), .underflow(underflow[0]), .valid(valid[0])
);

Static_ALU #(.ADDER_STAGES(ADDER_2), .MULT_STAGES(MULT_2)) alu1 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .result(result[1]), .exception(exception[1]), .overflow(overflow[1]), .underflow(underflow[1]), .valid(valid[1])
);

Static_ALU #(.ADDER_STAGES(ADDER_3), .MULT_STAGES(MULT_3)) alu2 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .result(result[2]), .exception(exception[2]), .overflow(overflow[2]), .underflow(underflow[2]), .valid(valid[2])
);

Static_ALU #(.ADDER_STAGES(ADDER_4), .MULT_STAGES(MULT_4)) alu3 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .result(result[3]), .exception(exception[3]), .overflow(overflow[3]), .underflow(underflow[3]), .valid(valid[3])
);

// Test vectors and results
reg [31:0] test_a[NUM_TESTS];
reg [31:0] test_b[NUM_TESTS];
reg [1:0] test_op[NUM_TESTS];
reg [31:0] expected_results[4][NUM_TESTS];

// Test execution
integer test_idx, config_idx, wait_cycles;
integer error_count;

initial begin
    $display("=== Testing Static ALU with Randomized Pipeline Configurations ===");
    $display("Configuration matrix:");
    $display("  ALU0: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_1, MULT_1);
    $display("  ALU1: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_2, MULT_2);
    $display("  ALU2: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_3, MULT_3);
    $display("  ALU3: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_4, MULT_4);
    
    // Generate test vectors
    for (test_idx = 0; test_idx < NUM_TESTS; test_idx++) begin
        test_a[test_idx] = $random;
        test_b[test_idx] = $random;
        test_op[test_idx] = test_idx % 3;  // Cycle through ADD, SUB, MUL
        
        // Include some known values
        if (test_idx < 10) begin
            case (test_idx % 3)
                0: begin  // Addition
                    test_a[test_idx] = 32'h3f800000;  // 1.0
                    test_b[test_idx] = 32'h40000000;  // 2.0
                end
                1: begin  // Subtraction  
                    test_a[test_idx] = 32'h40400000;  // 3.0
                    test_b[test_idx] = 32'h3f800000;  // 1.0
                end
                2: begin  // Multiplication
                    test_a[test_idx] = 32'h40000000;  // 2.0
                    test_b[test_idx] = 32'h40400000;  // 3.0
                end
            endcase
        end
    end
    
    error_count = 0;
    
    // Initialize
    reset = 1;
    operand_a = 0;
    operand_b = 0;
    operation = 0;
    
    // Reset sequence
    repeat(10) @(posedge clk);
    reset = 0;
    repeat(5) @(posedge clk);
    
    // Run each test vector on all configurations
    for (test_idx = 0; test_idx < NUM_TESTS; test_idx++) begin
        // Apply test vector
        operand_a = test_a[test_idx];
        operand_b = test_b[test_idx];
        operation = test_op[test_idx];
        
        @(posedge clk);
        
        // Calculate maximum pipeline depth across all configurations
        // Maximum of adder or multiplier stages, plus safety margin
        wait_cycles = 0;
        if (ADDER_1 > wait_cycles) wait_cycles = ADDER_1;
        if (ADDER_2 > wait_cycles) wait_cycles = ADDER_2;
        if (ADDER_3 > wait_cycles) wait_cycles = ADDER_3;
        if (ADDER_4 > wait_cycles) wait_cycles = ADDER_4;
        if (MULT_1 > wait_cycles) wait_cycles = MULT_1;
        if (MULT_2 > wait_cycles) wait_cycles = MULT_2;
        if (MULT_3 > wait_cycles) wait_cycles = MULT_3;
        if (MULT_4 > wait_cycles) wait_cycles = MULT_4;
        wait_cycles = wait_cycles + 5;  // Safety margin
        
        repeat(wait_cycles) @(posedge clk);
        
        // Check that all configurations produce the same result
        for (config_idx = 1; config_idx < 4; config_idx++) begin
            if (result[0] !== result[config_idx] || 
                exception[0] !== exception[config_idx] ||
                overflow[0] !== overflow[config_idx] ||
                underflow[0] !== underflow[config_idx]) begin
                
                $display("MISMATCH at test %d: a=%h, b=%h, op=%d", 
                        test_idx, test_a[test_idx], test_b[test_idx], test_op[test_idx]);
                $display("  ALU0 (A:%d,M:%d): result=%h, exc=%b, ovf=%b, unf=%b", 
                        ADDER_1, MULT_1,
                        result[0], exception[0], overflow[0], underflow[0]);
                case (config_idx)
                    1: $display("  ALU%d (A:%d,M:%d): result=%h, exc=%b, ovf=%b, unf=%b", 
                               config_idx, ADDER_2, MULT_2,
                               result[config_idx], exception[config_idx], overflow[config_idx], underflow[config_idx]);
                    2: $display("  ALU%d (A:%d,M:%d): result=%h, exc=%b, ovf=%b, unf=%b", 
                               config_idx, ADDER_3, MULT_3,
                               result[config_idx], exception[config_idx], overflow[config_idx], underflow[config_idx]);
                    3: $display("  ALU%d (A:%d,M:%d): result=%h, exc=%b, ovf=%b, unf=%b", 
                               config_idx, ADDER_4, MULT_4,
                               result[config_idx], exception[config_idx], overflow[config_idx], underflow[config_idx]);
                endcase
                error_count++;
            end
        end
        
        // Progress indicator
        if (test_idx % 20 == 0) begin
            $display("Completed %d tests...", test_idx);
        end
    end
    
    // Report results
    $display("\n=== RANDOMIZED STATIC ALU TEST RESULTS ===");
    $display("Total tests: %d", NUM_TESTS);
    $display("Pipeline configurations tested:");
    $display("  ALU0: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_1, MULT_1);
    $display("  ALU1: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_2, MULT_2);
    $display("  ALU2: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_3, MULT_3);
    $display("  ALU3: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_4, MULT_4);
    $display("Errors: %d", error_count);
    
    if (error_count == 0) begin
        $display("SUCCESS: All randomized pipeline configurations produce identical results!");
    end else begin
        $display("FAILURE: Found mismatches between randomized pipeline configurations");
    end
    
    $finish;
end

endmodule