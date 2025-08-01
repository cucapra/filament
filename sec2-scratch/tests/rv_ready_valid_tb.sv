// Testbench for RV_ReadyValid module
module rv_ready_valid_tb;
    parameter PIPELINE_STAGES = 3;
    parameter CLK_PERIOD = 10;

    logic clk, reset;
    logic valid_in, valid_out, ready_in, ready_out;
    logic [1:0] current_state, next_state;
    logic [$clog2(PIPELINE_STAGES+1)-1:0] stage_counter;

    // Instantiate RV_ReadyValid
    RV_ReadyValid #(.PIPELINE_STAGES(PIPELINE_STAGES)) dut (
        .clk(clk),
        .reset(reset),
        .valid_in(valid_in),
        .valid_out(valid_out),
        .ready_in(ready_in),
        .ready_out(ready_out),
        .stage_counter(stage_counter),
        .current_state(current_state),
        .next_state(next_state)
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
        valid_out = 0;
        ready_in = 1;
        current_state = 2'b00; // IDLE
        repeat(2) @(posedge clk);
        reset = 0;
        @(posedge clk);

        // Test IDLE to COMPUTE
        valid_in = 1;
        @(posedge clk);
        valid_in = 0;
        current_state = next_state;
        $display("State after valid_in: %b", current_state);

        // Test COMPUTE to VALID
        repeat(PIPELINE_STAGES-1) begin
            @(posedge clk);
            current_state = next_state;
        end
        $display("State after pipeline: %b", current_state);

        // Test VALID to IDLE
        valid_out = 1;
        ready_in = 1;
        @(posedge clk);
        current_state = next_state;
        $display("State after valid_out & ready_in: %b", current_state);

        $finish;
    end
endmodule
