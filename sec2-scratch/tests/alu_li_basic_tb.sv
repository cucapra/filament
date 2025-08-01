// Basic testbench for ALU_LI module
module alu_li_basic_tb;
    parameter WIDTH = 32;
    parameter ADD_S = 2;
    parameter MUL_S = 2;
    parameter CLK_PERIOD = 10;

    logic clk, reset;
    logic [WIDTH-1:0] a_in, b_in;
    logic op_in, valid_in, ready_out, valid_out, ready_in;
    logic [WIDTH-1:0] result_out;

    // Instantiate ALU_LI
    ALU_LI #(.WIDTH(WIDTH), .ADD_S(ADD_S), .MUL_S(MUL_S)) dut (
        .clk(clk),
        .reset(reset),
        .a_in(a_in),
        .b_in(b_in),
        .op_in(op_in),
        .valid_in(valid_in),
        .ready_out(ready_out),
        .result_out(result_out),
        .valid_out(valid_out),
        .ready_in(ready_in)
    );

    // Clock generation
    initial begin
        clk = 0;
        forever #(CLK_PERIOD/2) clk = ~clk;
    end

    // Test sequence
    initial begin
        reset = 1;
        valid_in = 0;
        ready_in = 1;
        a_in = 32'h3f800000; // 1.0
        b_in = 32'h40000000; // 2.0
        op_in = 0; // Add
        repeat(2) @(posedge clk);
        reset = 0;
        @(posedge clk);

        // Test add
        valid_in = 1;
        @(posedge clk);
        valid_in = 0;
        wait(valid_out);
        $display("Add result: %h", result_out);
        @(posedge clk);

        // Test mul
        op_in = 1;
        valid_in = 1;
        @(posedge clk);
        valid_in = 0;
        wait(valid_out);
        $display("Mul result: %h", result_out);
        @(posedge clk);

        $finish;
    end
endmodule
