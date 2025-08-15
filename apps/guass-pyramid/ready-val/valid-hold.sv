/* verilator lint_off DECLFILENAME */
/* verilator lint_off MULTITOP */

/**
* A module with consumer side valid signal that pulses for one cycle and
* producer side ready-valid interface.
*/
module ValidHold#(
  parameter W = 32
) (
  input logic clk,
  input logic reset,

  input logic valid_i,
  output logic ready_i,
  input logic[W-1:0] in,

  input logic ready_o,
  output logic valid_o,
  output logic[W-1:0] out
);

// Do we currently have a valid value?
logic st, nxt_st;
logic[W-1:0] val;

localparam EMPTY = 0;
localparam FULL = 1;

always_comb begin
  case (st)
    EMPTY: begin
      if (valid_i) nxt_st = FULL;
      else nxt_st = EMPTY;
    end
    FULL: begin
      if (ready_o) nxt_st = EMPTY;  // Consumer Txn will occur.
      else nxt_st = FULL;
    end
  endcase
end

always_ff @(posedge clk) begin
  if (reset) begin
    st <= EMPTY;
  end else begin
    st <= nxt_st;
  end
end

always_ff @(posedge clk) begin
  if (reset) begin
    val <= '0;
  end else if (st == EMPTY && valid_i) begin
    val <= in;
  end
end

`ifndef SYNTHESIS
// Error out if the downstream producer attempts to send a value when not
// ready.
always_ff @(posedge clk) begin
  if (st == FULL & valid_i)
    $error("Downstream producer attempted to send value while module was not ready.");
end
`endif

assign valid_o = st == FULL;
assign ready_i = st == EMPTY;
assign out = val;

endmodule

// Wrapper for aetherling module that accepts N chunks at a time for a 16x16
/* verilator lint_off UNDRIVEN */
/* verilator lint_off UNUSED */
module AetherlingConv#(
  parameter N = 16
) (
  input logic clk,
  input logic reset,

  input logic[N-1:0][7:0] in,
  input logic valid_i,

  output logic[N-1:0][7:0] out,
  output logic valid_o
);
generate
// Increment by two
for (genvar i = 0; i < N; i++) begin : Loop
  always_ff @(posedge clk) begin
    if (reset) out[i] <= '0;
    else out[i] <= in[i] + 2;
  end
end

always_ff @(posedge clk) begin
  if (reset) valid_o <= '0;
  else valid_o <= valid_i;
end

endgenerate

endmodule
/* verilator lint_on UNUSED */
/* verilator lint_on UNDRIVEN */

module Conv2D#(
  parameter N = 16
) (
  input logic clk,
  input logic reset,

  // Input interface
  input logic valid_i,
  output logic ready_i,
  input logic[15:0][7:0] i,

  // Output interface
  output logic valid_o,
  input logic ready_o,
  output logic[15:0][7:0] o
);

// Interface with the convolution module
logic conv_valid_i, conv_valid_o;
logic[N-1:0][7:0] conv_out, conv_in;
AetherlingConv#(.N) Conv(
  .clk, .reset,
  .in(conv_in),
  .valid_i(conv_valid_i),
  .valid_o(conv_valid_o),
  .out(conv_out)
);

// States
localparam IDLE=0, PROC_SEND=1, PROC_RECV=2, WRITING=3;

// Store the inputs till the computation is done.
logic[15:0][7:0] in, out;

always_ff @(posedge clk) begin
  if (reset) in <= '0;
  else if (st == IDLE && valid_i & ready_i) in <= i;
  else in <= in;
end

// The chunk we are working on.
localparam Chunks = 16 / N;
logic[3:0] idx, nxt_idx;
always_ff @(posedge clk) begin
  if (reset) idx <= '0;
  else idx <= nxt_idx;
end

wire last_chunk = idx == Chunks[3:0]-1;


// State machine
logic[1:0] st, nxt_st;
always_comb begin
  nxt_st = st;
  nxt_idx = idx;
  conv_valid_i = 0;

  case (st)
    IDLE: begin
      if (valid_i) nxt_st = PROC_SEND;
    end
    PROC_SEND: begin
      nxt_st = PROC_RECV;
      nxt_idx = idx + 1;
      conv_valid_i = 1;
    end
    PROC_RECV: begin
      // If the convolution module has returned a valid value.
      if (conv_valid_o) begin
        if (last_chunk) nxt_st = WRITING; // This is the last chunk. Finish processing.
        else nxt_st = PROC_SEND;          // More chunks remain.
      end
    end
    WRITING: begin
      if (ready_o) nxt_st = IDLE;
    end
  endcase
end
always_ff @(posedge clk) begin
  if (reset) st <= IDLE;
  else st <= nxt_st;
end

// The input to convolution module.
always_comb begin
  conv_in = '0;
  for (int j = 0; j < Chunks; j++) begin
    if (idx == j[3:0]) begin
      conv_in = in[(N*(j+1)-1)+:N];
    end
  end
end

// Collect output from the convolution module.
always_comb begin
  out = o;
  for (int j = 0; j < Chunks; j++) begin
    // If the output is valid;
    if (conv_valid_o && idx == j[3:0]) begin
      out[(N*(j+1)-1)+:N] = conv_out;
    end
  end
end
always_ff @(posedge clk) begin
  o <= out;
end

assign valid_o = st == WRITING;
assign ready_i = st == IDLE;

endmodule
