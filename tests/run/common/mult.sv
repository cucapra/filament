module Mult(
  input wire clk,
  input wire reset,
  /* @inteface<T, 2> */ input wire go_T,
  /* @[T, T+2]       */ input logic [31:0] left,
  /* @[T, T+1]       */ input logic [31:0] right,
  /* @[T+2, T+3]     */ output logic [31:0] out
);
  logic [31:0] rtmp;
  logic [31:0] out_tmp;
  logic nxt, cond;

  logic [31:0] r_nxt;
  logic [31:0] out_nxt;

  assign out_nxt =  nxt ? left * rtmp : '0;

  assign r_nxt = go_T ? right : '0;
  always_ff @(posedge clk) begin
    if (reset)
      rtmp <= 0;
    else
      rtmp <= r_nxt;
  end

  // Perform the computation
  always_ff @(posedge clk) begin
    if (reset)
      out_tmp <= 0;
    else
      out_tmp <= out_nxt;
  end

  // Mark next cycle for computation
  always_ff @(posedge clk) begin
    if (go_T)
      nxt <= 1;
    else
      nxt <= 0;
  end

  assign out = out_tmp;

endmodule
