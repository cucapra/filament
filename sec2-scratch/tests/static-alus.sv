// Test module for multi-stage pipeline functionality using random vectors
module test_pipeline;

    parameter WIDTH = 32;
    parameter CLK_PERIOD = 10;
    parameter NUM_TESTS = 10000;  // Number of random tests to run
    parameter STAGES = 3;

    logic clk;
    logic [WIDTH-1:0] a, b;
    logic [WIDTH-1:0] add_result_1stage, add_result_3stage;
    logic [WIDTH-1:0] mul_result_1stage, mul_result_3stage;

    // Delayed results from 1-stage pipeline (delayed by 2 cycles to match 3-stage timing)
    logic [WIDTH-1:0] add_result_1stage_delayed, mul_result_1stage_delayed;

    // Test counters
    int test_count = 0;
    int mismatch_count = 0;
    int seed = 42;  // Fixed seed for reproducibility

    // Single-stage modules (default)
    Add #(.WIDTH(WIDTH), .PIPELINE_STAGES(1)) add_1stage (
        .clk(clk), .a(a), .b(b), .sum(add_result_1stage)
    );

    Mul #(.WIDTH(WIDTH), .PIPELINE_STAGES(1)) mul_1stage (
        .clk(clk), .a(a), .b(b), .product(mul_result_1stage)
    );

    // Multi-stage modules (3 stages)
    Add #(.WIDTH(WIDTH), .PIPELINE_STAGES(STAGES)) add_3stage (
        .clk(clk), .a(a), .b(b), .sum(add_result_3stage)
    );

    Mul #(.WIDTH(WIDTH), .PIPELINE_STAGES(STAGES)) mul_3stage (
        .clk(clk), .a(a), .b(b), .product(mul_result_3stage)
    );

    // Shift registers to delay 1-stage results by 2 cycles to match 3-stage timing
    // 3-stage pipeline has 3 cycles latency, 1-stage has 1 cycle, so delay = 3-1 = 2
    Shift_register #(.WIDTH(WIDTH), .STAGES(STAGES-1)) add_delay (
        .clk(clk), .data_in(add_result_1stage), .data_out(add_result_1stage_delayed)
    );

    Shift_register #(.WIDTH(WIDTH), .STAGES(STAGES-1)) mul_delay (
        .clk(clk), .data_in(mul_result_1stage), .data_out(mul_result_1stage_delayed)
    );

    // Clock generation
    initial begin
        clk = 0;
        forever #(CLK_PERIOD/2) clk = ~clk;
    end

    // Helper function to generate random bit patterns
    function automatic [31:0] get_random_bits();
        return $random(seed);
    endfunction

    // Test procedure
    initial begin
        // Initialize inputs
        a = 32'h0;
        b = 32'h0;

        $display("Testing pipeline stages with random vectors...");
        $display("Running %0d random tests comparing 1-stage vs 3-stage pipelines", NUM_TESTS);
        $display("");

        // Wait for reset and pipeline filling (3 cycles for 3-stage + 2 cycles for delay registers)
        repeat(10) @(posedge clk);

        // Run random tests
        for (int i = 0; i < NUM_TESTS; i++) begin
            logic [WIDTH-1:0] test_a, test_b;

            // Generate random test vectors
            test_a = get_random_bits();
            test_b = get_random_bits();

            // Apply inputs
            a = test_a;
            b = test_b;

            // Wait one cycle for computation
            @(posedge clk);

            // After sufficient pipeline filling, compare delayed 1-stage with 3-stage results
            if (i >= 5) begin  // Start comparing after pipelines are filled
                if (add_result_1stage_delayed !== add_result_3stage ||
                    mul_result_1stage_delayed !== mul_result_3stage) begin
                    mismatch_count++;
                    if (mismatch_count <= 10) begin  // Only show first 10 mismatches
                        $display("MISMATCH #%0d:", mismatch_count);
                        $display("  Test #%0d: A=0x%08h, B=0x%08h", i, test_a, test_b);
                        $display("  Add: 1-stage(delayed)=0x%08h, 3-stage=0x%08h", add_result_1stage_delayed, add_result_3stage);
                        $display("  Mul: 1-stage(delayed)=0x%08h, 3-stage=0x%08h", mul_result_1stage_delayed, mul_result_3stage);
                        $display("");
                    end
                end
            end

            test_count++;

            // Show progress every 10%
            if (i % (NUM_TESTS / 10) == 0 && i > 0) begin
                $display("Progress: %0d%% (%0d/%0d tests), Mismatches: %0d",
                        (i * 100) / NUM_TESTS, i, NUM_TESTS, mismatch_count);
            end
        end

        // Print final results
        $display("\n=== PIPELINE TEST RESULTS ===");
        $display("Total tests run: %0d", test_count);
        $display("Mismatches found: %0d", mismatch_count);
        if (mismatch_count == 0) begin
            $display("✓ SUCCESS: All pipeline configurations produce identical results!");
        end else begin
            $display("✗ FAILURE: Found %0d mismatches out of %0d tests", mismatch_count, test_count);
            $display("Error rate: %f%%", (real'(mismatch_count) / real'(test_count)) * 100.0);
        end

        $display("\nPipeline test completed!");
        $finish;
    end

endmodule
