`timescale 1ns / 1ps

module test_fp_variants;

// Test parameters
parameter NUM_TESTS = 1000;
parameter CLK_PERIOD = 10;

// Clock and reset
reg clk;
reg reset;

// Test inputs
reg [31:0] test_a, test_b;
reg [31:0] test_vectors_a [0:NUM_TESTS-1];
reg [31:0] test_vectors_b [0:NUM_TESTS-1];

// Adder outputs
wire [31:0] add_result_1, add_result_2, add_result_3, add_result_4;

// Multiplier outputs  
wire [31:0] mult_result_1, mult_result_2, mult_result_3, mult_result_4;
wire mult_exc_1, mult_exc_2, mult_exc_3, mult_exc_4;
wire mult_ovf_1, mult_ovf_2, mult_ovf_3, mult_ovf_4;
wire mult_unf_1, mult_unf_2, mult_unf_3, mult_unf_4;

// Instantiate adder variants
FP_Adder_1Stage add1 (
    .clk(clk), .reset(reset),
    .a(test_a), .b(test_b),
    .result(add_result_1)
);

FP_Adder_2Stage add2 (
    .clk(clk), .reset(reset),
    .a(test_a), .b(test_b),
    .result(add_result_2)
);

FP_Adder_3Stage add3 (
    .clk(clk), .reset(reset),
    .a(test_a), .b(test_b),
    .result(add_result_3)
);

FP_Adder_4Stage add4 (
    .clk(clk), .reset(reset),
    .a(test_a), .b(test_b),
    .result(add_result_4)
);

// Instantiate multiplier variants
FP_Mult_1Stage mult1 (
    .clk(clk), .reset(reset),
    .a(test_a), .b(test_b),
    .result(mult_result_1),
    .exception(mult_exc_1),
    .overflow(mult_ovf_1),
    .underflow(mult_unf_1)
);

FP_Mult_2Stage mult2 (
    .clk(clk), .reset(reset),
    .a(test_a), .b(test_b),
    .result(mult_result_2),
    .exception(mult_exc_2),
    .overflow(mult_ovf_2),
    .underflow(mult_unf_2)
);

FP_Mult_3Stage mult3 (
    .clk(clk), .reset(reset),
    .a(test_a), .b(test_b),
    .result(mult_result_3),
    .exception(mult_exc_3),
    .overflow(mult_ovf_3),
    .underflow(mult_unf_3)
);

FP_Mult_4Stage mult4 (
    .clk(clk), .reset(reset),
    .a(test_a), .b(test_b),
    .result(mult_result_4),
    .exception(mult_exc_4),
    .overflow(mult_ovf_4),
    .underflow(mult_unf_4)
);

// Clock generation
initial begin
    clk = 0;
    forever #(CLK_PERIOD/2) clk = ~clk;
end

// Test vectors generation
initial begin
    // Generate random floating point test vectors
    for (int i = 0; i < NUM_TESTS; i++) begin
        test_vectors_a[i] = $random;
        test_vectors_b[i] = $random;
        
        // Include some special cases
        if (i < 10) begin
            case (i)
                0: begin test_vectors_a[i] = 32'h3f800000; test_vectors_b[i] = 32'h40000000; end // 1.0 + 2.0
                1: begin test_vectors_a[i] = 32'h40400000; test_vectors_b[i] = 32'h40800000; end // 3.0 + 4.0
                2: begin test_vectors_a[i] = 32'h3f800000; test_vectors_b[i] = 32'hbf800000; end // 1.0 + (-1.0)
                3: begin test_vectors_a[i] = 32'h00000000; test_vectors_b[i] = 32'h3f800000; end // 0.0 + 1.0
                4: begin test_vectors_a[i] = 32'h3f800000; test_vectors_b[i] = 32'h00000000; end // 1.0 + 0.0
                5: begin test_vectors_a[i] = 32'h3f800000; test_vectors_b[i] = 32'h40000000; end // 1.0 * 2.0
                6: begin test_vectors_a[i] = 32'h40400000; test_vectors_b[i] = 32'h40800000; end // 3.0 * 4.0
                7: begin test_vectors_a[i] = 32'h3f800000; test_vectors_b[i] = 32'hbf800000; end // 1.0 * (-1.0)
                8: begin test_vectors_a[i] = 32'h00000000; test_vectors_b[i] = 32'h3f800000; end // 0.0 * 1.0
                9: begin test_vectors_a[i] = 32'h3f800000; test_vectors_b[i] = 32'h00000000; end // 1.0 * 0.0
            endcase
        end
    end
end

// Test execution
integer test_idx;
integer error_count_add, error_count_mult;
integer max_pipeline_delay = 4;

initial begin
    $display("Starting differential testing of floating point variants...");
    
    // Initialize
    error_count_add = 0;
    error_count_mult = 0;
    reset = 1;
    test_a = 0;
    test_b = 0;
    
    // Reset sequence
    repeat(5) @(posedge clk);
    reset = 0;
    repeat(2) @(posedge clk);
    
    // Run tests
    for (test_idx = 0; test_idx < NUM_TESTS; test_idx++) begin
        // Apply test vector
        test_a = test_vectors_a[test_idx];
        test_b = test_vectors_b[test_idx];
        
        // Wait for pipeline to settle
        repeat(max_pipeline_delay + 2) @(posedge clk);
        
        // Check adder results
        if (add_result_1 !== add_result_2 || add_result_1 !== add_result_3 || add_result_1 !== add_result_4 ||
            add_result_2 !== add_result_3 || add_result_2 !== add_result_4 || add_result_3 !== add_result_4) begin
            $display("ADDER MISMATCH at test %d: a=%h, b=%h", test_idx, test_a, test_b);
            $display("  1-stage: %h", add_result_1);
            $display("  2-stage: %h", add_result_2);
            $display("  3-stage: %h", add_result_3);
            $display("  4-stage: %h", add_result_4);
            error_count_add++;
        end
        
        // Check multiplier results
        if (mult_result_1 !== mult_result_2 || mult_result_1 !== mult_result_3 || mult_result_1 !== mult_result_4 ||
            mult_result_2 !== mult_result_3 || mult_result_2 !== mult_result_4 || mult_result_3 !== mult_result_4 ||
            mult_exc_1 !== mult_exc_2 || mult_exc_1 !== mult_exc_3 || mult_exc_1 !== mult_exc_4 ||
            mult_ovf_1 !== mult_ovf_2 || mult_ovf_1 !== mult_ovf_3 || mult_ovf_1 !== mult_ovf_4 ||
            mult_unf_1 !== mult_unf_2 || mult_unf_1 !== mult_unf_3 || mult_unf_1 !== mult_unf_4) begin
            $display("MULTIPLIER MISMATCH at test %d: a=%h, b=%h", test_idx, test_a, test_b);
            $display("  1-stage: %h (exc=%b, ovf=%b, unf=%b)", mult_result_1, mult_exc_1, mult_ovf_1, mult_unf_1);
            $display("  2-stage: %h (exc=%b, ovf=%b, unf=%b)", mult_result_2, mult_exc_2, mult_ovf_2, mult_unf_2);
            $display("  3-stage: %h (exc=%b, ovf=%b, unf=%b)", mult_result_3, mult_exc_3, mult_ovf_3, mult_unf_3);
            $display("  4-stage: %h (exc=%b, ovf=%b, unf=%b)", mult_result_4, mult_exc_4, mult_ovf_4, mult_unf_4);
            error_count_mult++;
        end
        
        // Progress indicator
        if (test_idx % 100 == 0) begin
            $display("Completed %d tests...", test_idx);
        end
    end
    
    // Report results
    $display("\n=== TEST RESULTS ===");
    $display("Total tests: %d", NUM_TESTS);
    $display("Adder errors: %d", error_count_add);
    $display("Multiplier errors: %d", error_count_mult);
    
    if (error_count_add == 0 && error_count_mult == 0) begin
        $display("SUCCESS: All pipeline variants produce identical results!");
    end else begin
        $display("FAILURE: Found mismatches between pipeline variants");
    end
    
    $finish;
end

endmodule