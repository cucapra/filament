`timescale 1ns / 1ps

module test_dynamic_alu_fixed;

// Test parameters
parameter CLK_PERIOD = 10;
parameter NUM_TESTS = 100;

// Randomized test configurations - different from static test
localparam CONFIG_COUNT = 4;
localparam ADDER_STAGES_0 = 3, ADDER_STAGES_1 = 1, ADDER_STAGES_2 = 4, ADDER_STAGES_3 = 2;  // Random order
localparam MULT_STAGES_0 = 1, MULT_STAGES_1 = 4, MULT_STAGES_2 = 2, MULT_STAGES_3 = 3;      // Random order

// Clock and reset
reg clk;
reg reset;

// Test inputs (broadcast to all ALUs)
reg [31:0] operand_a, operand_b;
reg [1:0] operation;
reg valid_in;

// Individual ready_in signals for each ALU
reg ready_in[0:3];

// ALU outputs
wire ready_out[0:3];
wire [31:0] result[0:3];
wire exception[0:3], overflow[0:3], underflow[0:3], valid_out[0:3];

// Test vectors and results
reg [31:0] test_a[0:NUM_TESTS-1];
reg [31:0] test_b[0:NUM_TESTS-1];
reg [1:0] test_op[0:NUM_TESTS-1];

// Result collection
reg [31:0] collected_results[0:3][0:NUM_TESTS-1];
reg collected_exceptions[0:3][0:NUM_TESTS-1];
reg [15:0] results_collected[0:3];  // Count of results collected per ALU

// Clock generation
initial begin
    clk = 0;
    forever #(CLK_PERIOD/2) clk = ~clk;
end

// Instantiate ALUs with different pipeline configurations
Dynamic_ALU #(.ADDER_STAGES(ADDER_STAGES_0), .MULT_STAGES(MULT_STAGES_0)) alu0 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .valid_in(valid_in), .ready_out(ready_out[0]), .result(result[0]), .exception(exception[0]), 
    .overflow(overflow[0]), .underflow(underflow[0]), .valid_out(valid_out[0]), .ready_in(ready_in[0])
);

Dynamic_ALU #(.ADDER_STAGES(ADDER_STAGES_1), .MULT_STAGES(MULT_STAGES_1)) alu1 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .valid_in(valid_in), .ready_out(ready_out[1]), .result(result[1]), .exception(exception[1]), 
    .overflow(overflow[1]), .underflow(underflow[1]), .valid_out(valid_out[1]), .ready_in(ready_in[1])
);

Dynamic_ALU #(.ADDER_STAGES(ADDER_STAGES_2), .MULT_STAGES(MULT_STAGES_2)) alu2 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .valid_in(valid_in), .ready_out(ready_out[2]), .result(result[2]), .exception(exception[2]), 
    .overflow(overflow[2]), .underflow(underflow[2]), .valid_out(valid_out[2]), .ready_in(ready_in[2])
);

Dynamic_ALU #(.ADDER_STAGES(ADDER_STAGES_3), .MULT_STAGES(MULT_STAGES_3)) alu3 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .valid_in(valid_in), .ready_out(ready_out[3]), .result(result[3]), .exception(exception[3]), 
    .overflow(overflow[3]), .underflow(underflow[3]), .valid_out(valid_out[3]), .ready_in(ready_in[3])
);

// Task to send a single transaction to all ALUs
task send_transaction(input [31:0] a, input [31:0] b, input [1:0] op);
    reg all_ready;
    integer timeout;
begin
    // Set up transaction
    operand_a = a;
    operand_b = b;
    operation = op;
    
    // Wait for all ALUs to be ready (handles different pipeline states)
    timeout = 0;
    all_ready = 0;
    while (!all_ready && timeout < 100) begin
        all_ready = ready_out[0] & ready_out[1] & ready_out[2] & ready_out[3];
        if (!all_ready) begin
            @(posedge clk);
            timeout++;
        end
    end
    
    if (timeout >= 100) begin
        $display("ERROR: Timeout waiting for all ALUs to be ready");
        $display("ready_out = %b%b%b%b", ready_out[3], ready_out[2], ready_out[1], ready_out[0]);
        $finish;
    end
    
    // Send transaction to all ALUs simultaneously
    valid_in = 1'b1;
    @(posedge clk);
    valid_in = 1'b0;
    
    $display("Sent transaction: a=%h, b=%h, op=%d", a, b, op);
end
endtask

// Task to collect results from all ALUs (with proper backpressure handling)
task collect_results();
    integer i;
    reg [3:0] pending_results;
    integer timeout;
    integer collection_round;
begin
    collection_round = 0;
    
    // Initially, no one is ready to receive
    for (i = 0; i < 4; i++) begin
        ready_in[i] = 1'b0;
    end
    
    // Wait for all ALUs to have valid outputs
    timeout = 0;
    pending_results = 4'b0000;
    while (pending_results != 4'b1111 && timeout < 200) begin
        pending_results = {valid_out[3], valid_out[2], valid_out[1], valid_out[0]};
        if (pending_results != 4'b1111) begin
            @(posedge clk);
            timeout++;
        end
    end
    
    if (timeout >= 200) begin
        $display("ERROR: Timeout waiting for all valid outputs");
        $display("valid_out = %b%b%b%b after %d cycles", valid_out[3], valid_out[2], valid_out[1], valid_out[0], timeout);
        $display("Results collected so far: %d %d %d %d", results_collected[0], results_collected[1], results_collected[2], results_collected[3]);
        $finish;
    end
    
    $display("All ALUs have valid outputs, collecting results...");
    
    // Collect results from each ALU one at a time
    for (i = 0; i < 4; i++) begin
        // Assert ready for this ALU
        ready_in[i] = 1'b1;
        @(posedge clk);
        
        // Capture result
        collected_results[i][results_collected[i]] = result[i];
        collected_exceptions[i][results_collected[i]] = exception[i];
        results_collected[i] = results_collected[i] + 1;
        
        $display("  ALU%d: result=%h, exception=%b", i, result[i], exception[i]);
        
        // De-assert ready and wait for valid to drop
        ready_in[i] = 1'b0;
        timeout = 0;
        while (valid_out[i] && timeout < 20) begin
            @(posedge clk);
            timeout++;
        end
        
        if (timeout >= 20) begin
            $display("WARNING: ALU%d valid did not drop after ready de-asserted", i);
        end
    end
    
    // Small delay before next transaction
    repeat(3) @(posedge clk);
end
endtask

// Main test sequence
integer test_idx, config_idx, error_count;

initial begin
    $display("=== Testing Dynamic ALU with Randomized Pipeline Configurations ===");
    $display("Configuration matrix:");
    $display("  ALU0: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_STAGES_0, MULT_STAGES_0);
    $display("  ALU1: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_STAGES_1, MULT_STAGES_1);
    $display("  ALU2: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_STAGES_2, MULT_STAGES_2);
    $display("  ALU3: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_STAGES_3, MULT_STAGES_3);
    
    // Generate test vectors
    for (test_idx = 0; test_idx < NUM_TESTS; test_idx++) begin
        test_a[test_idx] = $random;
        test_b[test_idx] = $random;
        test_op[test_idx] = test_idx % 3;  // Cycle through ADD, SUB, MUL
        
        // Include some known values for the first few tests
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
    
    // Initialize
    error_count = 0;
    for (config_idx = 0; config_idx < 4; config_idx++) begin
        results_collected[config_idx] = 0;
        ready_in[config_idx] = 1'b0;
    end
    
    reset = 1;
    operand_a = 0;
    operand_b = 0;
    operation = 0;
    valid_in = 0;
    
    // Reset sequence
    repeat(10) @(posedge clk);
    reset = 0;
    repeat(5) @(posedge clk);
    
    $display("Starting test sequence with %d transactions...", NUM_TESTS);
    
    // Run test sequence
    for (test_idx = 0; test_idx < NUM_TESTS; test_idx++) begin
        // Send transaction to all ALUs
        send_transaction(test_a[test_idx], test_b[test_idx], test_op[test_idx]);
        
        // Collect results from all ALUs
        collect_results();
        
        // Progress indicator
        if (test_idx % 20 == 0) begin
            $display("Completed %d/%d tests...", test_idx+1, NUM_TESTS);
        end
    end
    
    $display("All transactions completed. Verifying results...");
    
    // Verify all ALUs produced the same number of results
    for (config_idx = 0; config_idx < 4; config_idx++) begin
        if (results_collected[config_idx] != NUM_TESTS) begin
            $display("ERROR: ALU%d collected %d results, expected %d", 
                    config_idx, results_collected[config_idx], NUM_TESTS);
            error_count++;
        end
    end
    
    // Compare results between all ALU configurations
    for (test_idx = 0; test_idx < NUM_TESTS; test_idx++) begin
        for (config_idx = 1; config_idx < 4; config_idx++) begin
            if (collected_results[0][test_idx] !== collected_results[config_idx][test_idx] ||
                collected_exceptions[0][test_idx] !== collected_exceptions[config_idx][test_idx]) begin
                $display("MISMATCH at test %d between ALU0 and ALU%d:", test_idx, config_idx);
                $display("  Input: a=%h, b=%h, op=%d", test_a[test_idx], test_b[test_idx], test_op[test_idx]);
                $display("  ALU0: result=%h, exception=%b", 
                        collected_results[0][test_idx], collected_exceptions[0][test_idx]);
                $display("  ALU%d: result=%h, exception=%b", config_idx,
                        collected_results[config_idx][test_idx], collected_exceptions[config_idx][test_idx]);
                error_count++;
                
                // Limit error output
                if (error_count >= 10) begin
                    $display("Too many errors, stopping comparison...");
                end
            end
        end
    end
    
    // Report final results
    $display("\n=== RANDOMIZED DYNAMIC ALU TEST RESULTS ===");
    $display("Total tests: %d", NUM_TESTS);
    $display("Pipeline configurations tested:");
    $display("  ALU0: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_STAGES_0, MULT_STAGES_0);
    $display("  ALU1: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_STAGES_1, MULT_STAGES_1);
    $display("  ALU2: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_STAGES_2, MULT_STAGES_2);
    $display("  ALU3: ADDER_STAGES=%d, MULT_STAGES=%d", ADDER_STAGES_3, MULT_STAGES_3);
    $display("Results collected per ALU: %d %d %d %d", 
            results_collected[0], results_collected[1], results_collected[2], results_collected[3]);
    $display("Errors found: %d", error_count);
    
    if (error_count == 0) begin
        $display("✅ SUCCESS: All randomized dynamic ALU configurations produce identical results!");
        $display("✅ Latency-insensitive interface working correctly with proper backpressure");
    end else begin
        $display("❌ FAILURE: Found %d mismatches between randomized configurations", error_count);
    end
    
    $finish;
end

endmodule