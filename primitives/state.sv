`default_nettype none

module Register #(
    parameter WIDTH = 32
) (
  input wire clk,
  input wire reset,
  input wire logic write_en,
  input wire logic [WIDTH-1:0] in,
  output logic [WIDTH-1:0] out
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

// Same as a register but does not have a write enable signal.
module Delay #(
    parameter WIDTH = 32
) (
  input wire clk,
  input wire reset,
  input wire logic [WIDTH-1:0] in,
  output logic [WIDTH-1:0] out
);

Register #(WIDTH) r (
  .clk(clk),
  .reset(reset),
  .write_en(1'b1),
  .in(in),
  .out(out)
);
endmodule

// A shift register that connects N delay elements in series.
module Shift #(
    parameter WIDTH = 32,
    parameter N = 1
) (
  input wire clk,
  input wire reset,
  input wire logic [WIDTH-1:0] in,
  output logic [WIDTH-1:0] out
);

if (N == 0) begin:pass_through
  wire unused = clk;
  assign out = in;
end:pass_through

else begin:chained
  logic [N:0][WIDTH-1:0] data_delayed;

  assign data_delayed[0] = in;
  assign out = data_delayed[N];

  genvar i;
  for(i=1; i<=N; i++) begin
      Delay #(WIDTH)
        ch_reg (
          .clk(clk),
          .reset(reset),
          .in(data_delayed[i-1]),
          .out(data_delayed[i])
        );
  end
end:chained

endmodule


// Similar to a register but provides access to its previous value only.
// Resetting behavior controlled using the SAFE parameter.
module Prev #(
    parameter WIDTH = 32,
    // If 0, reset to 'x, otherwise reset to 0
    parameter SAFE = 0
) (
  input wire clk,
  input wire reset,
  input wire logic write_en,
  input wire logic [WIDTH-1:0] in,
  output logic [WIDTH-1:0] prev
);
  always_ff @(posedge clk) begin
    if (reset)
      if (SAFE == 0)
        prev <= 'x;
      else
        prev <= '0;
    else if (write_en)
      prev <= in;
    else
      prev <= prev;
  end
endmodule

module ContPrev #(
    parameter WIDTH = 32,
    parameter SAFE = 0
) (
  input wire clk,
  input wire reset,
  input wire logic [WIDTH-1:0] in,
  output logic [WIDTH-1:0] prev
);

Prev #(WIDTH, SAFE) r (
  .clk(clk),
  .reset(reset),
  .write_en(1'b1),
  .in(in),
  .prev(prev)
);
endmodule


`default_nettype wire