`timescale 1ns / 1ps

module test_dynamic_alu;

// Test parameters
parameter CLK_PERIOD = 10;
parameter NUM_TESTS = 50;

// Test configurations - using localparam with individual constants
localparam ADDER_1 = 1, ADDER_2 = 2, ADDER_3 = 3, ADDER_4 = 4;
localparam MULT_1 = 2, MULT_2 = 3, MULT_3 = 1, MULT_4 = 4;

// Clock and reset
reg clk;
reg reset;

// Test inputs
reg [31:0] operand_a, operand_b;
reg [1:0] operation;
reg valid_in;
reg ready_in;

// ALU outputs for different configurations
wire ready_out[4];
wire [31:0] result[4];
wire exception[4], overflow[4], underflow[4], valid_out[4];

// Test control
reg [31:0] test_a[NUM_TESTS];
reg [31:0] test_b[NUM_TESTS];
reg [1:0] test_op[NUM_TESTS];

// Clock generation
initial begin
    clk = 0;
    forever #(CLK_PERIOD/2) clk = ~clk;
end

// Instantiate ALUs with different pipeline configurations
Dynamic_ALU #(.ADDER_STAGES(ADDER_1), .MULT_STAGES(MULT_1)) alu0 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .valid_in(valid_in), .ready_out(ready_out[0]), .result(result[0]), .exception(exception[0]), 
    .overflow(overflow[0]), .underflow(underflow[0]), .valid_out(valid_out[0]), .ready_in(ready_in)
);

Dynamic_ALU #(.ADDER_STAGES(ADDER_2), .MULT_STAGES(MULT_2)) alu1 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .valid_in(valid_in), .ready_out(ready_out[1]), .result(result[1]), .exception(exception[1]), 
    .overflow(overflow[1]), .underflow(underflow[1]), .valid_out(valid_out[1]), .ready_in(ready_in)
);

Dynamic_ALU #(.ADDER_STAGES(ADDER_3), .MULT_STAGES(MULT_3)) alu2 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .valid_in(valid_in), .ready_out(ready_out[2]), .result(result[2]), .exception(exception[2]), 
    .overflow(overflow[2]), .underflow(underflow[2]), .valid_out(valid_out[2]), .ready_in(ready_in)
);

Dynamic_ALU #(.ADDER_STAGES(ADDER_4), .MULT_STAGES(MULT_4)) alu3 (
    .clk(clk), .reset(reset), .operand_a(operand_a), .operand_b(operand_b), .operation(operation),
    .valid_in(valid_in), .ready_out(ready_out[3]), .result(result[3]), .exception(exception[3]), 
    .overflow(overflow[3]), .underflow(underflow[3]), .valid_out(valid_out[3]), .ready_in(ready_in)
);

// Test execution
integer test_idx, config_idx;
integer error_count;
integer sent_count, received_count[4];
reg [31:0] expected_results[4][NUM_TESTS];
reg [31:0] received_results[4][NUM_TESTS];
integer timeout_counter;

// Task to send a transaction (waits for all ALUs to be ready)
task send_transaction(input [31:0] a, input [31:0] b, input [1:0] op);
begin
    // Set up the transaction
    operand_a = a;
    operand_b = b;
    operation = op;
    
    // Wait for ALL ALUs to be ready
    timeout_counter = 0;
    while ((!ready_out[0] || !ready_out[1] || !ready_out[2] || !ready_out[3]) && timeout_counter < 100) begin
        @(posedge clk);
        timeout_counter++;
    end
    
    if (timeout_counter >= 100) begin
        $display("ERROR: Not all ALUs ready for input after %d cycles", timeout_counter);
        $display("ready_out = %b%b%b%b", ready_out[3], ready_out[2], ready_out[1], ready_out[0]);
        $finish;
    end
    
    // Now send the transaction to all ALUs
    valid_in = 1'b1;
    @(posedge clk);
    valid_in = 1'b0;
    
    $display("Sent transaction: a=%h, b=%h, op=%d", a, b, op);
end
endtask

// Task to receive one result from a specific ALU
task receive_result_from_alu(input integer alu_id);
integer timeout;
begin
    // Set ready to receive
    ready_in = 1'b1;
    
    // Wait for the specified ALU to produce valid output
    timeout = 0;
    while (!valid_out[alu_id] && timeout < 1000) begin
        @(posedge clk);
        timeout++;
    end
    if (timeout >= 1000) begin
        $display("ERROR: Timeout waiting for valid_out from config %d after %d cycles", alu_id, timeout);
        $display("valid_out = %b%b%b%b", valid_out[3], valid_out[2], valid_out[1], valid_out[0]);
        $display("ready_in = %b", ready_in);
        $finish;
    end
    
    // Capture the result
    received_results[alu_id][received_count[alu_id]] = result[alu_id];
    received_count[alu_id]++;
    
    // Wait for the valid to drop (consumed)
    @(posedge clk);
    ready_in = 1'b0;
    
    // Give some time for the handshake to complete
    repeat(2) @(posedge clk);
end
endtask

// Task to receive results from all ALUs (one result each)
task receive_results();
begin
    receive_result_from_alu(0);
    receive_result_from_alu(1);
    receive_result_from_alu(2);
    receive_result_from_alu(3);
end
endtask

initial begin
    $display("Testing Dynamic ALU with different pipeline configurations...");
    
    // Generate test vectors
    for (test_idx = 0; test_idx < NUM_TESTS; test_idx++) begin
        test_a[test_idx] = $random;
        test_b[test_idx] = $random;
        test_op[test_idx] = test_idx % 3;  // Cycle through ADD, SUB, MUL
        
        // Include some known values
        if (test_idx < 15) begin
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
    sent_count = 0;
    for (config_idx = 0; config_idx < 4; config_idx++) begin
        received_count[config_idx] = 0;
    end
    
    reset = 1;
    operand_a = 0;
    operand_b = 0;
    operation = 0;
    valid_in = 0;
    ready_in = 0;
    
    // Reset sequence
    repeat(10) @(posedge clk);
    reset = 0;
    repeat(20) @(posedge clk);  // Give more time after reset
    
    $display("After reset: ready_out = %b%b%b%b", ready_out[3], ready_out[2], ready_out[1], ready_out[0]);
    
    // Send all test transactions
    for (test_idx = 0; test_idx < NUM_TESTS; test_idx++) begin
        send_transaction(test_a[test_idx], test_b[test_idx], test_op[test_idx]);
        sent_count++;
        
        // Receive results more frequently to prevent FIFO overflow
        if (test_idx % 3 == 2) begin
            receive_results();
        end
        
        if (test_idx % 20 == 0) begin
            $display("Sent %d transactions...", test_idx);
        end
    end
    
    // Receive remaining results
    while (received_count[0] < NUM_TESTS) begin
        receive_results();
    end
    
    $display("All transactions completed. Verifying results...");
    
    // Compare results from all configurations
    for (test_idx = 0; test_idx < NUM_TESTS; test_idx++) begin
        for (config_idx = 1; config_idx < 4; config_idx++) begin
            if (received_results[0][test_idx] !== received_results[config_idx][test_idx]) begin
                $display("MISMATCH at test %d: a=%h, b=%h, op=%d", 
                        test_idx, test_a[test_idx], test_b[test_idx], test_op[test_idx]);
                $display("  Config 0: %h", received_results[0][test_idx]);
                $display("  Config %d: %h", config_idx, received_results[config_idx][test_idx]);
                error_count++;
            end
        end
    end
    
    // Report results
    $display("\n=== DYNAMIC ALU TEST RESULTS ===");
    $display("Total tests: %d", NUM_TESTS);
    $display("Errors: %d", error_count);
    
    if (error_count == 0) begin
        $display("SUCCESS: All pipeline configurations produce identical results!");
    end else begin
        $display("FAILURE: Found mismatches between pipeline configurations");
    end
    
    $finish;
end

endmodule