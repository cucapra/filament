`default_nettype none

module Latch (
  input wire clk,
  input wire reset,
  input wire write_en,
  input wire logic [31:0] in,
  output wire logic [31:0] out
);
  always_ff @(posedge clk) begin
    if (reset)
      out <= 0;
    else if (write_en)
      out <= in;
    else
      out <= out;
  end
endmodule
