`timescale 1ns/1ps

module Blur_tb;

// Clock and reset
logic clk;
logic reset;

// Test parameters
parameter D = 8;  // Image dimension (must be >= 4)
parameter N = 4;  // Conv2D parameter (can be 1, 2, 4, 8, or 16)

// DUT signals
logic valid_i;
logic ready_i;
logic[D-1:0][D-1:0][7:0] data_in;
logic valid_o;
logic ready_o;
logic[D-3:0][D-3:0][7:0] data_out;
logic[1:0] blur_state;
int cycles, start_v, end_v;

// Verification variables
logic pass;
logic[7:0] expected;

// Instantiate DUT
Blur #(.D0(D), .D1(D), .N(N)) dut (
  .clk(clk),
  .reset(reset),
  .state(blur_state),
  .valid_i(valid_i),
  .ready_i(ready_i),
  .in(data_in),
  .valid_o(valid_o),
  .ready_o(ready_o),
  .out(data_out)
);

// Clock generation
initial begin
  clk = 0;
  forever #5 clk = ~clk;  // 100MHz clock
end

// Dump waveforms
initial begin
  $dumpfile("blur_tb.vcd");
  $dumpvars(0, Blur_tb);
end

always_ff @(posedge clk) begin
  if (reset) cycles <= '0;
  else cycles <= cycles + 1;

  if (cycles > 200) begin
    $display("timeout at 200 cycles!");
    $finish;
  end

  if (valid_i) start_v <= cycles;
  if (valid_o) end_v <= cycles;
end

// Test stimulus
initial begin
  // Initialize signals
  reset = 1;
  valid_i = 0;
  ready_o = 0;
  data_in = '0;

  // Hold reset for 5 cycles
  repeat(5) @(posedge clk);
  reset = 0;
  @(posedge clk);

  // Prepare input data [0..D*D-1] arranged as DxD image
  for (int i = 0; i < D; i++) begin
    for (int j = 0; j < D; j++) begin
      data_in[i][j] = 8'(i*D + j);
    end
  end

  $display("Starting test with D=%0d, N=%0d", D, N);
  $display("Input data (%0dx%0d):", D, D);
  for (int i = 0; i < D; i++) begin
    for (int j = 0; j < D; j++) begin
      $write("%3d ", data_in[i][j]);
    end
    $write("\n");
  end

  // Wait for module to be ready
  @(posedge clk);
  while (!ready_i) begin
    @(posedge clk);
  end

  // Send input data
  $display("\nSending data to Blur module...");
  valid_i = 1;
  @(posedge clk);

  // Check if transaction occurred
  if (ready_i && valid_i) begin
    $display("Transaction accepted by module");
  end

  valid_i = 0;  // Clear valid after one cycle

  // Wait for output to be valid
  $display("Waiting for output...");
  while (!valid_o) begin
    @(posedge clk);
    $display("Cycle %0d: State = %0d", cycles, blur_state);
  end

  // Assert ready to accept output
  ready_o = 1;
  @(posedge clk);

  // Print output
  $display("\nOutput data received (%0dx%0d):", D-2, D-2);
  for (int i = 0; i < D-2; i++) begin
    for (int j = 0; j < D-2; j++) begin
      $write("%3d ", data_out[i][j]);
    end
    $write("\n");
  end

  // Verify output (should be corresponding input + 2)
  // The center (D-2)x(D-2) portion of the input should have 2 added
  $display("\nVerifying output (each element should be corresponding input + 2):");
  pass = 1;
  for (int i = 0; i < D-2; i++) begin
    for (int j = 0; j < D-2; j++) begin
      // Output[i][j] corresponds to input[i+1][j+1] after blur convolution
      expected = 8'((i+1)*D + (j+1)) + 2;
      if (data_out[i][j] !== expected) begin
        $display("ERROR: data_out[%0d][%0d] = %3d, expected %3d", i, j, data_out[i][j], expected);
        pass = 0;
      end
    end
  end

  if (pass) begin
    $display("PASS: All outputs are correct!");
  end else begin
    $display("FAIL: Output mismatch detected!");
  end

  ready_o = 0;

  // Wait a few cycles then end
  repeat(10) @(posedge clk);

  $display("\nTest completed! Latency: %0d, Cycles: %0d", end_v-start_v, cycles);
  $finish;
end

endmodule