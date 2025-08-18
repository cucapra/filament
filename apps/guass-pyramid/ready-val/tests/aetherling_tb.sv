`timescale 1ns/1ps

module AetherlingConv_tb;

// Clock and reset
logic clk;
logic reset;

// Test parameters
parameter N = 4;  // Can be 1, 2, 4, 8, or 16

// DUT signals
logic valid_i;
logic valid_o;
logic[N-1:0][7:0] data_in;
logic[N-1:0][7:0] data_out;
int cycles, start_cycle, end_cycle;

// Verification variables
logic pass;

// Instantiate DUT
AetherlingConv #(.N(N)) dut (
  .clk(clk),
  .valid_i(valid_i),
  .valid_o(valid_o),
  .I(data_in),
  .O(data_out)
);

// Clock generation
initial begin
  clk = 0;
  forever #5 clk = ~clk;  // 100MHz clock
end

// Dump waveforms
initial begin
  $dumpfile("aetherling_tb.vcd");
  $dumpvars(0, AetherlingConv_tb);
end

always_ff @(posedge clk) begin
  if (reset) cycles <= '0;
  else cycles <= cycles + 1;

  if (cycles > 5000) begin
    $display("TIMEOUT: Test exceeded 5000 cycles!");
    $finish;
  end
end

// Test stimulus
initial begin
  // Initialize signals
  reset = 1;
  valid_i = 0;
  data_in = '0;

  // Hold reset for 5 cycles
  repeat(5) @(posedge clk);
  reset = 0;
  @(posedge clk);

  // Prepare input data [0..N-1]
  for (int i = 0; i < N; i++) begin
    data_in[i] = i[7:0];
  end

  $display("=== AETHERLINGCONV TEST ===");
  $display("Parameter N: %0d", N);
  $display("Input data: ");
  for (int i = 0; i < N; i++) begin
    $write("%3d ", data_in[i]);
  end
  $display("");

  // Send input data
  $display("Sending data to AetherlingConv module...");
  start_cycle = cycles + 1; // Mark the cycle when we assert valid_i
  valid_i = 1;
  @(posedge clk);
  valid_i = 0;  // Clear valid after one cycle

  // Wait for output to be valid
  $display("Waiting for output...");
  $display("Cycle %0d: valid_i=1, valid_o=%b", cycles, valid_o);
  while (!valid_o) begin
    @(posedge clk);
    $display("Cycle %0d: valid_i=0, valid_o=%b", cycles, valid_o);
  end
  end_cycle = cycles; // The cycle when valid_o appears
  $display("valid_o asserted at cycle %0d", end_cycle);

  // Print output
  $display("Output data received:");
  for (int i = 0; i < N; i++) begin
    $write("%3d ", data_out[i]);
  end
  $display("");

  // Verify output (should be input + 2)
  $display("Verifying output (each element should be input + 2):");
  pass = 1;
  for (int i = 0; i < N; i++) begin
    logic[7:0] expected = i[7:0] + 2;
    if (data_out[i] !== expected) begin
      $display("ERROR: data_out[%0d] = %3d, expected %3d", i, data_out[i], expected);
      pass = 0;
    end
  end

  if (pass) begin
    $display("PASS: All outputs are correct!");
  end else begin
    $display("FAIL: Output mismatch detected!");
  end

  // Wait a few cycles then end
  repeat(10) @(posedge clk);

  $display("\n=== AETHERLINGCONV LATENCY RESULTS ===");
  $display("Parameter N: %0d", N);
  $display("Transaction start cycle: %0d", start_cycle);
  $display("Transaction end cycle: %0d", end_cycle);
  $display("Total latency: %0d cycles", end_cycle - start_cycle + 1);
  $display("Elements processed: %0d", N);
  $display("Cycles per element: %.1f", real'(end_cycle - start_cycle + 1) / real'(N));
  $display("Total simulation cycles: %0d", cycles);
  $finish;
end

endmodule