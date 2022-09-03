`default_nettype none

module Latch (
  input wire clk,
  input wire reset,
  input wire logic write_en,
  input wire logic _go_S, // unused
  input wire logic [31:0] in,
  output logic [31:0] out
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

// Implements a simple mutliplexer
module Mux(
  input wire logic _go, // unused
  input wire logic sel,
  input wire logic [31:0] in0,
  input wire logic [31:0] in1,
  output logic [31:0] out
);
  assign out = sel ? in0 : in1;
endmodule