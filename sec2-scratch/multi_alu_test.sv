`timescale 1ns / 1ps

module multi_alu_test;
    parameter WIDTH = 32;
    parameter NUM_TESTS = 10;

    // Clock and reset
    logic clk;
    logic reset;

    // Test input arrays
    logic [WIDTH-1:0] test_a [NUM_TESTS-1:0];
    logic [WIDTH-1:0] test_b [NUM_TESTS-1:0];
    logic test_op [NUM_TESTS-1:0];

    // Current test inputs (fed to ALUs)
    logic [WIDTH-1:0] a, b;
    logic op;

    // ALU output arrays
    logic [WIDTH-1:0] result_equal [NUM_TESTS-1:0];     // ALU with (1,1) configuration
    logic [WIDTH-1:0] result_add_heavy [NUM_TESTS-1:0]; // ALU with (3,1) configuration
    logic [WIDTH-1:0] result_mul_heavy [NUM_TESTS-1:0]; // ALU with (1,3) configuration

    // Current ALU outputs
    logic [WIDTH-1:0] alu_equal_out;
    logic [WIDTH-1:0] alu_add_heavy_out;
    logic [WIDTH-1:0] alu_mul_heavy_out;

    // Test configuration parameters
    localparam ALU_EQUAL_ADD_STAGES = 1;
    localparam ALU_EQUAL_MUL_STAGES = 1;
    localparam ALU_ADD_HEAVY_ADD_STAGES = 3;
    localparam ALU_ADD_HEAVY_MUL_STAGES = 1;
    localparam ALU_MUL_HEAVY_ADD_STAGES = 1;
    localparam ALU_MUL_HEAVY_MUL_STAGES = 3;

    // Calculate ALU latencies
    localparam ALU_EQUAL_LATENCY = (ALU_EQUAL_ADD_STAGES > ALU_EQUAL_MUL_STAGES) ?
                                   ALU_EQUAL_ADD_STAGES : ALU_EQUAL_MUL_STAGES;
    localparam ALU_ADD_HEAVY_LATENCY = (ALU_ADD_HEAVY_ADD_STAGES > ALU_ADD_HEAVY_MUL_STAGES) ?
                                      ALU_ADD_HEAVY_ADD_STAGES : ALU_ADD_HEAVY_MUL_STAGES;
    localparam ALU_MUL_HEAVY_LATENCY = (ALU_MUL_HEAVY_ADD_STAGES > ALU_MUL_HEAVY_MUL_STAGES) ?
                                      ALU_MUL_HEAVY_ADD_STAGES : ALU_MUL_HEAVY_MUL_STAGES;

    // Test indices and statistics
    integer current_test;
    integer result_index_equal;
    integer result_index_add_heavy;
    integer result_index_mul_heavy;
    integer mismatch_count_equal_vs_add_heavy = 0;
    integer mismatch_count_equal_vs_mul_heavy = 0;
    integer mismatch_count_add_heavy_vs_mul_heavy = 0;

    // Clock generation
    initial begin
        clk = 0;
        forever #5 clk = ~clk; // 100MHz clock
    end

    // Instantiate ALU with equal pipeline stages (1,1)
    ALU #(
        .WIDTH(WIDTH),
        .ADD_S(ALU_EQUAL_ADD_STAGES),
        .MUL_S(ALU_EQUAL_MUL_STAGES)
    ) alu_equal (
        .clk(clk),
        .reset(reset),
        .op(op),
        .a(a),
        .b(b),
        .result(alu_equal_out)
    );

    // Instantiate ALU with longer add pipeline (3,1)
    ALU #(
        .WIDTH(WIDTH),
        .ADD_S(ALU_ADD_HEAVY_ADD_STAGES),
        .MUL_S(ALU_ADD_HEAVY_MUL_STAGES)
    ) alu_add_heavy (
        .clk(clk),
        .reset(reset),
        .op(op),
        .a(a),
        .b(b),
        .result(alu_add_heavy_out)
    );

    // Instantiate ALU with longer mul pipeline (1,3)
    ALU #(
        .WIDTH(WIDTH),
        .ADD_S(ALU_MUL_HEAVY_ADD_STAGES),
        .MUL_S(ALU_MUL_HEAVY_MUL_STAGES)
    ) alu_mul_heavy (
        .clk(clk),
        .reset(reset),
        .op(op),
        .a(a),
        .b(b),
        .result(alu_mul_heavy_out)
    );

    // Result capture logic - store ALU outputs in arrays based on latency
    always @(posedge clk) begin
        if (!reset) begin
            // Store equal ALU results (latency = 1)
            if (result_index_equal >= ALU_EQUAL_LATENCY && result_index_equal < NUM_TESTS + ALU_EQUAL_LATENCY) begin
                result_equal[result_index_equal - ALU_EQUAL_LATENCY] <= alu_equal_out;
            end

            // Store add-heavy ALU results (latency = 3)
            if (result_index_add_heavy >= ALU_ADD_HEAVY_LATENCY && result_index_add_heavy < NUM_TESTS + ALU_ADD_HEAVY_LATENCY) begin
                result_add_heavy[result_index_add_heavy - ALU_ADD_HEAVY_LATENCY] <= alu_add_heavy_out;
            end

            // Store mul-heavy ALU results (latency = 3)
            if (result_index_mul_heavy >= ALU_MUL_HEAVY_LATENCY && result_index_mul_heavy < NUM_TESTS + ALU_MUL_HEAVY_LATENCY) begin
                result_mul_heavy[result_index_mul_heavy - ALU_MUL_HEAVY_LATENCY] <= alu_mul_heavy_out;
            end

            // Increment result indices
            result_index_equal <= result_index_equal + 1;
            result_index_add_heavy <= result_index_add_heavy + 1;
            result_index_mul_heavy <= result_index_mul_heavy + 1;
        end else begin
            result_index_equal <= 0;
            result_index_add_heavy <= 0;
            result_index_mul_heavy <= 0;
        end
    end

    // Helper function to generate random bit patterns
    function automatic [31:0] get_random_bits();
        return $random;
    endfunction

    // Generate test vector arrays
    initial begin
        $display("Generating %0d random test vectors...", NUM_TESTS);
        for (int i = 0; i < NUM_TESTS; i++) begin
            test_a[i] = get_random_bits();
            test_b[i] = get_random_bits();
            test_op[i] = $random % 2;
        end
        $display("Test vector generation complete.");
    end

    // Test stimulus and computation
    initial begin
        $dumpfile("multi_alu_test.vcd");
        $dumpvars(0, multi_alu_test);

        // Initialize
        reset = 1;
        a = 0;
        b = 0;
        op = 0;
        current_test = 0;

        // Reset sequence
        @(posedge clk);
        @(posedge clk);
        reset = 0;

        $display("Starting multi-ALU array-based test with %0d vectors", NUM_TESTS);
        $display("ALU configurations:");
        $display("  Equal stages: Add=%0d, Mul=%0d (latency=%0d)",
                 ALU_EQUAL_ADD_STAGES, ALU_EQUAL_MUL_STAGES, ALU_EQUAL_LATENCY);
        $display("  Add-heavy:    Add=%0d, Mul=%0d (latency=%0d)",
                 ALU_ADD_HEAVY_ADD_STAGES, ALU_ADD_HEAVY_MUL_STAGES, ALU_ADD_HEAVY_LATENCY);
        $display("  Mul-heavy:    Add=%0d, Mul=%0d (latency=%0d)",
                 ALU_MUL_HEAVY_ADD_STAGES, ALU_MUL_HEAVY_MUL_STAGES, ALU_MUL_HEAVY_LATENCY);
        $display("");

        // Feed test vectors to ALUs
        for (current_test = 0; current_test < NUM_TESTS; current_test++) begin
            a = test_a[current_test];
            b = test_b[current_test];
            op = test_op[current_test];

            @(posedge clk);

            if (current_test % 100 == 0) begin
                $display("Fed %0d test vectors to ALUs...", current_test + 1);
            end
        end

        // Wait for all ALU pipelines to flush
        $display("Waiting for ALU pipelines to complete...");
        repeat (ALU_ADD_HEAVY_LATENCY + 2) @(posedge clk);

        // Compare results between ALUs
        compare_alu_results();

        $finish;
    end

    // Task to compare results between ALU output arrays
    task compare_alu_results();
        begin
            $display("");
            $display("=== Comparing ALU Results ===");
            $display("Comparing %0d results from each ALU...", NUM_TESTS);

            // Compare equal ALU vs add-heavy ALU
            for (int i = 0; i < NUM_TESTS; i++) begin
                if (result_equal[i] !== result_add_heavy[i]) begin
                    mismatch_count_equal_vs_add_heavy++;
                    if (mismatch_count_equal_vs_add_heavy <= 10) begin
                        $display("MISMATCH %0d (Equal vs Add-heavy): Test %0d, Op=%0d, A=%h, B=%h",
                                 mismatch_count_equal_vs_add_heavy, i, test_op[i], test_a[i], test_b[i]);
                        $display("  Equal: %h, Add-heavy: %h", result_equal[i], result_add_heavy[i]);
                    end
                end
            end

            // Compare equal ALU vs mul-heavy ALU
            for (int i = 0; i < NUM_TESTS; i++) begin
                if (result_equal[i] !== result_mul_heavy[i]) begin
                    mismatch_count_equal_vs_mul_heavy++;
                    if (mismatch_count_equal_vs_mul_heavy <= 10) begin
                        $display("MISMATCH %0d (Equal vs Mul-heavy): Test %0d, Op=%0d, A=%h, B=%h",
                                 mismatch_count_equal_vs_mul_heavy, i, test_op[i], test_a[i], test_b[i]);
                        $display("  Equal: %h, Mul-heavy: %h", result_equal[i], result_mul_heavy[i]);
                    end
                end
            end

            // Compare add-heavy ALU vs mul-heavy ALU
            for (int i = 0; i < NUM_TESTS; i++) begin
                if (result_add_heavy[i] !== result_mul_heavy[i]) begin
                    mismatch_count_add_heavy_vs_mul_heavy++;
                    if (mismatch_count_add_heavy_vs_mul_heavy <= 10) begin
                        $display("MISMATCH %0d (Add-heavy vs Mul-heavy): Test %0d, Op=%0d, A=%h, B=%h",
                                 mismatch_count_add_heavy_vs_mul_heavy, i, test_op[i], test_a[i], test_b[i]);
                        $display("  Add-heavy: %h, Mul-heavy: %h", result_add_heavy[i], result_mul_heavy[i]);
                    end
                end
            end

            // Final results
            $display("");
            $display("=== Multi-ALU Array Comparison Results ===");
            $display("Total tests compared: %0d", NUM_TESTS);
            $display("Equal vs Add-heavy mismatches: %0d (%.2f%%)",
                     mismatch_count_equal_vs_add_heavy,
                     (mismatch_count_equal_vs_add_heavy * 100.0) / NUM_TESTS);
            $display("Equal vs Mul-heavy mismatches: %0d (%.2f%%)",
                     mismatch_count_equal_vs_mul_heavy,
                     (mismatch_count_equal_vs_mul_heavy * 100.0) / NUM_TESTS);
            $display("Add-heavy vs Mul-heavy mismatches: %0d (%.2f%%)",
                     mismatch_count_add_heavy_vs_mul_heavy,
                     (mismatch_count_add_heavy_vs_mul_heavy * 100.0) / NUM_TESTS);

            if (mismatch_count_equal_vs_add_heavy == 0 &&
                mismatch_count_equal_vs_mul_heavy == 0 &&
                mismatch_count_add_heavy_vs_mul_heavy == 0) begin
                $display("✓ ALL TESTS PASSED - All ALU configurations produce identical results!");
            end else begin
                integer total_mismatches = mismatch_count_equal_vs_add_heavy +
                                          mismatch_count_equal_vs_mul_heavy +
                                          mismatch_count_add_heavy_vs_mul_heavy;
                $display("✗ TESTS FAILED - %0d total mismatches detected", total_mismatches);
            end

            // Print all output arrays
            $display("");
            $display("=== ALU Output Arrays ===");

            $display("Equal ALU outputs (ADD_S=%0d, MUL_S=%0d):", ALU_EQUAL_ADD_STAGES, ALU_EQUAL_MUL_STAGES);
            for (int i = 0; i < NUM_TESTS; i++) begin
                if (result_equal[i] != result_add_heavy[i])
                  $display("[%0d] : MISMATCH", i);
                else
                  $display("[%0d]", i);

                $display("  %h", result_equal[i]);
                $display("  %h", result_add_heavy[i]);
                $display("  %h", result_mul_heavy[i]);
            end
        end
    endtask

    // Timeout watchdog
    initial begin
        #1000000; // 1ms timeout
        $display("ERROR: Test timeout reached");
        $finish;
    end

endmodule
