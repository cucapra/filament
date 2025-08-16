`timescale 1ns/1ps

module Conv2D_tb;

// Clock and reset
logic clk;
logic reset;

// Test parameters
parameter N = 16;  // Can be 1, 2, 4, 8, or 16

// DUT signals
logic valid_i;
logic ready_i;
logic[15:0][7:0] data_in;
logic valid_o;
logic ready_o;
logic[15:0][7:0] data_out;
int cycles, start_v, end_v;

// Verification variables
logic pass;
logic[7:0] expected;

// Instantiate DUT
Conv2D #(.N(N)) dut (
  .clk(clk),
  .reset(reset),
  .valid_i(valid_i),
  .ready_i(ready_i),
  .i(data_in),
  .valid_o(valid_o),
  .ready_o(ready_o),
  .o(data_out)
);

// Clock generation
initial begin
  clk = 0;
  forever #5 clk = ~clk;  // 100MHz clock
end

// Dump waveforms
initial begin
  $dumpfile("conv2d_tb.vcd");
  $dumpvars(0, Conv2D_tb);
end

always_ff @(posedge clk) begin
  if (reset) cycles <= '0;
  else cycles <= cycles + 1;

  if (cycles > 50) begin
    $display("timeout at 100 cycles!");
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

  // Prepare input data [0..15]
  for (int i = 0; i < 16; i++) begin
    data_in[i] = i[7:0];
  end

  $display("Starting test with N=%0d", N);
  $display("Input data: ");
  for (int i = 0; i < 16; i++) begin
    $write("%3d ", data_in[i]);
    if ((i+1) % 4 == 0) $write("\n");
  end

  // Wait for module to be ready
  @(posedge clk);
  while (!ready_i) begin
    @(posedge clk);
  end

  // Send input data
  $display("\nSending data to Conv2D module...");
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
  end

  // Assert ready to accept output
  ready_o = 1;
  @(posedge clk);

  // Print output
  $display("\nOutput data received:");
  for (int i = 0; i < 16; i++) begin
    $write("%3d ", data_out[i]);
    if ((i+1) % 4 == 0) $write("\n");
  end

  // Verify output (should be input + 2)
  $display("\nVerifying output (each element should be input + 2):");
  pass = 1;
  for (int i = 0; i < 16; i++) begin
    expected = i[7:0] + 2;
    if (data_out[i] !== expected) begin
      $display("ERROR: data_out[%2d] = %3d, expected %3d", i, data_out[i], expected);
      pass = 0;
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
