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
logic [5:0][N-1:0][7:0] pipeline;
logic [5:0] val;

generate
for (genvar i = 0; i < 5; i++) begin : Shift
  always_ff @(posedge clk) begin
    pipeline[i+1] <= pipeline[i];
    val[i+1] <= val[i];
  end
end

// Increment by two
for (genvar i = 0; i < N; i++) begin : Loop
  always_ff @(posedge clk) begin
    pipeline[0][i] <= in[i] + 2;
  end
end
endgenerate

always_ff @(posedge clk) begin
  val[0] <= valid_i;
end

assign out = pipeline[5];
assign valid_o = val[5];

endmodule
/* verilator lint_on UNUSED */
/* verilator lint_on UNDRIVEN */

/*
module AetherlingConvAdapt #(parameter N = 16) (
  input logic clk,

  input logic valid_i,
  output logic ready_i,
  input logic[N-1:0][7:0] in,

  input logic valid_o,
  output logic ready_o,
  input logic[N-1:0][7:0] out
);

// Only allow the known bindings
generate
  if (N != 1 & N != 2 & N != 4 & N != 8 & N != 16 & N != 48 & N != 144)
    $error("Incorrect binding for conv 16: %0d", N);
endgenerate

localparam LATENCY =
  N == 1 ? 7 :
  N == 48 ? 12 :
  N == 144 ? 21 :
  6;  // N = 4, 8, 16

logic [$clog2(N):0] val, val_nxt;
always_ff @(posedge clk) begin
  if (reset) valid <= 0;
  else val <= { val_nxt[$clog2(N):1], 0 };
end
always_comb begin
  val_nxt = val;
  if (val_i) val_nxt[0] = 1;
end

localparam II =
  N == 144 ? 9 :
  N == 48 ? 3  :
  1;
// TODO;


// The signaling logic within Aetherling is broken so we generate our
// own signals.
AetherlingConv#(.N(N)) Conv(
  .clk, .reset,
  .in(conv_in),
  .valid_i(conv_valid_i),
  .valid_o(),
  .out(conv_out)
);

endmodule
*/

module FastConv2D#(
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

// Store the inputs and outputs till txns occur.
logic[15:0][7:0] in, out;

always_ff @(posedge clk) begin
  if (reset) in <= '0;
  else if (valid_i & ready_i) in <= i;
  else in <= in;
end

// Interface with the convolution module
logic conv_valid_i, conv_valid_o;
logic[N-1:0][7:0] conv_out;
logic[N-1:0][7:0] conv_in;
AetherlingConv#(.N(N)) Conv(
  .clk, .reset,
  .in(conv_in),
  .valid_i(conv_valid_i),
  .valid_o(conv_valid_o),
  .out(conv_out)
);


// The chunk we are working on.
localparam Chunks = 16 / N;
localparam Chunks_1 = Chunks - 1;

// Uses two state machines to interface with the input and output sides of the
// aetherling conv module.

// The send interface will send new inputs to the convolution implementation
// and wait till all the outputs have been read off.
localparam S_IDLE=0, S_PROC=1, S_BLOCKED=2;
logic[1:0] send_st, send_nxt;
wire last_send = send_idx == Chunks_1[3:0];

always_ff @(posedge clk) begin
  if (reset) send_st <= S_IDLE;
  else send_st <= send_nxt;
end

always_comb begin
  send_nxt = send_st;
  case (send_st)
    S_IDLE: if (valid_i) send_nxt = S_PROC;
    S_PROC: if (last_send) send_nxt = S_BLOCKED;
    S_BLOCKED: if (valid_o & ready_o) send_nxt = S_IDLE;
  endcase
end

assign conv_valid_i = send_st == S_PROC;

// The input to convolution module.
logic[3:0] send_idx;
always_ff @(posedge clk) begin
  // This assume that we can send new inputs to the module every cycle.
  if (send_st == S_PROC & ~last_send) send_idx <= send_idx + 1;
  else send_idx <= 0;
end
always_comb begin
  conv_in = '0;
  for (int j = 0; j < Chunks; j++) begin
    if (send_idx == j[3:0]) begin
      conv_in = in[N*j+:N];
    end
  end
end


// The recieve side will wait on the output from the conv module and be
// blocked till the consumer downstream accepts the output.
localparam R_IDLE=0, R_WAIT=1, R_BLOCKED=2;
logic[1:0] recv_st, recv_nxt;
wire last_recv = recv_idx == Chunks_1[3:0];

always_ff @(posedge clk) begin
  if (reset) recv_st <= S_IDLE;
  else recv_st <= recv_nxt;
end

always_comb begin
  recv_nxt = recv_st;
  case (recv_st)
    R_IDLE: if (valid_i) recv_nxt = R_WAIT;
    R_WAIT: if (last_recv) recv_nxt = R_BLOCKED;
    R_BLOCKED: if (ready_o) recv_nxt = R_IDLE;
  endcase
end

// Collect output from the convolution module.
logic[3:0] recv_idx;
always_ff @(posedge clk) begin
  if (recv_st == R_WAIT & conv_valid_o) recv_idx <= recv_idx + 1;
  else recv_idx <= 0;
end
always_comb begin
  out = o;
  for (int j = 0; j < Chunks; j++) begin
    // If the output is valid;
    if (conv_valid_o && recv_idx == j[3:0]) begin
      // $display("writing to chunk %0d: [%0d:%0d]", j[3:0], N*j+N, N*j);
      out[N*j+:N] = conv_out;
    end
  end
end
always_ff @(posedge clk) begin
  o <= out;
end

assign valid_o = recv_st == R_BLOCKED;
assign ready_i = send_st == S_IDLE;

endmodule

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
logic[N-1:0][7:0] conv_out;
logic[N-1:0][7:0] conv_in;
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
  else if (valid_i & ready_i) in <= i;
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

logic[2:0] counter;
always_ff @(posedge clk) begin
  if (counter == LATENCY-1) counter <= 0;
  else if (st == PROC_SEND || st == PROC_RECV) counter <= counter + 1;
  else counter <= 0;
end

localparam LATENCY = N == 1 ? 7 : 6;

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
      if (counter >= LATENCY-1) begin
        // This is the last chunk. Finish processing.
        if (last_chunk) begin
          nxt_idx = '0;
          nxt_st = WRITING;
        end else begin
          // We know this suceeds because conv is fully pipelined
          conv_valid_i = 1;
          nxt_idx = idx + 1;
          nxt_st = PROC_RECV;
        end
      end
    end
    WRITING: begin
      if (ready_o) begin
        // if the upstream computation is sending new input, then we can
        // start processing immediately.
        if (valid_i) nxt_st = PROC_SEND;
        else nxt_st = IDLE;
      end
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
    if (st == PROC_RECV) begin
      if (nxt_idx == j[3:0]) begin
        conv_in = in[N*j+:N];
      end
    end
    if (st == PROC_SEND) begin
      if (idx == j[3:0]) begin
        conv_in = in[N*j+:N];
      end
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
// We can accept new inputs in the writing stage too if the consumer is
// ready to accept the output.
assign ready_i = st == IDLE | (st == WRITING & ready_o);

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
  parameter N = 16
) (
  input logic clk,
  input logic reset,

  output logic[1:0] state,

  input logic valid_i,
  output logic ready_i,
  input logic[D0-1:0][D1-1:0][7:0] in,

  output logic valid_o,
  input logic ready_o,
  output logic[D0-3:0][D1-3:0][7:0] out  // Convolution reduces size by kernel-2
);
// TODO: Implement Gaussian blur convolution with proper state machine
// Should apply blur kernel to each pixel neighborhood and manage ready/valid protocol

logic conv_ready_i, conv_ready_o, conv_valid_i, conv_valid_o;
logic[15:0][7:0] conv_in, conv_out;

Conv2D#(.N(N)) conv2d(
  .clk, .reset,
  .valid_i(conv_valid_i),
  .valid_o(conv_valid_o),
  .ready_i(conv_ready_i),
  .ready_o(conv_ready_o),
  .i(conv_in),
  .o(conv_out)
);

localparam Idle=0, Send_Conv=1, Recv_Conv=2, Writing=3;

logic[1:0] st, nxt_st;
assign state = st;
always_ff @(posedge clk) begin
  if (reset) st <= '0;
  else st <= nxt_st;
end

// We are going to send an input to the module immediately.
wire early_send = ~last_chunk & conv_valid_o & conv_ready_i;

always_comb begin
  nxt_st = st;
  conv_valid_i = 0;
  case (st)
    Idle: begin  // If there is a new input, we start processing it.
      if (valid_i) nxt_st = Send_Conv;
    end
    Send_Conv: begin
      conv_valid_i = 1;
      if (conv_ready_i) nxt_st = Recv_Conv;
    end
    Recv_Conv: begin
      if (conv_valid_o) begin
        if (last_chunk) nxt_st = Writing;
        else if (conv_ready_i) begin
          // If the convolution modules is already ready to process a
          // new input, we will attempt to send one.
          conv_valid_i = 1;
          nxt_st = Recv_Conv;
        end else begin
          nxt_st = Send_Conv;
        end
      end
    end
    Writing: if (ready_o) nxt_st = Idle;
  endcase
end

// Send the ready signal to the convolution module.
assign conv_ready_o = st == Recv_Conv;

// The current image we are working on. We latch the value when a valid input
// is accepted.
logic[D0-1:0][D1-1:0][7:0] image;
always_ff @(posedge clk) begin
  if (reset) image <= '0;
  else if (st == Idle && valid_i) image <= in;
  else image <= image;
end

// Index management logic: determines which part of the input image we are
// working on and where to place the output;

// Calculate tile dimensions
localparam D0_BITS = $clog2(D0);
localparam D1_BITS = $clog2(D1);
localparam bit[D0_BITS:0] TILES_Y = (D0-2)/2;  // Number of tile rows
localparam bit[D1_BITS:0] TILES_X = (D1-2)/2;  // Number of tile columns

// Two explicit tile indices to avoid division/modulo
logic[D0_BITS:0] tile_i, nxt_tile_i;
logic[D1_BITS:0] tile_j, nxt_tile_j;

// Last tile condition
wire last_chunk = (tile_i == TILES_Y-1) && (tile_j == TILES_X-1);

always_comb begin
  nxt_tile_i = tile_i;
  nxt_tile_j = tile_j;

  if (st == Recv_Conv && conv_valid_o) begin
    if (tile_j == TILES_X-1) begin
      // End of row, move to next row, reset column
      nxt_tile_j = 0;
      nxt_tile_i = tile_i + 1;
    end else begin
      // Move to next column in same row
      nxt_tile_j = tile_j + 1;
    end
  end else if (st == Writing && ready_o) begin
    // Reset to beginning
    nxt_tile_i = 0;
    nxt_tile_j = 0;
  end
end

always_ff @(posedge clk) begin
  if (reset) begin
    tile_i <= '0;
    tile_j <= '0;
  end else begin
    tile_i <= nxt_tile_i;
    tile_j <= nxt_tile_j;
  end
end


// Set up the input for the convolution
always_comb begin
  conv_in = '0;
  // Extract 4x4 window starting at (2*tile_i, 2*tile_j)
  // Flatten to 16 elements for Conv2D
  for (int r = 0; r < 4; r++) begin
    for (int c = 0; c < 4; c++) begin
      if (early_send)
        conv_in[r*4 + c] = image[2*nxt_tile_i + r][2*nxt_tile_j + c];
      else
        conv_in[r*4 + c] = image[2*tile_i + r][2*tile_j + c];
    end
  end
end

// Capture output when valid
logic[D0-3:0][D1-3:0][7:0] tmp_out;
always_comb begin
  tmp_out = out; // Preserve existing values
  if (st == Recv_Conv && conv_valid_o) begin
    // Conv2D returns 16 values as 4x4, extract center 2x2
    // Place at output[2*tile_i:2*tile_i+1][2*tile_j:2*tile_j+1]
    // Center 2x2 from 4x4 are at indices [1][1], [1][2], [2][1], [2][2]
    // In flattened array: 5, 6, 9, 10
    tmp_out[2*tile_i][2*tile_j]     = conv_out[5];  // [1][1]
    tmp_out[2*tile_i][2*tile_j+1]   = conv_out[6];  // [1][2]
    tmp_out[2*tile_i+1][2*tile_j]   = conv_out[9];  // [2][1]
    tmp_out[2*tile_i+1][2*tile_j+1] = conv_out[10]; // [2][2]
  end
end
always_ff @(posedge clk) begin
  out <= tmp_out;
end

// Explicitly ignore the unused part of the results.
/* verilator lint_off UNUSEDSIGNAL */
logic[3:0][7:0] ignore_top, ignore_bot;
logic[1:0][7:0] ignore_r, ignore_l;
/* verilator lint_on UNUSEDSIGNAL */

assign ignore_l[0] = conv_out[4];
assign ignore_l[1] = conv_out[8];
assign ignore_r[0] = conv_out[7];
assign ignore_r[1] = conv_out[11];
assign ignore_top = conv_out[3:0];
assign ignore_bot = conv_out[15:12];

assign ready_i = st == Idle;
assign valid_o = st == Writing;

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
      logic[W-1:0] level0_times_3_sat =
        |level0_times_3[W+1:W] ? '1 : level0_times_3[W-1:0];
      logic[W-1:0] level0_three_quarters = level0_times_3_sat >> 2;

      // Calculate 1/4 * level1: divide by 4
      logic[W-1:0] level1_quarter = level1[i][j] >> 2;

      // Add the weighted components
      out[i][j] = level0_three_quarters + level1_quarter;
    end
  end
end

endmodule

module Pyramid #(
  parameter Blur0_N = 16,
  parameter Blur1_N = 16,
  parameter BlurUp_N = 16
) (
  input logic clk,
  input logic reset,

  input logic valid_i,
  output logic ready_i,
  input logic[7:0][7:0][7:0] in,  // 8x8 input image

  output logic valid_o,
  input logic ready_o,
  output logic[7:0][7:0][7:0] out,  // 8x8 output image

  /// Debug signals
  // Blur states
  output logic[3:0] st,
  output logic[1:0] blur0_st,
  output logic[1:0] blur1_st,
  output logic[1:0] blur_up_st,

  // Latched outputs
  output logic[7:0][7:0][7:0] level0_stable,
  output logic[3:0][3:0][7:0] level1_stable,
  output logic[7:0][7:0][7:0] upsampled_stable
);

// Main state machine
localparam bit[3:0]
  Idle=0,
  Level0_Send=1, Level0_Recv=2,
  Level1_Send=3, Level1_Recv=4,
  Upsample_Send=5, Upsample_Recv=6,
  Blend=7,
  Writing=8;

logic[3:0] nxt_st;
always_comb begin
  nxt_st = st;
  // All signals are deasserted unless in specific state.
  blur0_valid_i = 0;
  blur0_ready_o = 0;
  blur1_valid_i = 0;
  blur1_ready_o = 0;
  blur_up_valid_i = 0;
  blur_up_ready_o = 0;

  case (st)
    Idle: begin
      if (valid_i) nxt_st = Level0_Send;
    end
    Level0_Send: begin
      blur0_valid_i = 1;
      if (blur0_ready_i) nxt_st = Level0_Recv;
    end
    Level0_Recv: begin
      blur0_ready_o = 1;
      if (blur0_valid_o) nxt_st = Level1_Send;
    end
    Level1_Send: begin
      blur1_valid_i = 1;
      if (blur1_ready_i) nxt_st = Level1_Recv;
    end
    Level1_Recv: begin
      blur1_ready_o = 1;
      if (blur1_valid_o) nxt_st = Upsample_Send;
    end
    Upsample_Send: begin
      blur_up_valid_i = 1;
      if (blur_up_ready_i) nxt_st = Upsample_Recv;
    end
    Upsample_Recv: begin
      blur_up_ready_o = 1;
      if (blur_up_valid_o) nxt_st = Blend;
    end
    Blend: begin
      nxt_st = Writing;
    end
    Writing: begin
      if (ready_o) nxt_st = Idle;
    end
    default: nxt_st = Idle;    // Should not happen.
  endcase
end
always_ff @(posedge clk) begin
  if (reset) st <= Idle;
  else st <= nxt_st;
end

// Initial state: Latch the input image when valid_i is asserted.
logic[7:0][7:0][7:0] in_stable;
always_ff @(posedge clk) begin
  if (st == Idle && valid_i)
    in_stable <= in;
  else
    in_stable <= in_stable;
end

// Implement Gaussian pyramid pipeline based on blur.fil lines 308-368
//
// Pipeline structure:
// 1. LEVEL 0 PATH (blur.fil lines 320-327):
//    - Pad input 8x8 → 10x10 using Pad module
//    - Blur 10x10 → 8x8 using Blur module
//    - Store level0 result for final blending

// Padding is combinational so we forward the signal directly.
wire[7:0][7:0][7:0] pad0_in = in_stable;
logic [9:0][9:0][7:0] pad0_out;
Pad#(.W(8), .D0(8), .D1(8)) pad0(.in(pad0_in), .out(pad0_out));

logic blur0_valid_i, blur0_valid_o, blur0_ready_i, blur0_ready_o;
logic[7:0][7:0][7:0] blur0_out;

Blur#(.N(Blur0_N), .D0(10), .D1(10)) blur0(
  .clk, .reset, .state(blur0_st),
  .in(pad0_out),   .valid_i(blur0_valid_i), .ready_i(blur0_ready_i),
  .out(blur0_out), .valid_o(blur0_valid_o), .ready_o(blur0_ready_o)
);


// Store the image produced by the first level once valid is asserted
// in the right state.
always_ff @(posedge clk) begin
  if (reset)
    level0_stable <= '0;
  else if (st == Level0_Recv && blur0_valid_o)
    level0_stable <= blur0_out;
  else
    level0_stable <= level0_stable;
end

// 2. LEVEL 1 PATH (blur.fil lines 329-339):
//    - Downsample level0 8x8 → 4x4 using Downsample module
//    - Pad 4x4 → 6x6 using Pad module
//    - Blur 6x6 → 4x4 using Blur module
//    - Store level1 result

logic[3:0][3:0][7:0] down_out;
Downsample#(.W(8), .D0(8), .D1(8)) down1 (
  .in(level0_stable), .out(down_out));

logic[5:0][5:0][7:0] pad1_out;
Pad#(.W(8), .D0(4), .D1(4)) pad1 (
  .in(down_out), .out(pad1_out)
);

// TODO: The output from pad1 is combinationally tied to the
// level0_stable.
// Should we expect it to be stable during the execution of the module?
logic blur1_valid_i, blur1_valid_o, blur1_ready_i, blur1_ready_o;
logic[3:0][3:0][7:0] blur1_out;
Blur#(.N(Blur1_N), .D0(6), .D1(6)) blur1(
  .clk, .reset, .state(blur1_st),
  .in(pad1_out),   .valid_i(blur1_valid_i), .ready_i(blur1_ready_i),
  .out(blur1_out), .valid_o(blur1_valid_o), .ready_o(blur1_ready_o)
);

always_ff @(posedge clk) begin
  if (reset)
    level1_stable <= '0;
  else if (st == Level1_Recv && blur1_valid_o)
    level1_stable <= blur1_out;
  else
    level1_stable <= level1_stable;
end

// 3. UPSAMPLE PATH (blur.fil lines 341-351):
//    - Upsample level1 4x4 → 8x8 using Upsample module
//    - Pad 8x8 → 10x10 using Pad module
//    - Blur 10x10 → 8x8 using Blur module
//    - Result is upsampled level1

logic[7:0][7:0][7:0] upsample_out;
Upsample#(.W(8), .D0(4), .D1(4)) upsample(
  .in(level1_stable), .out(upsample_out)
);

logic[9:0][9:0][7:0] pad_up_out;
Pad#(.W(8), .D0(8), .D1(8)) pad_up(
  .in(upsample_out), .out(pad_up_out)
);

logic blur_up_valid_i, blur_up_valid_o, blur_up_ready_i, blur_up_ready_o;
logic[7:0][7:0][7:0] blur_up_out;
Blur#(.N(BlurUp_N), .D0(10), .D1(10)) blur_up(
  .clk, .reset, .state(blur_up_st),
  .in(pad_up_out),   .valid_i(blur_up_valid_i), .ready_i(blur_up_ready_i),
  .out(blur_up_out), .valid_o(blur_up_valid_o), .ready_o(blur_up_ready_o)
);
always_ff @(posedge clk) begin
  if (reset)
    upsampled_stable <= '0;
  else if (st == Upsample_Recv && blur_up_valid_o)
    upsampled_stable <= blur_up_out;
  else
    upsampled_stable <= upsampled_stable;
end


// 4. BLENDING (blur.fil lines 353-360):
//    - Blend level0 and upsampled level1 using Blend module
//    - Formula: 0.75 * level0 + 0.25 * level1
//    - Output final 8x8 result
logic[7:0][7:0][7:0] blended_out;
Blend#(.W(8), .D0(8), .D1(8)) blend(
  .level0(level0_stable),
  .level1(upsampled_stable),
  .out(blended_out)
);

always_ff @(posedge clk) begin
  if (reset) out <= '0;
  else if (st == Blend) out <= blended_out;
  else out <= out;
end

assign ready_i = st == Idle;
assign valid_o = st == Writing;

endmodule
