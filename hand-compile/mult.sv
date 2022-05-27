module Mult(
  input wire go,
  input wire clk,
  input wire reset,
  /* @[T, T+2]    */ input logic [31:0] left,
  /* @[T, T+1]    */ input logic [31:0] right,
  /* @[T+2, T+3]  */ output logic [31:0] out
);
  logic [31:0] rtmp;
  logic [31:0] out_tmp;
  logic nxt, cond;

  logic [31:0] r_nxt;
  logic [31:0] out_nxt;

  assign r_nxt = go ? right : '0;
  assign out_nxt =  nxt ? left * rtmp : '0;

  always_ff @(posedge clk) begin
    if (reset)
      rtmp <= 0;
    else
      rtmp <= r_nxt;
  end

  // Perform the computation
  always_ff @(posedge clk) begin
    if (reset)
      out_tmp <= out_nxt;
    else
      out_tmp <= out_nxt;
  end

  // Mark next cycle for computation
  always_ff @(posedge clk) begin
    if (go)
      nxt <= 1;
    else
      nxt <= 0;
  end

  assign out = out_tmp;

endmodule
