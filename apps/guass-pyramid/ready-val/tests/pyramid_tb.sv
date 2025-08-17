/* verilator lint_off DECLFILENAME */
/* verilator lint_off MULTITOP */

module pyramid_tb;
  parameter Blur0_N = 16;
  parameter Blur1_N = 16;
  parameter BlurUp_N = 16;
  parameter TIMEOUT = 10000; // Max cycles before timeout

  logic clk, reset;
  logic valid_i, ready_i, valid_o, ready_o;
  logic[7:0][7:0][7:0] in, out;
  
  // Debug signals
  logic[3:0] st;
  logic[1:0] blur0_st, blur1_st, blur_up_st;
  logic[7:0][7:0][7:0] level0_stable;
  logic[3:0][3:0][7:0] level1_stable;
  logic[7:0][7:0][7:0] upsampled_stable;

  // Instantiate the Pyramid module
  Pyramid #(
    .Blur0_N(Blur0_N),
    .Blur1_N(Blur1_N),
    .BlurUp_N(BlurUp_N)
  ) dut (
    .clk(clk),
    .reset(reset),
    .valid_i(valid_i),
    .ready_i(ready_i),
    .in(in),
    .valid_o(valid_o),
    .ready_o(ready_o),
    .out(out),
    .st(st),
    .blur0_st(blur0_st),
    .blur1_st(blur1_st),
    .blur_up_st(blur_up_st),
    .level0_stable(level0_stable),
    .level1_stable(level1_stable),
    .upsampled_stable(upsampled_stable)
  );

  // Clock generation
  always #5 clk = ~clk;

  // Simulation control and monitoring
  int cycle_count = 0;
  
  initial begin
    $dumpfile("pyramid_tb.vcd");
    $dumpvars(0, pyramid_tb);
    
    // Initialize signals
    clk = 0;
    reset = 1;
    valid_i = 0;
    ready_o = 1; // Always ready to receive output
    
    // Create a simple test pattern for input
    for (int i = 0; i < 8; i++) begin
      for (int j = 0; j < 8; j++) begin
        in[i][j] = 8'(i * 8 + j + 1); // Pattern: 1,2,3...64
      end
    end
    
    $display("=== Pyramid Testbench ===");
    $display("Parameters: Blur0_N=%0d, Blur1_N=%0d, BlurUp_N=%0d", Blur0_N, Blur1_N, BlurUp_N);
    $display("Input pattern:");
    for (int i = 0; i < 8; i++) begin
      $write("  ");
      for (int j = 0; j < 8; j++) begin
        $write("%3d ", in[i][j]);
      end
      $display("");
    end
    $display("");

    // Reset sequence
    repeat(3) @(posedge clk);
    reset = 0;
    
    // Wait for ready_i
    while (!ready_i) begin
      @(posedge clk);
      cycle_count++;
      if (cycle_count > TIMEOUT) begin
        $error("TIMEOUT: Module never became ready (ready_i)");
        $finish;
      end
    end
    
    $display("Module ready after %0d cycles", cycle_count);
    
    // Assert valid_i for one cycle
    valid_i = 1;
    @(posedge clk);
    cycle_count++;
    valid_i = 0;
    
    $display("Input submitted at cycle %0d", cycle_count);
    
    // Wait for output to be valid
    while (!valid_o) begin
      @(posedge clk);
      cycle_count++;
      if (cycle_count > TIMEOUT) begin
        $error("TIMEOUT: Module never produced output (valid_o) after %0d cycles", TIMEOUT);
        $error("Final state: st=%0d, blur0_st=%0d, blur1_st=%0d, blur_up_st=%0d", 
               st, blur0_st, blur1_st, blur_up_st);
        $finish;
      end
      
      // Progress indicator every 100 cycles
      if (cycle_count % 100 == 0) begin
        $display("Cycle %0d: st=%0d, blur0_st=%0d, blur1_st=%0d, blur_up_st=%0d", 
                 cycle_count, st, blur0_st, blur1_st, blur_up_st);
      end
    end
    
    $display("\n=== RESULTS ===");
    $display("Processing completed in %0d cycles", cycle_count);
    
    $display("\nFinal output:");
    for (int i = 0; i < 8; i++) begin
      $write("  ");
      for (int j = 0; j < 8; j++) begin
        $write("%3d ", out[i][j]);
      end
      $display("");
    end
    
    $display("\nDebug signals:");
    $display("Final state: %0d", st);
    $display("Blur states: blur0=%0d, blur1=%0d, blur_up=%0d", blur0_st, blur1_st, blur_up_st);
    
    $display("\nlevel0_stable (8x8):");
    for (int i = 0; i < 8; i++) begin
      $write("  ");
      for (int j = 0; j < 8; j++) begin
        $write("%3d ", level0_stable[i][j]);
      end
      $display("");
    end
    
    $display("\nlevel1_stable (4x4):");
    for (int i = 0; i < 4; i++) begin
      $write("  ");
      for (int j = 0; j < 4; j++) begin
        $write("%3d ", level1_stable[i][j]);
      end
      $display("");
    end
    
    $display("\nupsampled_stable (8x8):");
    for (int i = 0; i < 8; i++) begin
      $write("  ");
      for (int j = 0; j < 8; j++) begin
        $write("%3d ", upsampled_stable[i][j]);
      end
      $display("");
    end
    
    $display("\n=== TEST COMPLETED SUCCESSFULLY ===");
    $finish;
  end

  // Timeout watchdog
  initial begin
    #(TIMEOUT * 10); // 10ns per cycle
    $error("GLOBAL TIMEOUT: Simulation exceeded maximum time");
    $finish;
  end

endmodule