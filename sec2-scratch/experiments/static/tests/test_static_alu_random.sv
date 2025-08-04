`timescale 1ns / 1ps

module test_static_alu_random;

// Test parameters
parameter CLK_PERIOD = 10;
parameter NUM_TESTS = 100;

// Random pipeline configurations (determined at start of simulation)
reg [2:0] adder_stages[0:3];
reg [2:0] mult_stages[0:3];
reg [3:0] max_pipeline_depth;

// Clock and reset
reg clk;
reg reset;

// Test inputs
reg [31:0] operand_a, operand_b;
reg [1:0] operation;

// ALU outputs for different configurations
wire [31:0] result[0:3];
wire exception[0:3], overflow[0:3], underflow[0:3], valid[0:3];

// Test vectors and results
reg [31:0] test_a[0:NUM_TESTS-1];
reg [31:0] test_b[0:NUM_TESTS-1];
reg [1:0] test_op[0:NUM_TESTS-1];

// Clock generation
initial begin
    clk = 0;
    forever #(CLK_PERIOD/2) clk = ~clk;
end

// Generate ALU instances with different random configurations
// We'll use 4 different combinations of stages (1-4 each)

// ALU 0: Random configuration 1
Static_ALU #(.ADDER_STAGES(1), .MULT_STAGES(2)) alu0 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .result(result[0]), .exception(exception[0]), .overflow(overflow[0]), .underflow(underflow[0]), .valid(valid[0])
);

// ALU 1: Random configuration 2  
Static_ALU #(.ADDER_STAGES(3), .MULT_STAGES(1)) alu1 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .result(result[1]), .exception(exception[1]), .overflow(overflow[1]), .underflow(underflow[1]), .valid(valid[1])
);

// ALU 2: Random configuration 3
Static_ALU #(.ADDER_STAGES(2), .MULT_STAGES(4)) alu2 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .result(result[2]), .exception(exception[2]), .overflow(overflow[2]), .underflow(underflow[2]), .valid(valid[2])
);

// ALU 3: Random configuration 4
Static_ALU #(.ADDER_STAGES(4), .MULT_STAGES(3)) alu3 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .result(result[3]), .exception(exception[3]), .overflow(overflow[3]), .underflow(underflow[3]), .valid(valid[3])
);

// Task to wait for specific ALU based on operation and pipeline depth
task wait_for_alu_result(input integer alu_id, input [1:0] op);
    integer wait_cycles;
    integer alu_adder_stages, alu_mult_stages;
    begin
        // Get the pipeline depths for this ALU
        case (alu_id)
            0: begin alu_adder_stages = 1; alu_mult_stages = 2; end
            1: begin alu_adder_stages = 3; alu_mult_stages = 1; end
            2: begin alu_adder_stages = 2; alu_mult_stages = 4; end
            3: begin alu_adder_stages = 4; alu_mult_stages = 3; end
        endcase
        
        // Calculate wait cycles based on operation type
        if (op == 2'b00 || op == 2'b01) begin  // ADD or SUB
            wait_cycles = alu_adder_stages + 2;  // Extra cycles for safety
        end else begin  // MUL
            wait_cycles = alu_mult_stages + 2; 
        end
        
        repeat(wait_cycles) @(posedge clk);
    end
endtask

// Main test sequence
integer test_idx, config_idx, error_count;
integer wait_cycles;

initial begin
    $display("=== Testing Static ALU with Random Pipeline Configurations ===");
    
    // Display the random configurations being tested
    $display("Configuration matrix:");
    $display("  ALU0: ADDER_STAGES=1, MULT_STAGES=2");
    $display("  ALU1: ADDER_STAGES=3, MULT_STAGES=1");
    $display("  ALU2: ADDER_STAGES=2, MULT_STAGES=4");
    $display("  ALU3: ADDER_STAGES=4, MULT_STAGES=3");
    
    // Store configurations for later use
    adder_stages[0] = 1; mult_stages[0] = 2;
    adder_stages[1] = 3; mult_stages[1] = 1;
    adder_stages[2] = 2; mult_stages[2] = 4;
    adder_stages[3] = 4; mult_stages[3] = 3;
    
    // Calculate maximum pipeline depth
    max_pipeline_depth = 0;
    for (config_idx = 0; config_idx < 4; config_idx++) begin
        if (adder_stages[config_idx] > max_pipeline_depth)
            max_pipeline_depth = adder_stages[config_idx];
        if (mult_stages[config_idx] > max_pipeline_depth)
            max_pipeline_depth = mult_stages[config_idx];
    end
    max_pipeline_depth = max_pipeline_depth + 3; // Safety margin
    
    $display("Maximum pipeline depth: %d cycles", max_pipeline_depth);
    
    // Generate test vectors
    for (test_idx = 0; test_idx < NUM_TESTS; test_idx++) begin
        test_a[test_idx] = $random;
        test_b[test_idx] = $random;
        test_op[test_idx] = test_idx % 3;  // Cycle through ADD, SUB, MUL
        
        // Include some known values
        if (test_idx < 9) begin
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
        
        // Wait for all pipelines to complete using maximum depth
        // This ensures all ALUs have produced their results
        repeat(max_pipeline_depth) @(posedge clk);
        
        // Check that all configurations produce the same result
        for (config_idx = 1; config_idx < 4; config_idx++) begin
            if (result[0] !== result[config_idx] || 
                exception[0] !== exception[config_idx] ||
                overflow[0] !== overflow[config_idx] ||
                underflow[0] !== underflow[config_idx]) begin
                
                $display("MISMATCH at test %d: a=%h, b=%h, op=%d", 
                        test_idx, test_a[test_idx], test_b[test_idx], test_op[test_idx]);
                $display("  ALU0 (A:%d,M:%d): result=%h, exc=%b, ovf=%b, unf=%b", 
                        adder_stages[0], mult_stages[0],
                        result[0], exception[0], overflow[0], underflow[0]);
                $display("  ALU%d (A:%d,M:%d): result=%h, exc=%b, ovf=%b, unf=%b", 
                        config_idx, adder_stages[config_idx], mult_stages[config_idx],
                        result[config_idx], exception[config_idx], overflow[config_idx], underflow[config_idx]);
                error_count++;
            end
        end
        
        // Progress indicator
        if (test_idx % 20 == 0) begin
            $display("Completed %d tests...", test_idx);
        end
    end
    
    // Report results
    $display("\n=== RANDOM STATIC ALU TEST RESULTS ===");
    $display("Total tests: %d", NUM_TESTS);
    $display("Pipeline configurations tested:");
    for (config_idx = 0; config_idx < 4; config_idx++) begin
        $display("  ALU%d: ADDER_STAGES=%d, MULT_STAGES=%d", 
                config_idx, adder_stages[config_idx], mult_stages[config_idx]);
    end
    $display("Errors: %d", error_count);
    
    if (error_count == 0) begin
        $display("SUCCESS: All random pipeline configurations produce identical results!");
    end else begin
        $display("FAILURE: Found mismatches between random pipeline configurations");
    end
    
    $finish;
end

endmodule