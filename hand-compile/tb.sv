`include "out.sv"
module TOP;

// Signals for the main module.
logic go, done, clk, reset;
logic [31:0] l0, l1, r0, r1, out0, out1;

main #() main (
  .go(go),
  .clk(clk),
  .reset(reset),
  .l0(l0),
  .l1(l1),
  .r0(r0),
  .r1(r1),
  .out0(out0),
  .out1(out1),
  .done(done)
);

// Cycle counter. Make this signed to catch errors with cycle simulation
// counts.
logic signed [63:0] cycle_count;

always_ff @(posedge clk) begin
  cycle_count <= cycle_count + 1;
end

// Output location of the VCD file
string OUT;
// Disable VCD tracing
int NOTRACE;
// Maximum number of cycles to simulate
int CYCLE_LIMIT;
// Dummy variable to track value returned by $value$plusargs
int CODE;

always #5 clk = ~clk;

always
  #10 if (cycle_count == 20)
    $finish;

initial begin
  CODE = $value$plusargs("OUT=%s", OUT);
  CODE = $value$plusargs("CYCLE_LIMIT=%d", CYCLE_LIMIT);
  if (CYCLE_LIMIT != 0) begin
    $display("cycle limit set to %d", CYCLE_LIMIT);
  end
  CODE = $value$plusargs("NOTRACE=%d", NOTRACE);
  if (NOTRACE == 0) begin
    $display("VCD tracing enabled");
    $dumpfile(OUT);
    $dumpvars(0,main);
  end else begin
    $display("VCD tracing disabled");
  end

  // Initial values
  go = 0;
  clk = 1;
  reset = 0;
  cycle_count = 0;

  // Reset phase for 5 cycles
  #10 reset = 1;
  repeat(4) begin
    #10;
  end

  #10;
  reset = 0;

  // 0
  go = 1;
  l0 = 10;
  r0 = 20;
  #10;

  // 1
  l0 = 10;
  go = 0; // NOTE: go needs to be explicitly set to 0
  r0 = 'x;
  #10;

  // 2
  l1 = 30;
  r1 = 40;
  l0 = 'x;
  #10;

  // 3
  l1 = 30;
  r1 = 'x;
  #10;

  // 4
  l1 = 'x;
end

endmodule
