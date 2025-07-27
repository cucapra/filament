`timescale 1ns / 1ps

module test_shift_register;
    parameter WIDTH = 32;

    logic clk;
    logic [WIDTH-1:0] data_in;
    logic [WIDTH-1:0] data_out_0, data_out_1, data_out_2, data_out_3;

    // Clock generation
    initial begin
        clk = 0;
        forever #5 clk = ~clk; // 100MHz clock
    end

    // Test different STAGES configurations
    Shift_register #(.WIDTH(WIDTH), .STAGES(0)) sr0 (
        .clk(clk),
        .data_in(data_in),
        .data_out(data_out_0)
    );

    Shift_register #(.WIDTH(WIDTH), .STAGES(1)) sr1 (
        .clk(clk),
        .data_in(data_in),
        .data_out(data_out_1)
    );

    Shift_register #(.WIDTH(WIDTH), .STAGES(2)) sr2 (
        .clk(clk),
        .data_in(data_in),
        .data_out(data_out_2)
    );

    Shift_register #(.WIDTH(WIDTH), .STAGES(3)) sr3 (
        .clk(clk),
        .data_in(data_in),
        .data_out(data_out_3)
    );

    // Test sequence
    initial begin
        $dumpfile("test_shift_register.vcd");
        $dumpvars(0, test_shift_register);

        // Initialize
        data_in = 32'h00000000;

        // Wait a few cycles for initialization
        repeat(2) @(posedge clk);

        $display("Testing Shift_register with different STAGES values");
        $display("Time | Input    | 0-stage  | 1-stage  | 2-stage  | 3-stage");
        $display("-----|----------|----------|----------|----------|----------");

        // Test sequence with known values
        test_value(32'h12345678, "Test 1");
        test_value(32'h87654321, "Test 2");
        test_value(32'hABCDEF00, "Test 3");
        test_value(32'hDEADBEEF, "Test 4");
        test_value(32'h00000000, "Test 5");
        test_value(32'hFFFFFFFF, "Test 6");

        // Wait a few more cycles to see final outputs
        repeat(4) begin
            @(posedge clk);
            $display("%4d | %h | %h | %h | %h | %h",
                     $time/10, data_in, data_out_0, data_out_1, data_out_2, data_out_3);
        end

        $display("");
        $display("=== Shift Register Test Analysis ===");
        $display("0-stage (passthrough): Should show current input immediately");
        $display("1-stage: Should show input delayed by 1 cycle");
        $display("2-stage: Should show input delayed by 2 cycles");
        $display("3-stage: Should show input delayed by 3 cycles");

        $finish;
    end

    task test_value(input [WIDTH-1:0] value, input string label);
        begin
            data_in = value;
            @(posedge clk);
            $display("%4d | %h | %h | %h | %h | %h   // %s",
                     $time/10, data_in, data_out_0, data_out_1, data_out_2, data_out_3, label);
        end
    endtask

endmodule
