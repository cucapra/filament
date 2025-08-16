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
AetherlingConv#(.N(N)) Conv(
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
localparam Chunks_1 = Chunks - 1;
logic[3:0] idx, nxt_idx;
always_ff @(posedge clk) begin
  if (reset) idx <= '0;
  else idx <= nxt_idx;
end

wire last_chunk = idx == Chunks_1[3:0];

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
      conv_valid_i = 1;
    end
    PROC_RECV: begin
      // If the convolution module has returned a valid value.
      if (conv_valid_o) begin
        // This is the last chunk. Finish processing.
        if (last_chunk) begin
          nxt_idx = '0;
          nxt_st = WRITING;
        end else begin
          nxt_idx = idx + 1;
          nxt_st = PROC_SEND;
        end
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
      conv_in = in[N*j+:N];
    end
  end
end

// Collect output from the convolution module.
always_comb begin
  out = o;
  for (int j = 0; j < Chunks; j++) begin
    // If the output is valid;
    if (conv_valid_o && idx == j[3:0]) begin
      // $display("writing to chunk %0d: [%0d:%0d]", j[3:0], N*j+N, N*j);
      out[N*j+:N] = conv_out;
    end
  end
end
always_ff @(posedge clk) begin
  o <= out;
  /*
  if (conv_valid_o) begin
    $write("out: ");
    for (int i = 0; i < 16; i++)
      $write("%0d,", out[i]);
    $write("; conv_out: ");
    for (int i = 0; i < N; i++)
      $write("%0d,", conv_out[i]);
    $display("");
  end
  */
end

assign valid_o = st == WRITING;
assign ready_i = st == IDLE;

endmodule

// Pad the input image by a row and a column;
module Pad#(
  parameter D0 = 8,
  parameter D1 = 8,
  parameter W = 8
) (
  input logic[D0-1:0][D1-1:0][W-1:0] in,
  output logic[D0+1:0][D1+1:0][W-1:0] out
);

always_comb begin
  out = '0;
  // Copy input to the center of the output array
  for (int i = 0; i < D0; i++) begin
    for (int j = 0; j < D1; j++) begin
      out[i+1][j+1] = in[i][j];
    end
  end
end

endmodule

// Downsample by taking every other row and column
module Downsample#(
  parameter D0 = 8,
  parameter D1 = 8,
  parameter W = 8
) (
  input logic[D0-1:0][D1-1:0][W-1:0] in,
  output logic[D0/2-1:0][D1/2-1:0][W-1:0] out
);
always_comb begin
  for (int i = 0; i < D0/2; i++) begin
    for (int j = 0; j < D1/2; j++) begin
      out[i][j] = in[2*i][2*j];
    end
  end
end
endmodule

// Upsample by inserting zeros between pixels (opposite of downsample)
// Takes a 4x4 input and produces an 8x8 output with zeros interpolated
// Implementation: Place input pixels at even indices, fill odd indices with zeros
module Upsample#(
  parameter D0 = 4,
  parameter D1 = 4,
  parameter W = 8
) (
  input logic[D0-1:0][D1-1:0][W-1:0] in,
  output logic[2*D0-1:0][2*D1-1:0][W-1:0] out
);

always_comb begin
  // Initialize all outputs to zero
  out = '0;

  // Place input pixels at even positions [2*i][2*j]
  for (int i = 0; i < D0; i++) begin
    for (int j = 0; j < D1; j++) begin
      out[2*i][2*j] = in[i][j];
      // out[2*i+1][2*j], out[2*i][2*j+1], out[2*i+1][2*j+1] remain zero
    end
  end
end

endmodule

// Gaussian blur convolution module
// Applies a blur kernel to reduce noise and create pyramid levels
// Should implement a 3x3 or 5x5 Gaussian kernel convolution
module Blur#(
  parameter D0 = 8,
  parameter D1 = 8,
  parameter W = 8
) (
  input logic clk,
  input logic reset,

  input logic valid_i,
  output logic ready_i,
  input logic[D0-1:0][D1-1:0][W-1:0] in,

  output logic valid_o,
  input logic ready_o,
  output logic[D0-2:0][D1-2:0][W-1:0] out  // Convolution reduces size by kernel-1
);
// TODO: Implement Gaussian blur convolution with proper state machine
// Should apply blur kernel to each pixel neighborhood and manage ready/valid protocol
endmodule

// Blend two pyramid levels with weighted combination
// Implements: 0.75 * level0 + 0.25 * level1 (3/4 + 1/4 blend)
module Blend#(
  parameter D0 = 8,
  parameter D1 = 8,
  parameter W = 8
) (
  input logic[D0-1:0][D1-1:0][W-1:0] level0,
  input logic[D0-1:0][D1-1:0][W-1:0] level1,
  output logic[D0-1:0][D1-1:0][W-1:0] out
);

always_comb begin
  for (int i = 0; i < D0; i++) begin
    for (int j = 0; j < D1; j++) begin
      // Calculate 3/4 * level0: multiply by 3, then divide by 4
      logic[W+1:0] level0_times_3 = level0[i][j] * 3;
      logic[W-1:0] level0_three_quarters = level0_times_3[W-1:0] >> 2;

      // Calculate 1/4 * level1: divide by 4
      logic[W-1:0] level1_quarter = level1[i][j] >> 2;

      // Add the weighted components
      out[i][j] = level0_three_quarters + level1_quarter;
    end
  end
end

endmodule

module Pyramid (
  input logic clk,
  input logic reset,

  input logic valid_i,
  output logic ready_i,
  input logic[7:0][7:0][7:0] in,  // 8x8 input image

  output logic valid_o,
  input logic ready_o,
  output logic[7:0][7:0][7:0] out  // 8x8 output image
);

// TODO: Implement Gaussian pyramid pipeline based on blur.fil lines 308-368
//
// Pipeline structure:
// 1. LEVEL 0 PATH (blur.fil lines 320-327):
//    - Pad input 8x8 → 10x10 using Pad module
//    - Blur 10x10 → 8x8 using Blur module
//    - Store level0 result for final blending
//
// 2. LEVEL 1 PATH (blur.fil lines 329-339):
//    - Downsample level0 8x8 → 4x4 using Downsample module
//    - Pad 4x4 → 6x6 using Pad module
//    - Blur 6x6 → 4x4 using Blur module
//    - Store level1 result
//
// 3. UPSAMPLE PATH (blur.fil lines 341-351):
//    - Upsample level1 4x4 → 8x8 using Upsample module
//    - Pad 8x8 → 10x10 using Pad module
//    - Blur 10x10 → 8x8 using Blur module
//    - Result is upsampled level1
//
// 4. BLENDING (blur.fil lines 353-360):
//    - Blend level0 and upsampled level1 using Blend module
//    - Formula: 0.75 * level0 + 0.25 * level1
//    - Output final 8x8 result
//
// State machine needed to coordinate:
// - Sequential processing through pipeline stages
// - Proper ready/valid handshaking between modules
// - Timing alignment for final blending step

endmodule
