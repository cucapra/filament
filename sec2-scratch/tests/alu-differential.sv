// Differential Testbench for ALU vs ALU_LI modules
module alu_li_tb;

    // Testbench parameters
    parameter WIDTH = 32;
    parameter CLK_PERIOD = 10;
    parameter NUM_TESTS = 10000;  // 10k tests for reasonable runtime

    // ALU Timing Parameters
    parameter ALU_LATENCY_CYCLES = 1;  // Number of cycles the ALU takes to compute result
    parameter STABILITY_CYCLES = 1;    // Additional cycles to wait for result stability
    // Total wait time = ALU_LATENCY_CYCLES + STABILITY_CYCLES
    // - For this ALU: Add/Mul modules take 1 cycle to register outputs
    // - Stability cycle ensures the always_comb output has settled
    // - For faster ALUs: reduce ALU_LATENCY_CYCLES
    // - For slower ALUs: increase ALU_LATENCY_CYCLES
    // - For marginal timing: increase STABILITY_CYCLES

    // Clock and reset
    logic clk;
    logic reset;

    // Static ALU interface (combinational, simple interface)
    logic [WIDTH-1:0] static_a;
    logic [WIDTH-1:0] static_b;
    logic static_op;
    logic [WIDTH-1:0] static_result;

    // Dynamic ALU_LI interface (latency insensitive with ready-valid)
    logic [WIDTH-1:0] dynamic_a_in;
    logic [WIDTH-1:0] dynamic_b_in;
    logic dynamic_op_in;
    logic dynamic_valid_in;
    logic dynamic_ready_out;
    logic [WIDTH-1:0] dynamic_result_out;
    logic dynamic_valid_out;
    logic dynamic_ready_in;

    // Test control and statistics
    int test_count;
    int mismatch_count;
    logic [WIDTH-1:0] test_a, test_b;
    logic test_op;
    logic [WIDTH-1:0] expected_result, actual_result;

    // Instantiate the Static ALU (from static-alu.sv)
    ALU #(.WIDTH(WIDTH)) static_alu (
        .clk(clk),
        .reset(reset),
        .op(static_op),
        .a(static_a),
        .b(static_b),
        .result(static_result)
    );

    // Instantiate the Dynamic ALU_LI (from dynamic-alu.sv)
    ALU_LI #(.WIDTH(WIDTH), .ADD_S(10), .MUL_S(3)) dynamic_alu (
        .clk(clk),
        .reset(reset),
        .a_in(dynamic_a_in),
        .b_in(dynamic_b_in),
        .op_in(dynamic_op_in),
        .valid_in(dynamic_valid_in),
        .ready_out(dynamic_ready_out),
        .result_out(dynamic_result_out),
        .valid_out(dynamic_valid_out),
        .ready_in(dynamic_ready_in)
    );

    // Clock generation
    initial begin
        clk = 0;
        forever #(CLK_PERIOD/2) clk = ~clk;
    end

    // Random seed for reproducible but varied tests
    int seed;

    // Helper function to generate random bit patterns
    function automatic [31:0] get_random_bits();
        return $random(seed);
    endfunction

    // Helper function to convert real to IEEE 754 (keeping for debugging)
    function automatic [31:0] real_to_fp(real r);
        // Note: $shortrealtobits not supported in iverilog, returning 0 for now
        return 32'h0;
    endfunction

    // Helper function to convert IEEE 754 to real (keeping for debugging)
    function automatic real fp_to_real([31:0] fp);
        // Note: $bitstoshortreal not supported in iverilog, returning 0.0 for now
        return 0.0;
    endfunction

    // Test procedure
    initial begin
        // Initialize signals
        reset = 1;
        dynamic_valid_in = 0;
        dynamic_ready_in = 1;  // Always ready to accept results
        dynamic_a_in = 32'h0;
        dynamic_b_in = 32'h0;
        dynamic_op_in = 0;
        static_a = 32'h0;
        static_b = 32'h0;
        static_op = 0;

        // Initialize test counters
        test_count = 0;
        mismatch_count = 0;
        seed = $time;  // Use current time as seed for randomization

        // Wait for reset deassertion
        repeat(5) @(posedge clk);
        reset = 0;
        repeat(3) @(posedge clk);

        $display("Starting differential ALU testbench...");
        $display("Testing %0d random bit patterns", NUM_TESTS);
        $display("ALU timing: %0d latency cycles + %0d stability cycles = %0d total cycles",
                 ALU_LATENCY_CYCLES, STABILITY_CYCLES, ALU_LATENCY_CYCLES + STABILITY_CYCLES);
        $display("Static ALU vs Dynamic ALU_LI comparison\n");

        // Run differential tests
        run_differential_tests();

        // Print final results
        $display("\n=== TEST RESULTS ===");
        $display("Total tests run: %0d", test_count);
        $display("Mismatches found: %0d", mismatch_count);
        if (mismatch_count == 0) begin
            $display("✓ ALL TESTS PASSED - ALU and ALU_LI produce identical results!");
        end else begin
            $display("✗ TESTS FAILED - Found %0d mismatches out of %0d tests", mismatch_count, test_count);
            $display("Error rate: %.6f%%", (real'(mismatch_count) / real'(test_count)) * 100.0);
        end

        $display("\nTestbench completed!");
        $finish;
    end

    // Main differential testing task
    task automatic run_differential_tests();
        begin
            int progress_interval = NUM_TESTS / 20;  // Show progress every 5%

            for (int i = 0; i < NUM_TESTS; i++) begin
                // Generate random bit patterns
                test_a = get_random_bits();
                test_b = get_random_bits();
                test_op = get_random_bits() & 1'b1;  // Random 0 or 1

                // Test both ALUs with the same inputs
                test_single_case(test_a, test_b, test_op);

                test_count++;

                // Show progress
                if (i % progress_interval == 0 && i > 0) begin
                    $display("Progress: %0d%% (%0d/%0d tests), Mismatches: %0d",
                            (i * 100) / NUM_TESTS, i, NUM_TESTS, mismatch_count);
                end
            end
        end
    endtask

    // Task to test a single case with both ALUs
    task automatic test_single_case(logic [WIDTH-1:0] a_val, logic [WIDTH-1:0] b_val, logic op_val);
        begin
            // Test static ALU (result available after ALU_LATENCY_CYCLES + STABILITY_CYCLES)
            static_a = a_val;
            static_b = b_val;
            static_op = op_val;

            // Wait for ALU computation and stability
            // ALU_LATENCY_CYCLES: Time for Add/Mul modules to compute and register outputs
            // STABILITY_CYCLES: Additional time for signal stability and setup/hold margins
            repeat(ALU_LATENCY_CYCLES + STABILITY_CYCLES) @(posedge clk);
            expected_result = static_result;

            // Test dynamic ALU_LI (latency insensitive with handshaking)
            test_dynamic_alu(a_val, b_val, op_val);

            // Compare results
            if (expected_result !== actual_result) begin
                mismatch_count++;
                if (mismatch_count <= 10) begin  // Only show first 10 mismatches to avoid spam
                    $display("MISMATCH #%0d:", mismatch_count);
                    $display("  Inputs: A=0x%08h, B=0x%08h, OP=%b", a_val, b_val, op_val);
                    $display("  Static ALU result:  0x%08h (%.6f)", expected_result, fp_to_real(expected_result));
                    $display("  Dynamic ALU result: 0x%08h (%.6f)", actual_result, fp_to_real(actual_result));
                    $display("");
                end
            end
        end
    endtask

    // Task to test the dynamic ALU_LI with proper handshaking
    task automatic test_dynamic_alu(logic [WIDTH-1:0] a_val, logic [WIDTH-1:0] b_val, logic op_val);
        begin
            // Set up inputs
            dynamic_a_in = a_val;
            dynamic_b_in = b_val;
            dynamic_op_in = op_val;

            // Wait for ready
            wait(dynamic_ready_out);

            // Assert valid input
            @(posedge clk);
            dynamic_valid_in = 1;

            // Wait for ready to go low (input accepted) or stay high
            @(posedge clk);
            while (dynamic_ready_out && dynamic_valid_in) @(posedge clk);
            dynamic_valid_in = 0;

            // Wait for valid output
            wait(dynamic_valid_out);
            @(posedge clk);

            // Capture result
            actual_result = dynamic_result_out;

            // Complete handshake (ready_in is always 1 in this test)
            @(posedge clk);

            // Wait for transaction to complete
            wait(!dynamic_valid_out || dynamic_ready_out);
        end
    endtask

    // Helper function for floating-point comparison (keeping for potential use)
    function automatic real abs_diff(real a, real b);
        return (a > b) ? (a - b) : (b - a);
    endfunction

    // Simplified monitor for debugging (can be enabled as needed)
    initial begin
        if (0) begin  // Set to 1 to enable detailed monitoring
            $monitor("Time=%0t: static_op=%b, static_result=%h, dynamic_op=%b, dynamic_ready_out=%b, dynamic_valid_in=%b, dynamic_valid_out=%b",
                     $time, static_op, static_result, dynamic_op_in, dynamic_ready_out, dynamic_valid_in, dynamic_valid_out);
        end
    end

endmodule
