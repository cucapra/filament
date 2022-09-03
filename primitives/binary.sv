`default_nettype none

module Add (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [31:0] left,
  input wire logic [31:0] right,
  output wire logic [31:0] out
);
  assign out = left + right;
endmodule

module MultComb (
  input wire logic _go, // Unused port, only used for modeling
  input wire logic [31:0] left,
  input wire logic [31:0] right,
  output wire logic [31:0] out
);
  assign out = left * right;
endmodule
